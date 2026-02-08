# kina (Kubernetes in Apple Container) - AI Assistant Context

## Project Overview
**Technology Stack**: Rust 2021 edition with clap CLI framework, Tokio async runtime, kube-rs for Kubernetes integration, Apple Container runtime
**Architecture**: Monolithic CLI with provider abstraction layer, following domain-driven layered architecture
**Domain**: Kubernetes orchestration and container management for macOS using Apple Container technology
**Development Phase**: Active development with established project structure, comprehensive tooling, and advanced development practices

## Task Tracking
Project tasks are tracked with **beads** (`bd`), a distributed git-backed issue tracker.
See [AGENTS.md](AGENTS.md) for beads workflow, session completion rules, and `bd` commands.
- Tasks stored in `.beads/` directory, synced via git
- Use `bd ready` to find actionable tasks with no blockers
- Use `bd list` to see all open tasks
- Workflow: one task = one branch = one PR
- Commit format: `type(scope): description` (no Co-Authored-By)

## Project Structure
- **kina-cli/src/**: Rust CLI source (cli/, core/, config/, errors/, utils/)
- **kina-cli/tests/**: CLI and config tests
- **kina-cli/manifests/**: Kubernetes manifests (nginx-ingress)
- **kina-cli/images/**: Custom node image Dockerfile and build scripts
- **docs/research/**: Apple Container, KIND, CNI/Cilium research
- **docs/planning/**: PRD and implementation plan
- **docs/development/**: Testing patterns

## AI Assistant Guidance

### Project-Specific Focus Areas
- **Rust Development**: Apply Cargo-based project management, CLI framework integration (clap), error handling (anyhow/thiserror)
- **Container Integration**: Use Apple Container native patterns, Docker API compatibility considerations
- **Kubernetes Operations**: Consider kube-rs client patterns, RBAC configuration, resource management
- **CLI Patterns**: Focus on command parsing, configuration management, output formatting

### Workflow
- **Beads-driven**: Use `bd ready` to find tasks, `bd update <id> --status in_progress` to claim
- **Branch per task**: `git checkout main && git pull` then `git checkout -b type/description`
- **Research-backed**: Reference docs/research/ for architecture decisions and constraints

### Anti-Fabrication Requirements
All AI assistants working on this project MUST adhere to strict factual accuracy:
- Base all outputs on actual project analysis using tool execution (Read, Glob, Bash)
- Execute validation tools before making claims about file existence or system capabilities
- Mark uncertain information as "requires analysis" or "needs validation"
- Use precise, factual language without superlatives or unsubstantiated performance claims
- Never fabricate time estimates, effort calculations, or completion timelines without measurement

### Development Context
- **Current State**: Phase 1-2 complete (infrastructure, provider, CLI, lifecycle management). Working on multi-node orchestration and advanced features.
- **Technology Requirements**: macOS 26+, Apple Container 0.5.0+, kubectl, mise, Nushell
- **Development Priorities**: Multi-node cluster support, CI pipeline with act/colima, advanced networking
- **Integration Goals**: Kind (Kubernetes in Docker) workflow compatibility using Apple Container technology