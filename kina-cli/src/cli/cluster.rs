use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::config::{CniPlugin, Config};
use crate::core::cluster::ClusterManager;
use crate::core::types::{ClusterInfo, CreateClusterOptions, LoadImageOptions};

/// Create a new Kubernetes cluster
#[derive(Args)]
pub struct CreateArgs {
    /// Name of the cluster
    #[arg(default_value = "kina")]
    pub name: String,

    /// Container image to use for the cluster
    #[arg(long, default_value = "kindest/node:v1.31.0")]
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

    /// CNI plugin to use (ptp or cilium)
    #[arg(long, value_enum, default_value = "ptp")]
    pub cni: CniPluginArg,
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
    /// Ingress NGINX Controller (kubernetes community)
    #[value(name = "ingress-nginx")]
    IngressNginx,
    /// CNI plugins
    Cni,
    /// CoreDNS
    Coredns,
    /// Metrics server
    #[value(name = "metrics-server")]
    MetricsServer,
}

impl CreateArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        info!("Creating cluster '{}'", self.name);

        let cluster_manager = ClusterManager::new(config)?;

        let options = CreateClusterOptions {
            name: self.name.clone(),
            image: self.image.clone(),
            config_file: self.config.as_ref().map(PathBuf::from),
            kubernetes_version: None,  // Use default
            workers: None,             // Use default
            control_plane_nodes: None, // Use default
            wait_timeout: self.wait,
            retain_on_failure: self.retain,
            skip_csr_approval: self.skip_csr_approval,
            cni_plugin: self.cni.clone().into(), // Convert CLI arg to config enum
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
            AddonType::IngressNginx => {
                self.install_ingress_nginx(&cluster_manager).await?;
            }
            AddonType::Cni => {
                self.install_cni(&cluster_manager).await?;
            }
            AddonType::Coredns => {
                self.install_coredns(&cluster_manager).await?;
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

        // Use the kubeconfig file path directly instead of content
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir)
            .join(".kube")
            .join(&self.cluster);

        if !kubeconfig_path.exists() {
            return Err(anyhow::anyhow!(
                "Kubeconfig file not found: {}",
                kubeconfig_path.display()
            ));
        }

        let kubeconfig_str = kubeconfig_path.to_string_lossy();

        // Get the current directory to find manifest files
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let manifest_dir = current_dir
            .join("kina-cli")
            .join("manifests")
            .join("nginx-ingress");

        if !manifest_dir.exists() {
            return Err(anyhow::anyhow!(
                "Nginx-ingress manifest directory not found: {}",
                manifest_dir.display()
            ));
        }

        // Define all required manifests in deployment order using local files
        let manifest_files = [
            // 1. Common resources (namespace, RBAC, ServiceAccount)
            ("ns-and-sa.yaml", "namespace and ServiceAccount"),
            ("rbac.yaml", "RBAC resources"),
            // 2. CRDs (Custom Resource Definitions)
            ("crds.yaml", "Custom Resource Definitions"),
            // 3. ConfigMap with default configuration
            ("nginx-config.yaml", "default configuration"),
            // 4. IngressClass
            ("ingress-class.yaml", "IngressClass"),
            // 5. DaemonSet deployment (better for single-node clusters)
            ("nginx-ingress-daemonset.yaml", "DaemonSet deployment"),
        ];

        // Apply each manifest in order
        for (i, (manifest_file, description)) in manifest_files.iter().enumerate() {
            let manifest_path = manifest_dir.join(manifest_file);

            if !manifest_path.exists() {
                warn!(
                    "Manifest file not found: {}, skipping",
                    manifest_path.display()
                );
                continue;
            }

            info!(
                "Applying manifest {}/{}: {} ({})",
                i + 1,
                manifest_files.len(),
                manifest_file,
                description
            );

            let output = std::process::Command::new("kubectl")
                .args([
                    "--kubeconfig",
                    &kubeconfig_str,
                    "apply",
                    "-f",
                    &manifest_path.to_string_lossy(),
                ])
                .output()
                .context(format!("Failed to apply manifest: {}", manifest_file))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Some resources might already exist or have expected warnings
                if stderr.contains("already exists") || stderr.contains("Warning") {
                    info!("Applied {} (with warnings/already exists)", manifest_file);
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to apply {}: {}\nStdout: {}",
                        manifest_file,
                        stderr,
                        stdout
                    ));
                }
            } else {
                info!("Successfully applied {}", manifest_file);
            }

            // Small delay between manifest applications
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("NGINX Ingress Controller (DaemonSet) installed successfully");
        info!("Waiting for nginx-ingress pods to start...");

        // Wait a moment for pods to be created
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    async fn install_ingress_nginx(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        Err(anyhow::anyhow!(
            "ingress-nginx installation not yet implemented. Use nginx-ingress instead."
        ))
    }

    async fn install_cni(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        Err(anyhow::anyhow!("CNI installation not yet implemented"))
    }

    async fn install_coredns(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        Err(anyhow::anyhow!("CoreDNS installation not yet implemented"))
    }

    async fn install_metrics_server(&self, _cluster_manager: &ClusterManager) -> Result<()> {
        Err(anyhow::anyhow!(
            "Metrics server installation not yet implemented"
        ))
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
