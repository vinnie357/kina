# Testing Context for Rust CLI Container Orchestration

## Project-Specific Guidance
Based on discovery analysis, this project requires comprehensive testing strategies for a Rust CLI application with assert_cmd and predicates for CLI testing, integration tests for Apple Container operations, and Tokio async testing patterns.

## Recommended Approaches
- **CLI Testing Framework**: Use assert_cmd (2.0) and predicates (3.0) for comprehensive command-line interface testing
- **Container Integration Testing**: Implement integration tests for Apple Container operations with proper setup and teardown
- **Async Testing Patterns**: Use Tokio test macros for async container operations and Kubernetes API client testing
- **Cargo Test Structure**: Leverage workspace testing with unit tests, integration tests, and CLI acceptance tests

## Integration Patterns
Testing integrates with the existing project architecture:
- Cargo test framework with workspace member testing coordination
- assert_cmd integration for CLI subcommand testing and output validation
- Test fixtures in kina-cli/tests/fixtures/ for container and Kubernetes configuration testing
- mise.toml task automation (mise run test) for comprehensive test execution
- Integration with Apple Container runtime for realistic container operation testing

## Quality Standards
- **CLI Test Coverage**: Comprehensive testing of all clap subcommands, argument parsing, and error handling
- **Container Operation Testing**: Proper testing of Apple Container lifecycle operations with cleanup
- **Async Test Reliability**: Reliable async testing with proper timeout handling and resource cleanup
- **Integration Test Isolation**: Proper test isolation for container state and Kubernetes cluster interactions

## Next Steps
- Execute mise run test to validate current test infrastructure
- Implement comprehensive CLI testing with assert_cmd for all subcommand operations
- Establish Apple Container integration testing patterns with proper mocking capabilities
- Create async testing patterns for Kubernetes API operations and concurrent container management