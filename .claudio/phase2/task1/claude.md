# Apple Container Provider Implementation

## Context Overview

This task implements the core ContainerProvider trait for Apple Container runtime, serving as the foundation for all cluster management operations in KINA. This provider abstracts Apple Container operations to match KIND's provider pattern.

## Objective

Implement a complete AppleContainerProvider that fulfills the ContainerProvider interface, enabling KINA to manage container lifecycle, networking, and orchestration using Apple Container runtime.

## Dependencies

**Required Prerequisites:**
- Phase 1 Task 2: Apple Container research completed with GO decision
- Phase 1 Task 3: Provider abstraction layer designed
- Phase 1 Task 1: Rust project structure established

## Current Project State

Based on discovery analysis:
- **Cargo Workspace**: Established with kina-cli package
- **Dependencies**: Container-related crates available (tokio, async-trait, etc.)
- **Provider Pattern**: Foundation exists in `src/core/provider.rs`
- **Architecture**: Domain-driven layered structure ready for implementation

## Implementation Requirements

### 1. ContainerProvider Trait Implementation

**Core Interface to Implement:**
```rust
#[async_trait]
pub trait ContainerProvider: Send + Sync {
    async fn provision(&self, config: &ClusterConfig) -> Result<(), KinaError>;
    async fn list_nodes(&self, cluster: &str) -> Result<Vec<Node>, KinaError>;
    async fn delete_nodes(&self, nodes: &[Node]) -> Result<(), KinaError>;
    async fn get_api_server_endpoint(&self, cluster: &str) -> Result<String, KinaError>;
    async fn exec_in_container(&self, container_id: &str, cmd: &[&str]) -> Result<ExecResult, KinaError>;
    async fn create_container(&self, spec: ContainerSpec) -> Result<Container, KinaError>;
    async fn write_file_to_container(&self, container_id: &str, path: &str, content: &str) -> Result<(), KinaError>;
    async fn find_control_plane_node(&self, cluster: &str) -> Result<Node, KinaError>;
}
```

### 2. Apple Container Runtime Integration

**Runtime Wrapper Implementation:**
```rust
pub struct AppleContainerRuntime {
    cli_path: PathBuf,
    timeout: Duration,
    logger: Logger,
}

impl AppleContainerRuntime {
    pub async fn new() -> Result<Self, KinaError> {
        // Detect Apple Container CLI path
        let cli_path = Self::detect_apple_container_cli().await?;

        // Verify runtime is available and working
        Self::verify_runtime_availability(&cli_path).await?;

        Ok(Self {
            cli_path,
            timeout: Duration::from_secs(300),
            logger: Logger::new("apple_container_runtime"),
        })
    }

    async fn detect_apple_container_cli() -> Result<PathBuf, KinaError> {
        // Based on Phase 1 research, determine CLI location
        // This implementation depends on Apple Container research findings
    }

    pub async fn create_container(&self, spec: &ContainerSpec) -> Result<Container, KinaError> {
        // Convert spec to Apple Container CLI command
        let mut cmd = Command::new(&self.cli_path);
        cmd.arg("run");

        // Add container configuration
        if spec.privileged {
            cmd.arg("--privileged");
        }

        if let Some(network) = &spec.network {
            cmd.args(&["--network", network]);
        }

        // Add volume mounts
        for mount in &spec.volume_mounts {
            cmd.args(&["-v", &format!("{}:{}", mount.host_path, mount.container_path)]);
        }

        // Add labels
        for (key, value) in &spec.labels {
            cmd.args(&["--label", &format!("{}={}", key, value)]);
        }

        // Set hostname and image
        cmd.args(&["--name", &spec.name]);
        cmd.args(&["--hostname", &spec.hostname]);
        cmd.arg(&spec.image);

        // Execute container creation
        let output = cmd.output().await
            .map_err(|e| KinaError::AppleContainerError {
                operation: "create_container".to_string(),
                details: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(KinaError::ContainerCreationFailed {
                container_name: spec.name.clone(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        // Return container information
        Ok(Container {
            id: spec.name.clone(), // Apple Container might use name as ID
            name: spec.name.clone(),
            status: ContainerStatus::Running,
            image: spec.image.clone(),
            labels: spec.labels.clone(),
        })
    }
}
```

### 3. Network Management

**Network Abstraction:**
```rust
pub struct NetworkManager {
    runtime: AppleContainerRuntime,
}

impl NetworkManager {
    pub async fn ensure_cluster_network(&self, cluster_name: &str) -> Result<ContainerNetwork, KinaError> {
        let network_name = format!("kina-{}", cluster_name);

        // Check if network already exists
        if let Some(network) = self.get_network(&network_name).await? {
            return Ok(network);
        }

        // Create new network for cluster
        self.create_cluster_network(&network_name, cluster_name).await
    }

    async fn create_cluster_network(&self, network_name: &str, cluster_name: &str) -> Result<ContainerNetwork, KinaError> {
        // Apple Container network creation (based on research findings)
        let mut cmd = Command::new(&self.runtime.cli_path);
        cmd.args(&["network", "create"]);
        cmd.args(&["--driver", "bridge"]);
        cmd.args(&["--subnet", "172.20.0.0/16"]); // KIND-compatible subnet
        cmd.args(&["--label", &format!("io.kina.cluster={}", cluster_name)]);
        cmd.arg(network_name);

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(KinaError::NetworkCreationFailed {
                network_name: network_name.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(ContainerNetwork {
            id: network_name.to_string(),
            name: network_name.to_string(),
            driver: "bridge".to_string(),
            subnet: "172.20.0.0/16".to_string(),
        })
    }
}
```

### 4. Container Specification Builder

**Fluent API for Container Creation:**
```rust
pub struct ContainerSpecBuilder {
    spec: ContainerSpec,
}

impl ContainerSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: ContainerSpec::default(),
        }
    }

    pub fn image(mut self, image: &str) -> Self {
        self.spec.image = image.to_string();
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.spec.name = name.to_string();
        self
    }

    pub fn hostname(mut self, hostname: &str) -> Self {
        self.spec.hostname = hostname.to_string();
        self
    }

    pub fn privileged(mut self, privileged: bool) -> Self {
        self.spec.privileged = privileged;
        self
    }

    pub fn network(mut self, network: &str) -> Self {
        self.spec.network = Some(network.to_string());
        self
    }

    pub fn volume_mount(mut self, host_path: &str, container_path: &str) -> Self {
        self.spec.volume_mounts.push(VolumeMount {
            host_path: host_path.to_string(),
            container_path: container_path.to_string(),
            read_only: false,
        });
        self
    }

    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.spec.labels.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Result<ContainerSpec, KinaError> {
        // Validate required fields
        if self.spec.image.is_empty() {
            return Err(KinaError::InvalidContainerSpec {
                reason: "Container image is required".to_string(),
            });
        }

        if self.spec.name.is_empty() {
            return Err(KinaError::InvalidContainerSpec {
                reason: "Container name is required".to_string(),
            });
        }

        Ok(self.spec)
    }
}
```

### 5. Error Handling and Recovery

**Apple Container Specific Errors:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum KinaError {
    #[error("Apple Container operation failed: {operation}")]
    AppleContainerError {
        operation: String,
        details: String,
    },

    #[error("Container creation failed for '{container_name}': {stderr}")]
    ContainerCreationFailed {
        container_name: String,
        stderr: String,
    },

    #[error("Network creation failed for '{network_name}': {stderr}")]
    NetworkCreationFailed {
        network_name: String,
        stderr: String,
    },

    #[error("Apple Container CLI not found or not working")]
    AppleContainerUnavailable,

    #[error("Invalid container specification: {reason}")]
    InvalidContainerSpec {
        reason: String,
    },
}

impl From<std::io::Error> for KinaError {
    fn from(err: std::io::Error) -> Self {
        KinaError::AppleContainerError {
            operation: "io_operation".to_string(),
            details: err.to_string(),
        }
    }
}
```

## Testing Strategy

### 1. Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    use mockall::{automock, predicate::*};

    #[automock]
    trait MockableAppleContainerRuntime {
        async fn create_container(&self, spec: &ContainerSpec) -> Result<Container, KinaError>;
        async fn delete_container(&self, container_id: &str) -> Result<(), KinaError>;
    }

    #[tokio::test]
    async fn test_container_spec_builder() {
        let spec = ContainerSpecBuilder::new()
            .image("kindest/node:v1.28.0")
            .name("test-node")
            .hostname("test-node")
            .privileged(true)
            .network("kina-test")
            .volume_mount("/var/lib/kubelet", "/var/lib/kubelet")
            .label("io.kina.cluster", "test-cluster")
            .build()
            .unwrap();

        assert_eq!(spec.image, "kindest/node:v1.28.0");
        assert_eq!(spec.name, "test-node");
        assert!(spec.privileged);
        assert_eq!(spec.network, Some("kina-test".to_string()));
        assert_eq!(spec.volume_mounts.len(), 1);
        assert_eq!(spec.labels.get("io.kina.cluster"), Some(&"test-cluster".to_string()));
    }

    #[tokio::test]
    async fn test_provider_provision() {
        let mut mock_runtime = MockMockableAppleContainerRuntime::new();

        mock_runtime
            .expect_create_container()
            .times(1)
            .returning(|_| Ok(Container {
                id: "test-container".to_string(),
                name: "test-node".to_string(),
                status: ContainerStatus::Running,
                image: "kindest/node:v1.28.0".to_string(),
                labels: HashMap::new(),
            }));

        // Test provider provisioning logic
        // This would test the actual provider implementation
    }
}
```

### 2. Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run when Apple Container is available
    async fn test_apple_container_integration() {
        // Verify Apple Container is available
        let runtime = AppleContainerRuntime::new().await.unwrap();

        // Test basic container creation
        let spec = ContainerSpecBuilder::new()
            .image("alpine:latest")
            .name("integration-test-container")
            .build()
            .unwrap();

        let container = runtime.create_container(&spec).await.unwrap();
        assert_eq!(container.name, "integration-test-container");

        // Clean up
        runtime.delete_container(&container.id).await.unwrap();
    }
}
```

## Performance Considerations

### 1. Async Operations
- All Apple Container CLI operations use async/await
- Concurrent container creation for multi-node clusters
- Connection pooling for Apple Container API if available

### 2. Caching Strategy
- Cache container and network information
- Optimize repeated Apple Container CLI calls
- Implement intelligent image pull caching

### 3. Resource Management
- Proper cleanup of containers and networks
- Memory-efficient container specification handling
- Stream processing for large container logs

## Integration Points

### 1. Configuration System
```rust
impl AppleContainerProvider {
    pub async fn from_config(config: &ProviderConfig) -> Result<Self, KinaError> {
        let runtime = AppleContainerRuntime::new().await?;

        Ok(Self {
            runtime,
            network_manager: NetworkManager::new(runtime.clone()),
            logger: Logger::new("apple_container_provider"),
            config: config.clone(),
        })
    }
}
```

### 2. Cluster Configuration
```rust
impl AppleContainerProvider {
    async fn create_node_container(&self, node_config: &NodeConfig, cluster_name: &str) -> Result<Container, KinaError> {
        let spec = ContainerSpecBuilder::new()
            .image(&node_config.image.as_ref().unwrap_or(&DEFAULT_NODE_IMAGE))
            .name(&format!("{}-{}", cluster_name, node_config.name))
            .hostname(&node_config.name)
            .privileged(true) // Required for Kubernetes nodes
            .network(&format!("kina-{}", cluster_name))
            // Essential Kubernetes volume mounts
            .volume_mount("/var/lib/kubelet", "/var/lib/kubelet")
            .volume_mount("/etc/kubernetes", "/etc/kubernetes")
            .volume_mount("/sys/fs/cgroup", "/sys/fs/cgroup")
            // Cluster and node identification labels
            .label("io.kina.cluster", cluster_name)
            .label("io.kina.role", &node_config.role.to_string())
            .label("io.kina.node.name", &node_config.name)
            .build()?;

        self.runtime.create_container(&spec).await
    }
}
```

## Implementation Dependencies

### Apple Container Research Integration
This implementation must align with findings from Phase 1 Task 2:
- **CLI Command Structure**: Use actual Apple Container CLI syntax discovered
- **Networking Model**: Implement based on validated networking capabilities
- **Volume Mounting**: Use verified mounting patterns for Kubernetes directories
- **Performance Characteristics**: Optimize based on measured performance patterns

### Provider Abstraction Compliance
Must implement the full ContainerProvider interface designed in Phase 1 Task 3:
- **Method Signatures**: Exact compliance with trait definition
- **Error Handling**: Consistent error types and patterns
- **Async Patterns**: Proper async/await usage throughout

## Success Criteria

### Functional Requirements
- [ ] All ContainerProvider trait methods implemented and functional
- [ ] Container lifecycle operations work reliably (create, start, stop, delete)
- [ ] Network management supports multi-container clusters
- [ ] Volume mounting works for Kubernetes requirements
- [ ] Error handling provides clear diagnostics and recovery guidance

### Performance Requirements
- [ ] Container creation time competitive with Docker Desktop
- [ ] Memory usage optimized for macOS environments
- [ ] Concurrent operations support for multi-node clusters
- [ ] Resource cleanup handles failure scenarios properly

### Integration Requirements
- [ ] Seamless integration with cluster lifecycle management
- [ ] Compatible with existing configuration system
- [ ] Proper logging and observability integration
- [ ] Test coverage >80% for core functionality

## Deliverables

### 1. Core Provider Implementation
- `src/core/apple_container_provider.rs` - Main provider implementation
- `src/core/apple_container_runtime.rs` - Low-level runtime wrapper
- `src/core/network_manager.rs` - Network management
- `src/core/container_spec.rs` - Container specification and builder

### 2. Error Handling
- `src/errors/apple_container.rs` - Apple Container specific errors
- Enhanced error context and recovery mechanisms

### 3. Testing Suite
- Unit tests for all provider methods
- Integration tests with actual Apple Container runtime
- Mock implementations for testing consumer code

### 4. Documentation
- API documentation for provider interface
- Integration guide for cluster lifecycle usage
- Troubleshooting guide for Apple Container issues

## Risk Mitigation

### Technical Risks
- **Apple Container API Changes**: Abstract CLI interactions for easier updates
- **Performance Issues**: Implement caching and optimization strategies
- **Resource Leaks**: Comprehensive cleanup and resource management

### Integration Risks
- **Provider Interface Changes**: Design for flexibility and extensibility
- **Configuration Incompatibility**: Validate against existing configuration patterns
- **Testing Complexity**: Mock external dependencies appropriately

## Next Phase Integration

This provider implementation enables:
- **Phase 2 Task 2**: Phased cluster lifecycle implementation
- **Phase 2 Task 3**: Kubernetes bootstrap actions
- **Phase 2 Task 4**: Multi-node orchestration
- **Phase 2 Task 5**: Core CLI commands implementation

The provider serves as the foundation for all subsequent cluster management operations in KINA.