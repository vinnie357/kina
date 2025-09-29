# Phase 2: Core Features - Apple Container Provider and Phased Lifecycle Management

**Phase Objectives**: Implement Apple Container provider following KIND's phased lifecycle patterns (Create → Configure → Bootstrap → Join) and establish core cluster management functionality

## Phase Overview
This phase translates KIND's proven cluster lifecycle management patterns to Apple Container runtime, implementing the provider pattern with phased cluster creation: container provisioning, configuration setup, Kubernetes bootstrapping, and multi-node joining orchestration.

## Key Deliverables
- Apple Container provider implementation following KIND's provider pattern
- Phased cluster lifecycle management (Create → Configure → Bootstrap → Join)
- Core kina CLI commands operational (create, delete, get)
- Container image and networking management with KIND-compatible patterns
- kubeadm integration for cluster initialization

## Task Breakdown

### Task 1: Apple Container Provider Implementation
**Objective**: Implement ContainerProvider trait for Apple Container runtime following KIND's provider abstraction
**Dependencies**: Phase 1 Tasks 1-3
**Acceptance Criteria**:
- AppleContainerProvider implements full ContainerProvider interface
- Container lifecycle operations (create, start, stop, delete) functional
- Error handling provides clear diagnostic information
- Async operations support concurrent container management

**Implementation Notes**:
```rust
use async_trait::async_trait;
use futures::future::try_join_all;

pub struct AppleContainerProvider {
    runtime: AppleContainerRuntime,
    logger: Logger,
    network_manager: NetworkManager,
}

#[async_trait]
impl ContainerProvider for AppleContainerProvider {
    async fn provision(&self, config: &ClusterConfig) -> Result<(), KinaError> {
        // Phase 1: Ensure node images are available
        self.ensure_node_images(config).await?;

        // Phase 2: Create Apple Container network
        let network = self.ensure_container_network(&config.name).await?;

        // Phase 3: Create containers concurrently (KIND pattern)
        let container_futures: Vec<_> = config.nodes.iter()
            .map(|node_config| self.create_node_container(node_config, &network))
            .collect();

        try_join_all(container_futures).await?;
        Ok(())
    }

    async fn list_nodes(&self, cluster: &str) -> Result<Vec<Node>, KinaError> {
        let containers = self.runtime.list_containers_by_label(
            &[("io.kina.cluster", cluster)]
        ).await?;

        Ok(containers.into_iter().map(|c| Node::from(c)).collect())
    }
}

impl AppleContainerProvider {
    async fn create_node_container(
        &self,
        node_config: &NodeConfig,
        network: &ContainerNetwork,
    ) -> Result<Container, KinaError> {
        let container_spec = ContainerSpec::builder()
            .image(&node_config.image.as_ref().unwrap_or(&DEFAULT_NODE_IMAGE))
            .hostname(&node_config.name)
            .network(network.id())
            .privileged(true)  // Required for systemd and Kubernetes
            .labels(&[
                ("io.kina.cluster", &node_config.cluster_name),
                ("io.kina.role", &node_config.role.to_string()),
            ])
            // Essential volume mounts for Kubernetes (KIND pattern)
            .volume("/var/lib/kubelet", "/var/lib/kubelet")
            .volume("/etc/kubernetes", "/etc/kubernetes")
            .volume("/sys/fs/cgroup", "/sys/fs/cgroup")
            .build()?;

        self.runtime.create_container(container_spec).await
    }
}
```

**Deliverables**:
- Complete AppleContainerProvider implementation with all trait methods
- Container specification builder for Kubernetes node requirements
- Network management for container-to-container communication
- Container labeling system for cluster and node identification

### Task 2: Phased Cluster Lifecycle Implementation
**Objective**: Implement KIND's phased cluster creation workflow with Apple Container
**Dependencies**: Task 1
**Acceptance Criteria**:
- Cluster creation follows KIND's sequential action pattern
- Each phase can be executed independently for debugging
- Error handling supports partial cleanup on failure
- Action pipeline supports extensibility for future features

**Implementation Notes**:
Based on KIND's cluster creation actions:
```rust
pub struct ClusterLifecycle {
    provider: Arc<dyn ContainerProvider>,
    logger: Logger,
}

impl ClusterLifecycle {
    pub async fn create_cluster(&self, options: &ClusterOptions) -> Result<(), KinaError> {
        // Cleanup guard for failure scenarios (KIND pattern)
        let cleanup_guard = CleanupGuard::new(&options.config.name, &self.provider);

        // Phase 1: Validate configuration and provider
        self.validate_options(options).await?;

        // Phase 2: Container provisioning
        self.provider.provision(&options.config).await?;

        // Phase 3: Kubernetes setup pipeline (KIND action pattern)
        let mut pipeline = ActionPipeline::new();

        pipeline
            .add_action(Box::new(LoadBalancerAction::new()))
            .add_action(Box::new(ConfigAction::new()));

        if !options.stop_before_kubernetes {
            pipeline
                .add_action(Box::new(KubeadmInitAction::new(&options.config)))
                .add_action(Box::new(InstallCniAction::new()))
                .add_action(Box::new(InstallStorageAction::new()))
                .add_action(Box::new(KubeadmJoinAction::new()))
                .add_action(Box::new(WaitForReadyAction::new(options.wait_for_ready)));
        }

        // Execute pipeline with comprehensive error handling
        pipeline.execute(&ActionContext::new(&options.config, &self.provider)).await?;
        cleanup_guard.disarm();

        Ok(())
    }
}

#[async_trait]
pub trait Action: Send + Sync {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

pub struct ActionPipeline {
    actions: Vec<Box<dyn Action>>,
}

impl ActionPipeline {
    pub async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        for (idx, action) in self.actions.iter().enumerate() {
            tracing::info!(
                "🔄 [{}/{}] {}: {}",
                idx + 1,
                self.actions.len(),
                action.name(),
                action.description()
            );

            action.execute(context).await.map_err(|e| {
                KinaError::ActionFailed {
                    action: action.name().to_string(),
                    source: Box::new(e),
                }
            })?;

            tracing::info!("✅ {} completed successfully", action.name());
        }
        Ok(())
    }
}
```

**Deliverables**:
- ClusterLifecycle struct with create/delete operations
- Action trait and ActionPipeline for extensible workflow execution
- CleanupGuard for automatic resource cleanup on failures
- Comprehensive logging and progress reporting

### Task 3: Kubernetes Bootstrap Actions Implementation
**Objective**: Implement kubeadm integration actions following KIND's bootstrap patterns
**Dependencies**: Task 2
**Acceptance Criteria**:
- KubeadmInitAction successfully initializes control plane
- Configuration generation matches KIND's kubeadm patterns
- CNI installation action supports multiple network plugins
- Storage provisioner action sets up local storage

**Implementation Notes**:
```rust
pub struct KubeadmInitAction {
    config: ClusterConfig,
    template_generator: KubeadmConfigGenerator,
}

#[async_trait]
impl Action for KubeadmInitAction {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        let control_plane_node = context.find_control_plane_node()?;

        // Generate kubeadm configuration (KIND pattern)
        let config_data = ConfigData::from_cluster_config(&self.config);
        let kubeadm_config = self.template_generator.generate_config(&config_data)?;

        // Write configuration to control plane container
        let config_path = "/tmp/kubeadm-config.yaml";
        context.provider.write_file_to_container(
            &control_plane_node.container_id,
            config_path,
            &kubeadm_config,
        ).await?;

        // Execute kubeadm init
        let init_cmd = [
            "kubeadm", "init",
            "--config", config_path,
            "--skip-phases", "preflight",
            "--ignore-preflight-errors", "all",
        ];

        let result = context.provider.exec_in_container(
            &control_plane_node.container_id,
            &init_cmd,
        ).await?;

        if !result.success {
            return Err(KinaError::KubeadmInitFailed {
                stderr: result.stderr,
                stdout: result.stdout,
            });
        }

        Ok(())
    }

    fn name(&self) -> &str { "kubeadm-init" }
    fn description(&self) -> &str { "Initialize Kubernetes control plane with kubeadm" }
}

pub struct InstallCniAction;

#[async_trait]
impl Action for InstallCniAction {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        let control_plane_node = context.find_control_plane_node()?;

        // Install CNI plugin (default to kindnet for KIND compatibility)
        let kindnet_manifest = include_str!("../manifests/kindnet.yaml");

        let apply_cmd = [
            "kubectl", "apply", "-f", "-"
        ];

        let result = context.provider.exec_in_container_with_stdin(
            &control_plane_node.container_id,
            &apply_cmd,
            kindnet_manifest,
        ).await?;

        if !result.success {
            return Err(KinaError::CniInstallFailed {
                stderr: result.stderr,
            });
        }

        Ok(())
    }

    fn name(&self) -> &str { "install-cni" }
    fn description(&self) -> &str { "Install CNI networking plugin" }
}
```

**Deliverables**:
- KubeadmInitAction with configuration generation and execution
- InstallCniAction supporting multiple CNI plugins
- InstallStorageAction for local storage provisioner
- Kubernetes manifest templates embedded in binary

### Task 4: Multi-Node Orchestration Actions
**Objective**: Implement KIND's multi-node join orchestration patterns
**Dependencies**: Task 3
**Acceptance Criteria**:
- KubeadmJoinAction handles both control-plane and worker nodes
- Join operations execute concurrently for workers
- Secondary control planes join sequentially with proper coordination
- Node readiness verification includes comprehensive health checks

**Implementation Notes**:
```rust
pub struct KubeadmJoinAction;

#[async_trait]
impl Action for KubeadmJoinAction {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        let all_nodes = context.provider.list_nodes(&context.cluster_name()).await?;

        // Join secondary control planes first (KIND pattern)
        let secondary_control_planes = all_nodes.iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .filter(|n| !n.is_primary_control_plane())
            .collect::<Vec<_>>();

        if !secondary_control_planes.is_empty() {
            self.join_secondary_control_planes(context, &secondary_control_planes).await?;
        }

        // Join worker nodes concurrently (KIND pattern)
        let workers = all_nodes.iter()
            .filter(|n| n.role == NodeRole::Worker)
            .collect::<Vec<_>>();

        if !workers.is_empty() {
            self.join_workers_concurrent(context, &workers).await?;
        }

        Ok(())
    }

    fn name(&self) -> &str { "kubeadm-join" }
    fn description(&self) -> &str { "Join additional nodes to the cluster" }
}

impl KubeadmJoinAction {
    async fn join_workers_concurrent(
        &self,
        context: &ActionContext,
        workers: &[&Node],
    ) -> Result<(), KinaError> {
        let join_futures: Vec<_> = workers.iter()
            .map(|worker| self.join_worker_node(context, worker))
            .collect();

        try_join_all(join_futures).await?;
        Ok(())
    }

    async fn join_worker_node(
        &self,
        context: &ActionContext,
        worker: &Node,
    ) -> Result<(), KinaError> {
        // Get join command from control plane (KIND pattern)
        let join_cmd = self.get_kubeadm_join_command(context).await?;

        // Execute join on worker node
        let exec_result = context.provider.exec_in_container(
            &worker.container_id,
            &["sh", "-c", &join_cmd],
        ).await?;

        if !exec_result.success {
            return Err(KinaError::NodeJoinFailed {
                node: worker.name.clone(),
                stderr: exec_result.stderr,
            });
        }

        Ok(())
    }
}
```

**Deliverables**:
- KubeadmJoinAction with sequential/concurrent join patterns
- Join command generation and token management
- Node role detection and join strategy selection
- Error handling with per-node failure reporting

### Task 5: Core CLI Commands Implementation
**Objective**: Implement core kina commands maintaining KIND command compatibility
**Dependencies**: Tasks 1-4
**Acceptance Criteria**:
- `kina create cluster` creates functional Kubernetes clusters
- `kina delete cluster` cleanly removes all resources
- `kina get clusters/nodes` provides comprehensive status information
- Command-line arguments match KIND patterns exactly

**Implementation Notes**:
```rust
// kina create cluster implementation
pub async fn run_create_cluster(args: &CreateClusterArgs) -> Result<(), KinaError> {
    let config = if let Some(config_path) = &args.config {
        ClusterConfig::from_file(config_path).await?
    } else {
        ClusterConfig::default_with_name(&args.name)
    };

    let provider = AppleContainerProvider::new().await?;
    let lifecycle = ClusterLifecycle::new(Arc::new(provider));

    let options = ClusterOptions {
        config,
        wait_for_ready: args.wait.unwrap_or(Duration::from_secs(300)),
        retain: args.retain,
        stop_before_kubernetes: false,
    };

    lifecycle.create_cluster(&options).await?;

    // Generate and export kubeconfig (KIND pattern)
    let kubeconfig_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kube")
        .join("config");

    export_kubeconfig(&args.name, &kubeconfig_path).await?;

    println!("✅ Cluster '{}' created successfully", args.name);
    println!("💡 You can now use kubectl with: export KUBECONFIG={}", kubeconfig_path.display());

    Ok(())
}

// kina get clusters implementation
pub async fn run_get_clusters() -> Result<(), KinaError> {
    let provider = AppleContainerProvider::new().await?;
    let clusters = provider.list_clusters().await?;

    if clusters.is_empty() {
        println!("No clusters found.");
        return Ok(());
    }

    println!("{:<20} {:<10} {:<15} {:<20}", "NAME", "STATUS", "CONTROL-PLANE", "WORKER");
    println!("{}", "=".repeat(65));

    for cluster in clusters {
        println!(
            "{:<20} {:<10} {:<15} {:<20}",
            cluster.name,
            cluster.status,
            cluster.control_plane_count(),
            cluster.worker_count(),
        );
    }

    Ok(())
}
```

**Deliverables**:
- Complete implementation of create, delete, get commands
- Command-line argument parsing with clap compatibility
- Kubeconfig generation and management
- Status reporting with tabular output formatting

### Task 6: Container Image and Network Management
**Objective**: Implement container image and network management following KIND patterns
**Dependencies**: Task 1
**Acceptance Criteria**:
- Node image pulling and caching system operational
- Container networking supports inter-pod communication
- Image management supports multiple Kubernetes versions
- Network cleanup removes all cluster-specific resources

**Implementation Notes**:
```rust
pub struct ImageManager {
    runtime: AppleContainerRuntime,
    cache_dir: PathBuf,
}

impl ImageManager {
    pub async fn ensure_node_image(&self, image: &str) -> Result<(), KinaError> {
        // Check if image exists locally
        if self.runtime.image_exists(image).await? {
            tracing::debug!("Image {} already exists locally", image);
            return Ok(());
        }

        // Pull image from registry
        tracing::info!("📥 Pulling node image: {}", image);
        self.runtime.pull_image(image).await?;

        Ok(())
    }

    pub async fn load_container_image(
        &self,
        cluster_name: &str,
        image_path: &Path,
    ) -> Result<(), KinaError> {
        let provider = AppleContainerProvider::new().await?;
        let nodes = provider.list_nodes(cluster_name).await?;

        // Load image to all cluster nodes (KIND pattern)
        let load_futures: Vec<_> = nodes.iter()
            .map(|node| self.load_image_to_node(node, image_path))
            .collect();

        try_join_all(load_futures).await?;
        Ok(())
    }
}

pub struct NetworkManager {
    runtime: AppleContainerRuntime,
}

impl NetworkManager {
    pub async fn ensure_cluster_network(&self, cluster_name: &str) -> Result<ContainerNetwork, KinaError> {
        let network_name = format!("kina-{}", cluster_name);

        // Create cluster-specific network (KIND pattern)
        if let Some(network) = self.runtime.get_network(&network_name).await? {
            return Ok(network);
        }

        let network_spec = NetworkSpec::builder()
            .name(&network_name)
            .driver("bridge")
            .enable_ipv6(false)
            .subnet("172.20.0.0/16")  // KIND-compatible subnet
            .labels(&[("io.kina.cluster", cluster_name)])
            .build()?;

        self.runtime.create_network(network_spec).await
    }
}
```

**Deliverables**:
- ImageManager with pull, cache, and load capabilities
- NetworkManager with cluster-specific network creation
- Container image loading to cluster nodes
- Network cleanup for cluster deletion

## Success Criteria
- Apple Container provider implements complete KIND provider interface
- Cluster creation follows KIND's phased lifecycle patterns
- Core CLI commands (create, delete, get) functional and KIND-compatible
- kubeadm integration successfully initializes clusters
- Multi-node orchestration supports both control-plane and worker nodes

## Critical Dependencies
- **Phase 1 completion**: Provider abstraction and CLI framework must be ready
- **Apple Container functionality**: Network and volume mounting capabilities validated
- **Kubernetes compatibility**: kubeadm and CNI integration verified

## Risk Mitigation
- **Apple Container limitations**: Implement graceful fallbacks for unsupported features
- **Kubernetes version compatibility**: Support matrix for kubeadm and CNI versions
- **Network complexity**: Start with simple bridge networking, enhance incrementally

## Integration Notes
- Foundation for Phase 3 advanced Kubernetes features and tool integration
- Architecture supports extension to multi-node complex topologies
- Provider pattern allows future runtime addition (Docker compatibility mode)

**Phase Completion Gate**: Core cluster lifecycle (create, bootstrap, delete) must be fully functional before Phase 3 advanced features