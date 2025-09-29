# Discovery Context for Rust CLI Container Orchestration

## Project-Specific Guidance
Based on discovery analysis, this project uses Rust 2021 edition with clap CLI framework, Tokio async runtime, kube-rs for Kubernetes integration, and Apple Container runtime technology.

## Recommended Approaches
- **Cargo Workspace Analysis**: Examine workspace dependencies and member package structure for comprehensive project understanding
- **CLI Framework Assessment**: Analyze clap derive patterns, subcommand architecture, and argument parsing strategies
- **Container Integration Discovery**: Investigate Apple Container runtime patterns, Docker API compatibility, and Kubernetes client configurations
- **Async Architecture Analysis**: Review Tokio-based async patterns, error handling with anyhow/thiserror, and tracing infrastructure

## Integration Patterns
This discovery process integrates with:
- Cargo dependency analysis for technology stack detection
- rustfmt.toml and clippy.toml for code quality assessment
- mise.toml task runner configuration for development workflow understanding
- Kubernetes manifest analysis for orchestration capability assessment
- Apple Container specific tooling and configuration discovery

## Quality Standards
- **Technology Detection**: Verify framework presence through actual Cargo.toml analysis
- **Architecture Pattern Recognition**: Identify layered architecture, provider abstraction patterns, and domain-driven design
- **Development Maturity Assessment**: Evaluate tooling sophistication, test coverage, and automation level
- **Container Integration Readiness**: Assess Apple Container, Kubernetes, and Docker compatibility patterns

## Next Steps
- Execute comprehensive file structure analysis using Read and Glob tools
- Analyze Cargo workspace configuration and dependency management patterns
- Assess CLI command architecture and provider abstraction implementation
- Evaluate container orchestration readiness and Kubernetes integration capabilities