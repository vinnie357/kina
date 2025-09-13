---
description: "Conduct comprehensive research for Rust CLI development with Kubernetes integration focus"
argument-hint: "<research_topic> [depth] [focus_area]"
---

Conduct comprehensive research and create expert analysis documents for Rust CLI development projects. Specializes in container orchestration, Kubernetes integration, and Rust ecosystem research.

**Research Areas:**
- **Rust CLI Frameworks**: clap, structopt, argh comparison and selection criteria
- **Container Integration**: Apple Container, Docker compatibility, container runtime APIs
- **Kubernetes Clients**: kube-rs, kubernetes-rs, kubectl integration patterns
- **CLI Architecture**: Command patterns, configuration management, error handling
- **Performance Optimization**: Binary size, startup time, memory usage optimization
- **Security Patterns**: Input validation, privilege management, credential handling
- **Testing Strategies**: Unit testing, integration testing, CLI acceptance testing
- **Distribution**: Packaging, installation, cross-platform compatibility

**Research Depth Levels:**
- `overview`: High-level comparison and recommendations
- `detailed`: In-depth analysis with implementation examples
- `comprehensive`: Complete research with benchmarks, trade-offs, and migration guides
- `comparison`: Side-by-side analysis of multiple solutions
- `integration`: Focus on system integration patterns and compatibility

**Focus Areas:**
- `performance`: Runtime performance, resource usage, optimization techniques
- `security`: Security best practices, vulnerability analysis, compliance patterns
- `integration`: API integration, system compatibility, workflow integration
- `ecosystem`: Rust crate ecosystem, dependency management, community resources
- `deployment`: Distribution, packaging, installation, CI/CD integration

**Example Usage:**
```bash
/claudio:research "Kubernetes Rust clients" detailed integration     # Detailed k8s client research
/claudio:research "CLI argument parsing" comparison                  # Compare clap vs alternatives
/claudio:research "Apple Container integration" comprehensive        # Complete container research
/claudio:research "Rust binary optimization" overview performance    # Performance-focused research
```

**Research Integration:**
- **Discovery Context**: Uses `.claudio/docs/discovery.md` for project-specific research direction
- **Implementation Focus**: Research aligned with identified technology stack and requirements
- **Output Format**: Creates `.claudio/research/` documents for integration with PRD and planning commands

**Rust CLI Research Specializations:**
- **CLI Framework Selection**: Performance, ergonomics, feature comparison for command-line frameworks
- **Container Runtime Integration**: Apple Container vs Docker, API compatibility, runtime performance
- **Kubernetes Client Libraries**: API coverage, async patterns, resource management strategies
- **Cross-Platform Development**: Target platform considerations, compilation strategies, dependency management
- **Error Handling Patterns**: CLI-appropriate error presentation, logging, debugging strategies

**Research Output:**
Creates specialized research documents in `.claudio/research/` including:
- Technology comparison matrices with quantitative analysis
- Implementation examples and code patterns
- Performance benchmarks and optimization recommendations
- Integration guides and compatibility assessments
- Security analysis and best practice documentation

**Integration with Workflow:**
- **PRD Enhancement**: Research findings automatically integrated into requirements documents
- **Planning Input**: Research recommendations influence implementation planning
- **Implementation Guidance**: Detailed research provides task-specific implementation patterns

Task with subagent_type: "research-specialist" - pass the project_path argument for comprehensive research and expert analysis creation focused on Rust CLI development with Kubernetes and container orchestration integration patterns.