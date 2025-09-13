---
description: "Create detailed task lists for Rust CLI development with parallel processing for phases and contexts"
argument-hint: "<plan_file_path> [project_name]"
---

Create task lists and specialized agent contexts for Rust CLI project implementation phases based on actual plan analysis.

**CRITICAL: NEVER fabricate task completion status or implementation timelines. Only mark tasks complete when acceptance criteria are met and Rust tests pass.**

**Rust CLI Development Focus:**
This task command creates detailed, executable tasks optimized for Rust CLI applications:

- **Cargo Integration**: Tasks for dependency management, workspace setup, build optimization
- **CLI Framework**: Tasks for command parsing, argument validation, help text generation
- **Container Integration**: Tasks for Apple Container CLI integration, Kubernetes API setup
- **Testing Strategy**: Tasks for unit testing, integration testing, CLI acceptance testing
- **Development Workflow**: Tasks for mise task runners, development environment setup

**Example Task Categories for CLI Projects:**
- **Project Setup**: Initialize Cargo workspace, configure CI/CD, setup development tools
- **CLI Architecture**: Implement command parsing with clap, configuration management, logging
- **Core Functionality**: Container management logic, Kubernetes API integration, error handling
- **Testing & Validation**: Unit tests, integration tests, CLI behavior validation
- **Documentation**: Help text, man pages, usage examples, developer guides

**Integration with Rust Ecosystem:**
- **Build System**: Cargo build profiles, feature flags, cross-compilation setup
- **Dependencies**: Version management, security auditing, license compliance
- **Testing Tools**: cargo test, cargo bench, integration test frameworks
- **Quality Tools**: rustfmt, clippy, cargo-audit integration tasks
- **Distribution**: Binary packaging, installation scripts, release automation

**Example Usage:**
```bash
/claudio:task .claudio/docs/plan.md kina              # Generate tasks for kina CLI project
/claudio:task ./my-cli-plan.md kubernetes-helper     # Tasks for Kubernetes CLI tool
/claudio:task .claudio/phase1/plan.md                # Phase-specific task generation
```

Task with subagent_type: "task-agent" - pass the project_path argument for parallel analysis across phase breakdown, context creation, and structure building optimized for Rust CLI development with container orchestration workflows.