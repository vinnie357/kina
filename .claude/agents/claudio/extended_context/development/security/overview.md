# Security Context for Rust CLI Container Orchestration

## Project-Specific Guidance
Based on discovery analysis, this project requires comprehensive security considerations for a Rust CLI application managing Apple Container instances and Kubernetes clusters with container runtime privileges on macOS.

## Recommended Approaches
- **Rust Security Toolchain**: Implement cargo-audit (configured in mise tasks) for dependency vulnerability scanning and secure Rust coding patterns
- **Container Runtime Security**: Validate Apple Container privilege isolation, secure container lifecycle management, and macOS system permission requirements
- **Kubernetes Security**: Implement proper RBAC configuration with kube-rs client, secure cluster authentication, and credential management patterns
- **CLI Security Patterns**: Secure clap argument parsing, input validation for container configurations, and sanitized error message handling

## Integration Patterns
Security integrates with the existing project architecture:
- Cargo dependency scanning through mise run audit for automated vulnerability detection
- Apple Container privilege validation and secure runtime API interactions
- kube-rs and k8s-openapi security patterns for Kubernetes client authentication and authorization
- Tokio async security patterns for concurrent container operations and resource management
- Configuration security through secure serialization/deserialization with serde validation

## Quality Standards
- **Memory Safety**: Leverage Rust's memory safety while carefully validating any unsafe Apple Container API interactions
- **Privilege Management**: Implement least-privilege principles for Apple Container operations and macOS system resource access
- **Credential Security**: Secure handling of Kubernetes credentials, kubeconfig files, and cluster authentication tokens
- **Attack Surface Minimization**: Validate all CLI inputs, container configurations, and external API interactions

## Next Steps
- Execute mise run audit to validate current dependency security status
- Research Apple Container security model and required macOS system permissions
- Implement secure kube-rs client configuration with proper authentication validation
- Establish comprehensive input validation for CLI arguments and container configuration files