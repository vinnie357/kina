# Code Quality Context for Rust CLI Container Orchestration

## Project-Specific Guidance
Based on discovery analysis, this project uses rustfmt, clippy, and cargo-audit with sophisticated quality tooling configured through mise task automation and comprehensive development practices.

## Recommended Approaches
- **Rust Quality Toolchain**: Leverage configured rustfmt.toml (max_width: 100), clippy.toml (cognitive_complexity_threshold: 30), and cargo-audit for security scanning
- **CLI Testing Patterns**: Use assert_cmd and predicates for CLI testing, integration tests with container runtime validation
- **Error Handling Standards**: Implement anyhow (1.0.99) and thiserror (1.0.69 + 2.0.16) for comprehensive error context and user-friendly CLI messages
- **Async Quality Patterns**: Ensure proper Tokio async patterns, error propagation, and tracing integration throughout container operations

## Integration Patterns
Code quality integrates with the existing project toolchain:
- Cargo workspace configuration with shared quality tool dependencies
- mise.toml task automation (mise run lint, mise run test, mise run ci)
- rustfmt configuration with Rust 2021 edition formatting standards
- clippy configuration with documentation requirements and complexity thresholds
- Pre-commit automation through mise tasks for format, lint, test, and audit workflows

## Quality Standards
- **Rust Idioms**: Follow established project patterns with proper ownership, async/await usage, and error handling
- **CLI User Experience**: Consistent clap derive patterns, structured output formatting (JSON, YAML, table), and actionable error messages
- **Container Integration**: Robust Apple Container error handling, Kubernetes API error propagation, and resource cleanup patterns
- **Performance Standards**: Efficient async operations, proper resource management, and minimal CLI startup overhead

## Next Steps
- Execute mise run lint to validate current quality tool configuration
- Analyze existing rustfmt.toml and clippy.toml settings for project-specific customizations
- Implement comprehensive CLI testing with assert_cmd for all subcommand operations
- Establish container integration testing patterns with Apple Container runtime validation