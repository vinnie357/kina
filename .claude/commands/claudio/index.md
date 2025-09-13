# Claudio Commands Index - Rust CLI Project

**Project**: kina (Kubernetes in Apple Container)
**Technology Focus**: Rust CLI Application with Container Orchestration
**Commands Generated**: 12 (Technology-Filtered)
**Last Updated**: 2025-01-20

## Project Context

This command index is localized for a **Rust CLI application** focused on **Kubernetes container orchestration** with **Apple Container integration**. All commands include Rust-specific examples, Cargo integration patterns, and container orchestration workflows.

**Technology Stack Detected**:
- **Primary Language**: Rust (Cargo-based project structure)
- **Application Type**: CLI application with command-line interface
- **Target Platform**: macOS 15.6+ with Apple Container runtime
- **Integration Focus**: Kubernetes API, container orchestration, kubectl compatibility

## Core Workflow Commands

### Discovery & Analysis
| Command | Description | Agent Dependencies | Rust CLI Focus |
|---------|-------------|-------------------|----------------|
| `/claudio:discovery` | Comprehensive Rust CLI project analysis | `discovery-agent`, `discovery-structure-analyzer`, `discovery-tech-analyzer`, `discovery-architecture-analyzer`, `discovery-integration-analyzer`, `discovery-consolidator` | Cargo.toml analysis, CLI framework detection, container integration assessment |
| `/claudio:research` | Research for Rust CLI development with K8s focus | `research-specialist` | CLI frameworks (clap, structopt), Kubernetes clients (kube-rs), container runtime APIs |

### Planning & Requirements
| Command | Description | Agent Dependencies | Rust CLI Focus |
|---------|-------------|-------------------|----------------|
| `/claudio:prd` | Product Requirements for Rust CLI applications | `prd-agent` | CLI command structure, container integration requirements, Kubernetes API specifications |
| `/claudio:plan` | Implementation planning for Rust CLI development | `plan-agent` | Cargo workspace setup, CLI architecture phases, container integration milestones |
| `/claudio:task` | Task breakdown for Rust CLI implementation | `task-agent` | Module scaffolding, command parsing tasks, container runtime integration |

### Development & Implementation
| Command | Description | Agent Dependencies | Rust CLI Focus |
|---------|-------------|-------------------|----------------|
| `/claudio:claudio` | Comprehensive Rust CLI orchestration | `discovery-agent`, `prd-agent`, `plan-agent`, `task-agent`, `implement-agent`, `research-specialist` | Full Rust development workflow with container orchestration |
| `/claudio:implement` | Execute Rust CLI implementation plans | `implement-agent` | Cargo project execution, CLI command implementation, Kubernetes client integration |
| `/claudio:code-quality` | Rust CLI code quality assessment | `code-quality-analyzer` | rustfmt, clippy, cargo-audit, CLI-specific quality checks |

### Testing & Validation
| Command | Description | Agent Dependencies | Rust CLI Focus |
|---------|-------------|-------------------|----------------|
| `/claudio:test` | Comprehensive testing for Rust CLI projects | `project-test-runner`, `test-review` | cargo test, CLI testing patterns, container integration testing |
| `/claudio:security-review` | Security review with STRIDE methodology | `security-review-coordinator`, `security-threat-modeler`, `security-architecture-analyst`, `vulnerability-assessment-specialist`, `security-diagram-generator` | Rust security patterns, container security, Kubernetes RBAC analysis |

### Documentation & Maintenance
| Command | Description | Agent Dependencies | Rust CLI Focus |
|---------|-------------|-------------------|----------------|
| `/claudio:documentation` | Generate comprehensive Rust CLI documentation | `documentation-coordinator`, `documentation-readme-creator`, `documentation-api-creator`, `documentation-user-guide-creator`, `documentation-developer-guide-creator` | CLI reference docs, container workflow guides, Kubernetes integration examples |
| `/claudio:update-docs` | Maintain Rust CLI project documentation | `documentation-coordinator` | CLI help synchronization, container example updates, Kubernetes pattern maintenance |

## Technology-Specific Features

### Rust Development Integration
- **Cargo Workspace**: Multi-package project coordination and dependency management
- **CLI Frameworks**: clap, structopt, argh integration analysis and implementation
- **Error Handling**: anyhow and thiserror integration patterns
- **Testing**: cargo test, proptest, criterion benchmarking integration
- **Quality Tools**: rustfmt, clippy, cargo-audit, cargo-deny integration

### Container Orchestration Integration
- **Apple Container**: Native macOS container runtime integration
- **Docker Compatibility**: Cross-platform container workflow support
- **Kubernetes API**: kube-rs client library integration and resource management
- **RBAC Configuration**: Service account setup and security analysis
- **Deployment Patterns**: Container orchestration and scaling strategies

### CLI Application Patterns
- **Command Structure**: Hierarchical subcommand architecture with clap
- **Configuration Management**: YAML/TOML config parsing with environment variable support
- **Output Formatting**: Structured output with JSON, YAML, and table formats
- **Interactive Workflows**: User input handling and confirmation prompts
- **Error Handling**: User-friendly error messages with actionable suggestions

## Excluded Commands

**Technology-Based Filtering Applied**:
- `phoenix-dev`: Excluded (Phoenix/Elixir not detected in technology stack)

## Agent Dependency Summary

**Total Required Agents**: 24 specialized agents
- **Discovery Agents**: 6 (comprehensive project analysis)
- **Development Agents**: 8 (planning, implementation, quality)
- **Documentation Agents**: 5 (comprehensive documentation generation)
- **Security Agents**: 5 (STRIDE methodology and threat modeling)

## Usage Patterns

### Typical Rust CLI Development Workflow
```bash
# 1. Project Discovery
/claudio:discovery                          # Analyze current Rust CLI project

# 2. Requirements & Planning
/claudio:prd feature "cluster management"   # Define CLI feature requirements
/claudio:plan feature "cluster commands"    # Create implementation phases

# 3. Development Execution
/claudio:task .claudio/docs/plan.md         # Break down into executable tasks
/claudio:implement phase phase1             # Execute implementation phases

# 4. Quality & Testing
/claudio:code-quality full                  # Run rustfmt, clippy, cargo-audit
/claudio:test integration                   # Execute comprehensive testing

# 5. Security & Documentation
/claudio:security-review container          # Container security analysis
/claudio:documentation cli                  # Generate CLI reference docs
```

### Container Integration Workflows
```bash
# Container-focused development
/claudio:research "Apple Container integration" comprehensive
/claudio:prd integration "container runtime compatibility"
/claudio:security-review . container --json

# Kubernetes integration
/claudio:research "Kubernetes Rust clients" detailed integration
/claudio:plan integration "Kubernetes API client"
/claudio:test integration kubernetes
```

## Integration Notes

- **Development Environment**: Compatible with mise task runners and cargo build systems
- **CI/CD Integration**: All commands support automated execution in CI/CD pipelines
- **Container Runtime**: Optimized for Apple Container with Docker compatibility
- **Kubernetes Integration**: Native support for kube-rs and kubernetes-rs client libraries
- **Quality Automation**: Integrated rustfmt, clippy, and security scanning workflows

---
*This index is automatically generated and maintained by the Claudio system based on project discovery analysis.*