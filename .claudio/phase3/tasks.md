# Phase 3: Advanced Features - Kubernetes Ecosystem Integration and Advanced Orchestration

**Phase Objectives**: Implement advanced Kubernetes features, ecosystem tool integration, enhanced cluster configuration, and KIND's sophisticated orchestration patterns including node image building and advanced networking

## Phase Overview
This phase extends KINA's capabilities beyond basic cluster management to include KIND's advanced features: node image building system, advanced networking configurations, Kubernetes ecosystem tool integrations, and enhanced cluster customization options. The focus is on feature parity with KIND's advanced use cases.

## Key Deliverables
- Node image building system using Apple Container's native image building
- Integration with kubectx, kubens, k9s and other Kubernetes tools
- Advanced cluster configuration options and multi-node orchestration
- Ingress controller support (nginx-ingress) with KIND compatibility
- Enhanced CLI with improved user experience and debugging capabilities

## Task Breakdown

### Task 1: Node Image Building System
**Objective**: Implement KIND's node image building system adapted for Apple Container runtime
**Dependencies**: Phase 2 Tasks 1-2
**Acceptance Criteria**:
- `kina build node-image` command functional with Kubernetes version selection
- Image building supports custom configurations and patches
- Built images compatible with cluster creation workflows
- Multi-architecture support (Intel and Apple Silicon)

**Implementation Notes**:
Based on KIND's node image building approach:
```rust
pub struct NodeImageBuilder {
    runtime: AppleContainerRuntime,
    base_image: String,
    kubernetes_version: String,
    build_cache: BuildCache,
}

impl NodeImageBuilder {
    pub async fn build(&self, config: &ImageBuildConfig) -> Result<String, KinaError> {
        let build_context = self.create_build_context(config).await?;

        // Use Apple Container's native image building with layer optimization
        let image_spec = ContainerImageSpec::builder()
            .base_image(&config.base_image.unwrap_or_else(|| self.default_base_image()))
            .working_dir("/")
            .layers(vec![
                self.create_system_layer().await?,
                self.create_kubernetes_layer(&config.kubernetes_version).await?,
                self.create_container_runtime_layer().await?,
                self.create_networking_layer().await?,
                self.create_systemd_layer().await?,
            ])
            .labels(&[
                ("io.kina.image.type", "node"),
                ("io.kina.kubernetes.version", &config.kubernetes_version),
                ("io.kina.build.timestamp", &chrono::Utc::now().to_rfc3339()),
            ])
            .build()?;

        let image_id = self.runtime.build_image(image_spec, &build_context).await?;
        self.build_cache.store_image_metadata(&image_id, config).await?;

        Ok(image_id)
    }

    async fn create_kubernetes_layer(&self, version: &str) -> Result<ImageLayer, KinaError> {
        let kubernetes_binaries = self.download_kubernetes_binaries(version).await?;

        LayerBuilder::new()
            .add_files(&[
                ("/usr/local/bin/kubelet", &kubernetes_binaries.kubelet),
                ("/usr/local/bin/kubeadm", &kubernetes_binaries.kubeadm),
                ("/usr/local/bin/kubectl", &kubernetes_binaries.kubectl),
            ])
            .run_commands(&[
                "chmod +x /usr/local/bin/kubelet",
                "chmod +x /usr/local/bin/kubeadm",
                "chmod +x /usr/local/bin/kubectl",
                "systemctl enable kubelet",
            ])
            .build()
    }

    async fn create_container_runtime_layer(&self) -> Result<ImageLayer, KinaError> {
        // Install containerd for Apple Container compatibility
        LayerBuilder::new()
            .run_commands(&[
                // Download and install containerd
                "curl -L https://github.com/containerd/containerd/releases/download/v1.7.0/containerd-1.7.0-darwin-amd64.tar.gz | tar -C /usr/local -xzf -",
                "systemctl enable containerd",
                // Configure containerd for Kubernetes
                "mkdir -p /etc/containerd",
                "containerd config default > /etc/containerd/config.toml",
                "sed -i 's/SystemdCgroup = false/SystemdCgroup = true/' /etc/containerd/config.toml",
            ])
            .build()
    }
}

pub struct BuildCache {
    cache_dir: PathBuf,
    runtime: AppleContainerRuntime,
}

impl BuildCache {
    pub async fn is_cached(&self, config: &ImageBuildConfig) -> Result<Option<String>, KinaError> {
        let cache_key = self.generate_cache_key(config);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
            let metadata: ImageMetadata = serde_json::from_reader(
                std::fs::File::open(&cache_path)?
            )?;

            // Verify image still exists in runtime
            if self.runtime.image_exists(&metadata.image_id).await? {
                return Ok(Some(metadata.image_id));
            }
        }

        Ok(None)
    }
}
```

**Deliverables**:
- NodeImageBuilder with Apple Container-optimized build process
- BuildCache system for efficient incremental builds
- CLI command `kina build node-image` with full argument support
- Multi-architecture build support for macOS platforms

### Task 2: Advanced Networking and Load Balancer
**Objective**: Implement KIND's advanced networking features including load balancer and ingress support
**Dependencies**: Phase 2 Task 6
**Acceptance Criteria**:
- Load balancer implementation for multi-control-plane clusters
- Ingress controller installation and configuration
- Port forwarding and service exposure capabilities
- Advanced network policies and CNI plugin selection

**Implementation Notes**:
```rust
pub struct LoadBalancerAction {
    config: ClusterConfig,
}

#[async_trait]
impl Action for LoadBalancerAction {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        let control_plane_nodes: Vec<_> = context.nodes().iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .collect();

        if control_plane_nodes.len() <= 1 {
            // Single control plane doesn't need load balancer
            return Ok(());
        }

        // Create HAProxy load balancer container (KIND pattern)
        let lb_config = self.generate_haproxy_config(&control_plane_nodes)?;

        let lb_container = ContainerSpec::builder()
            .image("haproxy:2.4")
            .name(&format!("{}-control-plane-lb", context.cluster_name()))
            .network(context.cluster_network().id())
            .port_mapping(6443, 6443) // Kubernetes API server
            .volume_from_string(&lb_config, "/usr/local/etc/haproxy/haproxy.cfg")
            .labels(&[
                ("io.kina.cluster", context.cluster_name()),
                ("io.kina.component", "loadbalancer"),
            ])
            .build()?;

        context.provider.create_container(lb_container).await?;

        Ok(())
    }

    fn name(&self) -> &str { "loadbalancer" }
    fn description(&self) -> &str { "Set up load balancer for multi-control-plane clusters" }
}

pub struct IngressControllerAction {
    ingress_type: IngressType,
}

#[async_trait]
impl Action for IngressControllerAction {
    async fn execute(&self, context: &ActionContext) -> Result<(), KinaError> {
        match self.ingress_type {
            IngressType::Nginx => self.install_nginx_ingress(context).await?,
            IngressType::Contour => self.install_contour_ingress(context).await?,
        }

        // Wait for ingress controller to be ready
        self.wait_for_ingress_ready(context).await?;

        Ok(())
    }

    async fn install_nginx_ingress(&self, context: &ActionContext) -> Result<(), KinaError> {
        let control_plane_node = context.find_control_plane_node()?;

        // Apply nginx-ingress manifest (KIND-compatible)
        let nginx_manifest = include_str!("../manifests/nginx-ingress.yaml");

        let apply_cmd = ["kubectl", "apply", "-f", "-"];

        let result = context.provider.exec_in_container_with_stdin(
            &control_plane_node.container_id,
            &apply_cmd,
            nginx_manifest,
        ).await?;

        if !result.success {
            return Err(KinaError::IngressInstallFailed {
                ingress_type: "nginx".to_string(),
                stderr: result.stderr,
            });
        }

        Ok(())
    }

    fn name(&self) -> &str { "ingress-controller" }
    fn description(&self) -> &str { "Install and configure ingress controller" }
}
```

**Deliverables**:
- LoadBalancerAction for multi-control-plane cluster support
- IngressControllerAction supporting nginx-ingress and other controllers
- Port forwarding and service exposure utilities
- Advanced CNI plugin selection and configuration

### Task 3: Kubernetes Ecosystem Tool Integration
**Objective**: Integrate with kubectx, kubens, k9s, and other essential Kubernetes tools
**Dependencies**: Phase 2 Tasks 4-5
**Acceptance Criteria**:
- kubectx/kubens integration with automatic context management
- k9s compatibility with KINA-created clusters
- Kubernetes tool detection and validation
- Automated tool installation and configuration

**Implementation Notes**:
```rust
pub struct KubernetesToolManager {
    tool_configs: HashMap<String, ToolConfig>,
    cluster_manager: Arc<dyn ContainerProvider>,
}

impl KubernetesToolManager {
    pub fn new() -> Self {
        let mut tool_configs = HashMap::new();

        tool_configs.insert("kubectx".to_string(), ToolConfig {
            name: "kubectx",
            check_command: vec!["kubectx", "--version"],
            install_instructions: "brew install kubectx",
            description: "Switch between Kubernetes contexts",
        });

        tool_configs.insert("kubens".to_string(), ToolConfig {
            name: "kubens",
            check_command: vec!["kubens", "--version"],
            install_instructions: "brew install kubectx", // kubens comes with kubectx
            description: "Switch between Kubernetes namespaces",
        });

        tool_configs.insert("k9s".to_string(), ToolConfig {
            name: "k9s",
            check_command: vec!["k9s", "version"],
            install_instructions: "brew install k9s",
            description: "Terminal UI for Kubernetes",
        });

        Self {
            tool_configs,
            cluster_manager: Arc::new(AppleContainerProvider::new().await.unwrap()),
        }
    }

    pub async fn validate_tools(&self) -> Result<ToolValidationReport, KinaError> {
        let mut report = ToolValidationReport::new();

        for (tool_name, config) in &self.tool_configs {
            let status = self.check_tool_status(config).await?;
            report.add_tool_status(tool_name.clone(), status);
        }

        Ok(report)
    }

    pub async fn setup_cluster_integration(&self, cluster_name: &str) -> Result<(), KinaError> {
        // Set up kubeconfig for tools
        let kubeconfig_path = self.export_cluster_kubeconfig(cluster_name).await?;

        // Configure kubectx context
        self.add_kubectx_context(cluster_name, &kubeconfig_path).await?;

        // Validate tool compatibility
        self.test_tool_compatibility(cluster_name).await?;

        Ok(())
    }

    async fn add_kubectx_context(&self, cluster_name: &str, kubeconfig: &Path) -> Result<(), KinaError> {
        let context_name = format!("kina-{}", cluster_name);

        // Merge kubeconfig into default kubectl config
        let merge_cmd = [
            "kubectl", "config", "view", "--flatten", "--merge"
        ];

        // Set environment variable for kubectx
        std::env::set_var("KUBECONFIG", kubeconfig);

        // Rename context to kina-{cluster_name} for clear identification
        let rename_cmd = [
            "kubectl", "config", "rename-context",
            &format!("kind-{}", cluster_name),
            &context_name
        ];

        Command::new(&rename_cmd[0])
            .args(&rename_cmd[1..])
            .status()
            .await?;

        println!("🔗 Added cluster '{}' to kubectx as '{}'", cluster_name, context_name);

        Ok(())
    }
}

#[derive(Clone)]
pub struct ToolConfig {
    pub name: &'static str,
    pub check_command: Vec<&'static str>,
    pub install_instructions: &'static str,
    pub description: &'static str,
}

pub struct ToolValidationReport {
    tool_statuses: HashMap<String, ToolStatus>,
}

#[derive(Debug)]
pub enum ToolStatus {
    Available { version: String },
    NotFound,
    Error { message: String },
}
```

**Deliverables**:
- KubernetesToolManager for tool detection and integration
- Automated kubeconfig management for tool compatibility
- Tool validation and installation guidance
- Integration testing framework for Kubernetes tools

### Task 4: Enhanced CLI Features and User Experience
**Objective**: Implement advanced CLI features including debugging, verbose output, and enhanced error handling
**Dependencies**: Phase 2 Task 5
**Acceptance Criteria**:
- Verbose and debug output modes with structured logging
- Progress indicators for long-running operations
- Enhanced error messages with troubleshooting suggestions
- Shell completion support for bash/zsh/fish

**Implementation Notes**:
```rust
use clap::{Parser, Subcommand};
use tracing::{info, debug, error, Level};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "kina")]
#[command(about = "Kubernetes in Apple Container - Local Kubernetes clusters")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable debug output
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output format (text, json, yaml)
    #[arg(long, global = true, default_value = "text")]
    pub output: OutputFormat,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create resources
    #[command(subcommand)]
    Create(CreateCommands),

    /// Delete resources
    #[command(subcommand)]
    Delete(DeleteCommands),

    /// Get information about resources
    #[command(subcommand)]
    Get(GetCommands),

    /// Load images into cluster
    #[command(subcommand)]
    Load(LoadCommands),

    /// Export cluster configuration
    #[command(subcommand)]
    Export(ExportCommands),

    /// Build node images
    #[command(subcommand)]
    Build(BuildCommands),

    /// Generate shell completion scripts
    Completion {
        /// Shell type
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

pub struct ProgressReporter {
    progress_bar: Option<ProgressBar>,
    phase_name: String,
}

impl ProgressReporter {
    pub fn new(phase_name: &str, total_steps: u64) -> Self {
        let progress_bar = if atty::is(atty::Stream::Stderr) {
            let pb = ProgressBar::new(total_steps);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>2}/{len:2} {msg}")
                    .unwrap()
                    .progress_chars("##-")
            );
            pb.set_message(format!("Starting {}", phase_name));
            Some(pb)
        } else {
            None
        };

        Self {
            progress_bar,
            phase_name: phase_name.to_string(),
        }
    }

    pub fn update(&self, step: u64, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.set_position(step);
            pb.set_message(message.to_string());
        } else {
            // Text-only output for non-TTY
            info!("[{}/{}] {}: {}", step, self.phase_name, self.phase_name, message);
        }
    }

    pub fn finish(&self, message: &str) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message(message.to_string());
        } else {
            info!("✅ {}: {}", self.phase_name, message);
        }
    }
}

pub fn setup_tracing(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let level = if cli.debug {
        Level::DEBUG
    } else if cli.verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

// Enhanced error handling with suggestions
#[derive(Debug, thiserror::Error)]
pub enum KinaError {
    #[error("Apple Container not found or not running")]
    AppleContainerUnavailable {
        #[help]
        suggestion: &'static str = "Please ensure Apple Container is installed and running. Check macOS version compatibility (15.6+).",
    },

    #[error("Cluster creation failed: {reason}")]
    ClusterCreationFailed {
        reason: String,
        #[help]
        suggestion: String,
    },

    #[error("Node image build failed: {details}")]
    ImageBuildFailed {
        details: String,
        #[help]
        suggestion: &'static str = "Check network connectivity and available disk space. Try cleaning image cache with 'kina build node-image --no-cache'.",
    },
}
```

**Deliverables**:
- Enhanced CLI with verbose/debug output modes and progress reporting
- Comprehensive error handling with troubleshooting suggestions
- Shell completion support for major shells
- Structured logging integration with tracing framework

### Task 5: Advanced Cluster Configuration and Customization
**Objective**: Implement KIND's advanced configuration options and cluster customization features
**Dependencies**: Phase 1 Task 6, Phase 2 Tasks
**Acceptance Criteria**:
- Support for kubeadm config patches and custom configurations
- Advanced networking configurations (IPv6, custom subnets)
- Feature gate and runtime config support
- Custom volume mounts and container configurations

**Implementation Notes**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedClusterConfig {
    // Basic cluster configuration
    #[serde(flatten)]
    pub base: ClusterConfig,

    // Advanced networking options
    #[serde(rename = "networking")]
    pub networking: AdvancedNetworkingConfig,

    // Container runtime configuration
    #[serde(rename = "containerRuntime")]
    pub container_runtime: ContainerRuntimeConfig,

    // Custom volume mounts
    #[serde(rename = "extraMounts")]
    pub extra_mounts: Vec<VolumeMount>,

    // Environment variables for containers
    #[serde(rename = "extraEnvVars")]
    pub extra_env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedNetworkingConfig {
    #[serde(flatten)]
    pub base: NetworkingConfig,

    /// Enable IPv6 support
    #[serde(rename = "ipFamily")]
    pub ip_family: IpFamily,

    /// Custom CNI configuration
    #[serde(rename = "cniConfig")]
    pub cni_config: Option<CniConfig>,

    /// DNS configuration
    #[serde(rename = "dnsConfig")]
    pub dns_config: DnsConfig,

    /// Port mappings for services
    #[serde(rename = "portMappings")]
    pub port_mappings: Vec<PortMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpFamily {
    Ipv4,
    Ipv6,
    Dual,
}

pub struct AdvancedConfigurationValidator;

impl AdvancedConfigurationValidator {
    pub fn validate(config: &AdvancedClusterConfig) -> Result<(), ConfigValidationError> {
        // Validate base configuration
        config.base.validate()?;

        // Validate IPv6 configuration
        if matches!(config.networking.ip_family, IpFamily::Ipv6 | IpFamily::Dual) {
            Self::validate_ipv6_support()?;
        }

        // Validate custom mounts
        for mount in &config.extra_mounts {
            Self::validate_mount_path(&mount.host_path, &mount.container_path)?;
        }

        // Validate CNI configuration
        if let Some(cni) = &config.networking.cni_config {
            Self::validate_cni_config(cni)?;
        }

        Ok(())
    }

    fn validate_ipv6_support() -> Result<(), ConfigValidationError> {
        // Check if Apple Container supports IPv6
        // This would need to be determined through actual Apple Container API calls
        Ok(())
    }

    fn validate_mount_path(host_path: &str, container_path: &str) -> Result<(), ConfigValidationError> {
        // Validate path formats and security constraints
        if container_path.starts_with("/sys") && !container_path.starts_with("/sys/fs/cgroup") {
            return Err(ConfigValidationError::UnsafeMountPath {
                path: container_path.to_string(),
                reason: "Mounting /sys paths (except /sys/fs/cgroup) is not allowed for security reasons".to_string(),
            });
        }

        Ok(())
    }
}

pub struct KubeadmConfigPatcher {
    templates: HashMap<String, String>,
}

impl KubeadmConfigPatcher {
    pub fn apply_patches(
        &self,
        base_config: &str,
        patches: &[String],
    ) -> Result<String, KinaError> {
        let mut config: serde_yaml::Value = serde_yaml::from_str(base_config)?;

        for patch in patches {
            let patch_value: serde_yaml::Value = serde_yaml::from_str(patch)?;
            Self::merge_yaml(&mut config, patch_value)?;
        }

        Ok(serde_yaml::to_string(&config)?)
    }

    fn merge_yaml(
        base: &mut serde_yaml::Value,
        patch: serde_yaml::Value,
    ) -> Result<(), KinaError> {
        match (base, patch) {
            (serde_yaml::Value::Mapping(ref mut base_map), serde_yaml::Value::Mapping(patch_map)) => {
                for (key, value) in patch_map {
                    base_map.insert(key, value);
                }
            }
            _ => {
                *base = patch;
            }
        }
        Ok(())
    }
}
```

**Deliverables**:
- AdvancedClusterConfig with comprehensive configuration options
- Configuration validation system with security checks
- KubeadmConfigPatcher for custom configuration application
- IPv6 and dual-stack networking support framework

### Task 6: Multi-Node Cluster Topologies and Orchestration
**Objective**: Implement KIND's support for complex multi-node cluster topologies
**Dependencies**: Phase 2 Task 4
**Acceptance Criteria**:
- Support for custom node roles and configurations
- High availability control plane configurations (3, 5, 7 nodes)
- Worker node scaling and management
- Complex networking topologies with multiple networks

**Implementation Notes**:
```rust
pub struct MultiNodeOrchestrator {
    provider: Arc<dyn ContainerProvider>,
    network_manager: NetworkManager,
    lifecycle_manager: ClusterLifecycle,
}

impl MultiNodeOrchestrator {
    pub async fn orchestrate_complex_cluster(
        &self,
        config: &AdvancedClusterConfig,
    ) -> Result<(), KinaError> {
        // Validate cluster topology
        self.validate_topology(config).await?;

        // Phase 1: Create all containers with proper networking
        let orchestration_plan = self.create_orchestration_plan(config)?;
        self.execute_container_creation(&orchestration_plan).await?;

        // Phase 2: Initialize primary control plane
        self.initialize_primary_control_plane(&orchestration_plan).await?;

        // Phase 3: Join secondary control planes (if any)
        if orchestration_plan.secondary_control_planes.len() > 0 {
            self.join_secondary_control_planes(&orchestration_plan).await?;
        }

        // Phase 4: Join worker nodes (concurrent)
        if orchestration_plan.workers.len() > 0 {
            self.join_workers_concurrent(&orchestration_plan).await?;
        }

        // Phase 5: Apply advanced configurations
        self.apply_advanced_configurations(config, &orchestration_plan).await?;

        Ok(())
    }

    async fn validate_topology(&self, config: &AdvancedClusterConfig) -> Result<(), KinaError> {
        let control_plane_count = config.base.nodes.iter()
            .filter(|n| n.role == NodeRole::ControlPlane)
            .count();

        // Validate HA requirements
        if control_plane_count > 1 {
            if control_plane_count % 2 == 0 {
                return Err(KinaError::InvalidTopology {
                    reason: "High availability control plane requires odd number of nodes (3, 5, 7, ...)".to_string(),
                });
            }

            if control_plane_count > 7 {
                return Err(KinaError::InvalidTopology {
                    reason: "Maximum of 7 control plane nodes supported".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn join_secondary_control_planes(
        &self,
        plan: &OrchestrationPlan,
    ) -> Result<(), KinaError> {
        // Sequential join for control planes to avoid race conditions
        for (idx, cp_node) in plan.secondary_control_planes.iter().enumerate() {
            tracing::info!("🔗 Joining control plane node {}/{}: {}",
                idx + 1,
                plan.secondary_control_planes.len(),
                cp_node.name
            );

            self.join_control_plane_node(cp_node, plan).await.map_err(|e| {
                KinaError::ControlPlaneJoinFailed {
                    node: cp_node.name.clone(),
                    source: Box::new(e),
                }
            })?;

            // Verify control plane is healthy before proceeding
            self.verify_control_plane_health(cp_node).await?;
        }

        Ok(())
    }

    async fn apply_advanced_configurations(
        &self,
        config: &AdvancedClusterConfig,
        plan: &OrchestrationPlan,
    ) -> Result<(), KinaError> {
        // Apply feature gates
        if !config.base.feature_gates.is_empty() {
            self.apply_feature_gates(&config.base.feature_gates, plan).await?;
        }

        // Configure advanced networking
        if matches!(config.networking.ip_family, IpFamily::Ipv6 | IpFamily::Dual) {
            self.configure_ipv6_networking(plan).await?;
        }

        // Apply custom volume mounts
        for mount in &config.extra_mounts {
            self.apply_volume_mount(mount, plan).await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct OrchestrationPlan {
    pub cluster_name: String,
    pub primary_control_plane: NodePlan,
    pub secondary_control_planes: Vec<NodePlan>,
    pub workers: Vec<NodePlan>,
    pub load_balancer: Option<LoadBalancerPlan>,
    pub networks: Vec<NetworkPlan>,
}

#[derive(Debug)]
pub struct NodePlan {
    pub name: String,
    pub role: NodeRole,
    pub container_spec: ContainerSpec,
    pub network_attachments: Vec<String>,
    pub custom_mounts: Vec<VolumeMount>,
}
```

**Deliverables**:
- MultiNodeOrchestrator for complex cluster topologies
- OrchestrationPlan system for coordinated cluster creation
- High availability control plane support (3, 5, 7 nodes)
- Advanced networking topology support with multiple networks

## Success Criteria
- Node image building system creates functional Kubernetes images
- Advanced networking supports load balancers and ingress controllers
- Kubernetes ecosystem tools integrate seamlessly with KINA clusters
- Enhanced CLI provides excellent user experience with debugging capabilities
- Multi-node orchestration supports complex HA topologies

## Critical Dependencies
- **Phase 2 completion**: Core cluster lifecycle and provider implementation
- **Apple Container image building**: Native image creation capabilities validated
- **Kubernetes tool compatibility**: Tool ecosystem compatibility verified

## Risk Mitigation
- **Image building complexity**: Incremental implementation starting with basic image creation
- **Tool integration challenges**: Fallback to manual configuration if automated integration fails
- **Advanced networking limitations**: Progressive enhancement based on Apple Container capabilities

## Integration Notes
- Foundation for Phase 4 performance optimization and production hardening
- Advanced features can be selectively enabled based on Apple Container capabilities
- Architecture supports gradual feature rollout and compatibility testing

**Phase Completion Gate**: Advanced features must maintain KIND compatibility while leveraging Apple Container advantages