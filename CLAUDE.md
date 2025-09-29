# kina (Kubernetes in Apple Container) - AI Assistant Context

## Project Overview
**Technology Stack**: Rust 2021 edition with clap CLI framework, Tokio async runtime, kube-rs for Kubernetes integration, Apple Container runtime
**Architecture**: Monolithic CLI with provider abstraction layer, following domain-driven layered architecture
**Domain**: Kubernetes orchestration and container management for macOS using Apple Container technology
**Development Phase**: Active development with established project structure, comprehensive tooling, and advanced development practices

## Available Agents

### Workflow Agents
- **discovery-agent**: Rust CLI project analysis with Cargo.toml analysis, container integration assessment
- **prd-agent**: Product requirements for CLI command structure and Kubernetes API specifications
- **plan-agent**: Implementation planning with Cargo workspace setup and container integration milestones
- **task-agent**: Task breakdown for module scaffolding and container runtime integration
- **research-specialist**: Research for CLI frameworks (clap, structopt), Kubernetes clients (kube-rs), container APIs

### Development Agents
- **code-quality-analyzer**: Rust code quality with rustfmt, clippy, cargo-audit, CLI-specific patterns
- **implement-agent**: Execute Rust CLI implementation with Cargo project execution and Kubernetes client integration
- **test-review**: Cargo test patterns, CLI testing, container integration testing recommendations
- **kina-test-runner**: Project-specific test execution for Rust CLI and Apple Container integration

### Security Agents
- **security-review-coordinator**: STRIDE methodology for Rust CLI and container orchestration security
- **security-threat-modeler**: CLI attack vectors, container security threats, Kubernetes integration analysis
- **security-architecture-analyst**: CLI security architecture, container integration security, RBAC patterns
- **vulnerability-assessment-specialist**: Rust dependency scanning, Kubernetes manifest analysis
- **security-diagram-generator**: Mermaid diagrams for CLI threat models and RBAC visualization

### Documentation Agents
- **documentation-coordinator**: Rust CLI documentation strategy coordination
- **documentation-readme-creator**: CLI installation guides and container orchestration workflows
- **documentation-api-creator**: Command reference and Kubernetes client usage patterns
- **documentation-user-guide-creator**: CLI workflow tutorials and container management guides
- **documentation-developer-guide-creator**: Rust CLI architecture and contribution guidelines

## Extended Context Structure
- **workflow/**: Discovery analysis, requirements definition, implementation planning, task breakdown
- **development/**: Code quality patterns, security frameworks, testing strategies
- **documentation/**: Generation templates and structured documentation patterns
- **research/**: Research methodology for CLI frameworks and container integration
- **templates/**: Agent architecture and workflow integration patterns

## AI Assistant Guidance

### Project-Specific Focus Areas
- **Rust Development**: Apply Cargo-based project management, CLI framework integration (clap), error handling (anyhow/thiserror)
- **Container Integration**: Use Apple Container native patterns, Docker API compatibility considerations
- **Kubernetes Operations**: Consider kube-rs client patterns, RBAC configuration, resource management
- **CLI Patterns**: Focus on command parsing, configuration management, output formatting

### Integration Patterns
- **Extended Context**: Reference Rust-specific patterns from workflow and development contexts
- **Agent Coordination**: Use specialized agents for container security and Kubernetes integration analysis
- **Discovery-Driven**: Base recommendations on actual project structure analysis from discovery findings

### Anti-Fabrication Requirements
All AI assistants working on this project MUST adhere to strict factual accuracy:
- Base all outputs on actual project analysis using tool execution (Read, Glob, Bash)
- Execute validation tools before making claims about file existence or system capabilities
- Mark uncertain information as "requires analysis" or "needs validation"
- Use precise, factual language without superlatives or unsubstantiated performance claims
- Never fabricate time estimates, effort calculations, or completion timelines without measurement

### Development Context
- **Current State**: Active development requiring Rust project structure enhancement and Apple Container integration
- **Technology Requirements**: macOS 15.6+, Apple Container runtime, kubectl integration
- **Development Priorities**: CLI command architecture refinement, container runtime optimization, Kubernetes client implementation
- **Integration Goals**: Kind (Kubernetes in Docker) workflow compatibility using Apple Container technology