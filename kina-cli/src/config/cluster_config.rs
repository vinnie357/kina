// Note: Some fields may be unused during development but are part of the KIND-compatible schema
use anyhow::Context;
/// KIND-compatible cluster configuration schema
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::core::types::NodeRole;
use crate::errors::{KinaError, KinaResult};

/// KIND-compatible cluster configuration
/// Based on KIND's cluster configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// API version (KIND compatibility)
    #[serde(rename = "apiVersion")]
    pub api_version: String,

    /// Resource kind (KIND compatibility)
    pub kind: String,

    /// Cluster name
    pub name: String,

    /// Node configuration
    pub nodes: Vec<NodeConfig>,

    /// Networking configuration
    pub networking: NetworkingConfig,

    /// Feature gates to enable/disable
    #[serde(rename = "featureGates")]
    pub feature_gates: BTreeMap<String, bool>,

    /// kubeadm configuration patches
    #[serde(rename = "kubeadmConfigPatches")]
    pub kubeadm_config_patches: Vec<String>,

    /// kubeadm configuration patches by target
    #[serde(rename = "kubeadmConfigPatchesJSON6902")]
    pub kubeadm_config_patches_json6902: Vec<JSON6902Patch>,

    /// Runtime configuration specific to kina/Apple Container
    #[serde(rename = "runtimeConfig", skip_serializing_if = "Option::is_none")]
    pub runtime_config: Option<RuntimeConfig>,
}

/// Node configuration for cluster nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node role (control-plane or worker)
    pub role: NodeRole,

    /// Container image for this node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Extra mounts for this node
    #[serde(rename = "extraMounts", skip_serializing_if = "Vec::is_empty", default)]
    pub extra_mounts: Vec<Mount>,

    /// Extra port mappings for this node
    #[serde(
        rename = "extraPortMappings",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub extra_port_mappings: Vec<PortMapping>,

    /// kubeadm configuration patches for this node
    #[serde(
        rename = "kubeadmConfigPatches",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub kubeadm_config_patches: Vec<String>,

    /// Labels for this node
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub labels: BTreeMap<String, String>,

    // Internal fields (not serialized)
    /// Internal cluster name reference
    #[serde(skip)]
    #[allow(dead_code)]
    pub cluster_name: String,

    /// Internal node name
    #[serde(skip)]
    #[allow(dead_code)]
    pub name: String,
}

/// Networking configuration (KIND compatible)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkingConfig {
    /// Disable default CNI
    #[serde(
        rename = "disableDefaultCNI",
        skip_serializing_if = "is_false",
        default
    )]
    pub disable_default_cni: bool,

    /// Kubernetes version for kubeproxy
    #[serde(rename = "kubeProxyMode", skip_serializing_if = "Option::is_none")]
    pub kube_proxy_mode: Option<String>,

    /// Pod subnet CIDR
    #[serde(rename = "podSubnet", skip_serializing_if = "Option::is_none")]
    pub pod_subnet: Option<String>,

    /// Service subnet CIDR
    #[serde(rename = "serviceSubnet", skip_serializing_if = "Option::is_none")]
    pub service_subnet: Option<String>,

    /// API server address
    #[serde(rename = "apiServerAddress", skip_serializing_if = "Option::is_none")]
    pub api_server_address: Option<String>,

    /// API server port
    #[serde(rename = "apiServerPort", skip_serializing_if = "Option::is_none")]
    pub api_server_port: Option<u16>,
}

/// Mount configuration for nodes (KIND compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// Host path to mount
    #[serde(rename = "hostPath")]
    pub host_path: PathBuf,

    /// Container path for the mount
    #[serde(rename = "containerPath")]
    pub container_path: PathBuf,

    /// Whether the mount is read-only
    #[serde(rename = "readOnly", skip_serializing_if = "is_false")]
    pub read_only: bool,

    /// SELinux options
    #[serde(rename = "selinuxRelabel", skip_serializing_if = "is_false")]
    pub selinux_relabel: bool,

    /// Propagation mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagation: Option<String>,
}

/// Port mapping for nodes (KIND compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    #[serde(rename = "containerPort")]
    pub container_port: u16,

    /// Host port (optional)
    #[serde(rename = "hostPort", skip_serializing_if = "Option::is_none")]
    pub host_port: Option<u16>,

    /// Listen address
    #[serde(rename = "listenAddress", skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<String>,

    /// Protocol (tcp/udp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

/// JSON 6902 patch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSON6902Patch {
    /// Group of the target resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// Version of the target resource
    pub version: String,

    /// Kind of the target resource
    pub kind: String,

    /// Patch operations
    pub patch: String,
}

/// Runtime configuration specific to kina/Apple Container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// CPU limit for containers
    #[serde(rename = "cpuLimit", skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,

    /// Memory limit for containers
    #[serde(rename = "memoryLimit", skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,

    /// Storage limit for containers
    #[serde(rename = "storageLimit", skip_serializing_if = "Option::is_none")]
    pub storage_limit: Option<String>,

    /// Enable container privileged mode
    #[serde(skip_serializing_if = "is_false", default)]
    pub privileged: bool,

    /// Additional container environment variables
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub environment: BTreeMap<String, String>,
}

/// Default values and constructors
impl Default for ClusterConfig {
    fn default() -> Self {
        Self::default_with_name("kina")
    }
}

impl ClusterConfig {
    /// Create a default cluster configuration with a specific name
    pub fn default_with_name(name: &str) -> Self {
        Self {
            api_version: "kind.x-k8s.io/v1alpha4".to_string(),
            kind: "Cluster".to_string(),
            name: name.to_string(),
            nodes: vec![NodeConfig {
                role: NodeRole::ControlPlane,
                image: None, // Will use default image
                extra_mounts: Vec::new(),
                extra_port_mappings: Vec::new(),
                kubeadm_config_patches: Vec::new(),
                labels: BTreeMap::new(),
                cluster_name: name.to_string(),
                name: format!("{}-control-plane", name),
            }],
            networking: NetworkingConfig::default(),
            feature_gates: BTreeMap::new(),
            kubeadm_config_patches: Vec::new(),
            kubeadm_config_patches_json6902: Vec::new(),
            runtime_config: Some(RuntimeConfig::default()),
        }
    }

    /// Load cluster configuration from file
    #[allow(dead_code)]
    pub async fn from_file<P: AsRef<Path>>(path: P) -> KinaResult<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read cluster config file: {}", path.display()))?;

        let mut config: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse cluster config file: {}", path.display()))?;

        // Assign node names and cluster references
        config.assign_node_names();

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Save cluster configuration to file
    #[allow(dead_code)]
    pub async fn to_file<P: AsRef<Path>>(&self, path: P) -> KinaResult<()> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content =
            serde_yaml::to_string(self).context("Failed to serialize cluster configuration")?;

        tokio::fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write cluster config file: {}", path.display()))?;

        Ok(())
    }

    /// Assign internal node names based on roles and cluster name
    #[allow(dead_code)]
    pub fn assign_node_names(&mut self) {
        let mut control_plane_count = 0;
        let mut worker_count = 0;

        for node in &mut self.nodes {
            node.cluster_name = self.name.clone();

            match node.role {
                NodeRole::ControlPlane => {
                    if control_plane_count == 0 {
                        node.name = format!("{}-control-plane", self.name);
                    } else {
                        node.name =
                            format!("{}-control-plane{}", self.name, control_plane_count + 1);
                    }
                    control_plane_count += 1;
                }
                NodeRole::Worker => {
                    node.name = format!("{}-worker{}", self.name, worker_count + 1);
                    worker_count += 1;
                }
            }
        }
    }

    /// Validate the cluster configuration
    #[allow(dead_code)]
    pub fn validate(&self) -> KinaResult<()> {
        // Check that we have at least one control plane node
        let control_plane_count = self
            .nodes
            .iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .count();

        if control_plane_count == 0 {
            return Err(KinaError::Operation(
                "Cluster configuration must have at least one control-plane node".to_string(),
            ));
        }

        // Validate cluster name
        if self.name.is_empty() {
            return Err(KinaError::Operation(
                "Cluster name cannot be empty".to_string(),
            ));
        }

        // Validate networking configuration
        self.networking.validate()?;

        // Validate each node configuration
        for (index, node) in self.nodes.iter().enumerate() {
            node.validate()
                .with_context(|| format!("Invalid configuration for node {}", index))?;
        }

        Ok(())
    }

    /// Get control plane nodes
    #[allow(dead_code)]
    pub fn control_plane_nodes(&self) -> Vec<&NodeConfig> {
        self.nodes
            .iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .collect()
    }

    /// Get worker nodes
    #[allow(dead_code)]
    pub fn worker_nodes(&self) -> Vec<&NodeConfig> {
        self.nodes
            .iter()
            .filter(|n| n.role == NodeRole::Worker)
            .collect()
    }

    /// Get the primary control plane node
    #[allow(dead_code)]
    pub fn primary_control_plane(&self) -> Option<&NodeConfig> {
        self.control_plane_nodes().into_iter().next()
    }
}

impl NetworkingConfig {
    #[allow(dead_code)]
    fn validate(&self) -> KinaResult<()> {
        // Add networking validation logic as needed
        Ok(())
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            cpu_limit: None,
            memory_limit: Some("2Gi".to_string()),
            storage_limit: Some("20Gi".to_string()),
            privileged: true, // Required for systemd in containers
            environment: BTreeMap::new(),
        }
    }
}

impl NodeConfig {
    #[allow(dead_code)]
    fn validate(&self) -> KinaResult<()> {
        // Validate extra mounts
        for mount in &self.extra_mounts {
            if mount.host_path.as_os_str().is_empty() {
                return Err(KinaError::Operation(
                    "Mount host path cannot be empty".to_string(),
                ));
            }
            if mount.container_path.as_os_str().is_empty() {
                return Err(KinaError::Operation(
                    "Mount container path cannot be empty".to_string(),
                ));
            }
        }

        // Validate port mappings
        for port in &self.extra_port_mappings {
            if port.container_port == 0 {
                return Err(KinaError::Operation(
                    "Container port cannot be 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Helper function for serde skip_serializing_if
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cluster_config() {
        let config = ClusterConfig::default_with_name("test-cluster");

        assert_eq!(config.name, "test-cluster");
        assert_eq!(config.api_version, "kind.x-k8s.io/v1alpha4");
        assert_eq!(config.kind, "Cluster");
        assert_eq!(config.nodes.len(), 1);
        assert_eq!(config.nodes[0].role, NodeRole::ControlPlane);
        assert_eq!(config.nodes[0].name, "test-cluster-control-plane");
    }

    #[test]
    fn test_yaml_serialization() {
        let config = ClusterConfig::default_with_name("test");
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Should be able to deserialize back
        let deserialized: ClusterConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.name, deserialized.name);
    }

    #[test]
    fn test_validation() {
        let mut config = ClusterConfig::default_with_name("test");

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Empty name should fail
        config.name = String::new();
        assert!(config.validate().is_err());

        // No control plane nodes should fail
        config.name = "test".to_string();
        config.nodes.clear();
        assert!(config.validate().is_err());
    }
}
