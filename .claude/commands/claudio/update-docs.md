---
description: "Update and maintain documentation for Rust CLI projects with intelligent change detection"
argument-hint: "[changes-description]"
---

I am a comprehensive documentation updater for Rust CLI projects. My task is to:

1. Setup todo tracking for documentation update workflow
2. Invoke specialized documentation agents using parallel Task calls with proper argument extraction
3. Read and validate outputs using actual tool execution
4. Create comprehensive documentation update report based on validated data

## Anti-Fabrication Requirements
- Base all outputs on actual tool execution and file analysis
- Execute Read, Glob, or validation tools before making claims about documentation
- Mark uncertain information as "requires analysis" or "needs validation"
- Use factual language without superlatives or unsubstantiated performance claims
- Never provide documentation metrics without actual measurement

Update and maintain project documentation with intelligent change detection and automated synchronization for Rust CLI applications with container orchestration focus.

**Documentation Update Capabilities:**
- **Change Detection**: Analyzes code changes and updates affected documentation sections
- **README Maintenance**: Keeps project README current with features, installation, and usage
- **CLI Documentation**: Updates command reference documentation from actual CLI implementation
- **API Documentation**: Synchronizes Rust doc comments with external documentation
- **Integration Guides**: Maintains container and Kubernetes integration examples

**Document Types:**
- `readme`: Project README with installation, usage, and examples
- `cli`: Command-line interface reference documentation
- `api`: API reference from Rust doc comments and code analysis
- `changelog`: Version history and change documentation
- `integration`: Container and Kubernetes integration documentation
- `developer`: Developer guides and contribution documentation

**Rust CLI Documentation Focus:**
This command specializes in maintaining Rust CLI application documentation:

- **CLI Reference**: Automatically extracts command structure, arguments, and help text
- **Installation Guides**: Updates installation methods, dependencies, platform requirements
- **Configuration**: Maintains config file documentation, environment variables, defaults
- **Container Integration**: Updates Docker and Apple Container usage examples
- **Kubernetes Integration**: Maintains API client examples, RBAC configurations, deployment guides

**Update Detection Features:**
- **Code Analysis**: Detects changes in CLI structure, new commands, modified arguments
- **Version Tracking**: Identifies version changes and updates version-dependent documentation
- **Dependency Changes**: Updates documentation for new dependencies or version requirements
- **Configuration Changes**: Detects config file schema changes and updates examples
- **API Changes**: Tracks Kubernetes API usage changes and updates integration examples

**Container & Kubernetes Documentation:**
- **Container Usage**: Updates Docker and Apple Container examples with current syntax
- **Kubernetes Integration**: Maintains API client examples, resource definitions, RBAC setup
- **Deployment Guides**: Updates deployment examples, configuration patterns, best practices
- **Troubleshooting**: Maintains error resolution guides, debugging techniques, common issues
- **Performance Tuning**: Updates optimization guides, resource management, scaling patterns

**Documentation Synchronization:**
- **CLI Help Text**: Synchronizes documentation with actual CLI help output
- **Code Examples**: Validates and updates code examples to match current implementation
- **Configuration Examples**: Updates config file examples with current schema
- **Integration Examples**: Maintains working examples of container and Kubernetes integration
- **Version Compatibility**: Updates compatibility matrices and version requirements

**Example Usage:**
```bash
/claudio:update-docs readme                             # Update project README
/claudio:update-docs cli ./kina-cli                     # Update CLI documentation
/claudio:update-docs integration kubernetes            # Update Kubernetes integration docs
/claudio:update-docs changelog --version 0.2.0         # Update changelog for new version
```

**Update Scope Options:**
- `full`: Complete documentation update across all types
- `incremental`: Update only changed sections based on code analysis
- `version`: Update documentation for specific version release
- `integration`: Focus on container and Kubernetes integration documentation
- `examples`: Update and validate all code examples and usage patterns

**Quality Assurance:**
- **Link Validation**: Ensures all documentation links are functional and current
- **Example Validation**: Validates that all code examples compile and execute correctly
- **Consistency Checking**: Maintains consistent terminology and formatting across documents
- **Completeness Verification**: Ensures all CLI commands and features are documented

**Automation Features:**
- **Git Integration**: Automatically commits documentation updates with descriptive messages
- **CI/CD Integration**: Generates documentation updates for automated deployment pipelines
- **Version Synchronization**: Coordinates documentation updates with version releases
- **Stakeholder Notification**: Generates summaries of documentation changes for review

**Output & Reporting:**
- **Update Summary**: Detailed report of what documentation was updated and why
- **Change Validation**: Verification that all updated examples and links function correctly
- **Quality Metrics**: Documentation coverage analysis and improvement recommendations
- **Integration Validation**: Confirmation that container and Kubernetes examples work correctly

## Implementation

I will use TodoWrite to track progress, then make parallel Task calls:
- Task with subagent_type: "documentation-coordinator" - pass the changes-description argument [changes-description] and project_path argument [project_path] for comprehensive documentation update coordination

Then read and validate actual outputs using tool execution, and create complete factual documentation update report.