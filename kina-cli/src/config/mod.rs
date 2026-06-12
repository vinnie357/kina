use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// Re-export cluster configuration
pub mod cluster_config;

/// Pinned kernel distribution configuration.
///
/// Ships with defaults that point to the validated release artifact.
/// Override in `~/.config/kina/config.toml` under `[kernel]` to pin a different version.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KernelConfig {
    /// GitHub release tag for the pinned kernel asset (e.g. `kernel-v6.18.5-kina.1`).
    pub tag: String,

    /// sha256 hex digest of the pinned `vmlinux` artifact (lowercase, 64 chars).
    pub sha256: String,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            tag: crate::core::kernel_fetch::KERNEL_TAG.to_string(),
            sha256: crate::core::kernel_fetch::KERNEL_SHA256.to_string(),
        }
    }
}

/// Main configuration structure for kina CLI application
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Default cluster configuration
    pub cluster: ClusterDefaults,

    /// Pinned kernel distribution settings (used when --cni cilium is selected).
    #[serde(default)]
    pub kernel: KernelConfig,

    /// Apple Container settings
    pub apple_container: AppleContainerConfig,

    /// Kubernetes settings
    pub kubernetes: KubernetesConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Path to the configuration file (not serialized)
    #[serde(skip)]
    pub config_file_path: Option<PathBuf>,
}

/// Default cluster settings for the CLI
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterDefaults {
    /// Default cluster name
    pub default_name: String,

    /// Default container image for clusters
    pub default_image: String,

    /// Default wait timeout for cluster operations (seconds)
    pub default_wait_timeout: u64,

    /// Directory for storing cluster data
    pub data_dir: PathBuf,

    /// Whether to retain clusters on failure by default
    pub retain_on_failure: bool,

    /// Default CNI plugin to use
    pub default_cni: CniPlugin,

    /// Optional path to a custom Linux kernel for node containers.
    /// When set, kina passes `--kernel <path>` to `container run` for every node container,
    /// booting each on the custom kernel (zero system mutation — no `container system kernel set`).
    /// When None (the default), the system default kernel is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_kernel_path: Option<PathBuf>,

    /// Per-role resource defaults (all optional).
    ///
    /// Resource precedence: CLI flag (--cpus/--memory) > per-role config default
    /// (control_plane_*/worker_*) > built-in default (DEFAULT_NODE_CPUS=4 / DEFAULT_NODE_MEMORY=4g).
    ///
    /// The single-node path (workers==0, combined control-plane+worker role) resolves
    /// using the control_plane_* slots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_plane_cpus: Option<u32>,

    /// Per-role memory default for control-plane nodes (e.g. "8g", "512m").
    /// See control_plane_cpus for the full precedence documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_plane_memory: Option<String>,

    /// Per-role CPU default for worker nodes.
    /// See control_plane_cpus for the full precedence documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_cpus: Option<u32>,

    /// Per-role memory default for worker nodes (e.g. "2g", "512m").
    /// See control_plane_cpus for the full precedence documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_memory: Option<String>,
}

/// CNI plugin options
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CniPlugin {
    /// PTP CNI with host-local IPAM (default, Apple Container compatible)
    Ptp,
    /// Cilium CNI (advanced features, requires compatible kernel)
    Cilium,
}

/// Apple Container specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppleContainerConfig {
    /// Path to Apple Container CLI (if not in PATH)
    pub cli_path: Option<PathBuf>,

    /// Default container runtime configuration
    pub runtime_config: RuntimeConfig,

    /// Network configuration
    pub network: NetworkConfig,
}

/// Container runtime configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeConfig {
    /// Default CPU limit for containers
    pub cpu_limit: Option<String>,

    /// Default memory limit for containers
    pub memory_limit: Option<String>,

    /// Default storage limit for containers
    pub storage_limit: Option<String>,
}

/// Network configuration for containers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    /// Default network name
    pub network_name: String,

    /// Enable IPv6 support
    #[allow(dead_code)]
    pub enable_ipv6: bool,

    /// Custom DNS servers
    #[allow(dead_code)]
    pub dns_servers: Vec<String>,
}

/// Kubernetes-specific configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KubernetesConfig {
    /// Kubernetes version to use by default
    pub default_version: String,

    /// Path to kubectl binary (if not in PATH)
    pub kubectl_path: Option<PathBuf>,

    /// Default namespace for operations
    pub default_namespace: String,

    /// Kubeconfig directory
    pub kubeconfig_dir: PathBuf,
}

/// Logging configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingConfig {
    /// Default log level
    pub level: String,

    /// Log format (json, text)
    pub format: String,

    /// Enable file logging
    pub file_logging: bool,

    /// Log file directory
    pub log_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        // Use XDG Base Directory specification
        let data_dir = if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("kina")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
                .join("kina")
        };

        let config_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("kina");

        Self {
            cluster: ClusterDefaults {
                default_name: "kina".to_string(),
                default_image: "kindest/node:v1.36.1".to_string(),
                default_wait_timeout: 300, // 5 minutes
                data_dir: data_dir.clone(),
                retain_on_failure: false,
                default_cni: CniPlugin::Ptp, // Default to PTP for Apple Container compatibility
                node_kernel_path: None,      // Stock kernel by default; set to enable custom kernel
                control_plane_cpus: None,
                control_plane_memory: None,
                worker_cpus: None,
                worker_memory: None,
            },
            kernel: KernelConfig::default(),
            apple_container: AppleContainerConfig {
                cli_path: None, // Will be detected automatically
                runtime_config: RuntimeConfig {
                    cpu_limit: None,
                    memory_limit: Some("2Gi".to_string()),
                    storage_limit: Some("20Gi".to_string()),
                },
                network: NetworkConfig {
                    network_name: "kina".to_string(),
                    enable_ipv6: false,
                    dns_servers: vec![],
                },
            },
            kubernetes: KubernetesConfig {
                default_version: "v1.36.1".to_string(),
                kubectl_path: None, // Will be detected automatically
                default_namespace: "default".to_string(),
                kubeconfig_dir: config_dir.join("kubeconfig"),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "text".to_string(),
                file_logging: false,
                log_dir: None,
            },
            config_file_path: None,
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path();

        if config_path.exists() {
            info!("Loading configuration from: {}", config_path.display());
            Self::load_from_file(&config_path)
        } else {
            info!("No configuration file found, using defaults");
            debug!("Expected config path: {}", config_path.display());

            let config = Self {
                config_file_path: Some(config_path.clone()),
                ..Default::default()
            };

            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }

            // Ensure directories exist
            config.ensure_directories()?;

            Ok(config)
        }
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;

        let mut config: Self = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => toml::from_str(&content)?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)?,
            Some("json") => serde_json::from_str(&content)?,
            _ => {
                // Try to detect format by content
                if let Ok(config) = toml::from_str::<Self>(&content) {
                    config
                } else if let Ok(config) = serde_yaml::from_str::<Self>(&content) {
                    config
                } else {
                    serde_json::from_str(&content)?
                }
            }
        };

        config.config_file_path = Some(path.to_path_buf());
        config.ensure_directories()?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        if let Some(config_path) = &self.config_file_path {
            // Ensure parent directory exists
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let content = toml::to_string_pretty(self)?;
            std::fs::write(config_path, content)?;

            info!("Configuration saved to: {}", config_path.display());
        } else {
            warn!("No config file path set, cannot save configuration");
        }

        Ok(())
    }

    /// Get the default configuration file path
    pub fn get_config_path() -> PathBuf {
        // Use XDG Base Directory specification: ~/.config/kina/config.toml
        // Always use ~/.config/kina regardless of platform for consistency
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("kina")
            .join("config.toml")
    }

    /// Ensure all required directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        // Create data directory
        if !self.cluster.data_dir.exists() {
            std::fs::create_dir_all(&self.cluster.data_dir)?;
            debug!(
                "Created data directory: {}",
                self.cluster.data_dir.display()
            );
        }

        // Create kubeconfig directory
        if !self.kubernetes.kubeconfig_dir.exists() {
            std::fs::create_dir_all(&self.kubernetes.kubeconfig_dir)?;
            debug!(
                "Created kubeconfig directory: {}",
                self.kubernetes.kubeconfig_dir.display()
            );
        }

        // Create log directory if file logging is enabled
        if self.logging.file_logging {
            if let Some(log_dir) = &self.logging.log_dir {
                if !log_dir.exists() {
                    std::fs::create_dir_all(log_dir)?;
                    debug!("Created log directory: {}", log_dir.display());
                }
            }
        }

        Ok(())
    }

    /// Merge with another configuration, preferring values from other
    #[allow(dead_code)]
    pub fn merge_with(&mut self, other: &Config) {
        // This is a simplified merge - in practice, you might want more sophisticated merging logic
        if other.cluster.default_name != self.cluster.default_name
            && other.cluster.default_name != "kina"
        {
            self.cluster.default_name = other.cluster.default_name.clone();
        }

        // Add more merge logic as needed
    }
}
