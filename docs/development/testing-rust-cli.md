# Rust CLI Testing Context for kina Project

## Project Testing Overview

### Technology Stack
- **Primary Language**: Rust
- **Testing Framework**: cargo test (Rust standard testing)
- **Project Type**: CLI application (Kubernetes in Apple Container)
- **Target Platform**: macOS 15.6+ with Apple Container runtime
- **Development Tools**: mise for task management and environment setup

### Test Runner Configuration
- **Standard Tests**: `cargo test` for unit and integration tests
- **Verbose Output**: `cargo test -- --nocapture` for detailed output
- **Pattern Filtering**: `cargo test [pattern]` for specific test execution
- **Parallel Execution**: Rust default parallel test execution
- **Documentation Tests**: Automatic doc test execution with cargo test

### Project Directory Structure
```
kina/
├── Cargo.toml              # Project manifest (to be created)
├── src/                    # Source code directory
│   ├── main.rs            # CLI entry point
│   ├── cli/               # Command line interface modules
│   ├── core/              # Core business logic
│   ├── container/         # Apple Container integration
│   └── k8s/               # Kubernetes workflow logic
├── tests/                 # Integration tests directory
│   ├── cli_tests.rs       # CLI command testing
│   ├── container_tests.rs # Apple Container integration tests
│   └── k8s_tests.rs       # Kubernetes workflow tests
└── mise.toml              # Development environment configuration
```

## Rust Testing Patterns

### Unit Testing Patterns
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_argument_parsing() {
        // Test CLI command parsing logic
    }

    #[test]
    fn test_container_configuration() {
        // Test Apple Container configuration
    }
}
```

### Integration Testing Patterns
```rust
// tests/cli_tests.rs
use std::process::Command;

#[test]
fn test_kina_cluster_create() {
    let output = Command::new("cargo")
        .args(&["run", "--", "cluster", "create", "test-cluster"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}
```

### CLI Testing Strategies
- **Command Parsing Tests**: Validate argument parsing and command structure
- **Error Handling Tests**: Test invalid inputs and error message clarity
- **Integration Tests**: End-to-end workflow testing with actual container operations
- **Mock Testing**: Apple Container API mocking for unit tests

## Common Issues and Solutions

### Pre-Implementation State Issues
- **No Cargo.toml**: Project is in planning phase, requires Rust project initialization
- **Missing src/ Directory**: Need to create basic Rust project structure
- **No Test Files**: Initial project setup requires test directory and file creation

### Development Environment Issues
- **Rust Toolchain**: Ensure rustc and cargo are installed and up-to-date
- **mise Configuration**: Validate mise.toml for consistent development environment
- **Apple Container CLI**: Verify Apple Container runtime is available and configured
- **Kubernetes Tools**: Ensure kubectl and related tools are installed

### Testing Framework Issues
- **Test Discovery**: Cargo automatically discovers tests in src/ and tests/ directories
- **Test Compilation**: Rust tests must compile before execution
- **Dependency Management**: Test dependencies should be in [dev-dependencies] section
- **Platform-Specific Tests**: Use cfg attributes for macOS-specific test logic

### Apple Container Integration Testing
- **Container Runtime Availability**: Test Apple Container CLI accessibility
- **Permission Issues**: Handle container runtime permission requirements
- **Resource Management**: Test container lifecycle and cleanup procedures
- **Network Configuration**: Validate container networking for Kubernetes clusters

## Integration Points

### CI/CD Workflow Integration
- **GitHub Actions**: Rust testing workflow with cargo test
- **Code Coverage**: cargo-tarpaulin for coverage reporting
- **Linting Integration**: clippy and rustfmt in testing pipeline
- **Security Scanning**: cargo-audit for dependency vulnerability checking

### Development Environment Setup
- **mise Tasks**: Standardized commands for test execution
- **Pre-commit Hooks**: Automated testing before git commits
- **IDE Integration**: VS Code or other IDE test runner integration
- **Documentation**: cargo doc testing for API documentation examples

### Performance and Optimization
- **Test Performance**: Monitor test execution time and resource usage
- **Parallel Testing**: Leverage Rust's parallel test execution capabilities
- **Integration Test Optimization**: Minimize external dependency calls in tests
- **Container Testing Efficiency**: Optimize Apple Container test setup and teardown

## Project-Specific Testing Requirements

### kina CLI Testing Priorities
1. **Command Structure Testing**: Validate CLI command hierarchy and argument parsing
2. **Apple Container Integration**: Test container runtime interactions and configuration
3. **Kubernetes Workflow Testing**: End-to-end cluster management operation testing
4. **Platform Compatibility**: Ensure macOS-specific functionality works correctly
5. **Error Handling**: Test error scenarios and user-friendly error messaging

### Test Coverage Areas
- **CLI Interface**: Command parsing, help text, error messages
- **Core Logic**: Cluster management algorithms and workflows
- **Container Operations**: Apple Container API interactions and lifecycle management
- **Kubernetes Integration**: kubectl command generation and execution
- **Configuration Management**: Settings persistence and environment handling

### Future Testing Considerations
- **Performance Benchmarking**: Container operation speed and resource usage
- **Stress Testing**: Large cluster management and concurrent operations
- **Integration Testing**: Full workflow testing with real Kubernetes clusters
- **User Experience Testing**: CLI usability and workflow efficiency validation