# Code Quality Troubleshooting for Rust CLI Container Orchestration

## Common Issues

### Rustfmt Configuration Conflicts with CLI Patterns
**Problem**: Default rustfmt rules don't align with clap derive patterns and CLI command structures
**Solution**: Leverage existing rustfmt.toml configuration (max_width: 100, edition 2021), validate formatting with mise run format, and adjust CLI-specific formatting for clap derive macros and subcommand definitions.

### Clippy Warnings for Async Container Operations
**Problem**: Clippy false positives for legitimate async patterns and Apple Container integration code
**Solution**: Use configured clippy.toml (cognitive_complexity_threshold: 30), add specific allow annotations for container runtime code, and validate with mise run lint for project-appropriate linting rules.

### Testing Container Runtime Dependencies
**Problem**: Quality gates fail due to Apple Container runtime requirements in CI/CD environments
**Solution**: Implement mock providers for container operations, use conditional compilation for platform-specific tests, and separate integration tests requiring actual Apple Container runtime.

### Assert_cmd Testing with Complex CLI Hierarchies
**Problem**: CLI testing becomes complex with nested subcommands and container state management
**Solution**: Use assert_cmd and predicates for command testing, implement test fixtures for container configurations, and create helper functions for CLI state setup and teardown.

### Error Handling Quality with Multiple Error Types
**Problem**: Inconsistent error handling between anyhow and thiserror usage across CLI and container operations
**Solution**: Establish clear error handling patterns for CLI (anyhow for main errors) vs library code (thiserror for custom types), ensure proper error context propagation, and validate error message quality.

## Debug Strategies
- **Quality Tool Validation**: Execute mise run ci to validate complete quality pipeline including format, lint, test, and audit steps
- **Container Testing**: Use conditional testing for Apple Container integration with proper mocking for CI environments
- **CLI Testing**: Implement comprehensive assert_cmd testing for all subcommands with container state validation
- **Error Quality**: Test error message clarity and actionability for CLI users with container operation failures
- **Documentation Quality**: Validate clap help text generation and rustdoc documentation consistency

## Getting Help
- **Rust Quality**: Cargo book and Rust quality guidelines for CLI application quality standards
- **CLI Testing**: assert_cmd documentation and CLI testing patterns for comprehensive command validation
- **Container Testing**: Apple Container documentation for testing strategies and mock implementation approaches
- **Async Quality**: Tokio documentation and async Rust patterns for quality async code in CLI applications
- **Project Specific**: mise.toml task configuration and existing quality tool setup for project-appropriate quality standards