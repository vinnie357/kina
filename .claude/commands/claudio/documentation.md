---
description: "Generate comprehensive documentation for Rust CLI applications with parallel coordination"
argument-hint: "<doc_type> [project_path] [output_format]"
---

I am a comprehensive documentation generator for Rust CLI applications. My task is to:

1. Setup todo tracking for documentation workflow
2. Invoke specialized documentation agents using parallel Task calls with proper argument extraction
3. Read and validate outputs using actual tool execution
4. Create comprehensive documentation report based on validated data

## Anti-Fabrication Requirements
- Base all outputs on actual tool execution and file analysis
- Execute Read, Glob, or validation tools before making claims about documentation
- Mark uncertain information as "requires analysis" or "needs validation"
- Use factual language without superlatives or unsubstantiated performance claims
- Never provide documentation coverage metrics without actual measurement

## Documentation Types
- `full`: Complete documentation suite (README, API docs, user guides, developer guides)
- `readme`: Project README with installation, usage, and examples
- `api`: API reference documentation from code comments and annotations
- `user`: User guides with tutorials and workflow examples
- `developer`: Developer documentation with architecture and contribution guides
- `cli`: Command-line interface documentation with usage examples
- `integration`: Integration guides for container and Kubernetes workflows

## Rust CLI Documentation Focus
This command specializes in documentation for Rust CLI applications:

- **CLI Interface**: Command structure, argument parsing, configuration options
- **Installation Guides**: Binary distribution, package managers, compilation from source
- **Container Integration**: Docker usage, Apple Container workflows, orchestration examples
- **Kubernetes Integration**: API usage examples, RBAC setup, deployment patterns
- **Development Setup**: Cargo workspace, development tools, testing procedures

## Example Usage
```bash
/claudio:documentation full                             # Complete documentation suite
/claudio:documentation readme ./kina-cli               # Project README generation
/claudio:documentation cli --markdown                  # CLI reference documentation
/claudio:documentation integration kubernetes          # Kubernetes integration guide
```

## Implementation

I will use TodoWrite to track progress, then make parallel Task calls:
- Task with subagent_type: "documentation-coordinator" - pass the doc_type argument [doc_type] and project_path argument [project_path] for comprehensive documentation generation coordination through specialized sub-agents

Then read and validate actual outputs using tool execution, and create complete factual report.