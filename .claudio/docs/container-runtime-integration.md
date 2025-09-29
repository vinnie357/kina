# Container Runtime Integration Strategy

**Generated**: 2025-09-14
**Based on**: KIND architectural analysis and Apple Container requirements
**Purpose**: Define container runtime integration strategy for Apple Container in KINA

## Overview

This document outlines the strategy for integrating Apple Container runtime into KINA, adapted from KIND's proven Docker integration patterns. The design provides a provider abstraction layer that enables Apple Container to function as the container runtime for Kubernetes clusters.

## KIND Integration Analysis

### KIND's Provider Pattern

KIND uses a provider abstraction that separates container runtime concerns from cluster management logic:

```go
// KIND's provider interface (adapted for reference)
type Provider interface {
    Create(ctx context.Context, name string, options ...CreateOption) error
    Delete(ctx context.Context, name string) error
    List(ctx context.Context) ([]string, error)
    KubeConfig(ctx context.Context, name string) (*kubeconfig.Config, error)
}
```

### KEY Insights from KIND

1. **Runtime Abstraction**: KIND abstracts Docker operations through a provider interface
2. **Container-as-Node**: Each Kubernetes node runs as a container with systemd
3. **Image Management**: Custom node images with pre-installed Kubernetes components
4. **Network Integration**: Container networking configured for Kubernetes cluster communication
5. **Volume Management**: Host path mounts for persistent storage and configuration

## Apple Container Provider Implementation

### Provider Trait Definition

```rust
// src/container/provider.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait ContainerProvider {
    /// Create a new container with systemd support
    async fn create_node_container(
        &self,
        name: &str,
        config: &NodeContainerConfig,
    ) -> Result<Container, ContainerError>;

    /// Delete a container by name
    async fn delete_container(&self, name: &str) -> Result<(), ContainerError>;

    /// List all containers managed by this provider
    async fn list_containers(&self) -> Result<Vec<ContainerInfo>, ContainerError>;

    /// Execute command in container
    async fn exec_command(
        &self,
        container_name: &str,
        command: &[String],
    ) -> Result<ExecResult, ContainerError>;

    /// Get container networking information
    async fn get_container_network(&self, name: &str) -> Result<NetworkInfo, ContainerError>;

    /// Load image into container runtime
    async fn load_image(&self, image_path: &Path) -> Result<String, ContainerError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeContainerConfig {
    pub name: String,
    pub image: String,
    pub hostname: String,
    pub privileged: bool,
    pub volumes: Vec<VolumeMount>,
    pub port_mappings: Vec<PortMapping>,
    pub environment: Vec<EnvironmentVar>,
    pub systemd_enabled: bool,
}

#[derive(Debug)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub status: ContainerStatus,
    pub network_info: NetworkInfo,
}
```

### Apple Container Implementation

```rust
// src/container/apple_provider.rs
use super::provider::{ContainerProvider, NodeContainerConfig, Container};
use std::process::Command;
use tokio::process::Command as AsyncCommand;

pub struct AppleContainerProvider {
    runtime_path: PathBuf,
}

impl AppleContainerProvider {
    pub fn new() -> Result<Self, ContainerError> {
        // Locate Apple Container CLI
        let runtime_path = Self::find_apple_container_cli()?;
        Ok(Self { runtime_path })
    }

    fn find_apple_container_cli() -> Result<PathBuf, ContainerError> {
        // Search for Apple Container CLI in standard locations
        let possible_paths = vec![
            PathBuf::from("/usr/local/bin/container"),
            PathBuf::from("/opt/apple/container/bin/container"),
            PathBuf::from("/System/Library/PrivateFrameworks/Container.framework/container"),
        ];

        for path in possible_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        Err(ContainerError::RuntimeNotFound(
            "Apple Container CLI not found. Please ensure macOS 15.6+ is installed.".to_string(),
        ))
    }
}

#[async_trait]
impl ContainerProvider for AppleContainerProvider {
    async fn create_node_container(
        &self,
        name: &str,
        config: &NodeContainerConfig,
    ) -> Result<Container, ContainerError> {
        // Build Apple Container creation command
        let mut cmd = AsyncCommand::new(&self.runtime_path);
        cmd.arg("create")
            .arg("--name")
            .arg(name)
            .arg("--hostname")
            .arg(&config.hostname);

        // Configure systemd support (critical for Kubernetes nodes)
        if config.systemd_enabled {
            cmd.arg("--systemd")
                .arg("--tmpfs")
                .arg("/tmp")
                .arg("--tmpfs")
                .arg("/run")
                .arg("--volume")
                .arg("/sys/fs/cgroup:/sys/fs/cgroup:ro");
        }

        // Configure privileged mode for Kubernetes operations
        if config.privileged {
            cmd.arg("--privileged");
        }

        // Configure volume mounts (following KIND patterns)
        for volume in &config.volumes {
            cmd.arg("--volume")
                .arg(format!("{}:{}:{}", volume.host_path, volume.container_path, volume.mode));
        }

        // Configure port mappings
        for port in &config.port_mappings {
            cmd.arg("--publish")
                .arg(format!("{}:{}/{}", port.host_port, port.container_port, port.protocol));
        }

        // Set environment variables
        for env in &config.environment {
            cmd.arg("--env")
                .arg(format!("{}={}", env.name, env.value));
        }

        // Specify the node image
        cmd.arg(&config.image);

        // Execute container creation
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(ContainerError::CreationFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Start the container
        self.start_container(&container_id).await?;

        // Get container information
        let container_info = self.inspect_container(&container_id).await?;

        Ok(Container {
            id: container_id,
            name: name.to_string(),
            status: ContainerStatus::Running,
            network_info: container_info.network_info,
        })
    }

    async fn exec_command(
        &self,
        container_name: &str,
        command: &[String],
    ) -> Result<ExecResult, ContainerError> {
        let mut cmd = AsyncCommand::new(&self.runtime_path);
        cmd.arg("exec")
            .arg(container_name)
            .args(command);

        let output = cmd.output().await?;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    async fn list_containers(&self) -> Result<Vec<ContainerInfo>, ContainerError> {
        let mut cmd = AsyncCommand::new(&self.runtime_path);
        cmd.arg("list")
            .arg("--format")
            .arg("json")
            .arg("--filter")
            .arg("label=io.x-k8s.kind.cluster"); // Filter for KIND-compatible containers

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(ContainerError::ListFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let containers: Vec<ContainerInfo> =
            serde_json::from_slice(&output.stdout)?;

        Ok(containers)
    }
}
```

### Node Image Building Strategy

```rust
// src/image/builder.rs - Apple Container node images
pub struct AppleNodeImageBuilder {
    provider: AppleContainerProvider,
    base_image: String,
    kubernetes_version: String,
}

impl AppleNodeImageBuilder {
    pub async fn build_node_image(
        &self,
        image_name: &str,
    ) -> Result<String, ImageBuildError> {
        // Create build configuration following KIND's Dockerfile patterns
        let build_config = self.create_build_configuration()?;

        // Build the image using Apple Container native building
        self.execute_apple_container_build(image_name, build_config).await
    }

    fn create_build_configuration(&self) -> Result<AppleContainerBuildConfig, ImageBuildError> {
        // Apple Container build configuration
        let config = AppleContainerBuildConfig {
            base_image: self.base_image.clone(),
            steps: vec![
                BuildStep::Run("apt-get update && apt-get install -y systemd"),
                BuildStep::Run(&format!(
                    "curl -LO https://dl.k8s.io/release/v{}/bin/linux/amd64/kubeadm",
                    self.kubernetes_version
                )),
                BuildStep::Run(&format!(
                    "curl -LO https://dl.k8s.io/release/v{}/bin/linux/amd64/kubelet",
                    self.kubernetes_version
                )),
                BuildStep::Run(&format!(
                    "curl -LO https://dl.k8s.io/release/v{}/bin/linux/amd64/kubectl",
                    self.kubernetes_version
                )),
                BuildStep::Run("chmod +x kubeadm kubelet kubectl"),
                BuildStep::Run("mv kubeadm kubelet kubectl /usr/bin/"),
                BuildStep::Copy("kubeadm-config.yaml", "/etc/kubernetes/kubeadm-config.yaml"),
                BuildStep::Run("systemctl enable kubelet"),
                BuildStep::Cmd(vec!["systemd".to_string()]),
            ],
        };

        Ok(config)
    }

    async fn execute_apple_container_build(
        &self,
        image_name: &str,
        config: AppleContainerBuildConfig,
    ) -> Result<String, ImageBuildError> {
        // Convert build configuration to Apple Container build command
        let mut cmd = AsyncCommand::new(&self.provider.runtime_path);
        cmd.arg("build")
            .arg("--tag")
            .arg(image_name)
            .arg("--base-image")
            .arg(&config.base_image);

        // Add build steps
        for step in config.steps {
            match step {
                BuildStep::Run(command) => {
                    cmd.arg("--run").arg(command);
                }
                BuildStep::Copy(src, dest) => {
                    cmd.arg("--copy").arg(format!("{}:{}", src, dest));
                }
                BuildStep::Cmd(args) => {
                    cmd.arg("--cmd").arg(args.join(" "));
                }
            }
        }

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(ImageBuildError::BuildFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(image_name.to_string())
    }
}
```

### Network Integration

```rust
// src/cluster/networking.rs - Apple Container networking
pub struct AppleContainerNetworking {
    provider: AppleContainerProvider,
}

impl AppleContainerNetworking {
    pub async fn setup_cluster_networking(
        &self,
        containers: &[Container],
        config: &NetworkingConfig,
    ) -> Result<NetworkSetup, NetworkError> {
        // 1. Configure container bridge networking
        self.setup_container_bridge().await?;

        // 2. Configure Kubernetes networking (CNI)
        self.setup_kubernetes_cni(containers, config).await?;

        // 3. Configure service networking
        self.setup_service_networking(config).await?;

        // 4. Set up load balancer for API server
        self.setup_api_server_lb(containers, config).await?;

        Ok(NetworkSetup::new())
    }

    async fn setup_container_bridge(&self) -> Result<(), NetworkError> {
        // Create Apple Container bridge network
        let mut cmd = AsyncCommand::new(&self.provider.runtime_path);
        cmd.arg("network")
            .arg("create")
            .arg("--driver")
            .arg("bridge")
            .arg("kina");

        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(NetworkError::BridgeSetupFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    async fn setup_kubernetes_cni(
        &self,
        containers: &[Container],
        config: &NetworkingConfig,
    ) -> Result<(), NetworkError> {
        for container in containers {
            // Install CNI plugins in each container
            let cni_install_cmd = vec![
                "bash".to_string(),
                "-c".to_string(),
                "curl -L https://github.com/containernetworking/plugins/releases/download/v1.3.0/cni-plugins-linux-amd64-v1.3.0.tgz | tar -xz -C /opt/cni/bin/".to_string(),
            ];

            self.provider
                .exec_command(&container.name, &cni_install_cmd)
                .await?;
        }

        Ok(())
    }
}
```

### Cluster Lifecycle with Apple Container

```rust
// src/cluster/manager.rs - Apple Container cluster management
pub struct AppleClusterManager {
    provider: AppleContainerProvider,
    image_builder: AppleNodeImageBuilder,
    networking: AppleContainerNetworking,
}

impl AppleClusterManager {
    pub async fn create_cluster(
        &self,
        name: &str,
        config: &ClusterConfig,
    ) -> Result<Cluster, ClusterError> {
        // Phase 1: Create - Prepare node image and containers
        let node_image = self.prepare_node_image(config).await?;
        let containers = self.create_node_containers(name, config, &node_image).await?;

        // Phase 2: Configure - Set up networking and storage
        self.configure_cluster_infrastructure(&containers, config).await?;

        // Phase 3: Bootstrap - Initialize Kubernetes with kubeadm
        self.bootstrap_kubernetes_cluster(&containers, config).await?;

        // Phase 4: Join - Finalize cluster setup
        self.finalize_cluster_setup(&containers, config).await?;

        Ok(Cluster::new(name.to_string(), containers))
    }

    async fn prepare_node_image(&self, config: &ClusterConfig) -> Result<String, ClusterError> {
        let image_name = format!("kina/node:{}", config.kubernetes_version);

        // Check if image already exists
        if !self.image_exists(&image_name).await? {
            self.image_builder.build_node_image(&image_name).await?;
        }

        Ok(image_name)
    }

    async fn create_node_containers(
        &self,
        cluster_name: &str,
        config: &ClusterConfig,
        node_image: &str,
    ) -> Result<Vec<Container>, ClusterError> {
        let mut containers = Vec::new();

        for (index, node_config) in config.nodes.iter().enumerate() {
            let container_name = format!("{}-{}", cluster_name, index);

            let container_config = NodeContainerConfig {
                name: container_name.clone(),
                image: node_image.to_string(),
                hostname: container_name.clone(),
                privileged: true, // Required for Kubernetes
                systemd_enabled: true, // Critical for Kubernetes nodes
                volumes: self.create_node_volumes(cluster_name, node_config)?,
                port_mappings: self.create_port_mappings(node_config)?,
                environment: vec![
                    EnvironmentVar::new("container", "apple-container"),
                    EnvironmentVar::new("KUBECONFIG", "/etc/kubernetes/admin.conf"),
                ],
            };

            let container = self.provider
                .create_node_container(&container_name, &container_config)
                .await?;

            containers.push(container);
        }

        Ok(containers)
    }

    async fn bootstrap_kubernetes_cluster(
        &self,
        containers: &[Container],
        config: &ClusterConfig,
    ) -> Result<(), ClusterError> {
        if let Some(control_plane) = containers.iter().find(|c| c.name.contains("control-plane")) {
            // Initialize control plane with kubeadm
            let kubeadm_init = vec![
                "kubeadm".to_string(),
                "init".to_string(),
                "--config".to_string(),
                "/etc/kubernetes/kubeadm-config.yaml".to_string(),
                "--skip-phases".to_string(),
                "preflight".to_string(), // Skip some checks that don't apply to containers
            ];

            let result = self.provider
                .exec_command(&control_plane.name, &kubeadm_init)
                .await?;

            if result.exit_code != 0 {
                return Err(ClusterError::KubernetesInitFailed(result.stderr));
            }

            // Configure kubectl for the container
            let kubectl_config = vec![
                "bash".to_string(),
                "-c".to_string(),
                "mkdir -p /root/.kube && cp /etc/kubernetes/admin.conf /root/.kube/config".to_string(),
            ];

            self.provider
                .exec_command(&control_plane.name, &kubectl_config)
                .await?;
        }

        Ok(())
    }
}
```

### Apple Container Specific Optimizations

```rust
// src/container/optimizations.rs
pub struct AppleContainerOptimizations;

impl AppleContainerOptimizations {
    /// Configure Apple Container specific performance optimizations
    pub fn configure_performance_settings() -> ContainerSettings {
        ContainerSettings {
            // Use Apple's native container acceleration
            native_acceleration: true,

            // Optimize for macOS filesystem performance
            filesystem_optimization: FileSystemOpt::AppleFS,

            // Configure memory management for macOS
            memory_management: MemoryOpt::UnifiedMemory,

            // Use Apple's container networking stack
            networking_stack: NetworkStack::Apple,
        }
    }

    /// Configure security settings leveraging Apple's container security
    pub fn configure_security_settings() -> SecuritySettings {
        SecuritySettings {
            // Use Apple's container sandboxing
            sandbox: SandboxType::Apple,

            // Leverage System Integrity Protection
            sip_integration: true,

            // Use Apple's secure container runtime
            secure_runtime: true,
        }
    }
}
```

## Integration Testing Strategy

```rust
// tests/integration/apple_container_tests.rs
#[tokio::test]
async fn test_apple_container_provider_basic_operations() {
    let provider = AppleContainerProvider::new().unwrap();

    let config = NodeContainerConfig {
        name: "test-node".to_string(),
        image: "alpine:latest".to_string(),
        systemd_enabled: false, // Simple test without systemd
        // ... other config
    };

    // Test container creation
    let container = provider.create_node_container("test-node", &config).await.unwrap();
    assert_eq!(container.name, "test-node");

    // Test command execution
    let result = provider.exec_command("test-node", &["echo", "hello"]).await.unwrap();
    assert_eq!(result.stdout.trim(), "hello");

    // Test container deletion
    provider.delete_container("test-node").await.unwrap();
}

#[tokio::test]
async fn test_kubernetes_node_container() {
    // Test creating a full Kubernetes node container with systemd
    let provider = AppleContainerProvider::new().unwrap();
    let builder = AppleNodeImageBuilder::new(provider.clone(), "v1.27.3");

    // Build node image
    let image = builder.build_node_image("test-kina-node").await.unwrap();

    // Create container with Kubernetes components
    let config = NodeContainerConfig::kubernetes_node("test-k8s-node", &image);
    let container = provider.create_node_container("test-k8s-node", &config).await.unwrap();

    // Verify Kubernetes components are available
    let result = provider.exec_command("test-k8s-node", &["kubelet", "--version"]).await.unwrap();
    assert!(result.stdout.contains("v1.27.3"));
}
```

## Migration from KIND

### Compatibility Layer

```rust
// src/compat/kind.rs - KIND compatibility layer
pub struct KindCompatibilityLayer {
    provider: AppleContainerProvider,
}

impl KindCompatibilityLayer {
    /// Convert KIND cluster configuration to KINA format
    pub fn convert_kind_config(kind_config: &str) -> Result<ClusterConfig, ConfigError> {
        let kind_config: KindConfig = serde_yaml::from_str(kind_config)?;

        let kina_config = ClusterConfig {
            kind: kind_config.kind,
            api_version: kind_config.api_version,
            metadata: kind_config.metadata,
            nodes: kind_config.nodes.into_iter()
                .map(|node| self.convert_kind_node(node))
                .collect(),
            networking: kind_config.networking,
        };

        Ok(kina_config)
    }

    /// Detect existing KIND clusters and provide migration guidance
    pub async fn detect_kind_clusters(&self) -> Result<Vec<KindClusterInfo>, MigrationError> {
        // Scan for KIND Docker containers
        let docker_containers = self.scan_docker_kind_containers().await?;

        // Analyze configurations
        let cluster_info = docker_containers.into_iter()
            .map(|container| self.analyze_kind_container(container))
            .collect();

        Ok(cluster_info)
    }
}
```

This container runtime integration strategy provides a solid foundation for Apple Container integration while maintaining compatibility with KIND's proven patterns and workflows.