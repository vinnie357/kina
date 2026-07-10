# kina - Kubernetes in Apple Container

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/vinnie357/kina)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Apple Container](https://img.shields.io/badge/Apple%20Container-1.0.0%2B-blue.svg)](https://github.com/apple/container)

**kina** is a Rust CLI tool for running local Kubernetes clusters using Apple Container technology. It provides similar functionality to [kind](https://kind.sigs.k8s.io/) (Kubernetes in Docker) but is optimized for macOS systems, leveraging native Apple Container technology for improved performance and integration.

## Table of Contents

- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Command Reference](#command-reference)
- [Configuration](#configuration)
- [Apple Container Integration](#apple-container-integration)
- [CNI Support](#cni-support)
- [Development](#development)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)

## Features

- 🏗️ **Native Apple Container Integration** - Leverage macOS container technology for optimal performance
- ☸️ **Kubernetes API Compatibility** - Full Kubernetes cluster functionality with kubectl integration
- 🌐 **CNI Plugin Support** - Choose between PTP (default) and Cilium for container networking
- 🔧 **Ingress & Gateway API** - Built-in support for nginx-ingress, or Traefik with Gateway API (mutually exclusive, one controller per cluster)
- 📊 **Metrics Server** - One-command install for `kubectl top` and Horizontal Pod Autoscaling
- ⚙️ **Flexible Configuration** - TOML-based configuration with sensible defaults
- 📋 **Comprehensive CLI** - Rich command set for cluster management and operations
- 🚀 **Development Ready** - Integrated development workflow with mise task automation

## Requirements

### System Requirements
- **macOS**: 26+ (macOS 15.6 may work with limitations)
- **Apple Container**: 1.0.0+ (auto-detected and validated at startup)
- **Rust**: 1.70+ (for building from source)

### Apple Container Installation
Apple Container is **required** for kina to work. Install it first:

**Option A — Homebrew (recommended):**
```bash
brew install container
```

**Option B — Signed pkg installer:**
1. **Download**: Get the latest signed `.pkg` from [Apple Container Releases](https://github.com/apple/container/releases)
2. **Install**: Double-click the `.pkg` file and follow the installer prompts

After installing via either method:
1. **Start Service**: Run `container system start` to start the API server
2. **Verify**: Check installation with `container --version`

**Note**: kina requires Apple Container **1.0.0 or later**. The version is automatically detected and validated when kina starts. Run `kina` (no arguments) to see your kina and Apple Container versions.

### Kubernetes Tools
- `kubectl` - Kubernetes command-line tool
- `kubectx` & `kubens` - Context and namespace management (optional, managed by mise)

### Development Tools (Optional)
- [mise](https://mise.jdx.dev/) - Development environment manager with task automation

## Installation

### Option 1: mise (GitHub backend)

If you have [mise](https://mise.jdx.dev/) installed, you can install kina directly from GitHub Releases without cloning the repository:

```bash
# Install the latest 0.2.0 release via mise github backend
mise use github:vinnie357/kina@0.2.0

# Or add to your project's mise.toml:
# [tools]
# "github:vinnie357/kina" = "0.2.0"
```

The mise github backend auto-selects the `aarch64-apple-darwin` binary on macOS (Apple Silicon). No `asset_pattern` option is needed.

> **Note**: `mise install github:vinnie357/kina@0.2.0` installs the kina CLI binary only. The Apple Container runtime (macOS 26+, `brew install container`) is required separately before creating clusters.

### Option 2: From Source

```bash
# Clone the repository
git clone https://github.com/vinnie357/kina.git
cd kina

# Install using Cargo (--force required: the version string is stamped at
# build time, so cargo's default "same version, skip" check would keep the
# old binary with a stale git sha).
cargo install --path kina-cli --force

# Confirm the git sha is present in the installed binary
kina --version

# OR using mise (if installed)
mise run kina:install
```

### Option 3: Development Setup with mise

```bash
# Clone the repository
git clone https://github.com/vinnie357/kina.git
cd kina

# Set up development environment (installs Rust, tools, and dependencies)
mise run setup:dev

# Build and install
mise run install
```

### Verification

```bash
# Verify installation (shows kina and Apple Container versions)
kina

# Check Apple Container availability (REQUIRED, 1.0.0+)
mise run container:check  # If using mise
# OR manually check:
container --version
container system start  # Start the service if not running

# Optional: Check Kubernetes tools
mise run k8s:check  # If using mise
kubectl version --client
```

**⚠️ Important**: Apple Container 1.0.0+ must be available before creating clusters. kina auto-detects and validates the version at startup. Run `kina status` to see Apple Container version information.

## Quick Start

```bash
kina create kina-test --workers 2 --cni cilium --kernel-path <path>
kina install nginx-ingress --cluster kina-test
kina install demo-app --cluster kina-test
kina verify kina-test
```

> `--kernel-path` is required until `kina-8` lands auto-download.

### Create Your First Cluster

```bash
# Create a cluster with default settings
kina create my-cluster

# Export kubeconfig to connect with kubectl
kina export my-cluster --format kubeconfig --output ~/.kube/my-cluster
export KUBECONFIG=~/.kube/my-cluster

# Verify cluster is working
kubectl get nodes
```

**Advanced Options:**
```bash
# Create cluster with Cilium CNI and wait for readiness
kina create demo --cni cilium --wait 300
```

### Install an Ingress Controller and Demo App

Install an ingress controller **before** the demo app — `kina install demo-app`
auto-selects its route object based on what's installed on the cluster: a
Gateway API `HTTPRoute` if Traefik (Gateway API) is present, otherwise a
plain nginx `Ingress`. Only **one** ingress controller may run per cluster,
since both nginx-ingress and Traefik are DaemonSets binding host ports 80/443
— installing a second one is rejected with an error.

```bash
# Install nginx-ingress (manifests embedded in binary — works from any directory)
kina install nginx-ingress --cluster my-cluster

# Install demo application (creates an nginx Ingress, since nginx-ingress is installed)
kina install demo-app --cluster my-cluster

# Install metrics-server (enables `kubectl top` and HPA)
kina install metrics-server --cluster my-cluster

# Verify the cluster end-to-end
kina verify my-cluster
```

**Alternative: Traefik (Gateway API)**
```bash
# Install traefik instead of nginx-ingress (also installs the Gateway API CRDs)
kina install traefik --cluster my-cluster

# Install demo application (creates a Gateway API HTTPRoute, since traefik is installed)
kina install demo-app --cluster my-cluster
```

### Check Cluster Status

```bash
# Basic status
kina status my-cluster

# Detailed status with pods and services
kina status my-cluster --verbose
```

### Integration Test Cluster

**Option A: Using mise (if installed)**
```bash
# Create an integration test cluster with ingress and demo app
mise run test:cluster

# Validate the most recent test cluster
mise run test:cluster:validate

# Clean up all test clusters (removes clusters with 'demo-' prefix)
mise run test:cluster:cleanup
```

**Option B: Manual setup (without mise)**
```bash
# Create cluster with nginx-ingress
kina create demo-cluster --wait 300
kina install nginx-ingress --cluster demo-cluster

# Check status
kina status demo-cluster --verbose
```

The demo cluster setup creates:
- A timestamped cluster (e.g., `demo-20241228-143022`)
- nginx-ingress controller installation and configuration
- A sample web application with 2 replicas
- Ingress routing for browser/curl access
- Complete Apple Container networking setup

### Verify Your Setup

After creating your first cluster, verify everything works:

```bash
# Check cluster status
kina status my-cluster

# List all pods (should show running status)
kubectl --kubeconfig ~/.kube/my-cluster get pods -A

# Verify nodes are ready
kubectl --kubeconfig ~/.kube/my-cluster get nodes
```

**Troubleshooting**: If cluster creation fails, check:
- Apple Container CLI is available: `container --version`
- Sufficient system resources (2GB+ RAM recommended)
- Try with `--retain` flag to debug: `kina create test-cluster --retain`

## Command Reference

### Cluster Management

```bash
# Create a new cluster
kina create [NAME] [OPTIONS]
  --image TEXT           Container image (default: kindest/node:v1.36.1)
  --config FILE          Cluster configuration file
  --wait SECONDS         Wait for cluster readiness
  --retain               Retain cluster on failure
  --cni ptp|cilium       CNI plugin (default: ptp)

# Delete a cluster
kina delete [NAME]
kina delete --all      # Delete all clusters

# List clusters
kina list              # Simple list
kina list --verbose    # Detailed information

# Show cluster status
kina status [NAME] [OPTIONS]
  --verbose              Show detailed information
  --output table|yaml|json
```

### Resource Operations

```bash
# Get cluster information
kina get clusters [NAME]
kina get kubeconfig [NAME]
kina get nodes [NAME]

# Load container images
kina load IMAGE --cluster NAME

# Export configurations
kina export [NAME] [OPTIONS]
  --format kubeconfig|config
  --output FILE
```

### Addon Management

```bash
# Install addons (only one ingress controller may be installed per cluster)
kina install nginx-ingress --cluster NAME    # NGINX Ingress Controller (classic Ingress)
kina install traefik --cluster NAME          # Gateway API controller (installs Gateway API CRDs)
kina install demo-app --cluster NAME         # Demo workload; auto-selects HTTPRoute or Ingress
kina install metrics-server --cluster NAME   # Enables `kubectl top` and HPA
```

### Cluster Operations

```bash
# Approve kubelet Certificate Signing Requests
kina approve-csr [NAME]

# Configuration management
kina config show
kina config set KEY VALUE
kina config get KEY
kina config reset
kina config path
```

## Configuration

### Configuration File Location

kina uses TOML configuration files located at:
```
~/.config/kina/config.toml
```

### Default Configuration

```toml
[cluster]
default_name = "kina"
default_image = "kindest/node:v1.36.1"
default_wait_timeout = 300
data_dir = "~/.local/share/kina"
retain_on_failure = false
default_cni = "ptp"

[apple_container]
cli_path = null  # Auto-detected

[apple_container.runtime_config]
cpu_limit = null
memory_limit = "2Gi"
storage_limit = "20Gi"

[apple_container.network]
network_name = "kina"
enable_ipv6 = false
dns_servers = []

[kubernetes]
default_version = "v1.36.1"
kubectl_path = null  # Auto-detected
default_namespace = "default"
kubeconfig_dir = "~/.config/kina/kubeconfig"

[logging]
level = "info"
format = "text"
file_logging = false
log_dir = null
```

### Environment Variables

```bash
# Configuration
export KINA_CONFIG_DIR="$HOME/.config/kina"
export KINA_DATA_DIR="$HOME/.local/share/kina"

# Logging
export RUST_LOG="info"
export RUST_BACKTRACE="1"
```

## Apple Container Integration

kina leverages Apple Container technology for running Kubernetes nodes:

### Container Management
- **Native Integration**: Uses Apple Container CLI for container lifecycle
- **Resource Limits**: Configurable CPU, memory, and storage limits
- **Network Integration**: Seamless integration with macOS networking
- **DNS Support**: Automatic DNS configuration for cluster access

### Cluster Architecture
```
┌─────────────────────────────────────────┐
│               macOS Host                │
│  ┌─────────────────────────────────────┐ │
│  │        Apple Container VM           │ │
│  │  ┌─────────────────────────────────┐ │ │
│  │  │     Kubernetes Node             │ │ │
│  │  │  • kubelet                      │ │ │
│  │  │  • containerd                   │ │ │
│  │  │  • CNI (PTP/Cilium)             │ │ │
│  │  └─────────────────────────────────┘ │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## CNI Support

### PTP CNI (Default)
- **Compatibility**: Optimized for Apple Container
- **Simplicity**: Point-to-point networking with host-local IPAM
- **Performance**: Minimal overhead for single-node clusters

### Cilium CNI
- **Advanced Features**: eBPF-based networking and security
- **Requirements**: Compatible kernel modules
- **Use Cases**: Complex networking requirements and observability

Cilium full-eBPF mode requires a custom Linux kernel that kina builds (`6.18.5-kina.1`); enable it per cluster with `kina create ... --cni cilium --kernel-path <path/to/vmlinux>`. Stock kernel (PTP) clusters need nothing extra. See [docs/development/custom-kernel.md](docs/development/custom-kernel.md) for build instructions, usage, distribution, and host-kernel gotchas. (A planned future release will fetch the kernel automatically — a one-time ~32 MB download — so `--kernel-path` will not be required; today it is the only supported path.)

```bash
# Create cluster with specific CNI
kina create test-ptp --cni ptp
kina create test-cilium --cni cilium
```

## Development

### Development Environment Setup

kina uses [mise](https://mise.jdx.dev/) for development environment management and task automation. This provides consistent tooling and streamlined workflows.

```bash
# Complete development environment setup
mise run setup:dev

# Individual setup steps (if you prefer manual setup)
mise run setup                    # Install Rust components (rustfmt, clippy, cargo-audit)
mise run k8s:install              # Install kubectl, kubectx, kubens via mise
mise run container:check          # Verify Apple Container CLI availability
mise run k8s:check                # Verify Kubernetes tools installation
```

**What `mise run setup:dev` does:**
- Installs Rust toolchain components (rustfmt, clippy)
- Installs cargo-audit for security dependency scanning
- Creates kina configuration directories (`~/.config/kina`, `~/.local/share/kina`)
- Installs Kubernetes tools (kubectx, kubens) via mise package manager
- Verifies Apple Container CLI is available (1.0.0+ required)
- Checks all tool installations

### Node Image Building

kina requires custom Kubernetes node images optimized for Apple Container. These images contain the necessary components for running Kubernetes nodes in Apple Container VMs.

```bash
# Build custom kina node image
mise run image:build

# Test the built node image
mise run image:test

# Build and test in one command
mise run image:validate

# List available images
mise run image:list

# Clean up unused images
mise run image:clean
```

**Node Image Components:**
- **Base System**: Ubuntu with systemd for container orchestration
- **Container Runtime**: containerd configured for Apple Container integration
- **Kubernetes Components**: kubelet, kubeadm, kubectl (v1.36.1)
- **CNI Plugins**: PTP and Cilium support
- **Init Scripts**: Apple Container-specific initialization and networking setup

The built images are tagged as `kina/node:v1.36.1` and can be used with:
```bash
kina create my-cluster --image kina/node:v1.36.1
```

### Task Tracking

kina uses [bees](https://github.com/ctxshift/bees) for task tracking, a lightweight SQLite-backed local issue tracker. Issues are stored in the `.bees/` directory (JSONL synced via git, SQLite db local-only).

```bash
bees ready                       # Find tasks ready to work on (no blockers)
bees list                        # List all open tasks
bees show <id>                   # View task details
bees update <id> --status in_progress  # Claim a task
bees close <id>                  # Complete a task
```

See [AGENTS.md](AGENTS.md) for the full bees workflow.

### Pre-commit and Secret Scanning

`mise run pre-commit` runs formatting, linting, tests, audit, and **gitleaks secret scanning** before each commit. Gitleaks is also available standalone:

```bash
mise run gitleaks                # Run gitleaks secret scanner
```

### Common Development Tasks

```bash
# Build and install
mise run build                   # Release build
mise run dev                     # Development build
mise run test                    # Run tests
mise run kina:install            # Install kina CLI from project root
mise run pre-commit              # Format, lint, test, audit, gitleaks
mise run ci                      # Run full CI pipeline locally
mise run release                 # Build optimized release binary

# Code quality
mise run fmt                     # Format code with rustfmt
mise run lint                    # Run clippy with strict settings
mise run audit                   # Security audit with cargo-audit
mise run check                   # Check code without building
mise run gitleaks                # Secret scanning with gitleaks

# Documentation and utilities
mise run docs                    # Generate and open documentation
mise run clean                   # Clean build artifacts
mise run watch                   # Watch files and rebuild on changes
mise run bench                   # Run benchmarks

# CLI testing
mise run kina -- create test     # Run kina with arguments (release build)
mise run kina:dev -- --help      # Run kina in dev mode (faster build)
mise run test:cli                # Basic CLI functionality tests

# Available tasks
mise tasks                       # List all available mise tasks

# Integration testing workflows
mise run test:cluster            # Create test cluster with ingress and demo app
mise run test:cluster:validate   # Validate most recent test cluster
mise run test:cluster:cleanup    # Clean up all test clusters
```

### Project Structure

```
kina/
├── kina-cli/                   # Main CLI application
│   ├── src/
│   │   ├── cli/               # Command implementations
│   │   ├── config/            # Configuration management
│   │   ├── core/              # Core cluster management
│   │   └── main.rs            # Application entry point
│   ├── tests/                 # Integration tests
│   ├── manifests/             # Kubernetes manifests
│   ├── images/                # Custom node image Dockerfile
│   └── Cargo.toml
├── scripts/                    # Extracted mise task scripts (Nushell)
├── docs/                       # Research, planning, and development docs
├── .bees/                      # Task tracking (bees)
├── CLAUDE.md                   # AI assistant context
├── AGENTS.md                   # Bees workflow for AI agents
├── mise.toml                   # Development automation
├── Cargo.toml                  # Workspace configuration
└── README.md
```

## Troubleshooting

### Common Issues

#### Apple Container Not Found
```bash
# Check Apple Container installation
container --version

# Start the Apple Container service if needed
container system start

# Check if service is running
container system status

# Verify PATH configuration
echo $PATH | grep container
```

**Solution**: If Apple Container is not found, install it from [Apple Container Releases](https://github.com/apple/container/releases). If installed but not working, restart the service with `container system restart`.

#### Cluster Creation Fails
```bash
# Check cluster status
kina status my-cluster --verbose

# Enable verbose logging
RUST_LOG=debug kina create my-cluster --retain

# Manual cleanup
kina delete my-cluster
```

#### Kubeconfig Issues
```bash
# Check kubeconfig location
kina config path
ls ~/.kube/

# Regenerate kubeconfig
kina export my-cluster --output ~/.kube/my-cluster
export KUBECONFIG=~/.kube/my-cluster
```

#### CNI Pod Failures
```bash
# Check CNI pod status
kubectl get pods -n kube-system -l k8s-app=cilium

# Approve pending CSRs
kina approve-csr my-cluster

# Check node readiness
kubectl get nodes
```

### Debug Commands

```bash
# Comprehensive cluster status
kina status my-cluster --verbose --output yaml

# Container inspection
container list
container inspect CONTAINER_NAME

# Kubernetes debugging
kubectl get events --sort-by='.lastTimestamp'
kubectl describe nodes
```

### Getting Help

- **GitHub Issues**: [Report bugs and feature requests](https://github.com/vinnie357/kina/issues)
- **Documentation**: [Project documentation](https://github.com/vinnie357/kina/docs)
- **Community**: [Discussions and support](https://github.com/vinnie357/kina/discussions)

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. **Fork and Clone**: Fork the repository and clone your fork
2. **Setup Environment**: Run `mise run setup:dev` for complete setup
3. **Find Work**: Run `bees ready` to find available tasks
4. **Create Branch**: Create a feature branch for your changes
5. **Develop**: Make changes with comprehensive tests
6. **Quality Checks**: Run `mise run pre-commit` before committing (includes gitleaks)
7. **Submit PR**: Create a pull request with clear description

### Code Quality

- **Formatting**: `mise run fmt` (rustfmt)
- **Linting**: `mise run lint` (clippy with strict settings)
- **Testing**: `mise run test` (comprehensive test suite)
- **Security**: `mise run audit` (cargo-audit dependency scanning)

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

**Note**: kina is in active development. While functional, some features are still being implemented. See the [project roadmap](https://github.com/vinnie357/kina/projects) for current status and planned features.