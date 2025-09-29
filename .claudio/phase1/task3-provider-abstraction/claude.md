# Task: Container Provider Abstraction Layer Design

You are working on a critical architecture task within Phase 1 of the kina implementation. This task focuses on designing the provider abstraction layer following KIND's provider pattern adapted for Apple Container capabilities.

## Task Objective:
Design provider abstraction layer following KIND's provider pattern, incorporating Apple Container research findings to create a robust, async-capable container provider interface.

## Task Requirements:
- Provider trait designed with Apple Container capabilities
- Error handling strategy defined for container operations
- Async operation patterns established
- Testing strategy for provider implementations defined

## Dependencies:
- Task 1 (Rust Project Structure) - Module foundation required
- Task 2 (Apple Container Research) - Capability assessment required

## Deliverables:
- ContainerProvider trait definition with comprehensive interface
- AppleContainerProvider skeleton implementation
- Error type definitions for container operations
- Testing infrastructure for provider implementations

## Context Integration:
- Phase Context: ../tasks.md
- Related Tasks: task1-rust-project-structure (foundation), task2-apple-container-research (constraints)
- Shared Resources: ../../shared/

## Implementation Guidelines:
**Provider Trait Pattern** (based on KIND's provider abstraction):
```rust
use async_trait::async_trait;

#[async_trait]
pub trait ContainerProvider: Send + Sync {
    async fn provision(&self, config: &ClusterConfig) -> Result<(), KinaError>;
    async fn list_nodes(&self, cluster: &str) -> Result<Vec<Node>, KinaError>;
    async fn delete_nodes(&self, nodes: &[Node]) -> Result<(), KinaError>;
    async fn get_api_server_endpoint(&self, cluster: &str) -> Result<String, KinaError>;
    async fn exec_in_container(&self, container_id: &str, cmd: &[&str]) -> Result<ExecResult, KinaError>;
}

pub struct AppleContainerProvider {
    runtime: AppleContainerRuntime,
    logger: Logger,
}
```

**Apple Container Specific Adaptations**:
- **VM-per-Container**: Design for VM management instead of traditional containers
- **Automatic Networking**: Remove network management functions, use automatic IP assignment
- **Single-Node Focus**: Optimize for single-node cluster patterns
- **Supported CLI Options**: Use `--name` only, avoid unsupported options

**Error Handling Strategy**:
- Structured error types using `thiserror` crate
- Provider-specific error variants
- Rich context for debugging and user feedback
- Error mapping from Apple Container runtime

## Success Criteria:
- Provider trait supports all required cluster operations
- Apple Container provider skeleton implements trait contract
- Error handling provides clear diagnostic information
- Testing infrastructure enables provider validation
- Design supports future runtime provider additions

## Next Steps:
After completing this task:
1. Update status.md with trait design and interface specification
2. Coordinate with task2-apple-container-research for constraint integration
3. Provide foundation for Phase 2 provider implementation