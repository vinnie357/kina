#![allow(dead_code)]
use crate::config::CniPlugin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export types from config module to avoid duplication
// NodeRole is defined in this module, no need to re-export

/// Options for creating a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClusterOptions {
    /// Name of the cluster
    pub name: String,
    /// Container image to use for nodes
    pub image: String,
    /// Optional configuration file path
    pub config_file: Option<PathBuf>,
    /// Kubernetes version
    pub kubernetes_version: Option<String>,
    /// Number of worker nodes
    pub workers: Option<u32>,
    /// Number of control plane nodes
    pub control_plane_nodes: Option<u32>,
    /// Wait timeout for cluster readiness
    pub wait_timeout: Option<u64>,
    /// Retain cluster on failure
    pub retain_on_failure: bool,
    /// Skip automatic kubelet CSR approval
    pub skip_csr_approval: bool,
    /// CNI plugin to use
    pub cni_plugin: CniPlugin,
}

/// Options for loading images into a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadImageOptions {
    /// Name of the cluster
    pub cluster: String,
    /// Docker image to load
    pub image: String,
    /// Optional image archive path
    pub archive: Option<PathBuf>,
}

/// Information about a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Name of the cluster
    pub name: String,
    /// Image used for the cluster nodes
    pub image: String,
    /// Current status of the cluster
    pub status: ClusterStatus,
    /// Creation timestamp
    pub created: String,
    /// List of nodes in the cluster
    pub nodes: Vec<NodeInfo>,
    /// Path to kubeconfig file
    pub kubeconfig_path: Option<String>,
}

/// Status of a cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClusterStatus {
    /// Cluster is running normally
    Running,
    /// Cluster is being created
    Creating,
    /// Cluster is stopped
    Stopped,
    /// Cluster is in an error state
    Error,
    /// Status is unknown
    Unknown,
}

/// Cluster state (alias for ClusterStatus for compatibility)
#[allow(dead_code)]
pub type ClusterState = ClusterStatus;

impl std::fmt::Display for ClusterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterStatus::Running => write!(f, "Running"),
            ClusterStatus::Creating => write!(f, "Creating"),
            ClusterStatus::Stopped => write!(f, "Stopped"),
            ClusterStatus::Error => write!(f, "Error"),
            ClusterStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Information about a node in a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Name of the node
    pub name: String,
    /// Role of the node (control-plane, worker)
    pub role: NodeRole,
    /// Status of the node
    pub status: String,
    /// Kubernetes version running on the node
    pub version: String,
    /// Container ID of the node (if applicable)
    pub container_id: Option<String>,
    /// IP address of the node (Apple Container VM IP)
    pub ip_address: Option<String>,
}

/// Node role enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeRole {
    ControlPlane,
    Worker,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::ControlPlane => write!(f, "control-plane"),
            NodeRole::Worker => write!(f, "worker"),
        }
    }
}

/// Result of executing a command in a container
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExecResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code of the command
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl ExecResult {
    /// Create a new successful result
    pub fn success(stdout: String) -> Self {
        Self {
            success: true,
            exit_code: Some(0),
            stdout,
            stderr: String::new(),
        }
    }

    /// Create a new failed result
    pub fn failure(exit_code: i32, stderr: String) -> Self {
        Self {
            success: false,
            exit_code: Some(exit_code),
            stdout: String::new(),
            stderr,
        }
    }
}

// Note: NetworkingConfig, NodeConfig, ClusterConfig, VolumeMount, and PortMapping
// are defined in config::cluster_config module to avoid duplication
