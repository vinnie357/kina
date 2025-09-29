# Task: Phased Cluster Lifecycle Implementation

You are working on a core orchestration task within Phase 2 of the kina implementation. This task focuses on implementing KIND's phased cluster creation workflow adapted for Apple Container's VM-per-container architecture.

## Task Objective:
Implement KIND's phased cluster creation workflow with Apple Container, creating a robust, extensible action pipeline that supports debugging, partial cleanup, and future feature additions.

## Task Requirements:
- Cluster creation follows KIND's sequential action pattern
- Each phase can be executed independently for debugging
- Error handling supports partial cleanup on failure
- Action pipeline supports extensibility for future features

## Dependencies:
- Task 1 (Apple Container Provider Implementation)

## Deliverables:
- ClusterLifecycle struct with create/delete operations
- Action trait and ActionPipeline for extensible workflow execution
- CleanupGuard for automatic resource cleanup on failures
- Comprehensive logging and progress reporting

## Context Integration:
- Phase Context: ../tasks.md
- Related Tasks: task1-provider-implementation (foundation), task3-kubernetes-bootstrap (consumer)
- Shared Resources: ../../shared/

## Implementation Guidelines:
**Action Pipeline Pattern** (KIND-inspired):
```rust
pub struct ClusterLifecycle {
    provider: Arc<dyn ContainerProvider>,
    logger: Logger,
}

impl ClusterLifecycle {
    pub async fn create_cluster(&self, options: &ClusterOptions) -> Result<(), KinaError> {
        let cleanup_guard = CleanupGuard::new(&options.config.name, &self.provider);

        // Phase-based execution with comprehensive error handling
        let mut pipeline = ActionPipeline::new();
        pipeline
            .add_action(Box::new(LoadBalancerAction::new()))
            .add_action(Box::new(ConfigAction::new()));

        pipeline.execute(&ActionContext::new(&options.config, &self.provider)).await?;
        cleanup_guard.disarm();
        Ok(())
    }
}
```

**Action System Design**:
- **Action Trait**: Standardized interface for pipeline steps
- **ActionContext**: Shared context for cross-action communication
- **ActionPipeline**: Sequential execution with error handling
- **CleanupGuard**: Automatic resource cleanup on failures

**Apple Container Adaptations**:
- **VM Provisioning**: Container creation adapted for VM architecture
- **Single-Node Optimization**: Pipeline optimized for single-node clusters
- **Network Simplification**: Remove Docker-style network creation steps
- **Validation Updates**: Apple Container specific validation steps

## Success Criteria:
- Cluster lifecycle follows predictable, debuggable phases
- Action pipeline supports independent phase execution
- Error handling provides partial cleanup capabilities
- Logging provides comprehensive progress tracking
- Pipeline design supports future feature additions

## Next Steps:
After completing this task:
1. Update status.md with lifecycle implementation status and action catalog
2. Coordinate with task3-kubernetes-bootstrap for Kubernetes action integration
3. Provide lifecycle foundation for cluster management commands