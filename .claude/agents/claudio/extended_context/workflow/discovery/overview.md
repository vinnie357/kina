# Discovery Context for Rust CLI Application

## Project-Specific Guidance
Based on discovery analysis, this project uses Rust for CLI development targeting Apple Container integration with Kubernetes orchestration on macOS platforms.

## Recommended Approaches
- **Rust Project Analysis**: Focus on Cargo.toml manifests, src/ structure, and CLI framework patterns
- **Apple Container Integration**: Analyze container runtime requirements and macOS-specific dependencies
- **Kubernetes Tooling**: Examine kubectl, kubectx, kubens integration patterns and cluster management workflows
- **CLI Application Patterns**: Identify command structure, argument parsing, and user interface design

## Integration Patterns
Discovery integrates with Rust development toolchain through:
- Cargo workspace analysis for multi-crate projects
- Apple Container CLI detection and configuration validation
- Kubernetes cluster configuration and deployment pattern analysis
- mise task management integration for development workflows

## Quality Standards
- **Technology Detection**: Accurately identify Rust crates, Apple Container dependencies, and Kubernetes configurations
- **Architecture Assessment**: Distinguish between CLI application patterns and container orchestration requirements
- **macOS Compatibility**: Validate platform-specific requirements and Apple Container runtime availability
- **Development Stage**: Assess implementation maturity from planning through production deployment

## Next Steps
- Initialize Cargo workspace structure analysis
- Validate Apple Container runtime availability and configuration
- Analyze Kubernetes integration requirements and cluster management patterns
- Document CLI command structure and user experience workflows