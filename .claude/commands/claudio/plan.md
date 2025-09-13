---
description: "Create actionable phase/task structures for Rust CLI development from any planning input"
argument-hint: "<plan_type> <input_source> [options]"
---

Create actionable implementation structures with organized phases and executable tasks from any planning input. Always generates `.claudio/phase*/` directories with ready-to-execute Rust development task contexts.

**Flexible Input Patterns:**
```bash
/claudio:plan feature "myprd.md"                           # Use external PRD file
/claudio:plan feature "add Kubernetes cluster management"  # Direct CLI feature description
/claudio:plan enhancement "improve performance, use research on Apple Container optimization"  # Description with research references
/claudio:plan myexisting_plan.md                          # Update existing plan structure
/claudio:plan refactor "modernize CLI argument parsing"    # Refactoring plan
```

**Plan Types:**
- `feature`: Single CLI feature implementation structure (commands, parsing, integration)
- `project`: Complete Rust CLI project implementation structure (architecture, modules, testing)
- `migration`: System/workflow migration structure (Docker to Apple Container, legacy tools)
- `refactor`: Code refactoring and improvement structure (module organization, performance)
- `integration`: System integration implementation structure (Kubernetes API, container runtimes)
- `enhancement`: CLI improvement and optimization structure (UX, performance, reliability)

**Input Sources:**
- **External Files**: Any .md file (PRDs, specs, requirements, existing plans)
- **Direct Descriptions**: CLI feature descriptions, command objectives, architecture requirements
- **Research References**: Automatically incorporates `.claudio/research/` documents when mentioned
- **Existing Plans**: Updates and enhances existing `.claudio/phase*/` structures

**Output Structure:**
- **Phase Directories**: `.claudio/phase1/`, `phase2/`, etc. with organized Rust development phases
- **Task Lists**: `tasks.md` files with specific, actionable Rust development tasks
- **Task Contexts**: Individual `claude.md` contexts for complex tasks with Rust-specific detailed guidance
- **Progress Tracking**: Status files for monitoring CLI implementation progress

**Research Integration:**
When you reference research (e.g., "use research on container orchestration"), the plan automatically:
- Locates matching `.claudio/research/` documents
- Incorporates research findings into Rust implementation task contexts
- References research in CLI development guidance
- Provides Rust ecosystem integration patterns

**Rust CLI Development Focus:**
This plan command specializes in:
- **Project Structure**: Cargo workspace setup, module organization, binary configuration
- **CLI Architecture**: Command parsing with clap, configuration management, error handling
- **Container Integration**: Apple Container CLI integration, Kubernetes API clients
- **Testing Strategy**: Unit tests with cargo test, integration testing, CLI acceptance testing
- **Build & Distribution**: Release builds, binary optimization, distribution packaging

**Example Phase Organization for CLI Projects:**
- **Phase 1**: Project setup (Cargo.toml, module structure, CI/CD)
- **Phase 2**: Core CLI framework (argument parsing, configuration, logging)
- **Phase 3**: Business logic (container management, Kubernetes integration)
- **Phase 4**: Testing & validation (unit tests, integration tests, CLI testing)
- **Phase 5**: Documentation & distribution (help text, man pages, packaging)

**Rust Development Task Examples:**
- Cargo dependency management with version resolution
- CLI command structure with clap argument parsing
- Error handling with thiserror and anyhow integration
- Container runtime integration with Apple Container CLI
- Kubernetes API client setup with kube-rs or kubernetes-rs

Task with subagent_type: "plan-agent" - pass the project_path argument to transform any planning input into organized, actionable phase/task structures ready for immediate Rust CLI development execution with container orchestration integration.