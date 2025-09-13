# Documentation Context for Rust CLI Application

## Project-Specific Guidance
Based on discovery analysis, this project requires comprehensive documentation for a Rust CLI application targeting developers working with Apple Container and Kubernetes integration on macOS platforms.

## Recommended Approaches
- **CLI Documentation**: Generate comprehensive command help, usage examples, and workflow documentation
- **API Documentation**: Create rustdoc documentation for all public APIs and Apple Container integration modules
- **User Guides**: Develop installation guides, getting started tutorials, and troubleshooting documentation
- **Developer Documentation**: Document architecture decisions, Apple Container integration patterns, and contribution guidelines

## Integration Patterns
Documentation integrates with Rust development ecosystem through:
- rustdoc generation for API documentation with Apple Container and Kubernetes integration examples
- CLI help text generation from clap or structopt with consistent formatting and comprehensive examples
- README.md maintenance with installation instructions, usage examples, and macOS-specific requirements
- GitHub documentation with contribution guidelines, issue templates, and development environment setup

## Quality Standards
- **Technical Accuracy**: Ensure all documentation reflects actual CLI behavior and Apple Container integration requirements
- **User Experience**: Provide clear examples, common workflow documentation, and troubleshooting guidance
- **Platform Specificity**: Include macOS-specific installation requirements, Apple Container setup, and system compatibility information
- **Maintenance Integration**: Automated documentation validation and synchronization with CLI command changes

## Next Steps
- Establish rustdoc standards for Apple Container and Kubernetes integration modules
- Create CLI help text templates with consistent formatting and comprehensive examples
- Develop user guide templates for installation, configuration, and common workflows
- Implement documentation validation tools to ensure accuracy and completeness