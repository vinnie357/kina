# KINA CLI Architecture Based on KIND Patterns

**Generated**: 2025-09-14
**Based on**: KIND architectural analysis and KINA requirements
**Purpose**: Define CLI command structure and patterns for KINA implementation

## Overview

This document defines the CLI architecture for KINA (Kubernetes in Apple Container) based on proven patterns from KIND (Kubernetes in Docker). The design maintains KIND workflow compatibility while leveraging Rust's strengths and Apple Container's native capabilities.

## Command Structure (KIND-Compatible)

### Primary Commands

**Cluster Management**
```bash
# Core cluster operations (KIND-compatible)
kina create cluster [NAME] [--config CONFIG_FILE]
kina delete cluster [NAME]
kina get clusters

# Extended cluster operations
kina start cluster [NAME]
kina stop cluster [NAME]
kina export kubeconfig [NAME]
```

**Image Management**
```bash
# Container image operations (adapted from KIND)
kina load docker-image [IMAGE] [--name CLUSTER]
kina build node-image [--image IMAGE] [--base-image BASE]
kina export logs [PATH] [--name CLUSTER]
```

**Configuration and Status**
```bash
# Configuration and diagnostics
kina get nodes [--name CLUSTER]
kina export kubeconfig [--name CLUSTER]
kina version
```

### Configuration File Support

**YAML Configuration (KIND-compatible schema)**
```yaml
# kina-config.yaml
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
metadata:
  name: development-cluster
nodes:
  - role: control-plane
    image: kindest/node:v1.27.3
    extraMounts:
      - hostPath: /path/to/local
        containerPath: /mnt/data
networking:
  apiServerAddress: "127.0.0.1"
  apiServerPort: 6443
```

## Rust CLI Implementation Architecture

### Module Structure (Adapted from KIND's Go packages)

```rust
// src/cli/mod.rs - Main CLI entry point
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kina")]
#[command(about = "Kubernetes in Apple Container - KIND for macOS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new cluster
    Create {
        #[command(subcommand)]
        resource: CreateCommands,
    },
    /// Delete resources
    Delete {
        #[command(subcommand)]
        resource: DeleteCommands,
    },
    /// Get information about resources
    Get {
        #[command(subcommand)]
        resource: GetCommands,
    },
    /// Load images into cluster
    Load {
        #[command(subcommand)]
        resource: LoadCommands,
    },
    /// Export cluster data
    Export {
        #[command(subcommand)]
        resource: ExportCommands,
    },
}
```

### Command Handlers (Provider Pattern)

```rust
// src/cli/commands/cluster.rs
use crate::cluster::ClusterManager;
use crate::container::AppleContainerProvider;

pub async fn create_cluster(
    name: Option<String>,
    config: Option<PathBuf>,
) -> Result<(), KinaError> {
    let provider = AppleContainerProvider::new()?;
    let manager = ClusterManager::new(provider);

    // KIND's phased approach: Create → Configure → Bootstrap → Join
    let config = load_or_default_config(config)?;
    let cluster = manager.create_cluster(name, config).await?;

    println!("✅ Cluster '{}' created successfully", cluster.name());
    Ok(())
}

pub async fn delete_cluster(name: Option<String>) -> Result<(), KinaError> {
    let provider = AppleContainerProvider::new()?;
    let manager = ClusterManager::new(provider);

    manager.delete_cluster(name).await?;
    println!("🗑️ Cluster deleted successfully");
    Ok(())
}
```

### Container Runtime Abstraction

```rust
// src/container/provider.rs - Apple Container provider
use async_trait::async_trait;

#[async_trait]
pub trait ContainerProvider {
    async fn create_container(&self, spec: ContainerSpec) -> Result<Container, ContainerError>;
    async fn delete_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn list_containers(&self) -> Result<Vec<Container>, ContainerError>;
    async fn exec_command(&self, id: &str, cmd: Vec<String>) -> Result<ExecResult, ContainerError>;
}

pub struct AppleContainerProvider {
    runtime: AppleContainerRuntime,
}

impl AppleContainerProvider {
    pub fn new() -> Result<Self, ContainerError> {
        let runtime = AppleContainerRuntime::connect()?;
        Ok(Self { runtime })
    }
}

#[async_trait]
impl ContainerProvider for AppleContainerProvider {
    async fn create_container(&self, spec: ContainerSpec) -> Result<Container, ContainerError> {
        // Use Apple Container CLI to create systemd-enabled containers
        // Following KIND's container-as-node pattern
        self.runtime.create_systemd_container(spec).await
    }
}
```

### Cluster Lifecycle Management

```rust
// src/cluster/lifecycle.rs - KIND's phased approach
pub struct ClusterManager<P: ContainerProvider> {
    provider: P,
    config_manager: ConfigManager,
}

impl<P: ContainerProvider> ClusterManager<P> {
    pub async fn create_cluster(
        &self,
        name: Option<String>,
        config: ClusterConfig,
    ) -> Result<Cluster, ClusterError> {
        let name = name.unwrap_or_else(|| "kind".to_string());

        // Phase 1: Create - Set up container infrastructure
        let container = self.create_cluster_container(&name, &config).await?;

        // Phase 2: Configure - Apply cluster configuration
        self.configure_cluster_networking(&container, &config).await?;

        // Phase 3: Bootstrap - Initialize Kubernetes with kubeadm
        self.bootstrap_kubernetes(&container, &config).await?;

        // Phase 4: Join - Finalize cluster setup
        self.finalize_cluster_setup(&container, &config).await?;

        Ok(Cluster::new(name, container))
    }
}
```

### Error Handling (KIND-inspired patterns)

```rust
// src/error.rs - Structured error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KinaError {
    #[error("Container runtime error: {0}")]
    ContainerRuntime(#[from] ContainerError),

    #[error("Cluster operation failed: {0}")]
    ClusterOperation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Apple Container not available: {0}")]
    AppleContainerMissing(String),

    #[error("Kubernetes setup failed: {0}")]
    KubernetesSetup(String),
}

// User-friendly error messages like KIND
impl KinaError {
    pub fn user_message(&self) -> String {
        match self {
            KinaError::AppleContainerMissing(_) => {
                "Apple Container is not available. Please ensure macOS 15.6+ and Apple Container CLI are installed.".to_string()
            }
            KinaError::ClusterOperation(msg) => {
                format!("Cluster operation failed: {}. Try 'kina get clusters' to see existing clusters.", msg)
            }
            _ => self.to_string(),
        }
    }
}
```

### Configuration Management

```rust
// src/config/types.rs - KIND-compatible configuration
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterConfig {
    pub kind: String, // "Cluster"
    pub api_version: String, // "kind.x-k8s.io/v1alpha4"
    pub metadata: ClusterMetadata,
    pub nodes: Vec<NodeConfig>,
    pub networking: Option<NetworkingConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeConfig {
    pub role: NodeRole,
    pub image: Option<String>,
    pub extra_mounts: Option<Vec<Mount>>,
    pub extra_port_mappings: Option<Vec<PortMapping>>,
}

// Default configuration following KIND patterns
impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            kind: "Cluster".to_string(),
            api_version: "kind.x-k8s.io/v1alpha4".to_string(),
            metadata: ClusterMetadata::default(),
            nodes: vec![NodeConfig::control_plane_default()],
            networking: Some(NetworkingConfig::default()),
        }
    }
}
```

## Integration with Apple Container

### Node Image Building Strategy

```rust
// src/image/builder.rs - Apple Container node images
pub struct NodeImageBuilder {
    base_image: String,
    kubernetes_version: String,
}

impl NodeImageBuilder {
    pub async fn build_node_image(&self) -> Result<String, ImageError> {
        // 1. Start with base macOS container image
        // 2. Install systemd (container compatibility)
        // 3. Install Kubernetes components (kubelet, kubeadm, kubectl)
        // 4. Configure container networking
        // 5. Set up Apple Container specific configurations

        self.build_apple_container_image().await
    }

    async fn build_apple_container_image(&self) -> Result<String, ImageError> {
        // Apple Container native image building
        // Following KIND's Dockerfile patterns but adapted for Apple Container
        let build_spec = AppleContainerBuildSpec {
            base_image: &self.base_image,
            layers: vec![
                Layer::run("install systemd"),
                Layer::run(&format!("install kubernetes {}", self.kubernetes_version)),
                Layer::copy("kubeadm-config", "/etc/kubernetes/"),
                Layer::cmd("systemd as init"),
            ],
        };

        AppleContainerRuntime::build_image(build_spec).await
    }
}
```

### Network and Storage Integration

```rust
// src/cluster/networking.rs - Apple Container networking
pub struct ClusterNetworking {
    provider: AppleContainerProvider,
}

impl ClusterNetworking {
    pub async fn setup_cluster_network(&self, container: &Container) -> Result<(), NetworkError> {
        // 1. Configure container networking (following KIND patterns)
        // 2. Set up Kubernetes networking (CNI)
        // 3. Configure service networking and ingress
        // 4. Apple Container specific network optimizations

        self.configure_apple_container_networking(container).await
    }
}
```

## Compatibility and Migration

### KIND Command Mapping

```bash
# Direct command compatibility
kind create cluster        → kina create cluster
kind delete cluster        → kina delete cluster
kind get clusters          → kina get clusters
kind load docker-image     → kina load docker-image
kind export kubeconfig     → kina export kubeconfig

# Extended Apple Container features
kina create cluster --native-performance  # Apple Container optimizations
kina get cluster --apple-container-info   # Apple Container specific status
```

### Migration Guide Integration

```rust
// src/cli/commands/migrate.rs - Migration assistance
pub fn check_kind_compatibility() -> Result<CompatibilityReport, MigrationError> {
    // Scan for existing KIND clusters and configurations
    // Provide migration recommendations
    // Generate KINA-compatible configurations
}
```

## Implementation Priority

1. **Phase 1**: Basic CLI structure with `create` and `delete` cluster commands
2. **Phase 2**: Apple Container provider and container lifecycle
3. **Phase 3**: Configuration file support and advanced commands
4. **Phase 4**: Image management and ecosystem integration
5. **Phase 5**: Performance optimization and KIND migration tools

## Testing Strategy

```rust
// tests/integration/cli_tests.rs
#[tokio::test]
async fn test_cluster_create_workflow() {
    // Test complete cluster creation following KIND patterns
    let output = Command::new("kina")
        .args(&["create", "cluster", "--name", "test"])
        .output()
        .await?;

    assert!(output.status.success());
    // Verify cluster is accessible via kubectl
    verify_kubectl_access("test").await?;
}
```

This CLI architecture provides KIND workflow compatibility while leveraging Rust's type safety and Apple Container's native capabilities for optimal macOS development experience.