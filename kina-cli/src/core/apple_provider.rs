#![allow(dead_code)]
/// Apple Container provider implementation following KIND's provider pattern
use async_trait::async_trait;
use chrono;
use futures::future::try_join_all;
use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::config::{ClusterConfig as KindClusterConfig, Config};
use crate::core::types::{ClusterStatus, ExecResult, NodeInfo, NodeRole};
// Removed unused imports
use crate::core::provider::*;
use crate::errors::{AppleContainerError, KinaError, KinaResult};

const DEFAULT_NODE_IMAGE: &str = "kindest/node:latest";
const KINA_NETWORK_PREFIX: &str = "kina";
const CONTAINER_LABEL_CLUSTER: &str = "io.kina.cluster";
const CONTAINER_LABEL_ROLE: &str = "io.kina.role";

/// Apple Container provider implementation
pub struct AppleContainerProvider {
    config: Config,
    cli_path: String,
    logger: Arc<dyn Logger>,
}

/// Logger trait for structured logging
pub trait Logger: Send + Sync {
    fn info(&self, message: &str);
    fn debug(&self, message: &str);
    fn warn(&self, message: &str);
}

/// Simple logger implementation
pub struct SimpleLogger;

impl Logger for SimpleLogger {
    fn info(&self, message: &str) {
        info!("{}", message);
    }

    fn debug(&self, message: &str) {
        debug!("{}", message);
    }

    fn warn(&self, message: &str) {
        warn!("{}", message);
    }
}

impl AppleContainerProvider {
    /// Create a new Apple Container provider
    pub async fn new(config: Config) -> KinaResult<Self> {
        let cli_path = Self::detect_cli_path(&config).await?;
        let logger = Arc::new(SimpleLogger);

        Ok(Self {
            config,
            cli_path,
            logger,
        })
    }

    /// Create provider with custom logger
    pub async fn new_with_logger(config: Config, logger: Arc<dyn Logger>) -> KinaResult<Self> {
        let cli_path = Self::detect_cli_path(&config).await?;

        Ok(Self {
            config,
            cli_path,
            logger,
        })
    }

    /// Detect Apple Container CLI path
    async fn detect_cli_path(config: &Config) -> KinaResult<String> {
        // First check configuration
        if let Some(path) = &config.apple_container.cli_path {
            let path_str = path.to_string_lossy().to_string();
            if tokio::fs::metadata(path).await.is_ok() {
                return Ok(path_str);
            }
        }

        // Try to find in PATH — only Apple Container CLI names
        let possible_names = ["container", "apple-container"];

        for name in &possible_names {
            if let Ok(output) = Command::new("which").arg(name).output().await {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        info!("Found container CLI at: {}", path);
                        return Ok(name.to_string()); // Return command name for PATH lookup
                    }
                }
            }
        }

        // Try common installation paths
        let common_paths = [
            "/usr/local/bin/apple-container",
            "/opt/homebrew/bin/apple-container",
            "/System/Library/PrivateFrameworks/ContainerManager.framework/Versions/A/Resources/apple-container",
        ];

        for path in &common_paths {
            if tokio::fs::metadata(path).await.is_ok() {
                info!("Found Apple Container CLI at: {}", path);
                return Ok(path.to_string());
            }
        }

        Err(KinaError::AppleContainer(AppleContainerError::CliNotFound))
    }

    /// Create a container network for the cluster
    async fn ensure_cluster_network(&self, cluster_name: &str) -> KinaResult<ContainerNetwork> {
        let network_name = format!("{}-{}", KINA_NETWORK_PREFIX, cluster_name);

        // Check if network already exists using JSON output
        let mut list_cmd = Command::new(&self.cli_path);
        list_cmd
            .arg("network")
            .arg("list")
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let list_output = list_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to list networks: {}", e),
            })
        })?;

        let stdout = String::from_utf8_lossy(&list_output.stdout);
        let network_exists = if list_output.status.success() && !stdout.trim().is_empty() {
            if let Ok(networks) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                networks.iter().any(|n| {
                    n.get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(|name| name == network_name)
                })
            } else {
                false
            }
        } else {
            false
        };

        if network_exists {
            self.logger
                .debug(&format!("Network {} already exists", network_name));
            return Ok(ContainerNetwork {
                id: network_name.clone(),
                name: network_name,
                driver: "bridge".to_string(),
                subnet: "172.20.0.0/16".to_string(),
                gateway: "172.20.0.1".to_string(),
            });
        }

        // Create new network
        let mut create_cmd = Command::new(&self.cli_path);
        create_cmd
            .arg("network")
            .arg("create")
            .arg("--driver")
            .arg("bridge")
            .arg("--subnet")
            .arg("172.20.0.0/16")
            .arg("--label")
            .arg(format!("{}={}", CONTAINER_LABEL_CLUSTER, cluster_name))
            .arg(&network_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let create_output = create_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::NetworkError {
                reason: format!("Failed to create network: {}", e),
            })
        })?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            return Err(KinaError::AppleContainer(
                AppleContainerError::NetworkError {
                    reason: format!("Network creation failed: {}", stderr),
                },
            ));
        }

        self.logger
            .info(&format!("Created cluster network: {}", network_name));

        Ok(ContainerNetwork {
            id: network_name.clone(),
            name: network_name,
            driver: "bridge".to_string(),
            subnet: "172.20.0.0/16".to_string(),
            gateway: "172.20.0.1".to_string(),
        })
    }

    /// Create a single node container
    async fn create_node_container(
        &self,
        node_config: &crate::config::cluster_config::NodeConfig,
        network: &ContainerNetwork,
        cluster_config: &KindClusterConfig,
    ) -> KinaResult<Container> {
        // Fix the lifetime issue by creating an owned string
        let default_image = DEFAULT_NODE_IMAGE.to_string();
        let image = node_config.image.as_ref().unwrap_or(&default_image);

        let container_name = format!(
            "{}-{}",
            cluster_config.name,
            match node_config.role {
                NodeRole::ControlPlane => "control-plane",
                NodeRole::Worker => "worker",
            }
        );

        let mut create_cmd = Command::new(&self.cli_path);
        create_cmd
            .arg("run")
            .arg("-d") // Detached
            .arg("--name")
            .arg(&container_name)
            .arg("--hostname")
            .arg(&container_name)
            .arg("--network")
            .arg(&network.name)
            .arg("--privileged") // Required for systemd
            .arg("--label")
            .arg(format!("{}={}", CONTAINER_LABEL_CLUSTER, cluster_config.name))
            .arg("--label")
            .arg(format!("{}={}", CONTAINER_LABEL_ROLE, node_config.role))
            // Essential volume mounts for Kubernetes
            .arg("--volume")
            .arg("/var/lib/kubelet:/var/lib/kubelet")
            .arg("--volume")
            .arg("/etc/kubernetes:/etc/kubernetes")
            .arg("--volume")
            .arg("/sys/fs/cgroup:/sys/fs/cgroup")
            // Temporary filesystem mounts
            .arg("--tmpfs")
            .arg("/tmp")
            .arg("--tmpfs")
            .arg("/run");

        // Add extra mounts from configuration
        for mount in &node_config.extra_mounts {
            let mount_spec = format!(
                "{}:{}{}",
                mount.host_path.display(),
                mount.container_path.display(),
                if mount.read_only { ":ro" } else { "" }
            );
            create_cmd.arg("--volume").arg(&mount_spec);
        }

        // Add extra port mappings
        for port in &node_config.extra_port_mappings {
            let port_spec = if let Some(host_port) = port.host_port {
                format!("{}:{}", host_port, port.container_port)
            } else {
                port.container_port.to_string()
            };
            create_cmd.arg("--publish").arg(&port_spec);
        }

        // Add environment variables from runtime config
        if let Some(runtime_config) = &cluster_config.runtime_config {
            for (key, value) in &runtime_config.environment {
                create_cmd.arg("--env").arg(format!("{}={}", key, value));
            }
        }

        // Add the container image
        create_cmd.arg(image);

        // Run systemd as the main process
        create_cmd.arg("/sbin/init");

        create_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let create_output = create_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to create container: {}", e),
            })
        })?;

        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr);
            return Err(KinaError::AppleContainer(
                AppleContainerError::RuntimeError {
                    reason: format!("Container creation failed: {}", stderr),
                },
            ));
        }

        let container_id = String::from_utf8_lossy(&create_output.stdout)
            .trim()
            .to_string();

        self.logger.info(&format!(
            "Created container {} ({})",
            container_name,
            &container_id[..12.min(container_id.len())]
        ));

        Ok(Container {
            id: container_id,
            name: container_name,
            image: image.clone(),
            state: ContainerState::Running,
            ip_address: None, // Will be determined later
            labels: {
                let mut labels = BTreeMap::new();
                labels.insert(
                    CONTAINER_LABEL_CLUSTER.to_string(),
                    cluster_config.name.clone(),
                );
                labels.insert(
                    CONTAINER_LABEL_ROLE.to_string(),
                    node_config.role.to_string(),
                );
                labels
            },
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get container IP address using inspect JSON output
    async fn get_container_ip(&self, container_id: &str) -> KinaResult<Option<String>> {
        let mut inspect_cmd = Command::new(&self.cli_path);
        inspect_cmd
            .arg("inspect")
            .arg(container_id)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = inspect_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to inspect container: {}", e),
            })
        })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(container) = serde_json::from_str::<serde_json::Value>(&stdout) {
                // Extract IP from networks[0].address, stripping CIDR suffix
                if let Some(address) = container
                    .get("networks")
                    .and_then(|n| n.as_array())
                    .and_then(|a| a.first())
                    .and_then(|n| n.get("address"))
                    .and_then(|a| a.as_str())
                {
                    let ip = address.split('/').next().unwrap_or(address);
                    if !ip.is_empty() {
                        return Ok(Some(ip.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl ContainerProvider for AppleContainerProvider {
    async fn provision(&self, config: &KindClusterConfig) -> KinaResult<()> {
        self.logger
            .info(&format!("Provisioning cluster: {}", config.name));

        // Ensure container network exists
        let network = self.ensure_cluster_network(&config.name).await?;

        // Create containers for all nodes concurrently
        let container_futures: Vec<_> = config
            .nodes
            .iter()
            .map(|node_config| self.create_node_container(node_config, &network, config))
            .collect();

        let _containers = try_join_all(container_futures).await?;

        self.logger.info(&format!(
            "Successfully provisioned cluster: {}",
            config.name
        ));
        Ok(())
    }

    async fn list_nodes(&self, cluster: &str) -> KinaResult<Vec<Node>> {
        let mut list_cmd = Command::new(&self.cli_path);
        list_cmd
            .arg("list")
            .arg("--all")
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = list_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to list containers: {}", e),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KinaError::AppleContainer(
                AppleContainerError::RuntimeError {
                    reason: format!("Container list failed: {}", stderr),
                },
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut nodes = Vec::new();

        let containers: Vec<serde_json::Value> = if stdout.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&stdout).map_err(|e| {
                KinaError::AppleContainer(AppleContainerError::RuntimeError {
                    reason: format!("Failed to parse container list JSON: {}", e),
                })
            })?
        };

        for container in containers {
            // Filter by kina cluster label
            let labels = container
                .get("configuration")
                .and_then(|c| c.get("labels"))
                .and_then(|l| l.as_object());

            let Some(labels) = labels else { continue };

            let cluster_label = labels.get(CONTAINER_LABEL_CLUSTER).and_then(|v| v.as_str());

            if cluster_label != Some(cluster) {
                continue;
            }

            let container_id = container
                .get("configuration")
                .and_then(|c| c.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let name = container_id.clone();

            let status = container
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let role_str = labels
                .get(CONTAINER_LABEL_ROLE)
                .and_then(|v| v.as_str())
                .unwrap_or("worker");

            let role = if role_str.contains("control-plane") {
                NodeRole::ControlPlane
            } else {
                NodeRole::Worker
            };

            // Extract IP from networks array, stripping CIDR suffix
            let ip_address = container
                .get("networks")
                .and_then(|n| n.as_array())
                .and_then(|a| a.first())
                .and_then(|n| n.get("address"))
                .and_then(|a| a.as_str())
                .map(|addr| addr.split('/').next().unwrap_or(addr).to_string());

            nodes.push(Node {
                name,
                role,
                container_id,
                ip_address,
                status,
                kubernetes_version: None,
                cluster_name: cluster.to_string(),
            });
        }

        Ok(nodes)
    }

    async fn delete_nodes(&self, nodes: &[Node]) -> KinaResult<()> {
        let delete_futures: Vec<_> = nodes
            .iter()
            .map(|node| async {
                let mut delete_cmd = Command::new(&self.cli_path);
                delete_cmd
                    .arg("delete")
                    .arg("--force")
                    .arg(&node.container_id)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let output = delete_cmd.output().await.map_err(|e| {
                    KinaError::AppleContainer(AppleContainerError::RuntimeError {
                        reason: format!("Failed to delete container {}: {}", node.name, e),
                    })
                })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(KinaError::AppleContainer(
                        AppleContainerError::RuntimeError {
                            reason: format!(
                                "Container deletion failed for {}: {}",
                                node.name, stderr
                            ),
                        },
                    ));
                }

                self.logger
                    .info(&format!("Deleted container: {}", node.name));
                Ok(())
            })
            .collect();

        try_join_all(delete_futures).await?;
        Ok(())
    }

    async fn get_api_server_endpoint(&self, cluster: &str) -> KinaResult<String> {
        let nodes = self.list_nodes(cluster).await?;

        let control_plane = nodes
            .iter()
            .find(|n| n.role == NodeRole::ControlPlane)
            .ok_or_else(|| {
                KinaError::Operation(format!(
                    "No control plane node found for cluster {}",
                    cluster
                ))
            })?;

        let ip = control_plane.ip_address.as_ref().ok_or_else(|| {
            KinaError::Operation(format!(
                "Control plane node {} has no IP address",
                control_plane.name
            ))
        })?;

        // Standard Kubernetes API server port
        Ok(format!("https://{}:6443", ip))
    }

    async fn exec_in_container(&self, container_id: &str, cmd: &[&str]) -> KinaResult<ExecResult> {
        let mut exec_cmd = Command::new(&self.cli_path);
        exec_cmd
            .arg("exec")
            .arg(container_id)
            .args(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = exec_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to exec in container: {}", e),
            })
        })?;

        if output.status.success() {
            Ok(ExecResult::success(
                String::from_utf8_lossy(&output.stdout).to_string(),
            ))
        } else {
            Ok(ExecResult::failure(
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn exec_in_container_with_stdin(
        &self,
        container_id: &str,
        cmd: &[&str],
        stdin: &str,
    ) -> KinaResult<ExecResult> {
        let mut exec_cmd = Command::new(&self.cli_path);
        exec_cmd
            .arg("exec")
            .arg("-i") // Interactive for stdin
            .arg(container_id)
            .args(cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = exec_cmd.spawn().map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to spawn exec process: {}", e),
            })
        })?;

        // Write to stdin
        if let Some(mut stdin_handle) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin_handle
                .write_all(stdin.as_bytes())
                .await
                .map_err(|e| {
                    KinaError::AppleContainer(AppleContainerError::RuntimeError {
                        reason: format!("Failed to write to stdin: {}", e),
                    })
                })?;
        }

        let output = child.wait_with_output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to wait for exec process: {}", e),
            })
        })?;

        if output.status.success() {
            Ok(ExecResult::success(
                String::from_utf8_lossy(&output.stdout).to_string(),
            ))
        } else {
            Ok(ExecResult::failure(
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    async fn write_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> KinaResult<()> {
        // Create a temporary file with the content
        let temp_file = tempfile::NamedTempFile::new().map_err(KinaError::Io)?;

        tokio::fs::write(temp_file.path(), content)
            .await
            .map_err(KinaError::Io)?;

        // Copy file to container
        let mut copy_cmd = Command::new(&self.cli_path);
        copy_cmd
            .arg("cp")
            .arg(temp_file.path())
            .arg(format!("{}:{}", container_id, path))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = copy_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to copy file to container: {}", e),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KinaError::AppleContainer(
                AppleContainerError::RuntimeError {
                    reason: format!("File copy failed: {}", stderr),
                },
            ));
        }

        Ok(())
    }

    async fn read_file_from_container(&self, container_id: &str, path: &str) -> KinaResult<String> {
        let mut copy_cmd = Command::new(&self.cli_path);
        copy_cmd
            .arg("cp")
            .arg(format!("{}:{}", container_id, path))
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = copy_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to copy file from container: {}", e),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KinaError::AppleContainer(
                AppleContainerError::RuntimeError {
                    reason: format!("File read failed: {}", stderr),
                },
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn list_clusters(&self) -> KinaResult<Vec<String>> {
        let mut list_cmd = Command::new(&self.cli_path);
        list_cmd
            .arg("list")
            .arg("--all")
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = list_cmd.output().await.map_err(|e| {
            KinaError::AppleContainer(AppleContainerError::RuntimeError {
                reason: format!("Failed to list clusters: {}", e),
            })
        })?;

        if !output.status.success() {
            return Ok(vec![]); // No containers found
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<serde_json::Value> = if stdout.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&stdout).unwrap_or_default()
        };

        let mut clusters: Vec<String> = containers
            .iter()
            .filter_map(|container| {
                container
                    .get("configuration")
                    .and_then(|c| c.get("labels"))
                    .and_then(|l| l.as_object())
                    .and_then(|labels| labels.get(CONTAINER_LABEL_CLUSTER))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        clusters.sort();
        clusters.dedup();
        Ok(clusters)
    }

    async fn cluster_exists(&self, cluster: &str) -> KinaResult<bool> {
        let clusters = self.list_clusters().await?;
        Ok(clusters.contains(&cluster.to_string()))
    }

    async fn get_cluster_info(&self, cluster: &str) -> KinaResult<ProviderClusterInfo> {
        let nodes = self.list_nodes(cluster).await?;

        if nodes.is_empty() {
            return Err(KinaError::Operation(format!(
                "Cluster {} not found or has no nodes",
                cluster
            )));
        }

        // Convert provider nodes to cluster nodes
        let _cluster_nodes: Vec<NodeInfo> = nodes
            .iter()
            .map(|n| NodeInfo {
                name: n.name.clone(),
                role: n.role.clone(),
                status: n.status.clone(),
                version: n
                    .kubernetes_version
                    .clone()
                    .unwrap_or("unknown".to_string()),
                container_id: Some(n.container_id.clone()),
                ip_address: n.ip_address.clone(),
            })
            .collect();

        Ok(ProviderClusterInfo {
            name: cluster.to_string(),
            status: ClusterStatus::Running, // Simplified for now
            nodes: nodes.clone(),
            created: chrono::Utc::now().to_rfc3339(),
            kubernetes_version: Some("v1.27.0".to_string()),
            api_server_endpoint: None,
        })
    }

    async fn cleanup_cluster(&self, cluster: &str) -> KinaResult<()> {
        self.logger
            .info(&format!("Cleaning up cluster: {}", cluster));

        // Delete all nodes
        let nodes = self.list_nodes(cluster).await?;
        if !nodes.is_empty() {
            self.delete_nodes(&nodes).await?;
        }

        // Delete cluster network
        let network_name = format!("{}-{}", KINA_NETWORK_PREFIX, cluster);
        let mut network_rm_cmd = Command::new(&self.cli_path);
        network_rm_cmd
            .arg("network")
            .arg("delete")
            .arg(&network_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let _output = network_rm_cmd.output().await; // Don't fail if network doesn't exist

        self.logger
            .info(&format!("Cluster {} cleaned up successfully", cluster));
        Ok(())
    }
}
