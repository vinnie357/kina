---
description: "Generate comprehensive documentation for Rust CLI applications with parallel coordination"
argument-hint: "[documentation_type] [project_path] [output_format]"
---

Generate comprehensive documentation for Rust CLI applications through parallel specialized documentation agents with focus on container orchestration and Kubernetes integration.

**Documentation Types:**
- `full`: Complete documentation suite (README, API docs, user guides, developer guides)
- `readme`: Project README with installation, usage, and examples
- `api`: API reference documentation from code comments and annotations
- `user`: User guides with tutorials and workflow examples
- `developer`: Developer documentation with architecture and contribution guides
- `cli`: Command-line interface documentation with usage examples
- `integration`: Integration guides for container and Kubernetes workflows

**Rust CLI Documentation Focus:**
This command specializes in documentation for Rust CLI applications:

- **CLI Interface**: Command structure, argument parsing, configuration options
- **Installation Guides**: Binary distribution, package managers, compilation from source
- **Container Integration**: Docker usage, Apple Container workflows, orchestration examples
- **Kubernetes Integration**: API usage examples, RBAC setup, deployment patterns
- **Development Setup**: Cargo workspace, development tools, testing procedures

**Documentation Capabilities:**
- **Code Analysis**: Extracts documentation from Rust doc comments and code structure
- **Usage Examples**: Generates realistic CLI usage examples with container workflows
- **Architecture Documentation**: System design, module organization, integration patterns
- **Tutorial Creation**: Step-by-step guides for common workflows and use cases
- **API Reference**: Complete CLI command reference with examples and explanations

**Container & Kubernetes Documentation:**
- **Container Workflows**: Docker image creation, Apple Container usage, runtime configuration
- **Kubernetes Integration**: API client usage, resource management, deployment examples
- **RBAC Configuration**: Service account setup, role definitions, security patterns
- **Monitoring Integration**: Observability, logging, metrics collection patterns
- **Troubleshooting Guides**: Common issues, debugging techniques, error resolution

**Documentation Sections Generated:**
- **Installation**: Platform-specific installation instructions, dependencies, prerequisites
- **Quick Start**: Essential workflows, basic commands, common use cases
- **Command Reference**: Complete CLI interface documentation with examples
- **Configuration**: Configuration file formats, environment variables, runtime options
- **Integration Examples**: Container orchestration workflows, Kubernetes deployments
- **Development Guide**: Building from source, testing, contributing guidelines

**Example Usage:**
```bash
/claudio:documentation full                             # Complete documentation suite
/claudio:documentation readme ./kina-cli               # Project README generation
/claudio:documentation cli --markdown                  # CLI reference documentation
/claudio:documentation integration kubernetes          # Kubernetes integration guide
```

**Output Formats:**
- **Markdown**: Standard markdown documentation for GitHub/GitLab
- **HTML**: Static site generation with navigation and search
- **Man Pages**: Unix manual pages for CLI commands
- **PDF**: Formatted documentation for offline distribution

**Documentation Structure:**
- **Root Documentation**: README.md, CHANGELOG.md, CONTRIBUTING.md
- **User Documentation**: User guides, tutorials, FAQ, troubleshooting
- **Developer Documentation**: Architecture, API reference, development setup
- **Integration Documentation**: Container workflows, Kubernetes patterns, CI/CD examples

**Quality Features:**
- **Link Validation**: Ensures all internal and external links are functional
- **Example Testing**: Validates that all code examples compile and execute correctly
- **Consistency Checking**: Maintains consistent terminology and formatting across documents
- **Accessibility**: Ensures documentation follows accessibility best practices

Task with subagent_type: "documentation-coordinator" - pass the project_path argument for comprehensive documentation generation coordination through specialized sub-agents optimized for Rust CLI applications with container orchestration and Kubernetes integration focus.