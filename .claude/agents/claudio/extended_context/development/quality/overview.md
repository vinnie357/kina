# Code Quality Context for Rust CLI Application

## Project-Specific Guidance
Based on discovery analysis, this project requires Rust-specific code quality standards for CLI development with Apple Container integration and Kubernetes client library usage.

## Recommended Approaches
- **Rust Quality Tools**: Implement rustfmt for formatting, clippy for linting, and cargo-audit for security scanning
- **CLI Testing Patterns**: Use assert_cmd for CLI testing, integration tests with real Apple Container instances
- **Error Handling Standards**: Implement anyhow or thiserror for comprehensive error handling throughout CLI operations
- **Documentation Requirements**: Ensure all public APIs have rustdoc documentation and CLI commands have help text

## Integration Patterns
Code quality integrates with Rust development ecosystem through:
- Cargo.toml configuration for quality tools and testing dependencies
- Pre-commit hooks for automated rustfmt and clippy validation
- GitHub Actions or mise task integration for continuous quality validation
- Apple Container and Kubernetes integration testing with quality gate validation

## Quality Standards
- **Rust Idioms**: Follow Rust naming conventions, ownership patterns, and error handling best practices
- **CLI User Experience**: Consistent command structure, helpful error messages, and comprehensive help documentation
- **Platform Reliability**: Robust error handling for Apple Container failures and Kubernetes connectivity issues
- **Performance Standards**: Efficient container operations and minimal startup time for CLI responsiveness

## Next Steps
- Configure rustfmt with project-specific formatting rules
- Set up clippy with CLI-specific lints and Apple Container integration validation
- Establish testing framework with assert_cmd and Apple Container integration tests
- Create documentation standards for CLI commands and API integration patterns