# Testing Troubleshooting for Rust CLI Container Orchestration

## Common Issues

### Assert_cmd Testing with Container Dependencies
**Problem**: CLI tests fail due to Apple Container runtime requirements or missing system dependencies
**Solution**: Implement conditional testing with feature flags, use mock providers for container operations in CI environments, and separate integration tests requiring actual Apple Container runtime.

### Async Test Reliability with Tokio
**Problem**: Async tests become flaky due to timing issues and concurrent container operations
**Solution**: Use proper Tokio test macros, implement timeouts for container operations, ensure proper async resource cleanup, and use test isolation for concurrent operations.

### Container State Management in Tests
**Problem**: Tests interfere with each other due to shared container state or cleanup failures
**Solution**: Implement proper test cleanup with Drop traits, use unique container names/IDs for test isolation, and ensure container removal in test teardown.

### CLI Output Testing with Complex Formats
**Problem**: Testing structured CLI output (JSON, YAML, table) becomes complex with assert_cmd predicates
**Solution**: Use predicates for flexible output matching, implement custom predicates for structured data validation, and test output format consistency across subcommands.

### Integration Testing with Kubernetes Clusters
**Problem**: Tests requiring Kubernetes cluster access fail in CI environments
**Solution**: Use mock Kubernetes clients for unit tests, implement optional integration tests for real cluster access, and use test cluster configurations for isolated testing.

## Debug Strategies
- **Test Execution**: Use mise run test for comprehensive test execution with proper environment setup
- **CLI Testing**: Execute cargo test --bin kina-cli for CLI-specific testing with assert_cmd validation
- **Container Testing**: Use conditional compilation and feature flags for Apple Container-dependent tests
- **Async Debugging**: Add tracing to async tests for debugging timing and resource issues
- **Test Isolation**: Ensure proper test cleanup and state isolation for reliable test execution

## Getting Help
- **Rust Testing**: Rust book testing chapter and cargo test documentation for comprehensive testing strategies
- **CLI Testing**: assert_cmd and predicates documentation for CLI testing patterns and output validation
- **Async Testing**: Tokio testing documentation for async test patterns and reliability strategies
- **Container Testing**: Apple Container documentation for testing strategies and integration approaches
- **Project Specific**: kina-cli/tests/ directory and mise.toml test configuration for project testing patterns