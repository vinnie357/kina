# Discovery Troubleshooting for Rust CLI Application

## Common Issues

### Rust Project Structure Not Detected
**Problem**: Discovery fails to identify Rust project components
**Solution**: Verify Cargo.toml presence in project root and check for src/ directory structure. For early-stage projects, create minimal Cargo.toml manifest.

### Apple Container Runtime Missing
**Problem**: Apple Container availability cannot be validated
**Solution**: Verify macOS version 15.6+ and Apple Container CLI installation. Use `which container` to check availability and validate container runtime permissions.

### Kubernetes Configuration Not Found
**Problem**: Discovery cannot locate Kubernetes integration patterns
**Solution**: Check for kubeconfig files, kubectl installation, and cluster configuration. Verify Kubernetes client library dependencies in planned Cargo.toml.

### CLI Framework Detection Issues
**Problem**: Command-line interface patterns not properly identified
**Solution**: Look for clap, structopt, or other CLI framework dependencies. Analyze main.rs for command parsing patterns and CLI application structure.

## Debug Strategies
- **Cargo Analysis**: Use `cargo metadata` to extract dependency information and workspace structure
- **Platform Validation**: Verify Apple Container runtime with `container --version` and system compatibility
- **Kubernetes Testing**: Test kubectl connectivity and cluster access for integration validation
- **Development Environment**: Validate mise configuration and development tool availability

## Getting Help
- **Rust Community**: Rust Users Forum and official documentation for CLI development patterns
- **Apple Container**: Apple Developer documentation for container runtime integration
- **Kubernetes**: Official Kubernetes documentation and kubectl reference guides
- **Project Resources**: README.md and .claudio/docs/ analysis documents for project-specific guidance