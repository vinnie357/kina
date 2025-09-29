# Kina: Kubernetes Cluster Management CLI for macOS

## Project Overview

Kina is a sophisticated Rust-based CLI application designed for Kubernetes cluster management using Apple Container technology. It provides a powerful, flexible tool for local Kubernetes orchestration on macOS, following a monolithic CLI architecture with a clean provider abstraction layer.

## Technology Stack

### Core Technologies
- **Programming Language**: Rust 2021 Edition (Minimum version: 1.70)
- **Primary Frameworks**:
  - CLI Framework: Clap v4.5.47 (with derive macros)
  - Async Runtime: Tokio v1.47.1 (full features)
  - Error Handling: Anyhow v1.0.99, Thiserror v1.0.69
  - Logging: Tracing v0.1.41 with structured logging

### Planned Integrations
- Kubernetes Client: kube-rs v0.87
- Container Runtimes: Apple Container, (Future) Docker support

## Quick Start

### Prerequisites
- **Operating System**: macOS 15.6+
- **Runtime**: Apple Container technology installed
- **Development Tools**:
  - Rust 1.70+ (stable channel)
  - mise (task runner)
  - Optional: Docker for additional container runtime support

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/kina.git
cd kina

# Build the project
mise run build

# Run tests
mise run test
```

## Essential Commands

### Cluster Management

```bash
# Create a new Kubernetes cluster
kina cluster create --name my-cluster

# List existing clusters
kina cluster list

# Delete a cluster
kina cluster delete --name my-cluster
```

### Configuration

```bash
# View current configuration
kina config view

# Set configuration parameters
kina config set cluster.runtime=apple-container
```

## Development Workflows

### Build and Test

```bash
# Standard build
mise run build

# Development build
mise run dev

# Run tests
mise run test

# Code quality checks
mise run lint
```

### Task Automation

The project uses `mise` for comprehensive task automation:
- `mise run build`: Release build
- `mise run dev`: Development build
- `mise run test`: Execute all tests
- `mise run lint`: Code quality checks

## Architecture Highlights

### Provider Abstraction
Kina implements a flexible `ContainerProvider` trait, enabling:
- Runtime-agnostic container management
- Easy extension to new container technologies
- Clean separation of container runtime concerns

### Design Patterns
- **Builder Pattern**: For complex container specification construction
- **Provider Pattern**: Abstracts container runtime operations
- **Command Pattern**: Encapsulates CLI subcommand logic
- **Domain-Driven Layered Architecture**

## Roadmap and Development Status

### Current Focus
- Rust CLI framework integration
- Apple Container runtime support
- Kubernetes cluster management primitives

### Planned Enhancements
1. Comprehensive Kubernetes client integration
2. Multi-runtime support
3. Advanced cluster lifecycle management
4. Enhanced testing infrastructure
5. Performance optimization

## Troubleshooting

### Common Issues
- Verify Apple Container runtime is correctly installed
- Check Rust toolchain compatibility
- Validate configuration file syntax
- Review tracing logs for detailed error information

## Contributing

1. Review contribution guidelines
2. Fork the repository
3. Create a feature branch
4. Implement your changes
5. Submit a pull request

## Security and Quality

- Comprehensive error handling
- Structured logging
- Planned security scanning with cargo-deny
- Rust's strong type system for robust implementation

## License

[Insert appropriate open-source license]

## Contact and Support

- GitHub Issues: Project repository
- Discussion Forums: TBD

---

*Crafted with Rust, powered by Apple Container technology, designed for Kubernetes enthusiasts.*