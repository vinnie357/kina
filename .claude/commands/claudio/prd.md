---
description: "Create comprehensive Product Requirements Documents for Rust CLI applications with research integration"
argument-hint: "<prd_type> <input_source> [additional_context]"
---

Create comprehensive Product Requirements Documents (PRDs) with clear requirements, success criteria, and implementation plans. Integrates with existing research and discovery analysis to create well-informed requirements for Rust CLI development projects.

**Flexible Input Patterns:**
```bash
/claudio:prd feature "CLI command system"                    # Direct feature description
/claudio:prd enhancement "use research on Kubernetes integration"  # With research integration
/claudio:prd integration "external_requirements.md"         # From external document
/claudio:prd migration "legacy Docker workflow analysis"    # Migration requirements
```

**PRD Types:**
- `feature`: New CLI feature development PRD (command structure, argument parsing, output formatting)
- `enhancement`: CLI improvement/enhancement PRD (performance, usability, integration)
- `integration`: System integration PRD (Kubernetes, container runtimes, CI/CD)
- `migration`: Data/workflow migration PRD (Docker to Apple Container, legacy systems)
- `full`: Complete product PRD with all CLI application components

**Input Sources:**
- **Direct Descriptions**: CLI feature descriptions, command objectives, workflow requirements
- **Research Integration**: Automatically incorporates `.claudio/research/` documents when referenced
- **External Documents**: Requirements specs, API documentation, existing PRDs
- **Discovery Context**: Uses existing `.claudio/docs/discovery.md` for Rust project context

**Research Integration:**
When you reference research (e.g., "use research on container orchestration"), the PRD automatically:
- Locates matching `.claudio/research/` documents
- Incorporates research findings into CLI requirements
- References research insights in implementation sections
- Provides Rust-specific integration patterns

**Rust CLI Application Focus:**
This PRD command specializes in:
- **Command Structure**: CLI argument patterns, subcommands, configuration options
- **Container Integration**: Apple Container, Docker compatibility, Kubernetes workflows
- **Development Tooling**: Cargo build systems, testing frameworks, CI/CD integration
- **Performance Requirements**: Binary size, startup time, memory usage for CLI tools
- **User Experience**: Terminal output formatting, error handling, help documentation

**Example PRD Sections for CLI Applications:**
- **Command Interface**: Argument structure, help text, configuration files
- **Integration Requirements**: Kubernetes API compatibility, container runtime support
- **Performance Criteria**: Command execution speed, resource utilization
- **Testing Strategy**: Unit tests, integration tests, CLI acceptance testing
- **Documentation**: Man pages, usage examples, developer guides

**Output:**
Creates `.claudio/docs/prd.md` with comprehensive CLI requirements that can be used by the plan command to generate actionable Rust implementation structures.

Task with subagent_type: "prd-agent" - pass the project_path argument for comprehensive Product Requirements Document creation with CLI application objectives, functional requirements, and success criteria enhanced by research integration and Rust development patterns.