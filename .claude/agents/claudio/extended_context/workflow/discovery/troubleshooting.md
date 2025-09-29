# Discovery Troubleshooting for Rust CLI Container Orchestration

## Common Issues

### Cargo Workspace Analysis Failures
**Problem**: Discovery fails to parse complex workspace structures or dependencies
**Solution**: Verify Cargo.toml workspace configuration syntax, check for valid member packages, and ensure all dependencies are properly declared. Use `cargo metadata --format-version 1` for validation.

### Apple Container Runtime Detection Issues
**Problem**: Apple Container availability cannot be validated or integrated
**Solution**: Verify macOS 15.6+ compatibility, check container CLI installation with `which container`, validate runtime permissions, and test basic container operations.

### Kubernetes Client Integration Problems
**Problem**: kube-rs dependencies not properly configured or Kubernetes API access fails
**Solution**: Verify k8s-openapi version compatibility, check kubeconfig availability, validate cluster access with `kubectl cluster-info`, and ensure proper RBAC permissions.

### CLI Framework Pattern Recognition Failures
**Problem**: Clap derive patterns or subcommand structures not detected
**Solution**: Analyze clap version compatibility (4.5+), verify derive feature enablement, check for custom CLI patterns, and validate argument parsing implementation.

### Async Runtime Configuration Issues
**Problem**: Tokio async patterns not properly identified or configured
**Solution**: Verify Tokio version compatibility (1.47+), check for proper async/await usage, validate runtime initialization with `#[tokio::main]`, and ensure error handling integration.

## Debug Strategies
- **Workspace Validation**: Use `cargo check --workspace` to validate project structure and dependencies
- **Container Runtime Testing**: Execute `container --version` and basic container operations for integration validation
- **Kubernetes Connectivity**: Test cluster access with `kubectl get nodes` and API client connectivity
- **CLI Framework Testing**: Validate clap patterns with `cargo run -- --help` and subcommand functionality
- **Development Environment**: Check mise configuration, tool availability, and development workflow execution

## Getting Help
- **Rust Cargo**: cargo book and workspace documentation for complex project structures
- **Apple Container**: Apple Developer container runtime documentation and integration guides
- **Kubernetes Client**: kube-rs documentation and Kubernetes API reference for client integration
- **CLI Development**: clap documentation and Rust CLI book for command-line application patterns
- **Project Specific**: .claudio/docs/discovery.md for detailed project analysis and technology assessment