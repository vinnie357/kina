# Task: Apple Container Provider Implementation

You are working on a core implementation task within Phase 2 of the kina implementation. This task focuses on implementing the ContainerProvider trait for Apple Container runtime following KIND's provider abstraction patterns.

## Task Objective:
Implement ContainerProvider trait for Apple Container runtime following KIND's provider abstraction, creating the foundation for cluster lifecycle management with VM-per-container architecture.

## Task Requirements:
- AppleContainerProvider implements full ContainerProvider interface
- Container lifecycle operations (create, start, stop, delete) functional
- Error handling provides clear diagnostic information
- Async operations support concurrent container management

## Dependencies:
- Phase 1 Tasks 1-3 (Project structure, research, provider abstraction)

## Deliverables:
- Complete AppleContainerProvider implementation with all trait methods
- Container specification builder for Kubernetes node requirements
- Network management for container-to-container communication
- Container labeling system for cluster and node identification

## Context Integration:
- Phase Context: ../tasks.md
- Related Tasks: task2-cluster-lifecycle (consumer), task6-image-network-management (parallel)
- Shared Resources: ../../shared/
- Phase 1 Foundation: ../../phase1/task3-provider-abstraction/

## Implementation Guidelines:
**Provider Implementation Pattern**:
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
        // Implementation following KIND patterns with Apple Container adaptations
    }
}
```

**Apple Container Specific Adaptations**:
- **VM-per-Container**: Each node runs in dedicated Linux VM
- **Automatic IP Assignment**: Leverage Apple Container DNS and networking
- **Single-Node Focus**: Optimize for single-node cluster patterns
- **Supported CLI Options**: Use `--name` for container naming, avoid unsupported flags

**Container Specification Builder**:
- Essential volume mounts for Kubernetes (/var/lib/kubelet, /etc/kubernetes)
- Proper labeling for cluster and node identification
- VM configuration for Kubernetes node requirements
- Security context appropriate for systemd and Kubernetes

## Success Criteria:
- Provider implements all required ContainerProvider trait methods
- Container operations work reliably with Apple Container runtime
- Error handling provides clear diagnostic information for troubleshooting
- Async operations support concurrent node management
- Integration tests validate provider functionality

## Next Steps:
After completing this task:
1. Update status.md with provider implementation status and capabilities
2. Coordinate with task2-cluster-lifecycle for lifecycle integration
3. Provide foundation for core cluster management operations