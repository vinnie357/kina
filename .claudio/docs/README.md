# Kina: Kubernetes in Apple Container (CLI)

## Project Overview

Kina is a macOS-native CLI tool designed to provide a lightweight, efficient alternative to Kind (Kubernetes in Docker) for local Kubernetes development. Leveraging Apple Container technology, Kina enables developers to create and manage single-node Kubernetes clusters directly on macOS.

## Technology Stack

- **Primary Language**: Rust
- **Target Platform**: macOS 15.6+
- **Container Runtime**: Apple Container
- **Kubernetes Compatibility**: Targets kubectl, kubectx, kubens, k9s integrations

## Quick Start Guide

### Prerequisites

- macOS 15.6 or later
- Apple Container runtime
- Basic Kubernetes knowledge

### Installation (Planned)

```bash
# Future installation method
brew install kina  # Placeholder for future Homebrew installation
```

## Essential Commands

### Cluster Management

```bash
# Create a new single-node Kubernetes cluster
kina create cluster

# List existing clusters
kina list clusters

# Delete a cluster
kina delete cluster [cluster-name]
```

### Development Workflows

```bash
# Initialize a new Kina project
kina init

# Configure cluster settings
kina config set [options]

# Switch between clusters
kina use-context [cluster-name]
```

## Development Status

- **Current Phase**: Planning and Requirements Gathering
- **Implementation**: Not yet started
- **Target Features**:
  - Single-node Kubernetes cluster management
  - Apple Container integration
  - Seamless kubectl workflow

## Planned Features

1. Single-node Kubernetes cluster creation
2. Cluster lifecycle management
3. Apple Container runtime integration
4. Compatibility with existing Kubernetes tools
5. Lightweight and fast cluster provisioning

## Development Roadmap

### Phase 1: Project Initialization
- [x] Project structure design
- [x] Technology stack selection
- [ ] Rust project setup
- [ ] Apple Container research

### Phase 2: Core Implementation
- [ ] Basic CLI structure
- [ ] Cluster creation logic
- [ ] Container runtime integration
- [ ] Testing framework implementation

## Contributing

Contributions are welcome! Please see our contribution guidelines for more information.

## License

[License information to be added]

## Support

For issues and support, please file a GitHub issue in the project repository.