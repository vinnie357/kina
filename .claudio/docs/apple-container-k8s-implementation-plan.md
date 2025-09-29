# Apple Container CLI Kubernetes Node Implementation Plan

## Executive Summary

This document outlines the plan to create a Kubernetes node implementation using Apple's native container CLI, similar to KIND's `kindest/node` but optimized for macOS containerization framework.

## Background & Analysis

### Apple Container CLI Capabilities
- Native macOS containerization without Docker Desktop
- `--virtualization` flag enables nested container execution
- OCI-compliant with proper VM-like isolation
- Direct integration with macOS kernel features

### KIND Architecture Lessons
From analyzing `kubernetes-sigs/kind`:
- **Base Image**: Debian with systemd as init system
- **Container Runtime**: Containerd (v2.1.4+) instead of Docker daemon
- **Kubernetes Components**: kubelet, kubeadm, kubectl, CNI plugins
- **Key Insight**: Systemd manages services inside containers for proper orchestration
- **Bootstrap Process**: Custom entrypoint initializes node with kubeadm

### Proof of Concept Success
Successfully demonstrated nested containers using:
```bash
container run --name dind-vm --virtualization --detach --platform linux/amd64 docker:dind
container exec dind-vm docker run --rm hello-world
```

## Implementation Plan

### Phase 1: Base Image Creation

#### 1.1 Create Custom Dockerfile
**File**: `Dockerfile.apple-k8s-node`

**Base Components**:
- Ubuntu 22.04 or Debian Bookworm (ARM64 for Apple Silicon)
- Systemd configuration for container execution
- Essential utilities and dependencies

**Key Systemd Modifications**:
```dockerfile
ENV container=docker
STOPSIGNAL SIGRTMIN+3
# Remove unnecessary services
# Configure journald for container mode
# Set proper cgroup configuration
```

#### 1.2 Install Container Runtime
**Containerd Installation**:
- Build containerd v2.1.4+ from source
- Configure with proper OCI runtime (runc v1.3.0+)
- Set up CNI plugins (v1.7.1+)
- Configure fuse-overlayfs for advanced storage

**Configuration Files**:
- `/etc/containerd/config.toml` - Containerd configuration
- `/etc/systemd/system/containerd.service` - Systemd service
- CNI network configurations

#### 1.3 Kubernetes Components
**Install Components**:
- kubelet (target K8s version)
- kubeadm (cluster bootstrapping)
- kubectl (client tools)
- crictl (container runtime interface)

**Configuration**:
- Kubelet configuration for containerd CRI
- Kubeadm cluster configuration templates
- RBAC and networking policies

### Phase 2: Entrypoint & Bootstrap

#### 2.1 Custom Entrypoint Script
**File**: `/usr/local/bin/entrypoint`

**Responsibilities**:
- Initialize systemd properly
- Start containerd service
- Configure networking
- Prepare for kubeadm init/join
- Handle graceful shutdown

#### 2.2 Node Initialization Scripts
**Bootstrap Process**:
- Pre-flight checks for container environment
- Network configuration (CNI setup)
- Certificate and token management
- Kubeadm configuration generation

### Phase 3: Apple Container Integration

#### 3.1 Build Process
**Build Command**:
```bash
container build -t apple-k8s-node:latest -f Dockerfile.apple-k8s-node .
```

#### 3.2 Node Startup Configuration
**Run Command Template**:
```bash
container run \
  --name k8s-control-plane \
  --virtualization \
  --detach \
  --platform linux/arm64 \
  --mount type=tmpfs,target=/tmp \
  --mount type=tmpfs,target=/run \
  --mount type=tmpfs,target=/run/lock \
  --volume /sys/fs/cgroup:/sys/fs/cgroup:ro \
  --publish 6443:6443 \
  --publish 2379-2380:2379-2380 \
  --publish 10250:10250 \
  --publish 10259:10259 \
  --publish 10257:10257 \
  apple-k8s-node:latest
```

### Phase 4: Cluster Management

#### 4.1 Control Plane Setup
**Initialize Cluster**:
```bash
container exec k8s-control-plane kubeadm init \
  --pod-network-cidr=10.244.0.0/16 \
  --service-cidr=10.96.0.0/12
```

#### 4.2 Worker Node Support
**Multi-node Cluster**:
- Create additional worker node containers
- Implement join token management
- Configure inter-node networking

#### 4.3 Networking Integration
**CNI Configuration**:
- Flannel or Calico for pod networking
- Service mesh compatibility
- Load balancer integration with macOS

### Phase 5: Tool Integration

#### 5.1 CLI Wrapper Tool
**Create**: `apple-k8s` CLI tool

**Features**:
- Cluster lifecycle management (create/delete)
- Image loading (equivalent to `kind load docker-image`)
- Configuration management
- Log aggregation

#### 5.2 Configuration Management
**Cluster Configuration**:
```yaml
# apple-k8s-config.yaml
apiVersion: apple-k8s.dev/v1alpha1
kind: Cluster
metadata:
  name: dev-cluster
spec:
  nodes:
  - role: control-plane
    image: apple-k8s-node:v1.29.0
    extraMounts:
    - hostPath: /Users/user/code
      containerPath: /code
  - role: worker
    replicas: 2
```

### Phase 6: Testing & Validation

#### 6.1 Conformance Testing
- Kubernetes conformance test suite
- CNI plugin compatibility
- Storage driver testing
- Multi-node cluster validation

#### 6.2 Performance Benchmarking
- Container startup time comparison with KIND
- Resource utilization analysis
- Network performance testing

## Technical Requirements

### System Dependencies
- macOS with Apple Container CLI installed
- Sufficient resources (8GB+ RAM recommended)
- Kernel support for nested virtualization

### Image Components
- **Base**: Ubuntu 22.04 ARM64
- **Container Runtime**: containerd 2.1.4+
- **Kubernetes**: v1.29.0+
- **CNI**: Flannel or Calico
- **Storage**: fuse-overlayfs for advanced features

### Network Architecture
- Container-to-container networking via Apple's framework
- Port forwarding for Kubernetes API access
- Service mesh integration capabilities

## Success Criteria

### Functional Requirements
1. ✅ Single-node Kubernetes cluster startup
2. ✅ Pod scheduling and execution
3. ✅ Service discovery and networking
4. ✅ Persistent volume support
5. ✅ Multi-node cluster support

### Performance Requirements
- Cluster startup < 60 seconds
- Resource overhead < 20% compared to KIND
- Native macOS integration advantages

### Compatibility Requirements
- Standard Kubernetes API compatibility
- Helm chart deployment support
- kubectl command compatibility
- CI/CD pipeline integration

## Implementation Timeline

**Week 1-2**: Phase 1 & 2 (Base image and entrypoint)
**Week 3**: Phase 3 (Apple Container integration)
**Week 4**: Phase 4 (Cluster management)
**Week 5**: Phase 5 (Tool integration)
**Week 6**: Phase 6 (Testing and validation)

## Risk Mitigation

### Technical Risks
- **Nested virtualization limitations**: Validate early with complex workloads
- **Systemd compatibility**: Test thoroughly with Apple's containerization
- **Network complexity**: Plan for macOS-specific networking challenges

### Mitigation Strategies
- Incremental development with frequent testing
- Fallback to Docker-in-Docker approach if needed
- Community engagement for testing and feedback

## Next Steps

1. **Immediate**: Create base Dockerfile and test systemd functionality
2. **Short-term**: Implement containerd integration and basic Kubernetes components
3. **Medium-term**: Build complete cluster management capabilities
4. **Long-term**: Optimize performance and create production-ready tooling

---

*This implementation plan leverages Apple's native containerization framework to create a Kubernetes development environment that's optimized for macOS, eliminating Docker Desktop dependencies while maintaining full Kubernetes compatibility.*