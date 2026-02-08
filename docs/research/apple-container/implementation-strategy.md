# Implementation Strategy for kina CLI

**Focus**: CLI design patterns, module architecture, and development roadmap

## Architecture Overview

```
KINA CLI Architecture with Apple Container Integration:

┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   kina CLI      │    │ Custom CRI Shim │    │ Apple Container │
│   (Rust)        │◄──►│   (Rust/gRPC)   │◄──►│  Framework      │
│                 │    │                 │    │   (Swift)       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  kube-rs Client │    │  gRPC Server    │    │ Virtualization  │
│  (K8s API)      │    │  (CRI Protocol) │    │  Framework      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Module Structure

```rust
// kina-cli/src/lib.rs - Primary module organization
pub mod cli;           // Command-line interface and argument parsing
pub mod config;        // Configuration management and validation
pub mod core {
    pub mod apple_container;    // Apple Container FFI bindings
    pub mod cri_shim;          // Custom CRI implementation
    pub mod kubernetes;        // kube-rs client integration
    pub mod cluster;           // Cluster lifecycle management
    pub mod provider;          // Container provider abstraction
}
pub mod utils;         // Utility functions and helpers
pub mod errors;        // Error handling and custom error types
```

## Command Line Interface Design

### Primary CLI Structure

```rust
// kina-cli/src/cli/mod.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kina")]
#[command(about = "Kubernetes in Apple Container - Local cluster management")]
#[command(version)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<std::path::PathBuf>,

    /// Container provider (apple-container, docker)
    #[arg(long, global = true, default_value = "apple-container")]
    pub provider: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage Kubernetes clusters
    Cluster(cluster::ClusterCommand),

    /// Manage container images
    Image(image::ImageCommand),

    /// Configuration management
    Config(config::ConfigCommand),

    /// Load container images into cluster
    Load(load::LoadCommand),

    /// Export cluster kubeconfig
    Export(export::ExportCommand),

    /// Get cluster information
    Get(get::GetCommand),

    /// Start CRI server (daemon mode)
    CriServer(cri::CriServerCommand),

    /// Debug and diagnostic commands
    Debug(debug::DebugCommand),
}
```

### Cluster Management Commands

```rust
// kina-cli/src/cli/cluster.rs
use clap::{Args, Subcommand};
use crate::core::cluster::{ClusterConfig, KinaCluster};

#[derive(Debug, Args)]
pub struct ClusterCommand {
    #[command(subcommand)]
    pub action: ClusterAction,
}

#[derive(Debug, Subcommand)]
pub enum ClusterAction {
    Create(CreateArgs),
    Delete(DeleteArgs),
    Start(StartArgs),
    Stop(StopArgs),
    List,
    Status(StatusArgs),
    Logs(LogsArgs),
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Cluster name
    pub name: String,

    /// Number of worker nodes
    #[arg(short, long, default_value = "1")]
    pub workers: u32,

    /// Kubernetes version
    #[arg(long, default_value = "v1.28.0")]
    pub k8s_version: String,

    /// Container image for nodes
    #[arg(long, default_value = "kindest/node")]
    pub image: String,

    /// Control plane configuration
    #[arg(long)]
    pub control_plane_endpoint: Option<String>,

    /// Pod subnet CIDR
    #[arg(long, default_value = "10.244.0.0/16")]
    pub pod_subnet: String,

    /// Service subnet CIDR
    #[arg(long, default_value = "10.96.0.0/12")]
    pub service_subnet: String,

    /// Wait for cluster to be ready
    #[arg(long)]
    pub wait: bool,

    /// Resource configuration
    #[command(flatten)]
    pub resources: ResourceArgs,
}

#[derive(Debug, Args)]
pub struct ResourceArgs {
    /// CPU limit per node
    #[arg(long)]
    pub cpu: Option<f64>,

    /// Memory limit per node (MB)
    #[arg(long)]
    pub memory: Option<u64>,

    /// Disk size per node (GB)
    #[arg(long)]
    pub disk: Option<u64>,
}

impl ClusterCommand {
    pub async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.action {
            ClusterAction::Create(args) => {
                println!("Creating cluster '{}' with {} workers", args.name, args.workers);

                let cluster_config = ClusterConfig {
                    name: args.name.clone(),
                    worker_nodes: args.workers,
                    kubernetes_version: args.k8s_version.clone(),
                    node_image: format!("{}:{}", args.image, args.k8s_version),
                    control_plane_endpoint: args.control_plane_endpoint.clone(),
                    pod_subnet: args.pod_subnet.clone(),
                    service_subnet: args.service_subnet.clone(),
                    resource_limits: self.build_resource_limits(args),
                };

                let mut cluster = KinaCluster::new(cluster_config).await?;
                cluster.create().await?;

                if args.wait {
                    cluster.wait_for_ready().await?;
                }

                println!("✅ Cluster '{}' created successfully", args.name);
                self.print_cluster_info(&cluster).await?;

                Ok(())
            }
            ClusterAction::Delete(args) => {
                println!("Deleting cluster '{}'", args.name);

                let cluster = KinaCluster::load(&args.name).await?;
                cluster.delete().await?;

                println!("✅ Cluster '{}' deleted", args.name);
                Ok(())
            }
            ClusterAction::List => {
                let clusters = KinaCluster::list_all().await?;

                if clusters.is_empty() {
                    println!("No clusters found");
                    return Ok(());
                }

                println!("NAME\t\tSTATUS\t\tNODES\t\tVERSION");
                for cluster in clusters {
                    println!("{}\t\t{}\t\t{}\t\t{}",
                        cluster.name(),
                        cluster.status().await?,
                        cluster.node_count().await?,
                        cluster.kubernetes_version()
                    );
                }
                Ok(())
            }
            // Additional command implementations...
        }
    }

    fn build_resource_limits(&self, args: &CreateArgs) -> crate::core::cluster::ResourceLimits {
        crate::core::cluster::ResourceLimits {
            cpu_cores: args.resources.cpu.unwrap_or(1.0),
            memory_mb: args.resources.memory.unwrap_or(2048),
            disk_gb: args.resources.disk.unwrap_or(20),
        }
    }

    async fn print_cluster_info(&self, cluster: &KinaCluster) -> Result<(), Box<dyn std::error::Error>> {
        println!("\nCluster Information:");
        println!("  Name: {}", cluster.name());
        println!("  Status: {}", cluster.status().await?);
        println!("  Kubernetes Version: {}", cluster.kubernetes_version());
        println!("  Control Plane Endpoint: {}", cluster.control_plane_endpoint().await?);
        println!("  Nodes: {}", cluster.node_count().await?);

        println!("\nTo interact with this cluster:");
        println!("  export KUBECONFIG=$(kina export kubeconfig {})", cluster.name());
        println!("  kubectl cluster-info");

        Ok(())
    }
}
```

## Core Cluster Management

### Cluster Lifecycle Implementation

```rust
// kina-cli/src/core/cluster.rs
use crate::core::apple_container::KinaContainerManager;
use crate::core::kubernetes::KubernetesClient;
use crate::core::provider::{ContainerProvider, AppleContainerProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub name: String,
    pub worker_nodes: u32,
    pub kubernetes_version: String,
    pub node_image: String,
    pub control_plane_endpoint: Option<String>,
    pub pod_subnet: String,
    pub service_subnet: String,
    pub cni_plugin: String,
    pub feature_gates: Vec<String>,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub disk_gb: u64,
}

pub struct KinaCluster {
    config: ClusterConfig,
    provider: Box<dyn ContainerProvider>,
    k8s_client: Option<KubernetesClient>,
    nodes: RwLock<HashMap<String, NodeInfo>>,
    state_path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    container_id: String,
    role: NodeRole,
    ip_address: std::net::IpAddr,
    status: NodeStatus,
}

#[derive(Debug, Clone)]
enum NodeRole {
    ControlPlane,
    Worker,
}

#[derive(Debug, Clone)]
enum NodeStatus {
    Creating,
    Ready,
    NotReady,
    Failed(String),
}

impl KinaCluster {
    pub async fn new(config: ClusterConfig) -> Result<Self, crate::errors::KinaError> {
        let provider = AppleContainerProvider::new().await?;
        let state_path = Self::cluster_state_path(&config.name);

        Ok(Self {
            config,
            provider: Box::new(provider),
            k8s_client: None,
            nodes: RwLock::new(HashMap::new()),
            state_path,
        })
    }

    pub async fn create(&mut self) -> Result<(), crate::errors::KinaError> {
        println!("🚀 Creating cluster '{}'", self.config.name);

        // Step 1: Create control plane node
        println!("📦 Creating control plane node");
        let control_plane = self.create_control_plane_node().await?;

        // Step 2: Initialize Kubernetes cluster
        println!("⚙️  Initializing Kubernetes cluster");
        self.initialize_kubernetes(&control_plane).await?;

        // Step 3: Configure networking
        println!("🌐 Setting up cluster networking");
        self.setup_cluster_networking().await?;

        // Step 4: Create worker nodes
        if self.config.worker_nodes > 0 {
            println!("👷 Creating {} worker node(s)", self.config.worker_nodes);
            self.create_worker_nodes().await?;
        }

        // Step 5: Save cluster state
        self.save_cluster_state().await?;

        println!("✅ Cluster '{}' created successfully", self.config.name);
        Ok(())
    }

    async fn create_control_plane_node(&self) -> Result<String, crate::errors::KinaError> {
        let container_config = self.build_control_plane_config()?;
        let container_id = self.provider.create_container(&container_config).await?;

        // Start the control plane container
        self.provider.start_container(&container_id).await?;

        // Wait for container to be ready
        self.wait_for_container_ready(&container_id).await?;

        // Update nodes cache
        let mut nodes = self.nodes.write().await;
        nodes.insert("control-plane".to_string(), NodeInfo {
            container_id: container_id.clone(),
            role: NodeRole::ControlPlane,
            ip_address: self.get_container_ip(&container_id).await?,
            status: NodeStatus::Ready,
        });

        Ok(container_id)
    }

    async fn initialize_kubernetes(&mut self, control_plane_id: &str) -> Result<(), crate::errors::KinaError> {
        // Generate kubeadm configuration
        let kubeadm_config = self.generate_kubeadm_config().await?;

        // Write kubeadm config to control plane
        self.provider.exec_container(
            control_plane_id,
            &["sh", "-c", &format!("cat > /tmp/kubeadm.yaml << 'EOF'\n{}\nEOF", kubeadm_config)]
        ).await?;

        // Initialize cluster with kubeadm
        self.provider.exec_container(
            control_plane_id,
            &["kubeadm", "init", "--config=/tmp/kubeadm.yaml", "--skip-phases=addon/kube-proxy"]
        ).await?;

        // Extract admin kubeconfig
        let kubeconfig = self.provider.exec_container(
            control_plane_id,
            &["cat", "/etc/kubernetes/admin.conf"]
        ).await?;

        // Save kubeconfig locally
        self.save_kubeconfig(&kubeconfig).await?;

        // Initialize Kubernetes client
        self.k8s_client = Some(KubernetesClient::new(&kubeconfig).await?);

        Ok(())
    }

    async fn setup_cluster_networking(&self) -> Result<(), crate::errors::KinaError> {
        let k8s_client = self.k8s_client.as_ref()
            .ok_or_else(|| crate::errors::KinaError::ClusterNotReady)?;

        // Install CNI plugin (Cilium for Apple Container compatibility)
        let cilium_manifest = self.generate_cilium_manifest().await?;
        k8s_client.apply_manifest(&cilium_manifest).await?;

        // Wait for CNI to be ready
        k8s_client.wait_for_pods_ready("kube-system", "k8s-app=cilium").await?;

        Ok(())
    }

    async fn create_worker_nodes(&self) -> Result<(), crate::errors::KinaError> {
        // Get join token from control plane
        let control_plane = self.nodes.read().await;
        let cp_node = control_plane.get("control-plane")
            .ok_or_else(|| crate::errors::KinaError::NodeNotFound("control-plane".to_string()))?;

        let join_command = self.provider.exec_container(
            &cp_node.container_id,
            &["kubeadm", "token", "create", "--print-join-command"]
        ).await?;

        drop(control_plane); // Release read lock

        // Create worker nodes in parallel
        let mut worker_futures = Vec::new();

        for i in 0..self.config.worker_nodes {
            let worker_name = format!("worker-{}", i + 1);
            let join_cmd = join_command.clone();
            let provider = &self.provider;

            let future = async move {
                let container_config = self.build_worker_config(&worker_name)?;
                let container_id = provider.create_container(&container_config).await?;

                provider.start_container(&container_id).await?;
                self.wait_for_container_ready(&container_id).await?;

                // Join cluster
                provider.exec_container(&container_id, &["sh", "-c", &join_cmd]).await?;

                Ok::<(String, String), crate::errors::KinaError>((worker_name, container_id))
            };

            worker_futures.push(future);
        }

        // Wait for all workers to be created
        let results = futures::future::try_join_all(worker_futures).await?;

        // Update nodes cache with worker information
        let mut nodes = self.nodes.write().await;
        for (worker_name, container_id) in results {
            nodes.insert(worker_name, NodeInfo {
                container_id: container_id.clone(),
                role: NodeRole::Worker,
                ip_address: self.get_container_ip(&container_id).await?,
                status: NodeStatus::Ready,
            });
        }

        Ok(())
    }

    fn build_control_plane_config(&self) -> Result<crate::core::apple_container::ContainerConfig, crate::errors::KinaError> {
        Ok(crate::core::apple_container::ContainerConfig {
            image: self.config.node_image.clone(),
            command: vec![], // Use default entrypoint
            environment: self.build_node_environment("control-plane")?,
            volumes: vec![
                crate::core::apple_container::VolumeMount {
                    source: self.cluster_data_path().to_string_lossy().to_string(),
                    target: "/var/lib/kina".to_string(),
                    read_only: false,
                }
            ],
            privileged: true, // Required for Kubernetes node
            resource_limits: Some(self.config.resource_limits.clone().into()),
        })
    }

    fn build_worker_config(&self, worker_name: &str) -> Result<crate::core::apple_container::ContainerConfig, crate::errors::KinaError> {
        Ok(crate::core::apple_container::ContainerConfig {
            image: self.config.node_image.clone(),
            command: vec![],
            environment: self.build_node_environment(worker_name)?,
            volumes: vec![
                crate::core::apple_container::VolumeMount {
                    source: self.cluster_data_path().to_string_lossy().to_string(),
                    target: "/var/lib/kina".to_string(),
                    read_only: false,
                }
            ],
            privileged: true,
            resource_limits: Some(self.config.resource_limits.clone().into()),
        })
    }

    fn build_node_environment(&self, node_name: &str) -> Result<HashMap<String, String>, crate::errors::KinaError> {
        let mut env = HashMap::new();

        env.insert("NODE_NAME".to_string(), node_name.to_string());
        env.insert("CLUSTER_NAME".to_string(), self.config.name.clone());
        env.insert("POD_SUBNET".to_string(), self.config.pod_subnet.clone());
        env.insert("SERVICE_SUBNET".to_string(), self.config.service_subnet.clone());

        // Feature gates
        if !self.config.feature_gates.is_empty() {
            env.insert("FEATURE_GATES".to_string(), self.config.feature_gates.join(","));
        }

        Ok(env)
    }

    async fn generate_kubeadm_config(&self) -> Result<String, crate::errors::KinaError> {
        let config = format!(r#"
apiVersion: kubeadm.k8s.io/v1beta3
kind: InitConfiguration
nodeRegistration:
  criSocket: unix:///var/run/kina-cri.sock
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: ClusterConfiguration
kubernetesVersion: {version}
networking:
  podSubnet: {pod_subnet}
  serviceSubnet: {service_subnet}
apiServer:
  certSANs:
    - localhost
    - 127.0.0.1
controllerManager:
  extraArgs:
    bind-address: "0.0.0.0"
scheduler:
  extraArgs:
    bind-address: "0.0.0.0"
etcd:
  local:
    extraArgs:
      listen-metrics-urls: "http://0.0.0.0:2381"
---
apiVersion: kubeproxy.config.k8s.io/v1alpha1
kind: KubeProxyConfiguration
bindAddress: "0.0.0.0"
metricsBindAddress: "0.0.0.0:10249"
"#,
            version = self.config.kubernetes_version,
            pod_subnet = self.config.pod_subnet,
            service_subnet = self.config.service_subnet
        );

        Ok(config)
    }

    // Additional helper methods...
}
```

## Provider Abstraction

```rust
// kina-cli/src/core/provider.rs
use async_trait::async_trait;
use crate::core::apple_container::ContainerConfig;

#[async_trait]
pub trait ContainerProvider: Send + Sync {
    async fn create_container(&self, config: &ContainerConfig) -> Result<String, crate::errors::KinaError>;
    async fn start_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError>;
    async fn stop_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError>;
    async fn remove_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError>;
    async fn exec_container(&self, container_id: &str, command: &[&str]) -> Result<String, crate::errors::KinaError>;
    async fn get_container_ip(&self, container_id: &str) -> Result<std::net::IpAddr, crate::errors::KinaError>;
    async fn list_containers(&self) -> Result<Vec<String>, crate::errors::KinaError>;
}

pub struct AppleContainerProvider {
    manager: crate::core::apple_container::KinaContainerManager,
}

impl AppleContainerProvider {
    pub async fn new() -> Result<Self, crate::errors::KinaError> {
        Ok(Self {
            manager: crate::core::apple_container::KinaContainerManager::new(),
        })
    }
}

#[async_trait]
impl ContainerProvider for AppleContainerProvider {
    async fn create_container(&self, config: &ContainerConfig) -> Result<String, crate::errors::KinaError> {
        self.manager.create_container(config).await
    }

    async fn start_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError> {
        self.manager.start_container(container_id).await
    }

    async fn stop_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError> {
        self.manager.stop_container(container_id).await
    }

    async fn remove_container(&self, container_id: &str) -> Result<(), crate::errors::KinaError> {
        self.manager.remove_container(container_id).await
    }

    async fn exec_container(&self, container_id: &str, command: &[&str]) -> Result<String, crate::errors::KinaError> {
        self.manager.exec_container(container_id, command).await
    }

    async fn get_container_ip(&self, container_id: &str) -> Result<std::net::IpAddr, crate::errors::KinaError> {
        self.manager.get_container_ip(container_id).await
    }

    async fn list_containers(&self) -> Result<Vec<String>, crate::errors::KinaError> {
        self.manager.list_containers().await
    }
}
```

## Configuration Management

```rust
// kina-cli/src/config/mod.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinaConfig {
    pub default_provider: String,
    pub clusters: Vec<ClusterConfig>,
    pub apple_container: AppleContainerConfig,
    pub kubernetes: KubernetesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleContainerConfig {
    pub runtime_path: PathBuf,
    pub image_storage_path: PathBuf,
    pub vm_resource_limits: ResourceLimits,
    pub cri_server: CRIServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIServerConfig {
    pub socket_path: PathBuf,
    pub log_level: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub default_version: String,
    pub node_image_repository: String,
    pub cni_plugin: String,
    pub feature_gates: Vec<String>,
}

impl Default for KinaConfig {
    fn default() -> Self {
        Self {
            default_provider: "apple-container".to_string(),
            clusters: vec![],
            apple_container: AppleContainerConfig {
                runtime_path: PathBuf::from("/usr/local/bin/container"),
                image_storage_path: PathBuf::from(dirs::home_dir().unwrap().join(".kina/images")),
                vm_resource_limits: ResourceLimits {
                    cpu_cores: 2.0,
                    memory_mb: 2048,
                    disk_gb: 20,
                },
                cri_server: CRIServerConfig {
                    socket_path: PathBuf::from("/var/run/kina-cri.sock"),
                    log_level: "info".to_string(),
                    timeout_seconds: 60,
                },
            },
            kubernetes: KubernetesConfig {
                default_version: "v1.28.0".to_string(),
                node_image_repository: "kindest/node".to_string(),
                cni_plugin: "cilium".to_string(),
                feature_gates: vec![],
            },
        }
    }
}

impl KinaConfig {
    pub fn load() -> Result<Self, crate::errors::KinaError> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| crate::errors::KinaError::ConfigError(e.to_string()))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| crate::errors::KinaError::ConfigError(e.to_string()))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<(), crate::errors::KinaError> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::errors::KinaError::ConfigError(e.to_string()))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::errors::KinaError::ConfigError(e.to_string()))?;

        std::fs::write(&config_path, content)
            .map_err(|e| crate::errors::KinaError::ConfigError(e.to_string()))?;

        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kina")
            .join("config.toml")
    }
}
```

## Development Phases

### Phase 1: Foundation (Weeks 1-2)
1. **Core CLI Structure**
   - Implement basic command parsing with clap
   - Configuration management system
   - Error handling and logging framework

2. **Apple Container Integration**
   - Swift-Rust FFI bindings using swift-bridge
   - Basic container lifecycle operations
   - Container provider abstraction layer

3. **Testing Framework**
   - Unit test structure
   - Integration test harness
   - CI/CD pipeline setup

### Phase 2: Core Functionality (Weeks 3-4)
1. **CRI Shim Implementation**
   - gRPC server for CRI protocol
   - Pod sandbox management
   - Container lifecycle integration

2. **Cluster Management**
   - Single-node cluster creation
   - Kubernetes initialization with kubeadm
   - CNI plugin integration (Cilium)

3. **Kubernetes Integration**
   - kube-rs client implementation
   - Kubeconfig management
   - Cluster status monitoring

### Phase 3: Advanced Features (Weeks 5-6)
1. **Multi-Node Support**
   - Worker node creation and joining
   - Node lifecycle management
   - Load balancing and HA considerations

2. **Advanced CLI Features**
   - Image management commands
   - Debug and diagnostic tools
   - Configuration validation and migration

3. **Production Readiness**
   - Comprehensive error handling
   - Performance optimization
   - Documentation and user guides