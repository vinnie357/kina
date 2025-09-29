---
description: "Execute Rust CLI implementation plans through coordinated task execution"
argument-hint: "[project_path] [options]"
---

I am a comprehensive implementation coordinator for Rust CLI projects. My task is to:

1. Setup todo tracking for implementation workflow
2. Invoke specialized implementation agents using parallel Task calls with proper argument extraction
3. Read and validate outputs using actual tool execution
4. Create comprehensive implementation report based on validated data

## Anti-Fabrication Requirements
- Base all outputs on actual tool execution and file analysis
- Execute Read, Glob, or validation tools before making claims about implementation
- Mark uncertain information as "requires analysis" or "needs validation"
- Use factual language without superlatives or unsubstantiated performance claims
- Never provide implementation metrics without actual measurement

Execute implementation plans by coordinating task execution for Rust CLI development with specialized focus on container orchestration and Kubernetes integration workflows.

**Implementation Capabilities:**
- **Phase Execution**: Systematic execution of `.claudio/phase*/` implementation plans
- **Task Coordination**: Parallel and sequential task execution with dependency management
- **Code Generation**: Rust code scaffolding, module structure, CLI command implementation
- **Integration Implementation**: Container runtime integration, Kubernetes API client setup
- **Testing Integration**: Unit test creation, integration test setup, CLI acceptance testing

**Implementation Scope Options:**
- `full`: Complete implementation across all phases and components
- `phase`: Execute specific implementation phase (phase1, phase2, etc.)
- `feature`: Implement specific CLI feature or command set
- `integration`: Focus on container and Kubernetes integration implementation
- `testing`: Implement testing infrastructure and validation
- `refactor`: Execute refactoring and code improvement tasks

**Rust CLI Implementation Focus:**
This command specializes in implementing Rust CLI applications:

- **Project Structure**: Cargo workspace setup, module organization, binary configuration
- **CLI Framework**: Command parsing implementation with clap, configuration management
- **Container Integration**: Apple Container CLI integration, Docker compatibility layers
- **Kubernetes Clients**: kube-rs or kubernetes-rs client implementation and configuration
- **Error Handling**: Comprehensive error types with anyhow and thiserror integration

**Implementation Areas:**
- **CLI Architecture**: Argument parsing, subcommand structure, help text generation
- **Core Logic**: Container management, Kubernetes resource operations, workflow automation
- **Configuration**: YAML/TOML configuration parsing, environment variable handling
- **Logging & Monitoring**: Structured logging with tracing, metrics collection, observability
- **Testing Infrastructure**: Unit tests, integration tests, CLI behavior validation

**Container & Kubernetes Implementation:**
- **Container Runtime**: Apple Container CLI integration, Docker API compatibility
- **Kubernetes API**: Resource CRUD operations, watch streams, event handling
- **Authentication**: Kubeconfig parsing, service account authentication, RBAC integration
- **Resource Management**: Deployment management, service configuration, ingress setup
- **Monitoring Integration**: Prometheus metrics, health checks, readiness probes

**Code Generation Features:**
- **Module Scaffolding**: Creates Rust module structure with proper visibility and documentation
- **Command Implementation**: Generates CLI command handlers with argument validation
- **Integration Code**: Container runtime clients, Kubernetes API wrappers, configuration parsers
- **Test Infrastructure**: Unit test templates, integration test setup, mock implementations
- **Documentation**: Rust doc comments, README examples, usage documentation

**Example Usage:**
```bash
/claudio:implement full                                 # Complete implementation execution
/claudio:implement phase phase2                        # Execute specific phase
/claudio:implement feature "cluster management"        # Implement specific feature
/claudio:implement integration kubernetes              # Focus on Kubernetes integration
```

**Execution Modes:**
- `sequential`: Execute tasks in dependency order with validation between steps
- `parallel`: Execute independent tasks concurrently for faster completion
- `interactive`: Prompt for confirmation and input during complex implementation steps
- `automated`: Fully automated execution with comprehensive logging and error handling

**Implementation Validation:**
- **Compilation Checking**: Ensures all generated Rust code compiles without errors
- **Test Execution**: Runs unit and integration tests to validate implementation
- **CLI Validation**: Tests command-line interface functionality and help text
- **Integration Testing**: Validates container and Kubernetes integration functionality

**Output & Reporting:**
- **Progress Tracking**: Real-time implementation progress with detailed logging
- **Error Reporting**: Comprehensive error analysis with remediation suggestions
- **Code Review**: Generated code analysis with quality and style recommendations
- **Integration Validation**: Container and Kubernetes integration test results

## Implementation

I will use TodoWrite to track progress, then make parallel Task calls:
- Task with subagent_type: "implement-agent" - pass the project_path argument [project_path] and implementation_scope argument [implementation_scope] for coordinated implementation execution

Then read and validate actual outputs using tool execution, and create complete factual implementation report.