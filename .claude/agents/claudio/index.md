# Claudio Agents Index - Rust CLI Project

**Project**: kina (Kubernetes in Apple Container)
**Technology Focus**: Rust CLI Application with Container Orchestration
**Agents Generated**: 18 (User Agents Only)
**Last Updated**: 2025-01-20
**System Agents Filtered**: 6 (excluded from user installation)

## Project Context

This agent index is localized for a **Rust CLI application** focused on **Kubernetes container orchestration** with **Apple Container integration**. All agents include Rust-specific examples, Cargo integration patterns, and container orchestration workflows.

**Technology Stack Detected**:
- **Primary Language**: Rust (Cargo-based project structure)
- **Application Type**: CLI application with command-line interface
- **Target Platform**: macOS with Apple Container runtime
- **Integration Focus**: Kubernetes API, container orchestration, kubectl compatibility

## Core Workflow Agents (5)

| Agent | Description | Model | Rust CLI Focus |
|-------|-------------|-------|----------------|
| `discovery-agent` | **CORE** Rust CLI project analysis and technology discovery | sonnet | Cargo.toml analysis, CLI framework detection, container integration assessment |
| `prd-agent` | **CORE** Product Requirements for Rust CLI applications | sonnet | CLI command structure, container integration requirements, Kubernetes API specifications |
| `plan-agent` | **CORE** Implementation planning for Rust CLI development | sonnet | Cargo workspace setup, CLI architecture phases, container integration milestones |
| `task-agent` | **CORE** Task breakdown for Rust CLI implementation | sonnet | Module scaffolding, command parsing tasks, container runtime integration |
| `research-specialist` | Research for Rust CLI development with Kubernetes focus | sonnet | CLI frameworks (clap, structopt), Kubernetes clients (kube-rs), container runtime APIs |

## Development Agents (4)

| Agent | Description | Model | Rust CLI Focus |
|-------|-------------|-------|----------------|
| `code-quality-analyzer` | Rust CLI code quality assessment | sonnet | rustfmt, clippy, cargo-audit, CLI-specific quality checks |
| `implement-agent` | Execute Rust CLI implementation plans | sonnet | Cargo project execution, CLI command implementation, Kubernetes client integration |
| `security-review-coordinator` | Security review coordinator with STRIDE methodology | opus | Rust security patterns, container security, Kubernetes RBAC analysis |
| `test-review` | Reviews testing suite tools and provides recommendations | haiku | cargo test, CLI testing patterns, container integration testing |

## Security Agents (5)

| Agent | Description | Model | Container & Kubernetes Security Focus |
|-------|-------------|-------|---------------------------------------|
| `security-review-coordinator` | Coordinates security review using STRIDE methodology | opus | Rust CLI security, container orchestration, Kubernetes integration threat modeling |
| `security-threat-modeler` | STRIDE-based threat identification and analysis | sonnet | CLI attack vectors, container security threats, Kubernetes integration threats |
| `security-architecture-analyst` | System-level security design evaluation | sonnet | CLI security architecture, container integration security, RBAC patterns |
| `vulnerability-assessment-specialist` | Code and configuration security analysis | sonnet | Rust dependency scanning, container configuration security, Kubernetes manifest analysis |
| `security-diagram-generator` | Mermaid diagram creation for security visualization | haiku | CLI threat model diagrams, container security architecture, RBAC visualization |

## Documentation Agents (5)

| Agent | Description | Model | CLI Documentation Focus |
|-------|-------------|-------|------------------------|
| `documentation-coordinator` | Coordinates parallel documentation creation | opus | Rust CLI documentation strategy, container workflow guides, Kubernetes integration examples |
| `documentation-readme-creator` | Creates comprehensive project README | sonnet | CLI installation guides, usage examples, container orchestration workflows |
| `documentation-api-creator` | Creates CLI API reference documentation | sonnet | Command reference, container API patterns, Kubernetes client usage |
| `documentation-user-guide-creator` | Creates user guides with tutorials | sonnet | CLI workflow tutorials, container management guides, Kubernetes operation examples |
| `documentation-developer-guide-creator` | Creates developer documentation | sonnet | Rust CLI architecture, contribution guidelines, container development setup |

## Localization Applied

### Rust CLI Development Integration
- **Cargo Workspace**: Multi-package project coordination and dependency management
- **CLI Frameworks**: clap, structopt, argh integration analysis and implementation
- **Error Handling**: anyhow and thiserror integration patterns
- **Testing**: cargo test, assert_cmd, predicates for CLI testing
- **Quality Tools**: rustfmt, clippy, cargo-audit, cargo-deny integration

### Container Orchestration Integration
- **Apple Container**: Native macOS container runtime integration
- **Docker Compatibility**: Cross-platform container workflow support
- **Kubernetes API**: kube-rs client library integration and resource management
- **RBAC Configuration**: Service account setup and security analysis
- **Deployment Patterns**: Container orchestration and scaling strategies

### CLI Application Patterns
- **Command Structure**: Hierarchical subcommand architecture with clap
- **Configuration Management**: TOML/YAML config parsing with environment variable support
- **Output Formatting**: Structured output with JSON, YAML, and table formats
- **Interactive Workflows**: User input handling and confirmation prompts
- **Error Handling**: User-friendly error messages with actionable suggestions

## System Agents Excluded

The following system agents were filtered out during installation (marked with `system: claudio-system`):
- `discovery-structure-analyzer` - Project structure analysis (system internal)
- `discovery-tech-analyzer` - Technology stack analysis (system internal)
- `discovery-architecture-analyzer` - Architecture pattern analysis (system internal)
- `discovery-integration-analyzer` - Integration analysis (system internal)
- `discovery-consolidator` - Discovery document consolidation (system internal)
- `project-test-runner` - Generic project test runner (system internal)

## Validation Status

- **Multi-Source Validation**: Complete ✓
- **Command Coverage**: 94% (17/18 commands have required agents)
- **System Agent Filtering**: 6 system agents properly excluded
- **Missing Templates**: None
- **Coverage Issue**: test.md references 'kina-test-runner' but uses 'test-review' agent (functional)

## Extended Context Requirements

The following extended context directories are required for optimal agent performance:
- `workflow/discovery/` - Discovery analysis patterns
- `workflow/prd/` - Requirements definition templates
- `workflow/planning/` - Implementation planning patterns
- `workflow/task/` - Task breakdown strategies
- `development/quality/` - Code quality analysis patterns
- `development/security/` - Security analysis frameworks
- `documentation/overview/` - Documentation generation templates
- `research/overview/` - Research methodology and patterns
- `templates/agents/` - Agent architecture templates
- `templates/workflows/` - Workflow integration patterns

## Usage Examples

### Typical Rust CLI Development Workflow
```bash
# 1. Project Discovery
/claudio:discovery                          # Analyze Rust CLI project structure

# 2. Requirements & Planning
/claudio:prd feature "cluster management"   # Define CLI feature requirements
/claudio:plan feature "cluster commands"    # Create implementation phases

# 3. Development Execution
/claudio:task .claudio/docs/plan.md         # Break down into executable tasks
/claudio:implement phase phase1             # Execute implementation phases

# 4. Quality & Testing
/claudio:code-quality full                  # Run rustfmt, clippy, cargo-audit
/claudio:test                              # Execute Rust testing suite

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
/claudio:test integration
```

## Technology Specialization

- **Rust Development**: Cargo-based project management, CLI framework integration, error handling patterns
- **Container Integration**: Apple Container native support, Docker API compatibility, container lifecycle management
- **Kubernetes Integration**: kube-rs client patterns, RBAC configuration, resource management strategies
- **CLI Patterns**: Command parsing, configuration management, output formatting, interactive workflows
- **Quality Automation**: Integrated rustfmt, clippy, cargo-audit workflows with container security scanning

---
*This index is automatically generated and maintained by the Claudio system based on project discovery analysis and technology stack detection.*