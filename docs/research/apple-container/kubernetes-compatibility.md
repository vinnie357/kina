# Kubernetes CRI Compatibility Analysis

**Focus**: Container Runtime Interface implementation requirements and Pod specification compatibility

## Current CRI Implementation Status

### CRI Compliance Assessment
- **Apple Container v0.1.0**: No native CRI implementation available
- **Multi-container Support**: Not supported (VM-per-container architecture)
- **kubelet Integration**: No existing CRI gRPC endpoints
- **Pod Orchestration**: Requires custom implementation

### CRI Architecture Requirements

Apple Container requires a custom CRI shim to interface with Kubernetes:

```
┌─────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   kubelet   │◄──►│   CRI Shim      │◄──►│ Apple Container │
│             │    │  (Custom gRPC)  │    │   Framework     │
└─────────────┘    └─────────────────┘    └─────────────────┘
```

## Required gRPC Services Implementation

### RuntimeService Interface

```protobuf
// Based on Kubernetes CRI v1 API specification
service RuntimeService {
    // Sandbox (Pod) lifecycle management
    rpc RunPodSandbox(RunPodSandboxRequest) returns (RunPodSandboxResponse);
    rpc StopPodSandbox(StopPodSandboxRequest) returns (StopPodSandboxResponse);
    rpc RemovePodSandbox(RemovePodSandboxRequest) returns (RemovePodSandboxResponse);
    rpc PodSandboxStatus(PodSandboxStatusRequest) returns (PodSandboxStatusResponse);
    rpc ListPodSandbox(ListPodSandboxRequest) returns (ListPodSandboxResponse);

    // Container lifecycle management
    rpc CreateContainer(CreateContainerRequest) returns (CreateContainerResponse);
    rpc StartContainer(StartContainerRequest) returns (StartContainerResponse);
    rpc StopContainer(StopContainerRequest) returns (StopContainerResponse);
    rpc RemoveContainer(RemoveContainerRequest) returns (RemoveContainerResponse);
    rpc ListContainers(ListContainersRequest) returns (ListContainersResponse);
    rpc ContainerStatus(ContainerStatusRequest) returns (ContainerStatusResponse);

    // Container execution
    rpc ExecSync(ExecSyncRequest) returns (ExecSyncResponse);
    rpc Exec(ExecRequest) returns (ExecResponse);
    rpc Attach(AttachRequest) returns (AttachResponse);
    rpc PortForward(PortForwardRequest) returns (PortForwardResponse);

    // System information
    rpc Version(VersionRequest) returns (VersionResponse);
    rpc Status(StatusRequest) returns (StatusResponse);
}

service ImageService {
    rpc ListImages(ListImagesRequest) returns (ListImagesResponse);
    rpc ImageStatus(ImageStatusRequest) returns (ImageStatusResponse);
    rpc PullImage(PullImageRequest) returns (PullImageResponse);
    rpc RemoveImage(RemoveImageRequest) returns (RemoveImageResponse);
    rpc ImageFsInfo(ImageFsInfoRequest) returns (ImageFsInfoResponse);
}
```

## Custom CRI Shim Implementation

### Rust CRI Shim Architecture

```rust
// kina-cli/src/core/cri_shim.rs
use tonic::{transport::Server, Request, Response, Status};
use k8s_cri::v1::runtime_service_server::{RuntimeService, RuntimeServiceServer};
use k8s_cri::v1::image_service_server::{ImageService, ImageServiceServer};
use k8s_cri::v1::*;
use crate::core::apple_container::KinaContainerManager;

pub struct AppleContainerCRIShim {
    container_manager: KinaContainerManager,
    pod_sandbox_cache: Arc<RwLock<HashMap<String, PodSandboxInfo>>>,
    container_cache: Arc<RwLock<HashMap<String, ContainerInfo>>>,
}

#[derive(Debug, Clone)]
struct PodSandboxInfo {
    id: String,
    config: PodSandboxConfig,
    container_id: String, // Apple Container VM ID
    state: PodSandboxState,
    created_at: i64,
}

#[derive(Debug, Clone)]
enum PodSandboxState {
    PodSandboxReady,
    PodSandboxNotReady,
}

#[tonic::async_trait]
impl RuntimeService for AppleContainerCRIShim {
    async fn run_pod_sandbox(
        &self,
        request: Request<RunPodSandboxRequest>,
    ) -> Result<Response<RunPodSandboxResponse>, Status> {
        let req = request.into_inner();
        let config = req.config.ok_or_else(|| {
            Status::invalid_argument("Pod sandbox config is required")
        })?;

        // Translate Kubernetes PodSandbox to Apple Container VM
        let vm_config = self.translate_pod_config(&config)?;

        // Create Apple Container VM for the pod
        let container_id = self.container_manager
            .create_container(&vm_config)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Start the container VM
        self.container_manager
            .start_container(&container_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Generate pod sandbox ID
        let pod_sandbox_id = format!("pod-{}", uuid::Uuid::new_v4());

        // Cache pod sandbox information
        let pod_info = PodSandboxInfo {
            id: pod_sandbox_id.clone(),
            config,
            container_id,
            state: PodSandboxState::PodSandboxReady,
            created_at: chrono::Utc::now().timestamp_nanos(),
        };

        let mut cache = self.pod_sandbox_cache.write().await;
        cache.insert(pod_sandbox_id.clone(), pod_info);

        Ok(Response::new(RunPodSandboxResponse {
            pod_sandbox_id,
        }))
    }

    async fn create_container(
        &self,
        request: Request<CreateContainerRequest>,
    ) -> Result<Response<CreateContainerResponse>, Status> {
        let req = request.into_inner();

        // Get pod sandbox information
        let cache = self.pod_sandbox_cache.read().await;
        let pod_info = cache.get(&req.pod_sandbox_id)
            .ok_or_else(|| Status::not_found("Pod sandbox not found"))?;

        // Since Apple Container uses VM-per-container, we need to handle
        // multi-container pods differently
        if req.config.is_none() {
            return Err(Status::invalid_argument("Container config is required"));
        }

        let container_config = req.config.unwrap();

        // For single-container pods, reuse the existing VM
        // For multi-container pods, this is a limitation that needs workaround
        let container_id = if self.is_single_container_pod(&pod_info.config).await? {
            // Reuse pod sandbox VM
            pod_info.container_id.clone()
        } else {
            // Multi-container limitation: create additional VM
            // This breaks pod networking assumptions
            return Err(Status::unimplemented(
                "Multi-container pods not fully supported due to VM-per-container architecture"
            ));
        };

        // Configure container within the VM
        let runtime_config = self.translate_container_config(&container_config)?;
        self.configure_container_in_vm(&container_id, &runtime_config).await
            .map_err(|e| Status::internal(e.to_string()))?;

        let response_container_id = format!("container-{}", uuid::Uuid::new_v4());

        Ok(Response::new(CreateContainerResponse {
            container_id: response_container_id,
        }))
    }

    // Additional CRI method implementations...
}

impl AppleContainerCRIShim {
    fn translate_pod_config(
        &self,
        config: &PodSandboxConfig
    ) -> Result<crate::core::apple_container::ContainerConfig, Status> {
        let metadata = config.metadata.as_ref()
            .ok_or_else(|| Status::invalid_argument("Pod metadata required"))?;

        // Create base Kubernetes node container
        let container_config = crate::core::apple_container::ContainerConfig {
            image: "kindest/node:v1.28.0".to_string(), // Kubernetes node image
            command: vec!["/usr/local/bin/entrypoint".to_string(), "/sbin/init".to_string()],
            environment: self.build_pod_environment(config)?,
            volumes: self.translate_pod_volumes(config)?,
            privileged: true, // Required for Kubernetes node functionality
            resource_limits: self.translate_resource_limits(config)?,
        };

        Ok(container_config)
    }

    fn build_pod_environment(
        &self,
        config: &PodSandboxConfig
    ) -> Result<HashMap<String, String>, Status> {
        let mut env = HashMap::new();

        // Add pod-specific environment variables
        if let Some(metadata) = &config.metadata {
            env.insert("POD_NAME".to_string(), metadata.name.clone());
            env.insert("POD_NAMESPACE".to_string(), metadata.namespace.clone());
            env.insert("POD_UID".to_string(), metadata.uid.clone());
        }

        // Add DNS configuration
        if let Some(dns_config) = &config.dns_config {
            if !dns_config.servers.is_empty() {
                env.insert("DNS_SERVERS".to_string(), dns_config.servers.join(","));
            }
            if !dns_config.searches.is_empty() {
                env.insert("DNS_SEARCH".to_string(), dns_config.searches.join(","));
            }
        }

        Ok(env)
    }

    async fn is_single_container_pod(
        &self,
        _config: &PodSandboxConfig
    ) -> Result<bool, Status> {
        // For now, assume single container pods
        // This would need more sophisticated logic based on pod spec
        Ok(true)
    }
}
```

## Pod Specification Compatibility

### Supported Pod Features

**Direct Support (Single-Container Pods):**
- Basic container execution within VM
- Environment variables and configuration
- Resource constraints (CPU, memory limits per VM)
- Volume mounts (directory sharing per container)
- Network policies (dedicated IP per container)
- DNS configuration
- Security contexts (VM-level isolation)

**Limited Support (Multi-Container Pods):**
- **Challenge**: VM-per-container conflicts with shared Pod networking
- **Workaround**: Network namespace sharing between VMs (complex)
- **Alternative**: Serialize multi-container pods into single container image

### Unsupported Features (Architecture Limitations)

**Multi-Container Pod Limitations:**
```rust
// Architecture limitation example
pub enum PodCompatibility {
    SingleContainer {
        // Full compatibility
        vm_id: String,
        container_config: ContainerConfig,
    },
    MultiContainer {
        // Limited compatibility - requires workarounds
        containers: Vec<ContainerConfig>,
        networking_strategy: NetworkingStrategy,
    }
}

#[derive(Debug)]
enum NetworkingStrategy {
    SeparateVMs, // Multiple VMs with network bridge (complex)
    SingleVM,    // All containers in one VM (not true to Apple Container model)
    Unsupported, // Reject multi-container pods
}
```

**Kubernetes Features Requiring Alternatives:**
- **Init Containers**: Requires sequential VM execution
- **Sidecar Containers**: Challenges with shared volumes and networking
- **Pod-level Volumes**: Shared volumes between containers in different VMs
- **Container-to-Container Communication**: localhost networking within Pod

### Pod Networking Considerations

**Apple Container Networking Model:**
```rust
// Networking translation for Kubernetes pods
pub struct PodNetworkConfig {
    pub pod_cidr: String,
    pub service_cidr: String,
    pub dns_servers: Vec<String>,
    pub vm_network_mode: VMNetworkMode,
}

#[derive(Debug, Clone)]
pub enum VMNetworkMode {
    Dedicated {
        // Each container VM gets dedicated IP
        ip_address: std::net::IpAddr,
        gateway: std::net::IpAddr,
    },
    Shared {
        // Multiple containers share network namespace (complex)
        bridge_name: String,
        shared_ip: std::net::IpAddr,
    }
}

impl PodNetworkConfig {
    pub fn translate_to_apple_container(&self) -> AppleContainerNetworkConfig {
        match &self.vm_network_mode {
            VMNetworkMode::Dedicated { ip_address, gateway } => {
                AppleContainerNetworkConfig {
                    mode: "dedicated".to_string(),
                    ip_address: ip_address.to_string(),
                    gateway: gateway.to_string(),
                    dns_servers: self.dns_servers.clone(),
                }
            }
            VMNetworkMode::Shared { .. } => {
                // Complex implementation required
                todo!("Shared networking requires custom bridge implementation")
            }
        }
    }
}
```

## CRI Shim Deployment Architecture

### gRPC Server Configuration

```rust
// kina-cli/src/core/cri_server.rs
use tonic::transport::Server;
use std::path::PathBuf;

pub struct CRIServer {
    shim: AppleContainerCRIShim,
    socket_path: PathBuf,
}

impl CRIServer {
    pub fn new(socket_path: PathBuf) -> Self {
        let container_manager = KinaContainerManager::new();
        let shim = AppleContainerCRIShim::new(container_manager);

        Self { shim, socket_path }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Remove existing socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        // Create Unix domain socket server
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(
            tokio::net::UnixListener::bind(&self.socket_path)?
        );

        println!("CRI server starting on socket: {:?}", self.socket_path);

        // Start gRPC server
        Server::builder()
            .add_service(RuntimeServiceServer::new(self.shim.clone()))
            .add_service(ImageServiceServer::new(self.shim.clone()))
            .serve_with_incoming(uds_stream)
            .await?;

        Ok(())
    }
}
```

### kubelet Integration

```yaml
# kubelet configuration for Apple Container CRI
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
containerRuntime: remote
containerRuntimeEndpoint: unix:///var/run/kina-cri.sock
imageServiceEndpoint: unix:///var/run/kina-cri.sock
runtimeRequestTimeout: "10m"
streamingConnectionIdleTimeout: "4h"

# Feature gates for testing
featureGates:
  KubeletTracing: true

# Resource management
maxPods: 110
podCIDR: "10.244.0.0/24"
clusterDNS:
  - "10.96.0.10"
clusterDomain: "cluster.local"
```

## Testing CRI Compliance

### CRI Validation Test Suite

```rust
#[cfg(test)]
mod cri_tests {
    use super::*;
    use k8s_cri::v1::*;

    #[tokio::test]
    async fn test_pod_sandbox_lifecycle() {
        let shim = AppleContainerCRIShim::new(KinaContainerManager::new());

        // Create pod sandbox
        let request = RunPodSandboxRequest {
            config: Some(PodSandboxConfig {
                metadata: Some(PodSandboxMetadata {
                    name: "test-pod".to_string(),
                    namespace: "default".to_string(),
                    uid: "12345".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let response = shim.run_pod_sandbox(Request::new(request)).await.unwrap();
        let pod_id = response.into_inner().pod_sandbox_id;
        assert!(!pod_id.is_empty());

        // Test pod sandbox status
        let status_request = PodSandboxStatusRequest {
            pod_sandbox_id: pod_id.clone(),
            verbose: false,
        };

        let status_response = shim.pod_sandbox_status(Request::new(status_request)).await.unwrap();
        let status = status_response.into_inner().status.unwrap();
        assert_eq!(status.state, pod_sandbox_state::State::PodSandboxReady as i32);

        // Cleanup
        let stop_request = StopPodSandboxRequest {
            pod_sandbox_id: pod_id.clone(),
        };
        shim.stop_pod_sandbox(Request::new(stop_request)).await.unwrap();

        let remove_request = RemovePodSandboxRequest {
            pod_sandbox_id: pod_id,
        };
        shim.remove_pod_sandbox(Request::new(remove_request)).await.unwrap();
    }

    #[tokio::test]
    async fn test_container_lifecycle() {
        let shim = AppleContainerCRIShim::new(KinaContainerManager::new());

        // First create pod sandbox
        let pod_request = RunPodSandboxRequest {
            config: Some(PodSandboxConfig {
                metadata: Some(PodSandboxMetadata {
                    name: "test-pod".to_string(),
                    namespace: "default".to_string(),
                    uid: "12345".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let pod_response = shim.run_pod_sandbox(Request::new(pod_request)).await.unwrap();
        let pod_id = pod_response.into_inner().pod_sandbox_id;

        // Create container
        let container_request = CreateContainerRequest {
            pod_sandbox_id: pod_id.clone(),
            config: Some(ContainerConfig {
                metadata: Some(ContainerMetadata {
                    name: "test-container".to_string(),
                    ..Default::default()
                }),
                image: Some(ImageSpec {
                    image: "alpine:latest".to_string(),
                    ..Default::default()
                }),
                command: vec!["sleep".to_string(), "30".to_string()],
                ..Default::default()
            }),
            sandbox_config: None,
        };

        let container_response = shim.create_container(Request::new(container_request)).await.unwrap();
        let container_id = container_response.into_inner().container_id;
        assert!(!container_id.is_empty());

        // Start container
        let start_request = StartContainerRequest { container_id: container_id.clone() };
        shim.start_container(Request::new(start_request)).await.unwrap();

        // Verify container is running
        let status_request = ContainerStatusRequest {
            container_id: container_id.clone(),
            verbose: false,
        };
        let status_response = shim.container_status(Request::new(status_request)).await.unwrap();
        let status = status_response.into_inner().status.unwrap();
        assert_eq!(status.state, container_state::State::ContainerRunning as i32);

        // Cleanup
        let stop_request = StopContainerRequest {
            container_id: container_id.clone(),
            timeout: 10,
        };
        shim.stop_container(Request::new(stop_request)).await.unwrap();

        let remove_request = RemoveContainerRequest { container_id };
        shim.remove_container(Request::new(remove_request)).await.unwrap();
    }
}
```

## Implementation Limitations and Workarounds

### Multi-Container Pod Challenges

**Problem**: Apple Container's VM-per-container model conflicts with Kubernetes' shared-namespace pod model.

**Workaround Options:**

1. **Single-Container Pod Focus**
   ```rust
   // Reject multi-container pods with clear error
   if container_count > 1 {
       return Err(Status::unimplemented(
           "Multi-container pods require shared networking implementation"
       ));
   }
   ```

2. **Container Image Consolidation**
   ```rust
   // Merge multi-container pod into single container image
   pub struct PodContainerMerger {
       pub fn merge_containers(&self, containers: &[ContainerConfig]) -> Result<ContainerConfig, Error> {
           // Create supervisord-style container image
           // Run multiple processes in single VM
           todo!("Implement container merging strategy")
       }
   }
   ```

3. **Network Bridge Implementation** (Complex)
   ```rust
   // Custom networking for multi-container pods
   pub struct PodNetworkBridge {
       pub async fn create_shared_network(&self, pod_id: &str) -> Result<NetworkConfig, Error> {
           // Create shared network namespace across multiple VMs
           // Complex implementation involving virtual networking
           todo!("Implement cross-VM networking")
       }
   }
   ```

### Recommended Approach

**Phase 1**: Single-container pod support with clear multi-container limitations
**Phase 2**: Container consolidation for simple multi-container use cases
**Phase 3**: Advanced networking for full multi-container pod support