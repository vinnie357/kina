# kina (Kubernetes in Apple Container) - AI Assistant Context

## Project Overview
**Technology Stack**: Rust 2021 edition with clap CLI framework, Tokio async runtime, kube-rs for Kubernetes integration, Apple Container runtime
**Architecture**: Monolithic CLI with provider abstraction layer, following domain-driven layered architecture
**Domain**: Kubernetes orchestration and container management for macOS using Apple Container technology
**Development Phase**: Active development with established project structure, comprehensive tooling, and advanced development practices

## Task Tracking

Use GitHub Issues. Read [AGENTS.md](AGENTS.md) for repository process.

## Project Structure
- **kina-cli/src/**: Rust CLI source (cli/, core/, config/, errors/, utils/)
- **kina-cli/tests/**: CLI and config tests
- **kina-cli/manifests/**: Kubernetes manifests (nginx-ingress, demo-app)
- **kina-cli/images/**: Custom node image Dockerfile and build scripts
- **scripts/**: Extracted mise task scripts (Nushell `.nu` and Bash `.sh`)
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
- **Issue-driven**: Claim the GitHub issue before implementation.
- **Worktree per task**: Follow `docs/agents/worktrees.md`; keep the primary checkout on clean main.
- **Research-backed**: Reference docs/research/ for architecture decisions and constraints
- **Discover tools with mise**: Run `mise tasks` to see all available development tasks. Task namespaces: `test:` (unit tests), `test:cluster:` (integration tests), `test:action:` (GitHub Actions), `kina:` (CLI), `image:` (node images), `k8s:` (Kubernetes tools), `worktree:` (clean-main worktrees), `council:` (plan review), `gitleaks:` (security scanning), `colima:` (Docker via colima)

### Anti-Fabrication Requirements
All AI assistants working on this project MUST adhere to strict factual accuracy:
- Base all outputs on actual project analysis using tool execution (Read, Glob, Bash)
- Execute validation tools before making claims about file existence or system capabilities
- Mark uncertain information as "requires analysis" or "needs validation"
- Use precise, factual language without superlatives or unsubstantiated performance claims
- Never fabricate time estimates, effort calculations, or completion timelines without measurement

### Development Context
- **Current State**: Phase 1-2 complete (infrastructure, provider, CLI, lifecycle management). Working on multi-node orchestration and advanced features.
- **Technology Requirements**: macOS 26+, Apple Container 1.0.0+, kubectl, mise, Nushell
- **Development Priorities**: Multi-node cluster support, CI pipeline with act/colima, advanced networking
- **Integration Goals**: Kind (Kubernetes in Docker) workflow compatibility using Apple Container technology

## Agent skills

### Issue tracker

GitHub Issues holds durable work. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five canonical triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

Use a single-context glossary and ADR layout. See `docs/agents/domain.md`.
