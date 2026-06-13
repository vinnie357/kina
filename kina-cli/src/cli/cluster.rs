use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::config::{CniPlugin, Config};
use crate::core::cluster::ClusterManager;
use crate::core::kernel_fetch;
use crate::core::types::{ClusterInfo, CreateClusterOptions, LoadImageOptions, NodeRole};
use crate::core::verify::{
    aggregate_verify, http_layer_pass, parse_dns_domain, probe_host, probe_passed, probe_url,
    render_demo_manifest, ProbeResult,
};

/// Create a new Kubernetes cluster
#[derive(Args)]
pub struct CreateArgs {
    /// Name of the cluster
    #[arg(default_value = "kina")]
    pub name: String,

    /// Container image to use for the cluster
    #[arg(long, default_value = "kindest/node:v1.36.1")]
    pub image: String,

    /// Configuration file for cluster creation
    #[arg(long, value_name = "FILE")]
    pub config: Option<String>,

    /// Wait for cluster to be ready
    #[arg(long)]
    pub wait: Option<u64>,

    /// Retain cluster after failure
    #[arg(long)]
    pub retain: bool,

    /// Skip automatic kubelet CSR approval (may cause TLS errors)
    #[arg(long)]
    pub skip_csr_approval: bool,

    /// Number of worker nodes (0 = single-node cluster with combined roles)
    #[arg(long, default_value = "0")]
    pub workers: u32,

    /// CNI plugin to use (ptp or cilium)
    #[arg(long, value_enum, default_value = "ptp")]
    pub cni: CniPluginArg,

    /// Path to a custom Linux kernel for node containers.
    /// When set, kina passes `--kernel <PATH>` to every node `container run` invocation,
    /// booting nodes on the custom kernel without mutating the system kernel.
    /// When omitted, the system default (stock) kernel is used.
    /// Backed by node_kernel_path (resolved via select_kernel_path precedence).
    #[arg(long = "kernel-path", value_name = "PATH")]
    pub node_kernel_path: Option<PathBuf>,

    /// CPU count for all nodes (overrides per-role config defaults and the built-in default of 4).
    /// Applies to both control-plane and worker nodes.
    #[arg(long, value_name = "N")]
    pub cpus: Option<u32>,

    /// Memory for all nodes (overrides per-role config defaults and the built-in default of 4g).
    /// Format: <positive-int><m|g> (e.g. "512m", "2g", "8g").
    /// Applies to both control-plane and worker nodes.
    #[arg(long, value_name = "SIZE")]
    pub memory: Option<String>,
}

/// Delete a Kubernetes cluster
#[derive(Args)]
pub struct DeleteArgs {
    /// Name of the cluster to delete
    #[arg(default_value = "kina")]
    pub name: String,

    /// Delete all clusters
    #[arg(long, conflicts_with = "name")]
    pub all: bool,
}

/// List existing clusters
#[derive(Args)]
pub struct ListArgs {
    /// Show additional details
    #[arg(short, long)]
    pub verbose: bool,
}

/// Get information about clusters or resources
#[derive(Args)]
pub struct GetArgs {
    /// Resource type to get information about
    #[arg(value_enum)]
    pub resource: GetResource,

    /// Name of the specific resource (optional)
    pub name: Option<String>,
}

/// Load container images into clusters
#[derive(Args)]
pub struct LoadArgs {
    /// Container image to load
    pub image: String,

    /// Target cluster name
    #[arg(long, default_value = "kina")]
    pub cluster: String,
}

/// Install addons to cluster
#[derive(Args)]
pub struct InstallArgs {
    /// Type of addon to install
    #[arg(value_enum)]
    pub addon: AddonType,

    /// Name of the cluster
    #[arg(long, default_value = "kina")]
    pub cluster: String,

    /// Version of the addon (optional, uses latest if not specified)
    #[arg(long)]
    pub version: Option<String>,

    /// Use custom configuration file
    #[arg(long, value_name = "FILE")]
    pub config: Option<String>,
}

/// Export cluster configuration
#[derive(Args)]
pub struct ExportArgs {
    /// Cluster name to export
    #[arg(default_value = "kina")]
    pub name: String,

    /// Output format
    #[arg(long, value_enum, default_value = "kubeconfig")]
    pub format: ExportFormat,

    /// Output file path
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,
}

/// Show detailed status of a cluster
#[derive(Args)]
pub struct StatusArgs {
    /// Name of the cluster to check status
    pub name: Option<String>,

    /// Show additional details (pods, services, etc.)
    #[arg(short, long)]
    pub verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    pub output: StatusOutputFormat,
}

/// Approve pending kubelet Certificate Signing Requests
#[derive(Args)]
pub struct ApproveCSRArgs {
    /// Name of the cluster to approve CSRs for
    #[arg(default_value = "kina")]
    pub name: String,
}

#[derive(clap::ValueEnum, Clone)]
pub enum GetResource {
    /// List clusters
    Clusters,
    /// Get kubeconfig
    Kubeconfig,
    /// Get cluster nodes
    Nodes,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ExportFormat {
    /// Export kubeconfig format
    Kubeconfig,
    /// Export cluster configuration
    Config,
}

#[derive(clap::ValueEnum, Clone)]
pub enum StatusOutputFormat {
    /// Table format (default)
    Table,
    /// YAML format
    Yaml,
    /// JSON format
    Json,
}

/// Supported addon types
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum AddonType {
    /// NGINX Ingress Controller (nginx.org)
    #[value(name = "nginx-ingress")]
    NginxIngress,
    /// Demo application (kina-demo-app with ingress)
    #[value(name = "demo-app")]
    DemoApp,
    /// Kubernetes Metrics Server (enables `kubectl top` and HPA)
    #[value(name = "metrics-server")]
    MetricsServer,
}

/// Verify a cluster's health end-to-end
#[derive(Args)]
pub struct VerifyArgs {
    /// Name of the cluster to verify (optional — auto-detects if only one cluster exists)
    pub cluster: Option<String>,
}

impl CreateArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        info!("Creating cluster '{}'", self.name);

        let cluster_manager = ClusterManager::new(config)?;

        // Resolve the kernel path, applying the zero-step default for --cni cilium.
        //
        // Precedence:
        //   1. --kernel-path flag (explicit, always wins)
        //   2. Cached pinned kernel in ~/.kina/kernels/<tag>/vmlinux (silent reuse)
        //   3. Auto-download the pinned kernel (first-run notice printed)
        //   4. Hard error with --kernel-path escape hatch (offline / unreachable)
        //
        // PTP and other CNI plugins use the stock kernel; kernel_fetch is never invoked.
        let cni_plugin: CniPlugin = self.cni.clone().into();
        let node_kernel_path = if kernel_fetch::requires_kernel(&cni_plugin) {
            // Check whether the pinned kernel is already cached.
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let pinned_cache = kernel_fetch::kernel_cache_file(&home, &config.kernel.tag);
            let cached_verified = if pinned_cache.exists() {
                // Verify sha256 of the cached file before trusting it.
                let file_sha = std::fs::read(&pinned_cache).ok().map(|bytes| {
                    use sha2::{Digest, Sha256};
                    hex::encode(Sha256::digest(&bytes))
                });
                file_sha
                    .map(|sha| kernel_fetch::verify_sha256(&sha, &config.kernel.sha256))
                    .unwrap_or(false)
            } else {
                false
            };

            let cached_pinned = if cached_verified {
                Some(pinned_cache.clone())
            } else {
                None
            };

            match kernel_fetch::resolve_kernel_for_cilium(
                self.node_kernel_path.clone(),
                cached_pinned,
                true, // fetch_ok: assume online; download errors surface via install_kernel
            ) {
                Ok(kernel_fetch::KernelChoice::ExplicitPath(p)) => Some(p),
                Ok(kernel_fetch::KernelChoice::CachedPinned(p)) => {
                    info!("Using cached pinned kernel: {}", p.display());
                    Some(p)
                }
                Ok(kernel_fetch::KernelChoice::FetchPinned) => {
                    // First run: print notice and download the pinned kernel.
                    let notice = kernel_fetch::first_run_notice(
                        &config.kernel.tag,
                        kernel_fetch::KERNEL_SIZE_BYTES,
                    );
                    println!("{}", notice);

                    let cache_dir = kernel_fetch::kernel_cache_dir(&home, &config.kernel.tag);
                    // Use pinned_download_url() — the GitHub release asset is named "Image",
                    // not "vmlinux".  The downloaded bytes are stored locally as "vmlinux"
                    // by install_kernel via kernel_cache_file.
                    let url = kernel_fetch::pinned_download_url();

                    struct HttpFetcher;
                    impl kernel_fetch::KernelFetcher for HttpFetcher {
                        fn fetch(
                            &self,
                            url: &str,
                            dest_tmp: &std::path::Path,
                        ) -> Result<u64, String> {
                            // Synchronous download via reqwest blocking client.
                            let mut response = reqwest::blocking::get(url)
                                .map_err(|e| format!("HTTP request failed: {}", e))?;
                            if !response.status().is_success() {
                                return Err(format!(
                                    "HTTP {} downloading {}",
                                    response.status(),
                                    url
                                ));
                            }
                            let mut file = std::fs::File::create(dest_tmp)
                                .map_err(|e| format!("cannot create temp file: {}", e))?;
                            let bytes = std::io::copy(&mut response, &mut file)
                                .map_err(|e| format!("write error: {}", e))?;
                            Ok(bytes)
                        }
                    }

                    let fetcher = HttpFetcher;
                    match kernel_fetch::install_kernel(
                        &fetcher,
                        &url,
                        &config.kernel.sha256,
                        &cache_dir,
                    ) {
                        Ok(path) => {
                            info!("Kernel installed to: {}", path.display());
                            Some(path)
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(e));
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(e));
                }
            }
        } else {
            // PTP and other CNI plugins: use the explicit flag or config default (stock kernel).
            crate::core::apple_container::select_kernel_path(
                self.node_kernel_path.clone(),
                config.cluster.node_kernel_path.clone(),
            )
        };

        // Resolve per-role resource values: CLI flag > per-role config default > built-in default.
        // The --cpus/--memory flags apply to ALL nodes (both control-plane and worker).
        use crate::core::apple_container::{
            resolve_cpus, resolve_memory, validate_resources, DEFAULT_NODE_CPUS,
            DEFAULT_NODE_MEMORY,
        };

        let control_plane_cpus = resolve_cpus(
            self.cpus,
            config.cluster.control_plane_cpus,
            DEFAULT_NODE_CPUS,
        );
        let control_plane_memory = resolve_memory(
            self.memory.as_deref(),
            config.cluster.control_plane_memory.as_deref(),
            DEFAULT_NODE_MEMORY,
        );
        let worker_cpus = resolve_cpus(self.cpus, config.cluster.worker_cpus, DEFAULT_NODE_CPUS);
        let worker_memory = resolve_memory(
            self.memory.as_deref(),
            config.cluster.worker_memory.as_deref(),
            DEFAULT_NODE_MEMORY,
        );

        // Validate resolved resource values before constructing options or touching containers.
        validate_resources(control_plane_cpus, &control_plane_memory)
            .context("Invalid control-plane resource specification")?;
        validate_resources(worker_cpus, &worker_memory)
            .context("Invalid worker resource specification")?;

        let options = CreateClusterOptions {
            name: self.name.clone(),
            image: self.image.clone(),
            config_file: self.config.as_ref().map(PathBuf::from),
            kubernetes_version: None, // Use default
            workers: if self.workers > 0 {
                Some(self.workers)
            } else {
                None
            },
            control_plane_nodes: None, // Use default
            wait_timeout: self.wait,
            retain_on_failure: self.retain,
            skip_csr_approval: self.skip_csr_approval,
            cni_plugin,
            node_kernel_path,
            control_plane_cpus,
            control_plane_memory,
            worker_cpus,
            worker_memory,
        };

        cluster_manager.create_cluster(options).await?;

        println!("✅ Cluster '{}' created successfully", self.name);
        Ok(())
    }
}

impl DeleteArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        if self.all {
            info!("Deleting all clusters");
            cluster_manager.delete_all_clusters().await?;
            println!("✅ All clusters deleted successfully");
        } else {
            info!("Deleting cluster '{}'", self.name);
            cluster_manager.delete_cluster(&self.name).await?;
            println!("✅ Cluster '{}' deleted successfully", self.name);
        }

        Ok(())
    }
}

impl ListArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;
        let clusters = cluster_manager.list_clusters().await?;

        if clusters.is_empty() {
            println!("No clusters found");
            return Ok(());
        }

        if self.verbose {
            println!(
                "{:<15} {:<20} {:<10} {:<25}",
                "NAME", "IMAGE", "STATUS", "CREATED"
            );
            println!("{}", "-".repeat(70));
            for cluster in clusters {
                println!(
                    "{:<15} {:<20} {:<10} {:<25}",
                    cluster.name, cluster.image, cluster.status, cluster.created
                );
            }
        } else {
            for cluster in clusters {
                println!("{}", cluster.name);
            }
        }

        Ok(())
    }
}

impl GetArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        match self.resource {
            GetResource::Clusters => {
                let clusters = cluster_manager.list_clusters().await?;
                if clusters.is_empty() {
                    println!("No clusters found.");
                    println!();
                    println!("To create a new cluster, run:");
                    println!("  kina create [cluster-name]");
                    return Ok(());
                }

                if let Some(name) = &self.name {
                    // Looking for a specific cluster
                    if let Some(cluster) = clusters.iter().find(|c| c.name == *name) {
                        println!("{:#?}", cluster);
                    } else {
                        let cluster_names: Vec<&str> =
                            clusters.iter().map(|c| c.name.as_str()).collect();
                        println!("Cluster '{}' does not exist.", name);
                        println!();
                        println!("Available clusters: {}", cluster_names.join(", "));
                    }
                } else {
                    // List all clusters
                    for cluster in &clusters {
                        println!("{}", cluster.name);
                    }
                }
            }
            GetResource::Kubeconfig => {
                let cluster_name = self.name.as_deref().unwrap_or("kina");

                // Check if clusters exist and if the specific cluster exists
                let clusters = cluster_manager.list_clusters().await?;
                if clusters.is_empty() {
                    println!("No clusters found.");
                    println!();
                    println!("To create a new cluster, run:");
                    println!("  kina create [cluster-name]");
                    return Ok(());
                }

                let cluster_exists = clusters.iter().any(|c| c.name == cluster_name);
                if !cluster_exists {
                    let cluster_names: Vec<&str> =
                        clusters.iter().map(|c| c.name.as_str()).collect();
                    println!("Cluster '{}' does not exist.", cluster_name);
                    println!();
                    println!("Available clusters: {}", cluster_names.join(", "));
                    println!();
                    println!("To get kubeconfig for a specific cluster, run:");
                    println!("  kina get kubeconfig <cluster-name>");
                    return Ok(());
                }

                let kubeconfig = cluster_manager.get_kubeconfig(cluster_name).await?;
                println!("{}", kubeconfig);
            }
            GetResource::Nodes => {
                let cluster_name = self.name.as_deref().unwrap_or("kina");

                // Check if clusters exist and if the specific cluster exists
                let clusters = cluster_manager.list_clusters().await?;
                if clusters.is_empty() {
                    println!("No clusters found.");
                    println!();
                    println!("To create a new cluster, run:");
                    println!("  kina create [cluster-name]");
                    return Ok(());
                }

                let cluster_exists = clusters.iter().any(|c| c.name == cluster_name);
                if !cluster_exists {
                    let cluster_names: Vec<&str> =
                        clusters.iter().map(|c| c.name.as_str()).collect();
                    println!("Cluster '{}' does not exist.", cluster_name);
                    println!();
                    println!("Available clusters: {}", cluster_names.join(", "));
                    println!();
                    println!("To get nodes for a specific cluster, run:");
                    println!("  kina get nodes <cluster-name>");
                    return Ok(());
                }

                let nodes = cluster_manager.get_nodes(cluster_name).await?;
                for node in nodes {
                    println!("{}", node);
                }
            }
        }

        Ok(())
    }
}

impl LoadArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        // Check if clusters exist and if the specific cluster exists
        let clusters = cluster_manager.list_clusters().await?;
        if clusters.is_empty() {
            println!("No clusters found.");
            println!();
            println!("To create a new cluster, run:");
            println!("  kina create [cluster-name]");
            return Ok(());
        }

        let cluster_exists = clusters.iter().any(|c| c.name == self.cluster);
        if !cluster_exists {
            let cluster_names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
            println!("Cluster '{}' does not exist.", self.cluster);
            println!();
            println!("Available clusters: {}", cluster_names.join(", "));
            println!();
            println!("To load image into a specific cluster, run:");
            println!("  kina load {} <cluster-name>", self.image);
            return Ok(());
        }

        info!(
            "Loading image '{}' into cluster '{}'",
            self.image, self.cluster
        );

        let options = LoadImageOptions {
            image: self.image.clone(),
            cluster: self.cluster.clone(),
            archive: None, // Not using archive file
        };

        cluster_manager.load_image(options).await?;

        println!(
            "✅ Image '{}' loaded successfully into cluster '{}'",
            self.image, self.cluster
        );
        Ok(())
    }
}

impl InstallArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        // Check if clusters exist and if the specific cluster exists
        let clusters = cluster_manager.list_clusters().await?;
        if clusters.is_empty() {
            println!("No clusters found.");
            println!();
            println!("To create a new cluster, run:");
            println!("  kina create [cluster-name]");
            return Ok(());
        }

        let cluster_exists = clusters.iter().any(|c| c.name == self.cluster);
        if !cluster_exists {
            let cluster_names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
            println!("Cluster '{}' does not exist.", self.cluster);
            println!();
            println!("Available clusters: {}", cluster_names.join(", "));
            println!();
            println!("To install addon to a specific cluster, run:");
            println!("  kina install {:?} --cluster <cluster-name>", self.addon);
            return Ok(());
        }

        info!(
            "Installing {:?} addon to cluster '{}'",
            self.addon, self.cluster
        );

        match &self.addon {
            AddonType::NginxIngress => {
                self.install_nginx_ingress(&cluster_manager).await?;
            }
            AddonType::DemoApp => {
                self.install_demo_app(&cluster_manager).await?;
            }
            AddonType::MetricsServer => {
                self.install_metrics_server(&cluster_manager).await?;
            }
        }

        println!(
            "✅ {:?} addon installed successfully to cluster '{}'",
            self.addon, self.cluster
        );
        Ok(())
    }

    async fn install_nginx_ingress(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        info!("Installing NGINX Ingress Controller (nginx.org) with complete deployment");

        let kubeconfig_str = kubeconfig_for(&self.cluster)?;

        // Nginx-ingress manifests embedded in the binary — works from any directory (AC2).
        let manifests: &[(&str, &str)] = &[
            // 1. Common resources (namespace, RBAC, ServiceAccount)
            (
                include_str!("../../manifests/nginx-ingress/ns-and-sa.yaml"),
                "namespace and ServiceAccount",
            ),
            (
                include_str!("../../manifests/nginx-ingress/rbac.yaml"),
                "RBAC resources",
            ),
            // 2. CRDs (Custom Resource Definitions)
            (
                include_str!("../../manifests/nginx-ingress/crds.yaml"),
                "Custom Resource Definitions",
            ),
            // 3. ConfigMap with default configuration
            (
                include_str!("../../manifests/nginx-ingress/nginx-config.yaml"),
                "default configuration",
            ),
            // 4. IngressClass
            (
                include_str!("../../manifests/nginx-ingress/ingress-class.yaml"),
                "IngressClass",
            ),
            // 5. DaemonSet deployment (better for single-node clusters)
            (
                include_str!("../../manifests/nginx-ingress/nginx-ingress-daemonset.yaml"),
                "DaemonSet deployment",
            ),
        ];

        // Apply each manifest in order
        for (i, (manifest_content, description)) in manifests.iter().enumerate() {
            info!(
                "Applying manifest {}/{}: {}",
                i + 1,
                manifests.len(),
                description
            );

            apply_manifest_via_kubectl(&kubeconfig_str, manifest_content, description)?;

            // Small delay between manifest applications
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("NGINX Ingress Controller (DaemonSet) installed successfully");
        info!("Waiting for nginx-ingress pods to start...");

        // Wait a moment for pods to be created
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    async fn install_demo_app(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        info!("Installing demo application to cluster '{}'", self.cluster);

        let kubeconfig_str = kubeconfig_for(&self.cluster)?;

        // Detect DNS domain from `container system dns list`; fall back to "test".
        let dns_domain = {
            let dns_output = std::process::Command::new("container")
                .args(["system", "dns", "list"])
                .output();
            match dns_output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    parse_dns_domain(&stdout)
                }
                _ => "test".to_string(),
            }
        };

        // Demo-app manifest embedded in the binary (AC1).
        let raw_manifest = include_str!("../../manifests/demo-app.yaml");
        let rendered = render_demo_manifest(raw_manifest, &self.cluster, &dns_domain);

        info!(
            "Applying demo-app manifest (cluster={}, domain={})",
            self.cluster, dns_domain
        );

        apply_manifest_via_kubectl(&kubeconfig_str, &rendered, "demo-app manifest")?;

        info!("Demo app applied; waiting for pods to be Ready...");

        // Wait for pods to be Ready
        let wait = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "wait",
                "--for=condition=Ready",
                "pods",
                "-l",
                "app=kina-demo-app",
                "--timeout=120s",
            ])
            .output();

        match wait {
            Ok(out) if out.status.success() => {
                info!("Demo app pods are Ready");
            }
            Ok(out) => {
                warn!(
                    "Demo app pods not Ready within timeout: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => {
                warn!("Failed to wait for demo app pods: {}", e);
            }
        }

        Ok(())
    }

    async fn install_metrics_server(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        // metrics-server release bundled with this manifest.
        const VERSION: &str = "v0.8.1";
        info!("Installing metrics-server {}", VERSION);

        let kubeconfig_str = kubeconfig_for(&self.cluster)?;

        // metrics-server manifest embedded in the binary — works from any directory.
        // The bundled manifest sets --kubelet-insecure-tls so it works against
        // kina/kind-style nodes whose kubelet serving certs are self-signed.
        apply_manifest_via_kubectl(
            &kubeconfig_str,
            include_str!("../../manifests/metrics-server/components.yaml"),
            "metrics-server components",
        )?;

        info!("metrics-server {} installed successfully", VERSION);
        Ok(())
    }
}

/// Resolve the kubeconfig file path for an addon's target cluster.
///
/// Kubeconfigs live at `$HOME/.kube/<cluster>`. Returns the path as an owned
/// `String`, or an error if `HOME` is unset or the file is missing.
fn kubeconfig_for(cluster: &str) -> Result<String> {
    let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
    let path = std::path::Path::new(&home_dir).join(".kube").join(cluster);
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "Kubeconfig file not found: {}",
            path.display()
        ));
    }
    Ok(path.to_string_lossy().into_owned())
}

/// Apply a single manifest by piping it to `kubectl apply -f -`.
///
/// `description` is used only for logging and error context. Applies are
/// treated as idempotent: a non-zero exit whose stderr mentions `already
/// exists` or `Warning` is logged and tolerated rather than failing, so
/// re-running an install is safe.
fn apply_manifest_via_kubectl(kubeconfig: &str, manifest: &str, description: &str) -> Result<()> {
    let mut child = std::process::Command::new("kubectl")
        .args(["--kubeconfig", kubeconfig, "apply", "-f", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn kubectl for: {}", description))?;

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        use std::io::Write as _;
        stdin
            .write_all(manifest.as_bytes())
            .context("Failed to write manifest to kubectl stdin")?;
    }

    let output = child
        .wait_with_output()
        .context(format!("Failed to wait for kubectl: {}", description))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Some resources might already exist or have expected warnings.
        if stderr.contains("already exists") || stderr.contains("Warning") {
            info!("Applied {} (with warnings/already exists)", description);
        } else {
            return Err(anyhow::anyhow!(
                "Failed to apply {}: {}\nStdout: {}",
                description,
                stderr,
                stdout
            ));
        }
    } else {
        info!("Successfully applied {}", description);
    }

    Ok(())
}

/// Args and implementation for `kina verify [cluster]`.
impl VerifyArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;
        let clusters = cluster_manager.list_clusters().await?;

        let cluster_name = match &self.cluster {
            Some(name) => name.clone(),
            None => {
                if clusters.len() == 1 {
                    clusters[0].name.clone()
                } else if clusters.is_empty() {
                    return Err(anyhow::anyhow!("No clusters found"));
                } else {
                    let names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
                    return Err(anyhow::anyhow!(
                        "Multiple clusters found ({}); specify one: kina verify <cluster>",
                        names.join(", ")
                    ));
                }
            }
        };

        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir)
            .join(".kube")
            .join(&cluster_name);
        let kubeconfig_str = kubeconfig_path.to_string_lossy().to_string();

        println!("Verifying cluster '{}'...", cluster_name);
        let mut all_pass = true;

        // (a) Node count — all nodes Ready
        let nodes_output = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "get",
                "nodes",
                "--no-headers",
            ])
            .output();

        match nodes_output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                let total = lines.len();
                let ready = lines
                    .iter()
                    .filter(|l| l.split_whitespace().nth(1) == Some("Ready"))
                    .count();
                let pass = ready == total && total > 0;
                if !pass {
                    all_pass = false;
                }
                println!(
                    "{} Nodes: {}/{} Ready",
                    if pass { "PASS" } else { "FAIL" },
                    ready,
                    total
                );
            }
            _ => {
                all_pass = false;
                println!("FAIL Nodes: unable to query");
            }
        }

        // (b) Cilium DaemonSet ready n/n + operator healthy
        let cilium_ds = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "get",
                "pods",
                "-n",
                "kube-system",
                "-l",
                "k8s-app=cilium",
                "--no-headers",
            ])
            .output();

        match cilium_ds {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                let total = lines.len();
                let ready = lines
                    .iter()
                    .filter(|l| {
                        l.split_whitespace()
                            .nth(1)
                            .is_some_and(|s| s.starts_with("1/1"))
                    })
                    .count();
                let pass = ready == total && total > 0;
                if !pass {
                    all_pass = false;
                }
                println!(
                    "{} Cilium DaemonSet: {}/{} Ready",
                    if pass { "PASS" } else { "FAIL" },
                    ready,
                    total
                );
            }
            _ => {
                // Cilium not installed — not a failure if PTP is in use
                println!("INFO Cilium: not found (may be using PTP CNI)");
            }
        }

        // (c) HTTP probe each worker node
        let dns_domain = {
            let dns_output = std::process::Command::new("container")
                .args(["system", "dns", "list"])
                .output();
            match dns_output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    parse_dns_domain(&stdout)
                }
                _ => "test".to_string(),
            }
        };

        let host_header = probe_host(&cluster_name, &dns_domain);

        let cluster_info_result = cluster_manager.get_cluster_status(&cluster_name).await;
        let node_ips: Vec<String> = match &cluster_info_result {
            Ok(info) => info
                .nodes
                .iter()
                .filter(|n| n.role == NodeRole::Worker)
                .filter_map(|n| n.ip_address.clone())
                .filter(|ip| !ip.is_empty())
                .collect(),
            Err(_) => vec![],
        };

        if node_ips.is_empty() {
            all_pass = false;
            println!("FAIL HTTP probes: node IPs could not be determined");
        }

        let mut probe_results: Vec<ProbeResult> = vec![];

        for ip in &node_ips {
            let url = probe_url(ip);
            let curl_out = std::process::Command::new("curl")
                .args([
                    "-s",
                    "--max-time",
                    "5",
                    "-H",
                    &format!("Host: {}", host_header),
                    &url,
                ])
                .output();

            let passed = match curl_out {
                Ok(out) => {
                    let body = String::from_utf8_lossy(&out.stdout);
                    probe_passed(&body)
                }
                Err(_) => false,
            };

            println!(
                "{} HTTP probe {} (Host: {}): {}",
                if passed { "PASS" } else { "FAIL" },
                url,
                host_header,
                if passed {
                    "200 + demo marker"
                } else {
                    "no demo marker"
                }
            );

            probe_results.push(ProbeResult {
                node: ip.clone(),
                passed,
            });
        }

        // http_layer_pass: returns false for empty node_ips (no-evidence FAIL)
        // and delegates to aggregate_verify for the probe results when IPs are present.
        let probes_pass = aggregate_verify(&probe_results);
        let http_pass = http_layer_pass(&node_ips, &probe_results);
        if !http_pass {
            all_pass = false;
            if !probes_pass {
                for r in &probe_results {
                    if !r.passed {
                        println!("  node {} did not return demo marker", r.node);
                    }
                }
            }
        }

        println!();
        if all_pass {
            println!("PASS verify '{}'", cluster_name);
            Ok(())
        } else {
            println!("FAIL verify '{}'", cluster_name);
            std::process::exit(1);
        }
    }
}

impl ExportArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        // Check if clusters exist and if the specific cluster exists
        let clusters = cluster_manager.list_clusters().await?;
        if clusters.is_empty() {
            println!("No clusters found.");
            println!();
            println!("To create a new cluster, run:");
            println!("  kina create [cluster-name]");
            return Ok(());
        }

        let cluster_exists = clusters.iter().any(|c| c.name == self.name);
        if !cluster_exists {
            let cluster_names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
            println!("Cluster '{}' does not exist.", self.name);
            println!();
            println!("Available clusters: {}", cluster_names.join(", "));
            println!();
            println!("To export a specific cluster, run:");
            println!("  kina export <cluster-name>");
            return Ok(());
        }

        let content = match self.format {
            ExportFormat::Kubeconfig => cluster_manager.get_kubeconfig(&self.name).await?,
            ExportFormat::Config => {
                warn!("Config export format not yet implemented");
                return Ok(());
            }
        };

        if let Some(output_file) = &self.output {
            std::fs::write(output_file, &content)?;
            println!("✅ Exported to '{}'", output_file);
        } else {
            println!("{}", content);
        }

        Ok(())
    }
}

impl StatusArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;
        let container_version = cluster_manager.container_version().to_string();

        // Handle the case where a specific cluster name is provided
        if let Some(cluster_name) = &self.name {
            // Get detailed cluster status for the specified cluster
            let cluster_info = cluster_manager.get_cluster_status(cluster_name).await?;

            match self.output {
                StatusOutputFormat::Table => {
                    self.print_table_format(&cluster_info, config, &container_version)
                        .await?
                }
                StatusOutputFormat::Yaml => {
                    let mut map = serde_json::to_value(&cluster_info)?;
                    if let Some(obj) = map.as_object_mut() {
                        obj.insert(
                            "apple_container_version".to_string(),
                            serde_json::Value::String(container_version.clone()),
                        );
                    }
                    println!("{}", serde_yaml::to_string(&map)?);
                }
                StatusOutputFormat::Json => {
                    let mut map = serde_json::to_value(&cluster_info)?;
                    if let Some(obj) = map.as_object_mut() {
                        obj.insert(
                            "apple_container_version".to_string(),
                            serde_json::Value::String(container_version.clone()),
                        );
                    }
                    println!("{}", serde_json::to_string_pretty(&map)?);
                }
            }

            return Ok(());
        }

        // Handle the case where no specific cluster name is provided
        let clusters = cluster_manager.list_clusters().await?;

        if clusters.is_empty() {
            // Gracefully handle no clusters case
            println!("No clusters found.");
            println!();
            println!("To create a new cluster, run:");
            println!("  kina create [cluster-name]");
            println!();
            println!("For more help, run:");
            println!("  kina create --help");
            return Ok(());
        }

        // Determine which cluster to show status for when multiple exist
        let cluster_name = match clusters.len() {
            1 => {
                // Auto-select the only cluster
                let cluster_name = &clusters[0].name;
                println!("Using cluster: {}", cluster_name);
                cluster_name.clone()
            }
            _ => {
                // Multiple clusters available
                if self.is_interactive() {
                    self.select_cluster_interactively(&clusters).await?
                } else {
                    // Non-interactive mode with multiple clusters
                    let cluster_names: Vec<&str> =
                        clusters.iter().map(|c| c.name.as_str()).collect();
                    return Err(anyhow::anyhow!(
                        "Multiple clusters found: {}. Please specify one with: kina status <cluster-name>",
                        cluster_names.join(", ")
                    ));
                }
            }
        };

        // Get detailed cluster status
        let cluster_info = cluster_manager.get_cluster_status(&cluster_name).await?;

        match self.output {
            StatusOutputFormat::Table => {
                self.print_table_format(&cluster_info, config, &container_version)
                    .await?
            }
            StatusOutputFormat::Yaml => {
                let mut map = serde_json::to_value(&cluster_info)?;
                if let Some(obj) = map.as_object_mut() {
                    obj.insert(
                        "apple_container_version".to_string(),
                        serde_json::Value::String(container_version.clone()),
                    );
                }
                println!("{}", serde_yaml::to_string(&map)?);
            }
            StatusOutputFormat::Json => {
                let mut map = serde_json::to_value(&cluster_info)?;
                if let Some(obj) = map.as_object_mut() {
                    obj.insert(
                        "apple_container_version".to_string(),
                        serde_json::Value::String(container_version.clone()),
                    );
                }
                println!("{}", serde_json::to_string_pretty(&map)?);
            }
        }

        Ok(())
    }

    /// Check if we're running in an interactive terminal
    fn is_interactive(&self) -> bool {
        // Check if stdin is a terminal (TTY)
        use std::os::unix::io::AsRawFd;
        unsafe { libc::isatty(std::io::stdin().as_raw_fd()) == 1 }
    }

    /// Interactive cluster selection
    async fn select_cluster_interactively(&self, clusters: &[ClusterInfo]) -> Result<String> {
        println!("Multiple clusters found. Please select one:");
        for (i, cluster) in clusters.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, cluster.name, cluster.status);
        }

        loop {
            print!("Enter cluster number (1-{}): ", clusters.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if let Ok(choice) = input.parse::<usize>() {
                if choice >= 1 && choice <= clusters.len() {
                    return Ok(clusters[choice - 1].name.clone());
                }
            }
            println!(
                "Invalid selection. Please enter a number between 1 and {}.",
                clusters.len()
            );
        }
    }

    async fn print_table_format(
        &self,
        cluster_info: &ClusterInfo,
        config: &Config,
        container_version: &str,
    ) -> Result<()> {
        println!("Cluster: {}", cluster_info.name);
        println!("Status: {}", cluster_info.status);
        println!("Image: {}", cluster_info.image);
        println!("Apple Container: {}", container_version);
        println!("Created: {}", cluster_info.created);

        if let Some(kubeconfig) = &cluster_info.kubeconfig_path {
            println!("Kubeconfig: {}", kubeconfig);
        }

        // Print nodes information
        if !cluster_info.nodes.is_empty() {
            println!("\nNodes:");
            println!(
                "{:<25} {:<15} {:<10} {:<15} {:<15}",
                "NAME", "STATUS", "ROLES", "VERSION", "IP"
            );
            println!("{}", "-".repeat(75));

            for node in &cluster_info.nodes {
                println!(
                    "{:<25} {:<15} {:<10} {:<15} {:<15}",
                    node.name,
                    node.status,
                    node.role,
                    node.version,
                    node.ip_address.as_deref().unwrap_or("N/A")
                );
            }
        }

        // Check cluster readiness status
        if let Err(e) = self.print_cluster_readiness(&cluster_info.name).await {
            warn!("Failed to check cluster readiness: {}", e);
        }

        // If verbose, show additional Kubernetes resources
        if self.verbose {
            if let Err(e) = self.print_verbose_details(&cluster_info.name, config).await {
                warn!("Failed to get verbose details: {}", e);
            }
        }

        Ok(())
    }

    async fn print_verbose_details(&self, cluster_name: &str, _config: &Config) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir)
            .join(".kube")
            .join(cluster_name);

        if !kubeconfig_path.exists() {
            return Ok(());
        }

        let kubeconfig_str = kubeconfig_path.to_string_lossy();

        println!("\nNamespaces:");
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "get",
                "namespaces",
                "--no-headers",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines().take(10) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        println!("  {} ({})", parts[0], parts[2]);
                    }
                }
            }
        }

        println!("\nPods (all namespaces):");
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "get",
                "pods",
                "-A",
                "--no-headers",
                "--field-selector=status.phase!=Succeeded",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();

                if lines.is_empty() {
                    println!("  No pods found");
                } else {
                    println!(
                        "  {:<20} {:<30} {:<10} {:<10}",
                        "NAMESPACE", "NAME", "READY", "STATUS"
                    );
                    println!("  {}", "-".repeat(70));

                    for line in lines.iter().take(15) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            println!(
                                "  {:<20} {:<30} {:<10} {:<10}",
                                parts[0], // namespace
                                parts[1], // name
                                parts[2], // ready
                                parts[3]  // status
                            );
                        }
                    }

                    if lines.len() > 15 {
                        println!("  ... and {} more pods", lines.len() - 15);
                    }
                }
            }
        }

        println!("\nServices:");
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                &kubeconfig_str,
                "get",
                "services",
                "-A",
                "--no-headers",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();

                if lines.is_empty() {
                    println!("  No services found");
                } else {
                    println!(
                        "  {:<20} {:<25} {:<15} {:<15}",
                        "NAMESPACE", "NAME", "TYPE", "CLUSTER-IP"
                    );
                    println!("  {}", "-".repeat(75));

                    for line in lines.iter().take(10) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            println!(
                                "  {:<20} {:<25} {:<15} {:<15}",
                                parts[0], // namespace
                                parts[1], // name
                                parts[2], // type
                                parts[3]  // cluster-ip
                            );
                        }
                    }

                    if lines.len() > 10 {
                        println!("  ... and {} more services", lines.len() - 10);
                    }
                }
            }
        }

        Ok(())
    }

    async fn print_cluster_readiness(&self, cluster_name: &str) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir)
            .join(".kube")
            .join(cluster_name);

        if !kubeconfig_path.exists() {
            return Ok(());
        }

        let kubeconfig_str = kubeconfig_path.to_string_lossy();

        println!("\n🔍 Cluster Readiness Status:");
        println!("{}", "-".repeat(50));

        // Check node readiness
        let nodes_ready = self.check_nodes_ready(&kubeconfig_str).await?;

        // Check core system pods
        let core_pods_ready = self.check_core_pods_ready(&kubeconfig_str).await?;

        // Check CNI status
        let cni_ready = self.check_cni_ready(&kubeconfig_str).await?;

        // Check ingress controller
        let ingress_ready = self.check_ingress_ready(&kubeconfig_str).await?;

        // Overall readiness assessment
        let overall_ready = nodes_ready && core_pods_ready && cni_ready && ingress_ready;

        println!(
            "\n📊 Overall Status: {}",
            if overall_ready {
                "✅ READY - Cluster is ready for applications"
            } else {
                "⚠️  NOT READY - Cluster is still initializing"
            }
        );

        if !overall_ready {
            println!("\n💡 Wait for all components to be ready before deploying applications.");
            println!(
                "   Run 'kina status {}' again to check progress.",
                cluster_name
            );
        }

        Ok(())
    }

    async fn check_nodes_ready(&self, kubeconfig_str: &str) -> Result<bool> {
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                kubeconfig_str,
                "get",
                "nodes",
                "--no-headers",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut ready_nodes = 0;
                let mut total_nodes = 0;

                for line in stdout.lines() {
                    total_nodes += 1;
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1] == "Ready" {
                        ready_nodes += 1;
                    }
                }

                let all_ready = ready_nodes == total_nodes && total_nodes > 0;
                println!(
                    "🖥️  Nodes: {}/{} Ready {}",
                    ready_nodes,
                    total_nodes,
                    if all_ready { "✅" } else { "❌" }
                );
                return Ok(all_ready);
            }
        }

        println!("🖥️  Nodes: Unknown ❌");
        Ok(false)
    }

    async fn check_core_pods_ready(&self, kubeconfig_str: &str) -> Result<bool> {
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                kubeconfig_str,
                "get",
                "pods",
                "-n",
                "kube-system",
                "--no-headers",
                "--field-selector=status.phase!=Succeeded",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut ready_pods = 0;
                let mut total_pods = 0;

                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        total_pods += 1;
                        let ready_status = parts[1]; // e.g., "1/1", "0/1"
                        if let Some((ready, total)) = ready_status.split_once('/') {
                            if ready == total && ready != "0" {
                                ready_pods += 1;
                            }
                        }
                    }
                }

                let all_ready = ready_pods == total_pods && total_pods > 0;
                println!(
                    "🔧 Core Pods: {}/{} Ready {}",
                    ready_pods,
                    total_pods,
                    if all_ready { "✅" } else { "❌" }
                );
                return Ok(all_ready);
            }
        }

        println!("🔧 Core Pods: Unknown ❌");
        Ok(false)
    }

    async fn check_cni_ready(&self, kubeconfig_str: &str) -> Result<bool> {
        // Check for Cilium pods
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                kubeconfig_str,
                "get",
                "pods",
                "-n",
                "kube-system",
                "-l",
                "k8s-app=cilium",
                "--no-headers",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();

                if lines.is_empty() {
                    println!("🌐 CNI (Cilium): Not found ❌");
                    return Ok(false);
                }

                let mut ready_cilium = 0;
                let total_cilium = lines.len();

                for line in lines {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let ready_status = parts[1];
                        if ready_status.starts_with("1/1") {
                            ready_cilium += 1;
                        }
                    }
                }

                let cilium_ready = ready_cilium == total_cilium;
                println!(
                    "🌐 CNI (Cilium): {}/{} Ready {}",
                    ready_cilium,
                    total_cilium,
                    if cilium_ready { "✅" } else { "❌" }
                );
                return Ok(cilium_ready);
            }
        }

        println!("🌐 CNI (Cilium): Unknown ❌");
        Ok(false)
    }

    async fn check_ingress_ready(&self, kubeconfig_str: &str) -> Result<bool> {
        // Check for nginx-ingress pods
        if let Ok(output) = std::process::Command::new("kubectl")
            .args([
                "--kubeconfig",
                kubeconfig_str,
                "get",
                "pods",
                "-n",
                "nginx-ingress",
                "--no-headers",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();

                if lines.is_empty() {
                    println!("🌍 Ingress Controller: Not found ❌");
                    return Ok(false);
                }

                let mut ready_ingress = 0;
                let total_ingress = lines.len();

                for line in lines {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let ready_status = parts[1];
                        if ready_status.starts_with("1/1") {
                            ready_ingress += 1;
                        }
                    }
                }

                let ingress_ready = ready_ingress == total_ingress;
                println!(
                    "🌍 Ingress Controller: {}/{} Ready {}",
                    ready_ingress,
                    total_ingress,
                    if ingress_ready { "✅" } else { "❌" }
                );
                return Ok(ingress_ready);
            }
        }

        println!("🌍 Ingress Controller: Unknown ❌");
        Ok(false)
    }
}

impl ApproveCSRArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        let cluster_manager = ClusterManager::new(config)?;

        // Check if the specified cluster exists
        let clusters = cluster_manager.list_clusters().await?;
        let cluster_exists = clusters.iter().any(|c| c.name == self.name);

        if !cluster_exists {
            if clusters.is_empty() {
                println!("No clusters found.");
                println!();
                println!("To create a new cluster, run:");
                println!("  kina create [cluster-name]");
                println!();
                println!("For more help, run:");
                println!("  kina create --help");
                return Ok(());
            } else {
                let cluster_names: Vec<&str> = clusters.iter().map(|c| c.name.as_str()).collect();
                println!("Cluster '{}' does not exist.", self.name);
                println!();
                println!("Available clusters: {}", cluster_names.join(", "));
                println!();
                println!("To approve CSRs for a specific cluster, run:");
                println!("  kina approve-csr <cluster-name>");
                return Ok(());
            }
        }

        info!("Approving pending kubelet CSRs for cluster '{}'", self.name);
        cluster_manager.approve_kubelet_csrs(&self.name).await?;

        println!("✅ Kubelet CSRs approved for cluster '{}'", self.name);
        println!("💡 This should fix TLS errors with kubectl logs/exec commands");
        Ok(())
    }
}

/// CNI plugin options for command line
#[derive(Clone, Debug, ValueEnum)]
pub enum CniPluginArg {
    /// PTP CNI with host-local IPAM (default, Apple Container compatible)
    Ptp,
    /// Cilium CNI (advanced features, requires compatible kernel)
    Cilium,
}

impl From<CniPluginArg> for CniPlugin {
    fn from(arg: CniPluginArg) -> Self {
        match arg {
            CniPluginArg::Ptp => CniPlugin::Ptp,
            CniPluginArg::Cilium => CniPlugin::Cilium,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubeconfig_for_nonexistent_cluster() {
        let result = kubeconfig_for("__kina_test_nonexistent_cluster_abc123__");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not found") || msg.contains("HOME"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn test_kubeconfig_for_existing_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let kube_dir = tmp.path().join(".kube");
        std::fs::create_dir_all(&kube_dir).unwrap();
        std::fs::write(kube_dir.join("test-cluster"), b"kubeconfig").unwrap();

        let old_home = std::env::var("HOME").ok();
        // Safety: this test must not run concurrently with other tests that read HOME.
        unsafe { std::env::set_var("HOME", tmp.path()) };

        let result = kubeconfig_for("test-cluster");

        match old_home {
            Some(h) => unsafe { std::env::set_var("HOME", h) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("test-cluster"));
    }
}
