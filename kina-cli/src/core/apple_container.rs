use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use tracing::{debug, info, warn};

use super::types::{
    ClusterInfo, ClusterStatus, CreateClusterOptions, LoadImageOptions, NodeInfo, NodeRole,
};
use crate::config::Config;

/// Client for interacting with Apple Container
pub struct AppleContainerClient {
    config: Config,
    cli_path: String,
}

impl AppleContainerClient {
    /// Create a new Apple Container client
    pub fn new(config: &Config) -> Result<Self> {
        let cli_path = if let Some(path) = &config.apple_container.cli_path {
            path.to_string_lossy().to_string()
        } else {
            // Try to detect Apple Container CLI in PATH
            Self::detect_cli_path()?
        };

        Ok(Self {
            config: config.clone(),
            cli_path,
        })
    }

    /// Detect Apple Container CLI path
    fn detect_cli_path() -> Result<String> {
        // Check specific paths first to avoid conflicts with system commands
        let specific_paths = [
            "/usr/local/bin/container",
            "/opt/homebrew/bin/container",
            "/usr/local/bin/apple-container",
            "/opt/homebrew/bin/apple-container",
            "/System/Library/PrivateFrameworks/ContainerManager.framework/Versions/A/Resources/apple-container",
        ];

        for path in &specific_paths {
            if std::path::Path::new(path).exists() {
                info!("Found Apple Container CLI at: {}", path);
                return Ok(path.to_string());
            }
        }

        // Only search PATH for container names, avoiding system commands like 'ac'
        let possible_names = ["container", "apple-container"];

        for name in &possible_names {
            if let Ok(output) = std::process::Command::new("which").arg(name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        info!("Found Apple Container CLI at: {}", path);
                        return Ok(path);
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Apple Container CLI not found. Please install Apple Container or specify the path in configuration."
        ))
    }

    /// Create a cluster using Apple Container
    pub async fn create_cluster(&self, options: &CreateClusterOptions) -> Result<()> {
        info!(
            "Creating cluster '{}' with image '{}'",
            options.name, options.image
        );

        // Note: Apple Container VMs cannot communicate with each other until macOS 26
        // Creating single-node cluster with combined control-plane/worker roles

        let node_name = format!("{}-control-plane", options.name);
        info!(
            "Creating single-node cluster with combined roles: {}",
            node_name
        );

        self.create_single_node(&options.name, &node_name, &options.image)
            .await?;

        info!("Cluster '{}' created successfully", options.name);
        Ok(())
    }

    /// Create a single node with combined control-plane and worker roles
    /// Note: Required due to Apple Container VM communication limitation until macOS 26
    async fn create_single_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
    ) -> Result<()> {
        info!("Creating single Kubernetes node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Create container with appropriate labels for single-node cluster
        let mut cmd = std::process::Command::new(&self.cli_path);

        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=control-plane,worker", // Combined roles
            "--label",
            "io.kina.primary=true",
            "--label",
            "io.kina.single-node=true",
            "--label",
            &image_label,
        ];

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Note: No port mapping needed - Apple Container VM gets its own IP
        // Kubernetes API server will be accessible at <vm-ip>:6443
        // Ingress controllers will be accessible at <vm-ip>:80, <vm-ip>:443
        // Services can be reached directly at VM IP address

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            "--env",
            "KINA_NODE_TYPE=single-node",
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        // Debug: Print the exact command being executed
        debug!(
            "Executing Apple Container command: {} {:?}",
            self.cli_path, args
        );

        let output = cmd
            .output()
            .context("Failed to create single-node cluster")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create single-node cluster '{}': {}",
                node_name,
                stderr
            ));
        }

        info!("Container '{}' created, waiting for startup...", node_name);

        // Wait for container to be fully running
        self.wait_for_container_ready(node_name).await?;

        // Get the VM IP address
        let vm_ip = self.get_container_ip(node_name).await?;
        info!("Container '{}' running at IP: {}", node_name, vm_ip);

        // Initialize Kubernetes cluster
        self.initialize_kubernetes_cluster(node_name, &vm_ip)
            .await?;

        // Generate and save kubeconfig immediately after cluster init
        // This ensures user has kubectl access even if CNI installation fails
        self.setup_kubeconfig(cluster_name, node_name, &vm_ip)
            .await?;

        // Remove control-plane taint for single-node scheduling
        self.remove_control_plane_taint(node_name).await?;

        // Install CNI plugin (now user has kubectl access if this fails)
        // Use default CNI from config for now - will be configurable in future updates
        self.install_cni_plugin(node_name).await?;

        info!(
            "Kubernetes cluster '{}' initialized successfully",
            cluster_name
        );
        Ok(())
    }

    /// Create a control plane node
    #[allow(dead_code)]
    async fn create_control_plane_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        is_primary: bool,
    ) -> Result<()> {
        info!("Creating control plane node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Create container with appropriate labels and configuration
        // Apple Container automatically assigns VM and IP address - no explicit network needed
        let mut cmd = std::process::Command::new(&self.cli_path);

        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=control-plane",
            "--label",
            &image_label,
        ];

        if is_primary {
            args.extend_from_slice(&["--label", "io.kina.primary=true"]);
        }

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        let output = cmd
            .output()
            .context("Failed to create control plane node")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create control plane node '{}': {}",
                node_name,
                stderr
            ));
        }

        debug!("Created control plane node '{}'", node_name);
        Ok(())
    }

    /// Create a worker node
    #[allow(dead_code)]
    async fn create_worker_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
    ) -> Result<()> {
        info!("Creating worker node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Apple Container automatically assigns VM and IP address - no explicit network needed
        let mut cmd = std::process::Command::new(&self.cli_path);
        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=worker",
            "--label",
            &image_label,
        ];

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        let output = cmd.output().context("Failed to create worker node")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create worker node '{}': {}",
                node_name,
                stderr
            ));
        }

        debug!("Created worker node '{}'", node_name);
        Ok(())
    }

    /// Delete a cluster
    pub async fn delete_cluster(&self, name: &str) -> Result<()> {
        info!("Deleting cluster '{}'", name);

        // Find all containers belonging to this cluster
        let clusters = self.list_clusters().await?;
        let cluster = clusters.iter().find(|c| c.name == name);

        if let Some(cluster) = cluster {
            // Delete all containers in the cluster
            for node in &cluster.nodes {
                if let Some(container_id) = &node.container_id {
                    self.delete_container(container_id).await?;
                }
            }

            // Remove kubeconfig context
            self.remove_kubeconfig_context(name).await?;

            // Note: No explicit network cleanup needed for Apple Container
            // VM-per-container architecture handles networking automatically
        } else {
            warn!("Cluster '{}' not found", name);
        }

        info!("Cluster '{}' deleted successfully", name);
        Ok(())
    }

    /// Delete a container
    async fn delete_container(&self, container_id: &str) -> Result<()> {
        debug!("Deleting container '{}'", container_id);

        // Try graceful stop, then force stop with SIGKILL
        let stop_attempts = [
            ("stop", None, None),                  // Graceful stop (SIGTERM, 5s default)
            ("stop", Some("SIGKILL"), Some("10")), // Force stop with SIGKILL and 10s timeout
        ];

        for (attempt, (cmd_name, signal, timeout)) in stop_attempts.iter().enumerate() {
            let signal_desc = signal.map(|s| format!(" with {}", s)).unwrap_or_default();
            info!(
                "Attempting to {} container '{}'{} (attempt {})",
                cmd_name,
                container_id,
                signal_desc,
                attempt + 1
            );

            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.arg(cmd_name);

            if let Some(sig) = signal {
                cmd.args(["--signal", sig]);
            }
            if let Some(time) = timeout {
                cmd.args(["--time", time]);
            }

            cmd.arg(container_id);

            let output = cmd
                .output()
                .context("Failed to execute container command")?;

            if output.status.success() {
                let stop_desc = if let Some(sig) = signal {
                    format!("{} ({})", cmd_name, sig)
                } else {
                    cmd_name.to_string()
                };
                info!("Successfully {} container '{}'", stop_desc, container_id);

                // Wait for container to fully stop - increased timeout for Apple Container VMs
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                break;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("is not running")
                    || stderr.contains("No such container")
                    || stderr.contains("cannot kill: container is not running")
                {
                    debug!(
                        "Container '{}' already stopped or doesn't exist",
                        container_id
                    );
                    break;
                } else if attempt == 0 {
                    warn!(
                        "Graceful stop failed for '{}': {}, trying force stop",
                        container_id, stderr
                    );
                    continue;
                } else {
                    warn!(
                        "All stop attempts failed for '{}': {}, proceeding with removal anyway",
                        container_id, stderr
                    );
                    break; // Try removal anyway - Apple Container might have state sync issues
                }
            }
        }

        // Remove the container - try normal removal first, then force if needed
        for (attempt, use_force) in [false, true].iter().enumerate() {
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.arg("delete"); // Use 'delete' not 'rm'

            if *use_force {
                cmd.arg("--force");
                info!("Attempting force removal of container '{}'", container_id);
            } else {
                info!("Attempting removal of container '{}'", container_id);
            }

            cmd.arg(container_id);

            let output = cmd.output().context("Failed to remove container")?;

            if output.status.success() {
                info!("Successfully removed container '{}'", container_id);
                break;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("No such container") {
                    debug!("Container '{}' already removed", container_id);
                    break;
                } else if attempt == 0 && stderr.contains("container is running") {
                    warn!("Normal removal failed (container still running), trying force removal");
                    continue;
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to remove container '{}': {}",
                        container_id,
                        stderr
                    ));
                }
            }
        }

        info!("Successfully deleted container '{}'", container_id);
        Ok(())
    }

    /// List clusters
    pub async fn list_clusters(&self) -> Result<Vec<ClusterInfo>> {
        debug!("Listing clusters from Apple Container");

        // Run 'container list' to list all containers and filter by kina labels
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["list", "--format", "json", "--all"]);

        let output = cmd
            .output()
            .context("Failed to execute Apple Container CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Apple Container CLI failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Apple Container ps output: {}", stdout);

        // Parse JSON output to find kina clusters
        let containers: Vec<serde_json::Value> = if stdout.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&stdout).context("Failed to parse Apple Container JSON output")?
        };

        let mut clusters = HashMap::new();

        for container in containers {
            // Check if this container has kina cluster labels
            if let Some(labels) = container
                .get("configuration")
                .and_then(|c| c.get("labels"))
                .and_then(|l| l.as_object())
            {
                if let Some(cluster_name) = labels.get("io.kina.cluster").and_then(|v| v.as_str()) {
                    let role = labels
                        .get("io.kina.role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let container_name = container
                        .get("configuration")
                        .and_then(|c| c.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    let state = container
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    // Extract IP address from container networks
                    let ip_address = container
                        .get("networks")
                        .and_then(|networks| networks.as_array())
                        .and_then(|networks| networks.first())
                        .and_then(|network| network.get("address"))
                        .and_then(|addr| addr.as_str())
                        .map(|addr| addr.split('/').next().unwrap_or(addr).to_string());

                    // Group containers by cluster name
                    let cluster_info =
                        clusters
                            .entry(cluster_name.to_string())
                            .or_insert_with(|| ClusterInfo {
                                name: cluster_name.to_string(),
                                image: labels
                                    .get("io.kina.image")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("kindest/node:latest")
                                    .to_string(),
                                status: if state == "running" {
                                    ClusterStatus::Running
                                } else {
                                    ClusterStatus::Stopped
                                },
                                nodes: Vec::new(),
                                created: "unknown".to_string(), // Container format doesn't expose creation time easily
                                kubeconfig_path: None,
                            });

                    // Add node information
                    cluster_info.nodes.push(NodeInfo {
                        name: container_name.to_string(),
                        role: if role == "control-plane" {
                            NodeRole::ControlPlane
                        } else {
                            NodeRole::Worker
                        },
                        status: state.to_string(),
                        version: "unknown".to_string(),
                        container_id: Some(container_name.to_string()),
                        ip_address,
                    });

                    // Update cluster status based on all containers
                    if state != "running" {
                        cluster_info.status = ClusterStatus::Stopped;
                    }
                }
            }
        }

        let result: Vec<ClusterInfo> = clusters.into_values().collect();
        debug!("Found {} kina clusters", result.len());

        Ok(result)
    }

    /// Get kubeconfig for a cluster
    pub async fn get_kubeconfig(&self, name: &str) -> Result<String> {
        debug!("Getting kubeconfig for cluster '{}'", name);

        // First try to read from standard kubectl location: ~/.kube/<cluster-name>
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir).join(".kube").join(name);

        if kubeconfig_path.exists() {
            return fs::read_to_string(kubeconfig_path).context("Failed to read kubeconfig file");
        }

        // If no local kubeconfig, try to generate one from the cluster
        self.generate_kubeconfig(name).await
    }

    /// Generate kubeconfig from a running cluster
    /// Note: Apple Container provides DNS resolution for container names,
    /// so kubeconfig can use container hostnames directly instead of IP addresses
    async fn generate_kubeconfig(&self, name: &str) -> Result<String> {
        info!("Generating kubeconfig for cluster '{}'", name);

        // Find the primary control plane node
        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' not found", name))?;

        let control_plane_node = cluster
            .nodes
            .iter()
            .find(|n| n.role == NodeRole::ControlPlane)
            .ok_or_else(|| anyhow::anyhow!("No control plane node found for cluster '{}'", name))?;

        if let Some(container_id) = &control_plane_node.container_id {
            // Try to execute kubectl config view in the control plane container
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["exec", container_id, "kubectl", "config", "view", "--raw"]);

            let output = cmd
                .output()
                .context("Failed to get kubeconfig from cluster")?;

            if output.status.success() {
                let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();

                // Get the VM's IP address to update the server URL
                if let Some(vm_ip) = &control_plane_node.ip_address {
                    info!("Updating kubeconfig server URL to use VM IP: {}", vm_ip);

                    // Replace localhost/127.0.0.1 references with the VM's IP
                    kubeconfig = kubeconfig
                        .replace("https://127.0.0.1:6443", &format!("https://{}:6443", vm_ip))
                        .replace("https://localhost:6443", &format!("https://{}:6443", vm_ip));

                    // Also replace any internal cluster IP with VM IP
                    if kubeconfig.contains("https://10.") || kubeconfig.contains("https://172.") {
                        // Use regex or string manipulation to replace internal IPs
                        // For now, use a simple approach
                        kubeconfig = kubeconfig
                            .lines()
                            .map(|line| {
                                if line.trim().starts_with("server: https://") {
                                    format!("    server: https://{}:6443", vm_ip)
                                } else {
                                    line.to_string()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }

                // Make user names cluster-specific to prevent conflicts in merged config
                let kubeconfig = self.make_user_names_cluster_specific(&kubeconfig, name)?;

                // Save the kubeconfig locally for future use
                self.save_kubeconfig(name, &kubeconfig).await?;

                return Ok(kubeconfig);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to get kubeconfig from cluster: {}", stderr);
            }
        }

        Err(anyhow::anyhow!(
            "Could not retrieve kubeconfig for cluster '{}'. The cluster may not be fully initialized.",
            name
        ))
    }

    /// Make user names cluster-specific to prevent conflicts in merged kubeconfig
    fn make_user_names_cluster_specific(
        &self,
        kubeconfig: &str,
        cluster_name: &str,
    ) -> Result<String> {
        let mut config: serde_yaml::Value =
            serde_yaml::from_str(kubeconfig).context("Failed to parse kubeconfig")?;

        // Create cluster-specific user name
        let cluster_specific_user = format!("{}-admin", cluster_name);

        // Update user name in users section
        if let Some(users) = config.get_mut("users").and_then(|u| u.as_sequence_mut()) {
            for user in users.iter_mut() {
                if let Some(name) = user.get("name").and_then(|n| n.as_str()) {
                    if name == "kubernetes-admin" {
                        user["name"] = serde_yaml::Value::String(cluster_specific_user.clone());
                    }
                }
            }
        }

        // Update user reference in contexts section
        if let Some(contexts) = config.get_mut("contexts").and_then(|c| c.as_sequence_mut()) {
            for context in contexts.iter_mut() {
                if let Some(context_obj) = context.get_mut("context") {
                    if let Some(user) = context_obj.get("user").and_then(|u| u.as_str()) {
                        if user == "kubernetes-admin" {
                            context_obj["user"] =
                                serde_yaml::Value::String(cluster_specific_user.clone());
                        }
                    }
                }
            }
        }

        let updated_kubeconfig =
            serde_yaml::to_string(&config).context("Failed to serialize updated kubeconfig")?;

        Ok(updated_kubeconfig)
    }

    /// Save kubeconfig using kubectl-native merging for proper context management
    async fn save_kubeconfig(&self, name: &str, kubeconfig: &str) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = std::path::Path::new(&home_dir).join(".kube");
        let global_config_path = kube_dir.join("config");
        let individual_path = kube_dir.join(name);

        // Ensure the .kube directory exists
        fs::create_dir_all(&kube_dir).context("Failed to create .kube directory")?;

        // Save individual file for backward compatibility and direct access
        fs::write(&individual_path, kubeconfig)
            .context("Failed to write individual kubeconfig file")?;

        // Use kubectl config merge approach by setting KUBECONFIG environment variable
        // This leverages kubectl's built-in merging logic which is much more reliable
        let existing_kubeconfig = if global_config_path.exists() {
            format!(
                "{}:{}",
                global_config_path.display(),
                individual_path.display()
            )
        } else {
            individual_path.display().to_string()
        };

        // Use kubectl config view --flatten to merge configs
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.env("KUBECONFIG", &existing_kubeconfig)
            .arg("config")
            .arg("view")
            .arg("--flatten");

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config view")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("kubectl config view failed: {}", stderr));
        }

        // Write the merged config back to the global config file
        let merged_config = String::from_utf8_lossy(&output.stdout);
        fs::write(&global_config_path, merged_config.as_bytes())
            .context("Failed to write merged kubeconfig")?;

        // Set the current context to the new cluster
        let context_name = name; // Use cluster name as context name
        let mut use_context_cmd = tokio::process::Command::new("kubectl");
        use_context_cmd
            .arg("config")
            .arg("use-context")
            .arg(context_name);

        let use_output = use_context_cmd
            .output()
            .await
            .context("Failed to set current context")?;
        if !use_output.status.success() {
            let stderr = String::from_utf8_lossy(&use_output.stderr);
            warn!(
                "Failed to set current context to '{}': {}",
                context_name, stderr
            );
        }

        info!(
            "Saved kubeconfig to: {} and merged into: {} using kubectl",
            individual_path.display(),
            global_config_path.display()
        );
        Ok(())
    }

    /// Remove kubeconfig context for deleted cluster using kubectl commands
    async fn remove_kubeconfig_context(&self, cluster_name: &str) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = std::path::Path::new(&home_dir).join(".kube");
        let individual_path = kube_dir.join(cluster_name);

        // Remove individual kubeconfig file
        if individual_path.exists() {
            fs::remove_file(&individual_path)
                .context("Failed to remove individual kubeconfig file")?;
            info!(
                "Removed individual kubeconfig: {}",
                individual_path.display()
            );
        }

        // Use kubectl to remove cluster components
        let user_name = format!("{}-admin", cluster_name);

        // Remove context (ignore errors if context doesn't exist)
        let _ = self.kubectl_delete_context(cluster_name).await;

        // Remove cluster (ignore errors if cluster doesn't exist)
        let _ = self.kubectl_delete_cluster(cluster_name).await;

        // Remove user (ignore errors if user doesn't exist)
        let _ = self.kubectl_delete_user(&user_name).await;

        info!(
            "Removed cluster '{}' from kubeconfig using kubectl commands",
            cluster_name
        );
        Ok(())
    }

    /// Delete context using kubectl config delete-context
    async fn kubectl_delete_context(&self, context_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-context").arg(context_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-context")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-context failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Delete cluster using kubectl config delete-cluster
    async fn kubectl_delete_cluster(&self, cluster_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-cluster").arg(cluster_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-cluster")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-cluster failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Delete user using kubectl config delete-user
    async fn kubectl_delete_user(&self, user_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-user").arg(user_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-user")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-user failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Load image into cluster
    pub async fn load_image(&self, options: &LoadImageOptions) -> Result<()> {
        info!(
            "Loading image '{}' into cluster '{}'",
            options.image, options.cluster
        );

        // Find all nodes in the cluster
        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .iter()
            .find(|c| c.name == options.cluster)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' not found", options.cluster))?;

        if cluster.nodes.is_empty() {
            return Err(anyhow::anyhow!(
                "No nodes found in cluster '{}'",
                options.cluster
            ));
        }

        // Load image into each node container
        for node in &cluster.nodes {
            if let Some(container_id) = &node.container_id {
                self.load_image_into_container(container_id, &options.image)
                    .await?;
            }
        }

        info!(
            "Image '{}' loaded successfully into cluster '{}'",
            options.image, options.cluster
        );
        Ok(())
    }

    /// Load image into a specific container
    async fn load_image_into_container(&self, container_id: &str, image: &str) -> Result<()> {
        debug!(
            "Loading image '{}' into container '{}'",
            image, container_id
        );

        // First, save the image to a tar file
        let temp_dir = std::env::temp_dir();
        let image_tar = temp_dir.join(format!("{}.tar", image.replace(['/', ':'], "_")));

        // Export the image
        let mut cmd = std::process::Command::new("docker");
        cmd.args(["save", "-o", &image_tar.to_string_lossy(), image]);

        let output = cmd
            .output()
            .context("Failed to export image with docker save")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to export image: {}", stderr));
        }

        // Copy the tar file into the container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "cp",
            &image_tar.to_string_lossy(),
            &format!("{}:/tmp/image.tar", container_id),
        ]);

        let output = cmd
            .output()
            .context("Failed to copy image tar to container")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up the temp file
            let _ = fs::remove_file(&image_tar);
            return Err(anyhow::anyhow!(
                "Failed to copy image to container: {}",
                stderr
            ));
        }

        // Load the image in the container using ctr (containerd CLI)
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_id,
            "ctr",
            "images",
            "import",
            "/tmp/image.tar",
        ]);

        let output = cmd.output().context("Failed to load image in container")?;

        // Clean up the temp file
        let _ = fs::remove_file(&image_tar);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to load image in container: {}",
                stderr
            ));
        }

        debug!(
            "Successfully loaded image '{}' into container '{}'",
            image, container_id
        );
        Ok(())
    }

    /// Wait for container to be ready
    async fn wait_for_container_ready(&self, container_name: &str) -> Result<()> {
        info!("Waiting for container '{}' to be ready...", container_name);

        for attempt in 1..=30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["list", "--format", "json"]);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Ok(containers) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout)
                    {
                        for container in containers {
                            if let Some(id) = container
                                .get("configuration")
                                .and_then(|c| c.get("id"))
                                .and_then(|v| v.as_str())
                            {
                                if id == container_name {
                                    if let Some(status) =
                                        container.get("status").and_then(|v| v.as_str())
                                    {
                                        if status == "running" {
                                            debug!(
                                                "Container '{}' is running after {} attempts",
                                                container_name, attempt
                                            );
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            debug!(
                "Container '{}' not ready yet, attempt {}/30",
                container_name, attempt
            );
        }

        Err(anyhow::anyhow!(
            "Container '{}' failed to become ready within 60 seconds",
            container_name
        ))
    }

    /// Get container IP address
    async fn get_container_ip(&self, container_name: &str) -> Result<String> {
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["list", "--format", "json"]);

        let output = cmd
            .output()
            .context("Failed to get container information")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to list containers: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).context("Failed to parse container list JSON")?;

        for container in containers {
            if let Some(id) = container
                .get("configuration")
                .and_then(|c| c.get("id"))
                .and_then(|v| v.as_str())
            {
                if id == container_name {
                    if let Some(networks) = container.get("networks").and_then(|v| v.as_array()) {
                        if let Some(network) = networks.first() {
                            if let Some(address) = network.get("address").and_then(|v| v.as_str()) {
                                let ip = address.split('/').next().unwrap_or(address);
                                return Ok(ip.to_string());
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not find IP address for container '{}'",
            container_name
        ))
    }

    /// Initialize Kubernetes cluster with kubeadm
    async fn initialize_kubernetes_cluster(&self, container_name: &str, vm_ip: &str) -> Result<()> {
        info!(
            "Initializing Kubernetes cluster in container '{}'",
            container_name
        );

        // Create kubeadm configuration
        let kubeadm_config = format!(
            r#"apiVersion: kubeadm.k8s.io/v1beta3
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: "{}"
  bindPort: 6443
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
    node-ip: "{}"
    provider-id: "kind://docker/kina-cluster/{}"
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: ClusterConfiguration
kubernetesVersion: v1.31.0
clusterName: "kina-cluster"
controlPlaneEndpoint: "{}:6443"
apiServer:
  certSANs:
  - "{}"
  - "{}"
  - "localhost"
  - "127.0.0.1"
  extraArgs:
    runtime-config: "api/all=true"
networking:
  serviceSubnet: "10.96.0.0/16"
  podSubnet: "10.244.0.0/16"
  dnsDomain: "cluster.local"
controllerManager:
  extraArgs:
    enable-hostpath-provisioner: "true"
scheduler: {{}}
etcd:
  local:
    dataDir: "/var/lib/etcd"
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: JoinConfiguration
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
    node-ip: "{}"
    provider-id: "kind://docker/kina-cluster/{}"
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
failSwapOn: false
authentication:
  anonymous:
    enabled: false
  webhook:
    enabled: true
authorization:
  mode: Webhook
serverTLSBootstrap: true
---
apiVersion: kubeproxy.config.k8s.io/v1alpha1
kind: KubeProxyConfiguration
bindAddress: "0.0.0.0"
healthzBindAddress: "0.0.0.0:10256"
metricsBindAddress: "0.0.0.0:10249"
clusterCIDR: "10.244.0.0/16"
"#,
            vm_ip, vm_ip, container_name, vm_ip, vm_ip, container_name, vm_ip, container_name
        );

        // Write kubeadm config to container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "sh",
            "-c",
            &format!("cat > /kind/kubeadm.conf << 'EOF'\n{}\nEOF", kubeadm_config),
        ]);

        let output = cmd.output().context("Failed to write kubeadm config")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to write kubeadm config: {}",
                stderr
            ));
        }

        // Initialize cluster with kubeadm
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "kubeadm",
            "init",
            "--config=/kind/kubeadm.conf",
            "--skip-phases=preflight",
            "--v=1",
        ]);

        info!("Running kubeadm init (this may take a few minutes)...");
        let output = cmd.output().context("Failed to run kubeadm init")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "kubeadm init failed:\nStdout: {}\nStderr: {}",
                stdout,
                stderr
            ));
        }

        info!("Kubernetes cluster initialized successfully");
        Ok(())
    }

    /// Remove control-plane taint for single-node scheduling
    async fn remove_control_plane_taint(&self, container_name: &str) -> Result<()> {
        info!("Removing control-plane taint for single-node scheduling");

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "kubectl",
            "--kubeconfig=/etc/kubernetes/admin.conf",
            "taint",
            "nodes",
            container_name,
            "node-role.kubernetes.io/control-plane:NoSchedule-",
        ]);

        let output = cmd
            .output()
            .context("Failed to remove control-plane taint")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Failed to remove control-plane taint (may already be removed): {}",
                stderr
            );
        } else {
            info!("Control-plane taint removed successfully");
        }

        Ok(())
    }

    /// Install CNI plugin based on configuration
    async fn install_cni_plugin(&self, container_name: &str) -> Result<()> {
        match self.config.cluster.default_cni {
            crate::config::CniPlugin::Ptp => self.install_ptp_cni(container_name).await,
            crate::config::CniPlugin::Cilium => self.install_cilium_cni(container_name).await,
        }
    }

    /// Install PTP CNI plugin optimized for Apple Container VMs
    async fn install_ptp_cni(&self, container_name: &str) -> Result<()> {
        info!("Installing PTP CNI plugin optimized for Apple Container VMs");

        // PTP CNI configuration that works with kata-containers kernel limitations
        let ptp_config = r#"{
  "cniVersion": "0.4.0",
  "name": "ptp-net",
  "plugins": [
    {
      "type": "ptp",
      "ipMasq": true,
      "ipam": {
        "type": "host-local",
        "subnet": "10.244.0.0/16",
        "routes": [
          { "dst": "0.0.0.0/0" }
        ]
      }
    },
    {
      "type": "portmap",
      "capabilities": {
        "portMappings": true
      }
    }
  ]
}"#;

        // Create CNI configuration directory and install config
        let install_cmd = format!(
            r#"mkdir -p /etc/cni/net.d && cat > /etc/cni/net.d/10-ptp.conflist << 'EOF'
{}
EOF"#,
            ptp_config
        );

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", &install_cmd]);

        let output = cmd
            .output()
            .context("Failed to install PTP CNI configuration")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to install PTP CNI configuration: {}",
                stderr
            ));
        }

        // Restart kubelet to pick up CNI configuration
        let restart_cmd = "systemctl restart kubelet";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", restart_cmd]);

        let output = cmd.output().context("Failed to restart kubelet")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Kubelet restart returned non-zero: {}", stderr);
        }

        info!("PTP CNI plugin installed successfully - compatible with Apple Container VMs");
        Ok(())
    }

    /// Install Cilium CNI plugin using standard Cilium CLI
    async fn install_cilium_cni(&self, container_name: &str) -> Result<()> {
        info!("Installing Cilium CNI plugin using standard Cilium CLI");

        // Set up environment (use the kubeconfig that kubeadm created)
        let kubeconfig_env = "export KUBECONFIG=/etc/kubernetes/admin.conf";

        // First install the Cilium CLI (running as root in container, no sudo needed)
        // Use arm64 architecture for Apple Silicon compatibility
        let install_cli_cmd = r#"
CILIUM_CLI_VERSION=$(curl -s https://raw.githubusercontent.com/cilium/cilium-cli/main/stable.txt)
CLI_ARCH=arm64
curl -L --fail --remote-name-all https://github.com/cilium/cilium-cli/releases/download/${CILIUM_CLI_VERSION}/cilium-linux-${CLI_ARCH}.tar.gz{,.sha256sum}
sha256sum --check cilium-linux-${CLI_ARCH}.tar.gz.sha256sum
tar xzvfC cilium-linux-${CLI_ARCH}.tar.gz /usr/local/bin
rm -f cilium-linux-${CLI_ARCH}.tar.gz cilium-linux-${CLI_ARCH}.tar.gz.sha256sum
"#;

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", install_cli_cmd]);

        let output = cmd.output().context("Failed to install Cilium CLI")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to install Cilium CLI: {}", stderr));
        }

        // Now install Cilium using the standard cilium install command with minimal Apple Container fix
        // Disable local node route management to fix: "address family not supported by protocol"
        let cilium_install_cmd = format!(
            "{} && cilium install --version 1.18.2 --set enableLocalNodeRoute=false",
            kubeconfig_env
        );

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", &cilium_install_cmd]);

        let output = cmd.output().context("Failed to install Cilium")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to install Cilium: {}", stderr));
        }

        info!("Cilium CNI plugin installed successfully using standard installation");
        Ok(())
    }

    /// Setup kubeconfig for external access
    async fn setup_kubeconfig(
        &self,
        cluster_name: &str,
        container_name: &str,
        vm_ip: &str,
    ) -> Result<()> {
        info!("Setting up kubeconfig for external access");

        // Get kubeconfig from container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "cat", "/etc/kubernetes/admin.conf"]);

        let output = cmd
            .output()
            .context("Failed to get kubeconfig from container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to get kubeconfig: {}", stderr));
        }

        let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();

        // Update server URL to use VM IP
        kubeconfig = kubeconfig
            .replace(
                "https://192.168.64.5:6443",
                &format!("https://{}:6443", vm_ip),
            )
            .replace("https://127.0.0.1:6443", &format!("https://{}:6443", vm_ip))
            .replace("https://localhost:6443", &format!("https://{}:6443", vm_ip));

        // Replace cluster name, context names, and user names
        let cluster_admin = format!("{}-admin", cluster_name);
        kubeconfig = kubeconfig
            .replace("name: kina-cluster", &format!("name: {}", cluster_name))
            .replace(
                "cluster: kina-cluster",
                &format!("cluster: {}", cluster_name),
            )
            .replace("kubernetes-admin@kina-cluster", cluster_name)
            .replace(
                "name: kubernetes-admin",
                &format!("name: {}", cluster_admin),
            )
            .replace(
                "user: kubernetes-admin",
                &format!("user: {}", cluster_admin),
            );

        // Save kubeconfig
        self.save_kubeconfig(cluster_name, &kubeconfig).await?;

        info!("Kubeconfig saved successfully");
        Ok(())
    }
}
