#![allow(dead_code)]
use thiserror::Error;

/// Main error types for kina CLI
#[derive(Error, Debug)]
pub enum KinaError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Cluster operation errors
    #[error("Cluster error: {0}")]
    Cluster(#[from] ClusterError),

    /// Apple Container integration errors
    #[error("Apple Container error: {0}")]
    AppleContainer(#[from] AppleContainerError),

    /// Kubernetes operation errors
    #[error("Kubernetes error: {0}")]
    Kubernetes(#[from] KubernetesError),

    /// I/O operation errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Network/HTTP errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Generic operation errors
    #[error("Operation error: {0}")]
    Operation(String),

    /// Anyhow context errors
    #[error(transparent)]
    Context(#[from] anyhow::Error),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found at: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("Configuration validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// Cluster operation errors
#[derive(Error, Debug)]
pub enum ClusterError {
    #[error("Cluster '{name}' already exists")]
    AlreadyExists { name: String },

    #[error("Cluster '{name}' not found")]
    NotFound { name: String },

    #[error("Cluster '{name}' is not running")]
    NotRunning { name: String },

    #[error("Cluster creation failed: {reason}")]
    CreationFailed { reason: String },

    #[error("Cluster deletion failed: {reason}")]
    DeletionFailed { reason: String },

    #[error("Cluster operation timed out after {seconds} seconds")]
    OperationTimeout { seconds: u64 },

    #[error("Invalid cluster state: {state}")]
    InvalidState { state: String },

    #[error("Kubeconfig error: {reason}")]
    KubeconfigError { reason: String },
}

/// Apple Container specific errors
#[derive(Error, Debug)]
pub enum AppleContainerError {
    #[error("Apple Container CLI not found")]
    CliNotFound,

    #[error("Apple Container CLI version not supported: {version}")]
    UnsupportedVersion { version: String },

    #[error("Apple Container command failed: {command}")]
    CommandFailed { command: String },

    #[error("Apple Container runtime error: {reason}")]
    RuntimeError { reason: String },

    #[error("Container image not found: {image}")]
    ImageNotFound { image: String },

    #[error("Network configuration error: {reason}")]
    NetworkError { reason: String },

    #[error("Resource limit exceeded: {resource} = {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },
}

/// Kubernetes operation errors
#[derive(Error, Debug)]
pub enum KubernetesError {
    #[error("kubectl not found")]
    KubectlNotFound,

    #[error("Kubeconfig not accessible: {path}")]
    KubeconfigNotAccessible { path: String },

    #[error("Cluster not reachable")]
    ClusterNotReachable,

    #[error("Resource not found: {resource}")]
    ResourceNotFound { resource: String },

    #[error("Kubernetes API error: {reason}")]
    ApiError { reason: String },

    #[error("Manifest validation failed: {reason}")]
    ManifestValidationFailed { reason: String },

    #[error("Node not ready: {node}")]
    NodeNotReady { node: String },
}

/// Result type alias for kina operations
pub type KinaResult<T> = Result<T, KinaError>;

impl From<serde_json::Error> for KinaError {
    fn from(err: serde_json::Error) -> Self {
        KinaError::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for KinaError {
    fn from(err: serde_yaml::Error) -> Self {
        KinaError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for KinaError {
    fn from(err: toml::de::Error) -> Self {
        KinaError::Serialization(err.to_string())
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(err: config::ConfigError) -> Self {
        match err {
            config::ConfigError::NotFound(_) => ConfigError::FileNotFound {
                path: "configuration".to_string(),
            },
            config::ConfigError::Type { .. } => ConfigError::InvalidFormat {
                reason: err.to_string(),
            },
            _ => ConfigError::ValidationFailed {
                reason: err.to_string(),
            },
        }
    }
}
