# Kina Project Development Standards

You are working on the Kina project - a Rust CLI application that provides macOS-native Kubernetes cluster management using Apple Container technology as an alternative to Kind.

## Project Context
- **Project**: Kina (Kubernetes in Apple Container)
- **Language**: Rust
- **Target Platform**: macOS 15.6+
- **Container Runtime**: Apple Container
- **Architecture**: CLI application with modular design

## Development Standards

### Code Organization
- **Main Entry**: `src/main.rs` - Application entry point
- **CLI Module**: `src/cli/` - Command line interface and argument parsing
- **Core Logic**: `src/core/` - Business logic for cluster and container management
- **Configuration**: `src/config/` - Configuration management and validation
- **Utilities**: `src/utils/` - Helper functions and common utilities
- **Error Handling**: `src/errors/` - Custom error types and error handling

### Rust Coding Standards
- **Formatting**: Use `cargo fmt` for consistent code formatting
- **Linting**: Address all `cargo clippy` warnings and suggestions
- **Documentation**: All public APIs must have rustdoc comments
- **Error Handling**: Use `Result<T, E>` for all fallible operations
- **Async Operations**: Use `tokio` for async container and CLI operations
- **Testing**: Unit tests for all modules, integration tests for CLI commands

### CLI Design Principles
- **Command Structure**: Follow kubectl patterns for familiarity
- **Help System**: Comprehensive help for all commands and subcommands
- **Error Messages**: Clear, actionable error messages with suggested fixes
- **Configuration**: Support both CLI flags and configuration files
- **Output Formats**: Support human-readable and machine-readable output

### Container Integration Standards
- **Apple Container CLI**: Use subprocess calls to Apple Container CLI
- **Error Handling**: Capture and translate container runtime errors
- **Resource Management**: Proper cleanup of containers and resources
- **Networking**: Consistent network configuration for all clusters
- **Persistence**: Proper handling of persistent volumes and data

### Kubernetes Compatibility
- **kubectl Integration**: Generate proper kubeconfig for kubectl access
- **API Compatibility**: Ensure Kubernetes API compatibility
- **Tool Integration**: Support kubectx, kubens, k9s, and other tools
- **Manifest Support**: Support standard Kubernetes manifest files
- **Namespace Management**: Proper namespace creation and management

### Testing Standards
- **Unit Tests**: Test individual functions and modules
- **Integration Tests**: Test CLI commands and container operations
- **End-to-End Tests**: Test complete cluster lifecycle workflows
- **Performance Tests**: Benchmark cluster creation and operation times
- **Compatibility Tests**: Validate integration with Kubernetes ecosystem tools

### Security Considerations
- **RBAC**: Implement proper role-based access control
- **Network Security**: Configure secure network policies by default
- **Secrets Management**: Proper handling of Kubernetes secrets
- **Container Security**: Security scanning and vulnerability assessment
- **Certificate Management**: Proper TLS certificate generation and rotation

### Documentation Requirements
- **Code Documentation**: Comprehensive rustdoc for all public APIs
- **User Guides**: Step-by-step guides for common workflows
- **Architecture Documentation**: System design and integration patterns
- **Troubleshooting**: Common issues and resolution procedures
- **Migration Guides**: Help users migrate from Kind to Kina

## Quality Gates
- All tests must pass: `cargo test`
- Code must be formatted: `cargo fmt --check`
- No clippy warnings: `cargo clippy -- -D warnings`
- Security audit passes: `cargo audit`
- Documentation is complete and accurate
- Integration tests validate all functionality

## Performance Targets
- Cluster creation: Under 30 seconds for basic cluster
- Memory usage: Reasonable footprint for development use
- Resource cleanup: Complete cleanup with no orphaned resources
- Command response: CLI commands respond within 2 seconds
- Startup time: Application starts within 1 second

## Compatibility Requirements
- **macOS Versions**: Support macOS 15.6 and later
- **Kubernetes Versions**: Support current and previous major versions
- **Tool Compatibility**: kubectl, kubectx, kubens, k9s integration
- **Workflow Compatibility**: Drop-in replacement for common Kind workflows
- **Migration Path**: Easy migration from existing Kind clusters