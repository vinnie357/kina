# Task: Configuration Schema and Validation System

You are working on a configuration management task within Phase 1 of the kina implementation. This task focuses on implementing a configuration system compatible with KIND's YAML format while adding Rust-specific validation capabilities.

## Task Objective:
Implement configuration system compatible with KIND's YAML format, providing robust validation, default configurations, and command-line override capabilities for seamless user experience.

## Task Requirements:
- Configuration schema supports KIND-compatible YAML format
- Validation system provides clear error messages
- Default configurations available for common scenarios
- Configuration merging supports command-line overrides

## Dependencies:
- Task 1 (Rust Project Structure) - Module foundation required

## Deliverables:
- Complete configuration type definitions with serde serialization
- Validation system with comprehensive error handling
- Default configuration templates for common scenarios
- Configuration file loading and merging logic

## Context Integration:
- Phase Context: ../tasks.md
- Related Tasks: task1-rust-project-structure (foundation), task4-cli-framework (parallel)
- Shared Resources: ../../shared/

## Implementation Guidelines:
**Configuration Schema** (KIND-compatible):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub name: String,
    pub nodes: Vec<NodeConfig>,
    pub networking: NetworkingConfig,
    #[serde(rename = "featureGates")]
    pub feature_gates: BTreeMap<String, bool>,
    #[serde(rename = "kubeadmConfigPatches")]
    pub kubeadm_config_patches: Vec<String>,
}
```

**Apple Container Adaptations**:
- **Single-Node Defaults**: Default configurations optimized for single-node clusters
- **VM-Specific Settings**: Configuration options for VM-per-container architecture
- **Network Simplification**: Remove Docker-style networking configurations
- **Image Settings**: Apple Container specific image configuration

**Validation Strategy**:
- Rust type system for compile-time validation
- Runtime validation for business logic constraints
- Clear error messages with suggestion for corrections
- Apple Container capability validation

**Default Configurations**:
- Single-node development cluster
- Multi-node cluster (when Apple Container supports inter-VM communication)
- Kubernetes version-specific defaults
- Apple Container optimized settings

## Success Criteria:
- Configuration system fully compatible with KIND YAML format
- Validation provides helpful error messages for common mistakes
- Default configurations work out-of-the-box for common scenarios
- Command-line overrides work seamlessly with file configuration
- Apple Container constraints properly validated

## Next Steps:
After completing this task:
1. Update status.md with configuration schema and validation capabilities
2. Coordinate with task4-cli-framework for CLI integration
3. Provide configuration foundation for Phase 2 cluster management