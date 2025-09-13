# kina (Kubernetes in Apple Container) - AI Assistant Context

## Project Overview
**Technology Stack**: Rust (planned primary language), Apple Container runtime, Kubernetes API
**Architecture**: CLI application with command-line interface for Kubernetes cluster management
**Domain**: Kubernetes orchestration and container management for macOS
**Development Phase**: Early planning stage with comprehensive documentation, no implementation code

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

## Claudio Agent System Integration

### System Architecture
This project is integrated with the **Claudio Agent Control System**, a sophisticated multi-agent workflow framework that coordinates specialized AI agents for complex development tasks.

### Agent Integration
The Claudio system provides specialized agents that can be invoked through the Task tool for complex development workflows. These agents are available for project-specific analysis and development tasks.

### Agent Coordination Patterns
- **Parallel Execution**: Multiple specialized agents work simultaneously on different aspects
- **Sequential Dependencies**: Discovery → PRD → Planning → Task breakdown workflow
- **Extended Context**: Agents reference project-specific patterns from extended context structure
- **Validation Requirements**: All agent outputs validated through tool execution before reporting

### Quality Assurance Framework
- **Anti-Fabrication Protocol**: All agents must base outputs on actual project analysis
- **Tool Validation**: Claims verified through Read, Glob, and Bash tool execution
- **Factual Reporting**: No fabricated metrics, superlatives, or unsubstantiated performance claims
- **Source Attribution**: All recommendations traced to specific analysis or measurement

## AI Assistant Guidance

### Anti-Fabrication Requirements
All AI assistants working on this project MUST adhere to strict factual accuracy standards:

**Core Principles:**
- Base all outputs on actual project analysis using tool execution (Read, Glob, Bash)
- Execute validation tools before making claims about file existence or system capabilities
- Mark uncertain information as "requires analysis", "needs validation", or "requires investigation"
- Use precise, factual language without superlatives or unsubstantiated performance claims
- Never fabricate time estimates, effort calculations, or completion timelines without measurement

**Prohibited Practices:**
- **Fabricated Metrics**: Never invent success percentages, performance numbers, or business impact data
- **Superlative Language**: Avoid "excellent", "comprehensive", "advanced", "optimal", "perfect"
- **Assumed Capabilities**: Don't claim features exist without tool verification through Read or Glob
- **Generic Claims**: Replace vague statements with specific, measurable observations
- **Unsubstantiated Testing**: Never report test results without actual execution and verification

**Validation Requirements:**
- **File Claims**: Use Read or Glob tools before claiming files exist or contain specific content
- **System Integration**: Use Bash or appropriate tools to verify system capabilities
- **Framework Detection**: Execute actual detection logic before claiming framework presence
- **Test Results**: Only report test outcomes after actual execution with tool verification
- **Performance Claims**: Base any performance statements on actual measurement or analysis

### Project-Specific Focus Areas
- **Rust Development**: Apply Cargo-based project management, CLI framework integration (clap), error handling
- **Container Integration**: Use Apple Container native patterns, Docker API compatibility considerations
- **Kubernetes Operations**: Consider kube-rs client patterns, RBAC configuration, resource management
- **CLI Patterns**: Focus on command parsing, configuration management, output formatting

### Integration Patterns
- **Extended Context**: Reference Rust-specific patterns from workflow and development contexts
- **Agent Coordination**: Use specialized agents for container security and Kubernetes integration analysis
- **Discovery-Driven**: Base recommendations on actual project structure analysis from discovery findings
- **Claudio Workflow**: Leverage the Claudio system for complex multi-step tasks requiring agent coordination
- **Tool Validation**: All integration recommendations validated through actual framework detection and analysis

### Development Workflow
- **Project Initialization**: Cargo workspace setup, CLI framework selection, development environment (mise)
- **Quality Automation**: Integrated rustfmt, clippy, cargo-audit with container security scanning
- **Container Focus**: Apple Container CLI integration, Kubernetes API client implementation
- **Testing Strategy**: CLI testing with assert_cmd, container integration testing patterns
- **Agent Coordination**: Leverage specialized agents through Task tool invocation for complex analysis and development tasks

## Claudio Best Practices

### Recommended Workflow Patterns
- **Discovery First**: Use discovery-agent through Task tool to establish project context and technology assessment
- **Sequential Planning**: Follow Discovery → PRD → Planning → Task breakdown for complex features using specialized agents
- **Parallel Analysis**: Run security, quality, and documentation analysis simultaneously through multiple agent invocations
- **Validation Focus**: Ensure all agent outputs are validated through actual tool execution

### Agent Usage Guidelines
- Use research-specialist for investigating new technologies or integration approaches
- Apply security-review-coordinator for comprehensive threat modeling and security assessment
- Leverage code-quality-analyzer for Rust-specific quality analysis with cargo tooling
- Execute project-test-runner for testing strategy development and validation

### Quality Assurance Integration
- All agents must validate file existence before making claims about project structure
- Framework detection must be verified through actual tool execution (cargo check, file analysis)
- Testing recommendations must be based on actual project structure analysis
- Performance claims require measurement or explicit marking as "requires analysis"

## Usage Context
- **Current State**: Planning phase requiring project structure initialization and Rust environment setup
- **Technology Requirements**: macOS 15.6+, Apple Container runtime, kubectl integration
- **Development Priorities**: Rust project scaffolding, CLI command architecture, container runtime research
- **Integration Goals**: Kind (Kubernetes in Docker) workflow compatibility using Apple Container technology
- **Claudio Integration**: Multi-agent workflow system available for complex development tasks and analysis