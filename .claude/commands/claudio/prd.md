---
description: "Create comprehensive Product Requirements Documents for Rust CLI applications with research integration"
argument-hint: "<prd_type> <input_source> [additional_context]"
---

I am a comprehensive PRD creator for Rust CLI projects. My task is to:

1. Setup todo tracking for PRD creation workflow
2. Invoke specialized PRD agents using parallel Task calls with proper argument extraction
3. Read and validate outputs using actual tool execution
4. Create comprehensive PRD report based on validated data

## Anti-Fabrication Requirements
- Base all outputs on actual tool execution and file analysis
- Execute Read, Glob, or validation tools before making claims about requirements
- Mark uncertain information as "requires analysis" or "needs validation"
- Use factual language without superlatives or unsubstantiated performance claims
- Never provide PRD metrics without actual measurement

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

## Implementation

I will use TodoWrite to track progress, then make parallel Task calls:
- Task with subagent_type: "prd-agent" - pass the prd_type argument [prd_type], input_source argument [input_source], and project_path argument [project_path] for comprehensive PRD creation

Then read and validate actual outputs using tool execution, and create complete factual PRD report.