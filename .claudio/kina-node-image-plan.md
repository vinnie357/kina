# kina-node Image Development Plan

**Project**: Custom Kubernetes node image for Apple Container VM architecture
**Image Name**: `kina-node`
**Base Architecture**: VM-per-node with internal container runtime

## Image Architecture Design

### Core Components

**Base Layer** (Ubuntu 22.04 LTS):
- Full Linux distribution optimized for VM environment
- systemd for service management
- Network utilities and SSH access
- Apple Container VM optimizations

**Container Runtime Layer**:
- containerd (not shared with host - internal to VM)
- runc container runtime
- CNI plugins for pod networking within VM
- Container runtime configured for Kubernetes

**Kubernetes Layer**:
- kubelet (Kubernetes node agent)
- kubeadm (cluster bootstrapping)
- kubectl (CLI tool)
- Kubernetes system services and configurations

**Network & Service Layer**:
- Configure for Apple Container automatic IP assignment
- CNI configuration for internal pod networking within single VM
- Network bridge setup within VM for pod-to-pod communication
- iptables rules for Kubernetes networking
- Direct service access via VM IP address (no port mapping needed)

## Image Build Strategy

### Phase 1: Base VM Image
**Dockerfile Approach**:
```dockerfile
FROM ubuntu:22.04

# Install system dependencies
RUN apt-get update && apt-get install -y \
    systemd systemd-sysv \
    curl wget gnupg lsb-release \
    ca-certificates apt-transport-https \
    iptables iproute2 \
    openssh-server \
    && rm -rf /var/lib/apt/lists/*

# Configure systemd for VM environment
RUN systemctl enable ssh
```

### Phase 2: Container Runtime Installation
```dockerfile
# Install containerd
RUN curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
RUN echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

RUN apt-get update && apt-get install -y containerd.io
RUN mkdir -p /etc/containerd
RUN containerd config default | tee /etc/containerd/config.toml
RUN systemctl enable containerd
```

### Phase 3: Kubernetes Components
```dockerfile
# Add Kubernetes repository
RUN curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.31/deb/Release.key | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
RUN echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.31/deb/ /' | tee /etc/apt/sources.list.d/kubernetes.list

# Install Kubernetes components
RUN apt-get update && apt-get install -y kubelet kubeadm kubectl
RUN apt-mark hold kubelet kubeadm kubectl
RUN systemctl enable kubelet
```

### Phase 4: VM & Networking Configuration
```dockerfile
# Configure for Apple Container VM environment
COPY kina-node-init.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/kina-node-init.sh

# Set hostname via environment (Apple Container DNS integration)
ENV HOSTNAME=""

# Configure container runtime for Kubernetes
COPY containerd-config.toml /etc/containerd/config.toml

# Set systemd as default init
CMD ["/sbin/init"]
```

## Implementation Steps

### Step 1: Create Dockerfile and Build Scripts
- Create `kina-cli/images/kina-node/Dockerfile`
- Build configuration files for containerd, kubelet
- Initialization scripts for VM startup

### Step 2: Update kina CLI Default Image
- Change default image from `kindest/node:latest` to `kina-node:latest`
- Update cluster creation to use VM-optimized image

### Step 3: Test and Iterate
- Build initial image: `docker build -t kina-node:latest .`
- Test with Apple Container: `container run kina-node:latest`
- Verify systemd, containerd, and kubelet services start correctly

### Step 4: Cluster Formation Testing
- Test single node cluster creation
- Test multi-node cluster with Apple Container DNS
- Validate Kubernetes pod scheduling and networking

## File Structure

```
kina-cli/
├── images/
│   └── kina-node/
│       ├── Dockerfile
│       ├── kina-node-init.sh
│       ├── containerd-config.toml
│       ├── kubelet-config.yaml
│       └── build.sh
└── src/
    └── core/
        └── apple_container.rs  # Updated to use kina-node image
```

## Testing Strategy

### Basic VM Tests
1. Image builds successfully
2. Container starts and systemd initializes
3. containerd service starts and responds
4. kubelet service starts (will fail without cluster, but should attempt)

### Cluster Integration Tests
1. Single control-plane node creation
2. Multi-node cluster formation
3. Pod scheduling and networking within VMs
4. Apple Container DNS resolution between nodes
5. kubectl connectivity and cluster operations

## Success Criteria

**Phase 1 Success (Core Image)**:
- [ ] kina-node image builds without errors
- [ ] Apple Container can run the image successfully
- [ ] systemd, containerd, kubelet services start in VM
- [ ] Single-node cluster initializes with combined control-plane/worker roles

**Phase 2 Success (CLI Integration)**:
- [ ] `kina create cluster` uses kina-node image by default
- [ ] Single node cluster initializes successfully
- [ ] kubectl can connect to cluster API server at VM IP address
- [ ] kubeconfig generation works with VM IP instead of localhost

**Phase 3 Success (Addon System)**:
- [ ] `kina install nginx-ingress` command works
- [ ] nginx-ingress controller deploys and runs in single-node cluster
- [ ] Ingress services accessible via VM IP on ports 80/443
- [ ] Kubernetes pods can be scheduled and run
- [ ] Full single-node Kubernetes functionality operational

**Phase 4 Success (Production Ready)**:
- [ ] Complete addon ecosystem (CNI, metrics-server, etc.)
- [ ] Comprehensive documentation and usage examples
- [ ] Performance optimization for single-node workloads
- [ ] Ready for macOS 26 multi-node upgrade path

This plan leverages the best of kindest/node (Kubernetes components) and docker:dind (self-contained runtime) concepts while optimizing for Apple Container's VM-per-container architecture.