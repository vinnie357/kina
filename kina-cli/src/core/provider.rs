#![allow(dead_code)]
/// Container provider abstraction layer following KIND's provider pattern
use async_trait::async_trait;
// Note: Serde imports removed as they're not currently used in this file
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::ClusterConfig;
use crate::core::types::{ClusterStatus, ExecResult, NodeRole};
use crate::errors::{KinaError, KinaResult};

/// Container specification for node creation
#[derive(Debug, Clone, Default)]
pub struct ContainerSpec {
    pub image: String,
    pub hostname: String,
    pub network_id: Option<String>,
    pub privileged: bool,
    pub labels: BTreeMap<String, String>,
    pub volumes: Vec<VolumeMount>,
    pub port_mappings: Vec<PortMapping>,
    pub environment: BTreeMap<String, String>,
    pub extra_mounts: Vec<ExtraMount>,
}

/// Volume mount specification
#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
}

/// Port mapping specification
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String, // "tcp" or "udp"
}

/// Extra mount specification for Kubernetes-specific paths
#[derive(Debug, Clone)]
pub struct ExtraMount {
    pub name: String,
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub mount_type: String, // "bind", "tmpfs", etc.
}

/// Container network specification
#[derive(Debug, Clone, Default)]
pub struct NetworkSpec {
    pub name: String,
    pub driver: String,
    pub subnet: String,
    pub ipv6_enabled: bool,
    pub labels: BTreeMap<String, String>,
}

/// Container information
#[derive(Debug, Clone)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub ip_address: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub created: String,
}

/// Container state
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Creating,
    Running,
    Stopped,
    Error,
    Unknown,
}

/// Container network information
#[derive(Debug, Clone)]
pub struct ContainerNetwork {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub subnet: String,
    pub gateway: String,
}

/// Node information
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub role: NodeRole,
    pub container_id: String,
    pub ip_address: Option<String>,
    pub status: String,
    pub kubernetes_version: Option<String>,
    pub cluster_name: String,
}

impl Node {
    pub fn is_primary_control_plane(&self) -> bool {
        self.role == NodeRole::ControlPlane && self.name.ends_with("-control-plane")
    }
}

/// Container provider trait following KIND's provider abstraction pattern
#[async_trait]
pub trait ContainerProvider: Send + Sync {
    /// Provision containers for a cluster based on configuration
    async fn provision(&self, config: &ClusterConfig) -> KinaResult<()>;

    /// List all nodes (containers) in a cluster
    async fn list_nodes(&self, cluster: &str) -> KinaResult<Vec<Node>>;

    /// Delete specific nodes from a cluster
    async fn delete_nodes(&self, nodes: &[Node]) -> KinaResult<()>;

    /// Get the API server endpoint for a cluster
    async fn get_api_server_endpoint(&self, cluster: &str) -> KinaResult<String>;

    /// Execute a command inside a container
    async fn exec_in_container(&self, container_id: &str, cmd: &[&str]) -> KinaResult<ExecResult>;

    /// Execute a command with stdin input
    async fn exec_in_container_with_stdin(
        &self,
        container_id: &str,
        cmd: &[&str],
        stdin: &str,
    ) -> KinaResult<ExecResult>;

    /// Write a file to a container
    async fn write_file_to_container(
        &self,
        container_id: &str,
        path: &str,
        content: &str,
    ) -> KinaResult<()>;

    /// Read a file from a container
    async fn read_file_from_container(&self, container_id: &str, path: &str) -> KinaResult<String>;

    /// List all clusters managed by this provider
    async fn list_clusters(&self) -> KinaResult<Vec<String>>;

    /// Check if a cluster exists
    async fn cluster_exists(&self, cluster: &str) -> KinaResult<bool>;

    /// Get cluster information
    async fn get_cluster_info(&self, cluster: &str) -> KinaResult<ProviderClusterInfo>;

    /// Clean up all resources for a cluster
    async fn cleanup_cluster(&self, cluster: &str) -> KinaResult<()>;
}

/// Provider cluster information structure (separate from core::cluster::ClusterInfo)
#[derive(Debug, Clone)]
pub struct ProviderClusterInfo {
    pub name: String,
    pub status: ClusterStatus,
    pub nodes: Vec<Node>,
    pub created: String,
    pub kubernetes_version: Option<String>,
    pub api_server_endpoint: Option<String>,
}

impl ProviderClusterInfo {
    pub fn control_plane_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .count()
    }

    pub fn worker_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.role == NodeRole::Worker)
            .count()
    }
}

/// Container specification builder following the builder pattern
#[derive(Default)]
pub struct ContainerSpecBuilder {
    spec: ContainerSpec,
}

impl ContainerSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: ContainerSpec {
                image: String::new(),
                hostname: String::new(),
                network_id: None,
                privileged: false,
                labels: BTreeMap::new(),
                volumes: Vec::new(),
                port_mappings: Vec::new(),
                environment: BTreeMap::new(),
                extra_mounts: Vec::new(),
            },
        }
    }

    pub fn image(mut self, image: &str) -> Self {
        self.spec.image = image.to_string();
        self
    }

    pub fn hostname(mut self, hostname: &str) -> Self {
        self.spec.hostname = hostname.to_string();
        self
    }

    pub fn network(mut self, network_id: &str) -> Self {
        self.spec.network_id = Some(network_id.to_string());
        self
    }

    pub fn privileged(mut self, privileged: bool) -> Self {
        self.spec.privileged = privileged;
        self
    }

    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.spec.labels.insert(key.to_string(), value.to_string());
        self
    }

    pub fn labels(mut self, labels: &[(&str, &str)]) -> Self {
        for (key, value) in labels {
            self.spec.labels.insert(key.to_string(), value.to_string());
        }
        self
    }

    pub fn volume(mut self, host_path: &str, container_path: &str) -> Self {
        self.spec.volumes.push(VolumeMount {
            host_path: PathBuf::from(host_path),
            container_path: PathBuf::from(container_path),
            read_only: false,
        });
        self
    }

    pub fn volume_ro(mut self, host_path: &str, container_path: &str) -> Self {
        self.spec.volumes.push(VolumeMount {
            host_path: PathBuf::from(host_path),
            container_path: PathBuf::from(container_path),
            read_only: true,
        });
        self
    }

    pub fn port_mapping(mut self, host_port: u16, container_port: u16) -> Self {
        self.spec.port_mappings.push(PortMapping {
            host_port,
            container_port,
            protocol: "tcp".to_string(),
        });
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.spec
            .environment
            .insert(key.to_string(), value.to_string());
        self
    }

    pub fn extra_mount(
        mut self,
        name: &str,
        host_path: &str,
        container_path: &str,
        mount_type: &str,
    ) -> Self {
        self.spec.extra_mounts.push(ExtraMount {
            name: name.to_string(),
            host_path: PathBuf::from(host_path),
            container_path: PathBuf::from(container_path),
            mount_type: mount_type.to_string(),
        });
        self
    }

    pub fn build(self) -> KinaResult<ContainerSpec> {
        if self.spec.image.is_empty() {
            return Err(KinaError::Operation(
                "Container image is required".to_string(),
            ));
        }
        if self.spec.hostname.is_empty() {
            return Err(KinaError::Operation(
                "Container hostname is required".to_string(),
            ));
        }
        Ok(self.spec)
    }
}

/// Network specification builder
#[derive(Default)]
pub struct NetworkSpecBuilder {
    spec: NetworkSpec,
}

impl NetworkSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: NetworkSpec {
                name: String::new(),
                driver: "bridge".to_string(),
                subnet: String::new(),
                ipv6_enabled: false,
                labels: BTreeMap::new(),
            },
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.spec.name = name.to_string();
        self
    }

    pub fn driver(mut self, driver: &str) -> Self {
        self.spec.driver = driver.to_string();
        self
    }

    pub fn subnet(mut self, subnet: &str) -> Self {
        self.spec.subnet = subnet.to_string();
        self
    }

    pub fn enable_ipv6(mut self, enabled: bool) -> Self {
        self.spec.ipv6_enabled = enabled;
        self
    }

    pub fn labels(mut self, labels: &[(&str, &str)]) -> Self {
        for (key, value) in labels {
            self.spec.labels.insert(key.to_string(), value.to_string());
        }
        self
    }

    pub fn build(self) -> KinaResult<NetworkSpec> {
        if self.spec.name.is_empty() {
            return Err(KinaError::Operation("Network name is required".to_string()));
        }
        if self.spec.subnet.is_empty() {
            return Err(KinaError::Operation(
                "Network subnet is required".to_string(),
            ));
        }
        Ok(self.spec)
    }
}
