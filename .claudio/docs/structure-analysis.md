# Project Structure Analysis

## Project Overview
**Project Type**: Rust workspace with CLI application for Kubernetes container management
**Primary Domain**: Kubernetes orchestration using Apple Container technology
**Architecture**: Modular CLI application with core container abstraction layer
**Development Stage**: Active development with established project structure

## Directory Organization
- **Total Directories**: 31
- **Total Files**: 60
- **Max Depth**: 6 levels (excluding system nested examples)
- **Organization Pattern**: Feature-based modular structure with clear separation of concerns

## Key Directories

### Source Code Directories
- **`kina-cli/src/`** - Main application source code
  - **`src/core/`** - Core container and Kubernetes abstraction modules
  - **`src/cli/`** - Command-line interface implementation
  - **`src/config/`** - Configuration management modules
  - **`src/utils/`** - Utility functions and helpers
  - **`src/errors/`** - Error handling definitions

### Testing Directories  
- **`kina-cli/tests/`** - Integration and CLI tests
  - **`tests/fixtures/`** - Test configuration files and data

### Configuration Directories
- **Root configuration files**: `Cargo.toml`, `rustfmt.toml`, `clippy.toml`, `mise.toml`
- **`kina-cli/templates/`** - Configuration templates for container networking

### Resource and Manifest Directories
- **`kina-cli/manifests/`** - Kubernetes manifest files
  - **`manifests/nginx-ingress/`** - NGINX ingress controller configurations
- **`kina-cli/images/`** - Container image build scripts and Dockerfiles
  - **`images/kina-node/`** - Node container image specifications

### Documentation Directories
- **`docs/`** - Project documentation (CNI and kubelet configuration guides)
- **`.claudio/`** - Claudio system installation (analysis and discovery outputs)

### Example and Script Directories
- **`kina-cli/examples/`** - Usage examples and demonstration configurations
  - **`examples/ingress/`** - Ingress controller examples and routing configurations
- **`kina-cli/scripts/`** - Development and deployment shell scripts

### Build and Distribution Directories
- **`target/`** - Rust build artifacts and compiled binaries (excluded from analysis)

## Directory Tree Structure
```
kina/
├── Cargo.toml (workspace configuration)
├── kina-cli/ (main application package)
│   ├── src/
│   │   ├── core/ (container and Kubernetes abstractions)
│   │   ├── cli/ (command-line interface)
│   │   ├── config/ (configuration management)
│   │   ├── utils/ (utility functions)
│   │   └── errors/ (error handling)
│   ├── tests/ (integration tests)
│   ├── manifests/ (Kubernetes configurations)
│   ├── images/ (container build specifications)
│   ├── examples/ (usage demonstrations)
│   ├── scripts/ (automation scripts)
│   └── templates/ (configuration templates)
├── docs/ (project documentation)
└── [configuration files: rustfmt.toml, clippy.toml, mise.toml]
```

## Organization Patterns

### File Naming Conventions
- **Rust modules**: snake_case naming (`apple_container.rs`, `cluster_config.rs`)
- **Directories**: kebab-case for multi-word names (`kina-cli`, `nginx-ingress`)
- **Configuration files**: lowercase with extensions (`rustfmt.toml`, `.gitignore`)
- **Scripts**: kebab-case shell scripts (`.sh` extension)

### Code Organization Structure
- **Modular design**: Clear separation between core logic, CLI interface, and configuration
- **Domain-driven modules**: Core modules organized by functionality (provider, kubernetes, cluster)
- **Feature isolation**: Examples, manifests, and scripts organized by specific use cases

### Project Structure Type
- **Cargo workspace**: Single workspace with one primary package (`kina-cli`)
- **Library + Binary**: Dual structure with both library (`lib.rs`) and binary (`main.rs`) targets
- **Layered architecture**: Core abstractions, CLI layer, and configuration management

## Configuration Management

### Build Configuration
- **`Cargo.toml`** - Workspace and dependency management with comprehensive dependency list
- **`clippy.toml`** - Rust linting configuration
- **`rustfmt.toml`** - Code formatting standards
- **`mise.toml`** - Development environment and tool version management

### Runtime Configuration
- **`kina-cli/templates/ptp-cni.conflist`** - Container network interface configuration template
- **Test fixtures**: `tests/fixtures/test-config.toml` for integration testing

## Technology Stack Indicators

### Primary Technologies
- **Language**: Rust (2021 edition, minimum version 1.70)
- **CLI Framework**: clap v4.4 with derive features
- **Async Runtime**: tokio with full feature set
- **Serialization**: serde with JSON, YAML, and TOML support

### Container and Kubernetes Integration
- **Container Technology**: Apple Container runtime integration
- **Kubernetes Client**: kube-rs (v0.87) and k8s-openapi (v0.20) for API interactions
- **Network Configuration**: CNI (Container Network Interface) support with bridge and PTP configurations

### Development Tooling
- **Testing**: assert_cmd and predicates for CLI testing
- **Logging**: tracing ecosystem with JSON output support
- **Error Handling**: anyhow and thiserror for comprehensive error management

## Project Integration Points

### Container Management
- **Apple Container Provider**: Core abstraction in `src/core/apple_container.rs`
- **Kubernetes API Integration**: Client implementations in `src/core/kubernetes.rs`
- **Cluster Management**: Cluster lifecycle in `src/core/cluster.rs`

### CLI Structure
- **Command Organization**: Modular command structure in `src/cli/`
- **Configuration Commands**: Dedicated config management in `src/cli/config_cmd.rs`

### Deployment Artifacts
- **Container Images**: Build scripts and Dockerfiles for node images
- **Kubernetes Manifests**: Ready-to-deploy YAML configurations for CNI and ingress
- **Example Configurations**: Comprehensive examples for different deployment scenarios
```

This analysis reveals a well-structured Rust project focused on Kubernetes container management using Apple Container technology, with clear modular organization and comprehensive tooling support.
