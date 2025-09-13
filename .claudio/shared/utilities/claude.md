# Kina Project Utilities and Helpers

You have access to shared utilities and helper functions for consistent implementation across all phases of the Kina project.

## Project Utilities Context
- **Project**: Kina (Kubernetes in Apple Container)
- **Shared Resources**: Common patterns and utilities for Rust CLI development
- **Integration Points**: Apple Container runtime, Kubernetes API, macOS system integration

## Development Utilities

### Cargo Workspace Utilities
```toml
# Example Cargo.toml structure for workspace
[workspace]
members = ["kina-cli", "kina-core", "kina-container"]
resolver = "2"

[workspace.dependencies]
clap = "4.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
```

### CLI Command Patterns
- **Command Structure**: Use clap derive API for consistent CLI structure
- **Subcommands**: Implement subcommands for cluster, config, and utility operations
- **Global Flags**: Support common flags like --verbose, --output, --kubeconfig
- **Error Handling**: Consistent error reporting with helpful messages
- **Configuration**: Support both CLI arguments and configuration files

### Container Integration Patterns
- **Apple Container CLI**: Execute container commands via subprocess
- **Command Building**: Build container commands programmatically
- **Output Parsing**: Parse container command output for status information
- **Error Translation**: Convert container runtime errors to user-friendly messages
- **Resource Tracking**: Track created containers for proper cleanup

### Kubernetes Integration Utilities
- **kubeconfig Generation**: Generate proper kubeconfig files for clusters
- **API Client**: Use kubernetes-client crate for API interactions
- **Manifest Processing**: Parse and validate Kubernetes manifests
- **Resource Management**: Track and manage Kubernetes resources
- **Status Monitoring**: Monitor cluster and resource health

### Configuration Management
- **Config File Locations**: Follow XDG Base Directory specification
- **Configuration Schema**: Use serde for configuration serialization
- **Validation**: Implement configuration validation and error reporting
- **Merging**: Support configuration from multiple sources (CLI, file, environment)
- **Secrets**: Secure handling of sensitive configuration data

### Logging and Observability
- **Structured Logging**: Use tracing crate for structured logging
- **Log Levels**: Support debug, info, warn, error log levels
- **Context**: Include relevant context in log messages
- **Performance Metrics**: Track operation timing and resource usage
- **User Feedback**: Provide progress indicators for long operations

### Testing Utilities
- **Test Fixtures**: Common test data and cluster configurations
- **Mock Objects**: Mock container runtime for testing
- **Assertion Helpers**: Custom assertions for cluster state validation
- **Integration Test Setup**: Helper functions for integration test environment
- **Performance Testing**: Utilities for benchmarking and performance validation

### Error Handling Patterns
- **Custom Error Types**: Define domain-specific error types
- **Error Context**: Provide helpful context for error diagnosis
- **User-Friendly Messages**: Convert technical errors to user-actionable messages
- **Error Recovery**: Implement graceful error recovery where possible
- **Logging**: Log errors with appropriate detail for debugging

## Implementation Guidelines

### Async Operation Patterns
- Use `tokio` runtime for all async operations
- Implement proper cancellation handling for long-running operations
- Use `futures` combinators for concurrent operations
- Handle timeouts appropriately for container and API operations

### Resource Management
- Implement RAII patterns for resource cleanup
- Track all created resources for proper cleanup
- Use `Drop` trait for automatic resource cleanup
- Implement graceful shutdown procedures

### Platform Integration
- Use platform-specific APIs for macOS integration
- Handle system permissions and security properly
- Integrate with macOS security frameworks as needed
- Follow macOS application development best practices

### Performance Optimization
- Use efficient data structures for resource tracking
- Implement caching for expensive operations
- Minimize subprocess overhead for container operations
- Use connection pooling for Kubernetes API calls

## Common Code Patterns

### Command Execution Pattern
```rust
// Example pattern for executing container commands
async fn execute_container_command(args: Vec<&str>) -> Result<String, ContainerError> {
    let output = Command::new("podman") // or Apple Container CLI
        .args(&args)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(ContainerError::ExecutionFailed(
            String::from_utf8(output.stderr)?
        ))
    }
}
```

### Configuration Loading Pattern
```rust
// Example pattern for configuration management
#[derive(Debug, Deserialize)]
pub struct KinaConfig {
    pub default_cluster_name: String,
    pub container_runtime: String,
    pub kubernetes_version: String,
}

impl KinaConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // Load from file, environment, and CLI args
        // Implement proper precedence and validation
    }
}
```

## Integration Helpers

### Apple Container Integration
- Research and document Apple Container CLI interface
- Create abstraction layer for container operations
- Implement proper error handling for container runtime failures
- Support for container lifecycle management

### Kubernetes API Integration
- Use official Kubernetes client library
- Implement proper authentication and authorization
- Handle API versioning and compatibility
- Support for custom resource definitions

### macOS System Integration
- Handle macOS security permissions
- Integrate with system networking
- Support for macOS application lifecycle
- Follow macOS application distribution standards