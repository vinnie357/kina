---
description: "Execute Rust CLI code quality assessment with project-aware tool detection"
argument-hint: "<assessment_type> [project_path] [report_format]"
---

Execute code quality assessment by running project-specific linting, formatting, and testing tools with factual analysis based on actual tool outputs for Rust CLI applications.

**CRITICAL: NEVER fabricate quality metrics or results. Only report actual tool execution outputs and real findings.**

**Assessment Types:**
- `full`: Assessment with all available Rust tools (rustfmt, clippy, cargo-audit, tests)
- `lint`: Static analysis and linting only (clippy, cargo-audit)
- `format`: Code formatting assessment only (rustfmt, cargo fmt --check)
- `test`: Test execution with coverage analysis (cargo test, cargo tarpaulin)
- `complexity`: Complexity and maintainability analysis (cargo complexity, loc)
- `coverage`: Coverage gap analysis with tarpaulin or llvm-cov
- `quick`: Essential metrics for rapid feedback (clippy, basic tests)

**Rust CLI Quality Focus:**
This command specializes in Rust CLI application quality assessment:

- **Cargo Integration**: Runs cargo-based quality tools with proper workspace handling
- **CLI-Specific Checks**: Command parsing validation, help text consistency, error handling patterns
- **Container Integration**: Security scanning for container-related dependencies and configurations
- **Performance Analysis**: Binary size analysis, compilation time, runtime performance metrics
- **Kubernetes Integration**: API client security, configuration validation, RBAC compliance

**Rust Quality Tools Integration:**
- **rustfmt**: Code formatting consistency with custom CLI formatting rules
- **clippy**: Advanced linting with CLI-specific lint configurations
- **cargo-audit**: Security vulnerability scanning for dependencies
- **cargo-deny**: License compliance and dependency policy enforcement
- **cargo-outdated**: Dependency freshness and update recommendations
- **cargo-bloat**: Binary size analysis and optimization opportunities

**CLI-Specific Quality Checks:**
- **Command Structure**: Argument parsing consistency, help text accuracy, subcommand validation
- **Error Handling**: Proper error propagation, user-friendly error messages, exit codes
- **Configuration**: Config file parsing, environment variable handling, default values
- **Integration**: Container runtime compatibility, Kubernetes API usage patterns
- **Security**: Input validation, privilege handling, credential management

**Container & Kubernetes Quality:**
- **Security Scanning**: Container image vulnerabilities, RBAC configurations
- **API Compliance**: Kubernetes API version compatibility, deprecation warnings
- **Resource Management**: Memory usage patterns, file handle management, cleanup procedures
- **Configuration Validation**: YAML parsing, schema validation, default fallbacks

**Example Usage:**
```bash
/claudio:code-quality full                              # Complete Rust CLI assessment
/claudio:code-quality lint ./kina-cli                   # Linting only for specific project
/claudio:code-quality test --coverage                   # Test execution with coverage
/claudio:code-quality quick                             # Fast feedback for CI/CD
```

**Note**: Optional command for enhanced project-specific quality analysis tailored to Rust CLI applications with container orchestration integration.

Task with subagent_type: "code-quality-analyzer" - pass the project_path argument for project-aware Rust CLI tool detection to execute appropriate quality tools and generate factual reports based on actual analysis with container orchestration quality patterns.