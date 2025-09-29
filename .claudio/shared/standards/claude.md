# Kina Project Development Standards

You are working on the Kina project - a Rust CLI application that provides macOS-native Kubernetes cluster management using Apple Container technology as an alternative to Kind.

## Project Context
- **Project**: Kina (Kubernetes in Apple Container)
- **Language**: Rust
- **Target Platform**: macOS 15.6+
- **Container Runtime**: Apple Container
- **Architecture**: CLI application with KIND-inspired modular design

## Apple Container Architecture Updates
Based on implementation plan research findings:
- **VM-per-Container**: Each container runs in dedicated lightweight Linux VM
- **Automatic IP Assignment**: No explicit network management required
- **VM Communication Limitation**: Inter-VM communication not available until macOS 26
- **Single-Node Focus**: Emphasis on single-node clusters with combined control-plane/worker roles
- **CLI Compatibility**: Use `--name` for container naming, no `--hostname` or `--privileged`
- **Self-contained VMs**: No host filesystem mounts required

## Development Standards

### Code Organization (KIND-Inspired)
Following KIND's proven package organization adapted for Rust:
- **Configuration**: `src/config/` - Types, validation, defaults (KIND: pkg/apis/config/)
- **Cluster Operations**: `src/cluster/` - Lifecycle, provider, orchestration (KIND: pkg/cluster/)
- **Container Integration**: `src/container/` - Runtime, image, network (KIND: pkg/cluster/providers/)
- **Image Management**: `src/image/` - Build, bootstrap (KIND: pkg/build/nodeimage/)
- **CLI Interface**: `src/cli/` - Commands, utilities (KIND: cmd/kind/)
- **Kubernetes Integration**: `src/k8s/` - kubeadm, client (KIND: pkg/kubeconfig/)

### Provider Abstraction Pattern
```rust
#[async_trait]
pub trait ContainerProvider: Send + Sync {
    async fn provision(&self, config: &ClusterConfig) -> Result<(), KinaError>;
    async fn list_nodes(&self, cluster: &str) -> Result<Vec<Node>, KinaError>;
    async fn delete_nodes(&self, nodes: &[Node]) -> Result<(), KinaError>;
    async fn get_api_server_endpoint(&self, cluster: &str) -> Result<String, KinaError>;
    async fn exec_in_container(&self, container_id: &str, cmd: &[&str]) -> Result<ExecResult, KinaError>;
}
```

### Rust Coding Standards
- **Formatting**: Use `cargo fmt` for consistent code formatting
- **Linting**: Address all `cargo clippy` warnings and suggestions
- **Documentation**: All public APIs must have rustdoc comments
- **Error Handling**: Use `Result<T, E>` with `thiserror` for structured errors
- **Async Operations**: Use `tokio` for async container and CLI operations
- **Testing**: Unit tests for all modules, integration tests for CLI commands

### CLI Design Principles (KIND Compatible)
- **Command Structure**: Mirror KIND command patterns exactly
- **Help System**: Comprehensive help matching KIND's format
- **Error Messages**: Clear, actionable error messages with suggested fixes
- **Configuration**: YAML compatibility with KIND configuration files
- **Output Formats**: Human-readable matching KIND's output style

### Apple Container Integration Standards
- **CLI Usage**: Use supported options only (`--name`, avoid `--hostname`, `--privileged`)
- **VM Management**: Treat containers as VMs with dedicated networking
- **Network Simplification**: Leverage automatic IP assignment and DNS
- **Resource Management**: Proper cleanup of VMs and resources
- **Single-Node Optimization**: Design for single-node cluster patterns

### Kubernetes Compatibility
- **kubectl Integration**: Generate proper kubeconfig for kubectl access
- **KIND Workflow Compatibility**: Drop-in replacement for KIND workflows
- **Tool Integration**: Support kubectx, kubens, k9s, and other tools
- **Manifest Support**: Support standard Kubernetes manifest files
- **API Compatibility**: Ensure Kubernetes API compatibility

### Testing Standards
- **Unit Tests**: Test individual functions and modules with mocks
- **Integration Tests**: Test CLI commands with `assert_cmd`
- **Provider Tests**: Mock Apple Container operations for testing
- **End-to-End Tests**: Test complete cluster lifecycle workflows
- **Compatibility Tests**: Validate integration with Kubernetes ecosystem tools

### Security Considerations
- **RBAC**: Implement proper role-based access control
- **Network Security**: Configure secure network policies by default
- **Secrets Management**: Proper handling of Kubernetes secrets
- **VM Security**: Leverage Apple Container VM isolation
- **Certificate Management**: Proper TLS certificate generation and rotation

### Quality Gates
- All tests must pass: `cargo test`
- Code must be formatted: `cargo fmt --check`
- No clippy warnings: `cargo clippy -- -D warnings`
- Security audit passes: `cargo audit`
- Documentation is complete and accurate
- Integration tests validate all functionality

### Performance Targets (Requires Measurement)
- Cluster creation: Target comparable to KIND performance
- Memory usage: Efficient resource utilization
- Resource cleanup: Complete cleanup with no orphaned resources
- Command response: CLI commands respond promptly
- Startup time: Fast application initialization

### Compatibility Requirements
- **macOS Versions**: Support macOS 15.6 and later (Apple Container requirement)
- **Kubernetes Versions**: Support current and previous major versions
- **Tool Compatibility**: kubectl, kubectx, kubens, k9s integration
- **Workflow Compatibility**: Drop-in replacement for common KIND workflows
- **Migration Path**: Easy migration from existing KIND clusters