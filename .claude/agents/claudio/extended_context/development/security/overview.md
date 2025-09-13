# Security Context for Rust CLI Application

## Project-Specific Guidance
Based on discovery analysis, this project requires security considerations for a Rust CLI application that manages Apple Container instances and Kubernetes clusters with elevated system privileges.

## Recommended Approaches
- **Rust Security Practices**: Implement cargo-audit for dependency vulnerability scanning and secure coding patterns
- **Container Runtime Security**: Validate Apple Container isolation, privilege management, and resource access controls
- **Kubernetes Security**: Implement proper RBAC configuration, secure cluster communication, and credential management
- **CLI Security Patterns**: Secure argument handling, input validation, and error message sanitization

## Integration Patterns
Security integrates with Rust development ecosystem through:
- Cargo dependency scanning with automated vulnerability detection
- Apple Container privilege validation and secure container lifecycle management
- Kubernetes client security with proper authentication and authorization patterns
- CLI input validation and secure error handling throughout user interactions

## Quality Standards
- **Memory Safety**: Leverage Rust's memory safety guarantees while validating unsafe Apple Container API interactions
- **Privilege Management**: Implement least-privilege principles for Apple Container operations and system resource access
- **Credential Security**: Secure storage and handling of Kubernetes credentials and cluster configuration
- **Attack Surface Minimization**: Minimize exposed functionality and validate all external inputs and API interactions

## Next Steps
- Configure cargo-audit for automated dependency vulnerability scanning
- Research Apple Container security model and privilege requirements
- Implement secure Kubernetes client configuration and credential management
- Establish input validation patterns for CLI argument and configuration handling