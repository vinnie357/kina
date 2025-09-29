# kina Single-Node Cluster Workflow

**Updated**: 2025-01-20
**Architecture**: Apple Container VM-per-cluster with single-node design

## Architecture Overview

### Single-Node Design Rationale

Due to Apple Container VM communication limitations until macOS 26, kina implements a **single-node cluster architecture** where each cluster runs in a dedicated VM with combined control-plane and worker roles.

### Key Benefits

- **Simplified Networking**: No inter-VM communication required
- **Direct Service Access**: Services accessible via VM IP address without port mapping
- **Resource Efficiency**: Single VM per cluster reduces overhead
- **Full Kubernetes Features**: Complete Kubernetes functionality within single node
- **Future Upgrade Path**: Ready for multi-node expansion in macOS 26

## Core Workflow

### 1. Cluster Creation

```bash
# Create single-node cluster
kina create my-cluster

# Create with specific image version
kina create my-cluster --image kindest/node:v1.31.0

# Create with custom configuration
kina create my-cluster --config cluster-config.yaml
```

**What happens:**
- Apple Container VM created with `my-cluster-control-plane` name
- VM gets automatic IP address (e.g., 192.168.64.5)
- Kubernetes cluster initialized with combined control-plane/worker roles
- kubeconfig generated pointing to `https://<vm-ip>:6443`

### 2. Addon Installation

```bash
# Install NGINX Ingress Controller (nginx.org - recommended)
kina install nginx-ingress --cluster my-cluster

# Install specific version
kina install nginx-ingress --cluster my-cluster --version 5.1.1

# Install other addons (future)
kina install metrics-server --cluster my-cluster
kina install cni --cluster my-cluster
```

**What happens:**
- Downloads official manifests from addon repositories
- Applies manifests to cluster using kubectl
- Configures addons for single-node operation with hostNetwork

### 3. Service Access

```bash
# Get cluster info
kina get clusters

# Get VM IP address
kina get kubeconfig my-cluster

# Access services directly via VM IP
curl http://192.168.64.5/      # HTTP ingress
curl https://192.168.64.5/     # HTTPS ingress
kubectl --kubeconfig ~/.kina/my-cluster/kubeconfig get nodes
```

## Service Architecture

### Direct VM IP Access

| Service Type | Access Method | Example |
|--------------|---------------|---------|
| Kubernetes API | `https://<vm-ip>:6443` | `https://192.168.64.5:6443` |
| HTTP Ingress | `http://<vm-ip>:80` | `http://192.168.64.5/` |
| HTTPS Ingress | `https://<vm-ip>:443` | `https://192.168.64.5/` |
| NodePort Services | `<vm-ip>:<nodeport>` | `192.168.64.5:30080` |
| LoadBalancer (via MetalLB) | `<vm-ip>:<port>` | `192.168.64.5:8080` |

### No Port Mapping Required

Unlike Docker-based solutions, Apple Container VMs have dedicated IP addresses, eliminating the need for:
- Port mapping (`-p 80:80`)
- Host networking conflicts
- Complex networking configuration

## Addon System Design

### Modular Installation

```bash
# Available addons
kina install nginx-ingress    # NGINX Inc. ingress controller
kina install ingress-nginx    # Community ingress controller (future)
kina install cni              # Container Network Interface (future)
kina install coredns          # CoreDNS (future)
kina install metrics-server   # Metrics server (future)
```

### Addon Configuration

**nginx-ingress (Recommended)**:
- Uses official nginx/nginx-ingress:5.1.1 image
- Configured with `hostNetwork: true` for single-node
- Binds directly to VM ports 80/443
- Includes proper RBAC and IngressClass setup
- Supports TLS passthrough and snippets

### Version Management

```bash
# List available versions
kina install nginx-ingress --help

# Install specific version
kina install nginx-ingress --version 5.0.0

# Use custom configuration
kina install nginx-ingress --config my-nginx-config.yaml
```

## Example Application Deployment

### 1. Create Cluster and Install Ingress

```bash
# Create cluster
kina create demo-cluster

# Install ingress controller
kina install nginx-ingress --cluster demo-cluster

# Verify cluster
kubectl get nodes
kubectl get pods -n nginx-ingress
```

### 2. Deploy Sample Application

```yaml
# demo-app.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: demo-app
spec:
  replicas: 2
  selector:
    matchLabels:
      app: demo-app
  template:
    metadata:
      labels:
        app: demo-app
    spec:
      containers:
      - name: demo-app
        image: nginx:alpine
        ports:
        - containerPort: 80
---
apiVersion: v1
kind: Service
metadata:
  name: demo-app-service
spec:
  selector:
    app: demo-app
  ports:
  - port: 80
    targetPort: 80
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: demo-app-ingress
spec:
  ingressClassName: nginx
  rules:
  - host: demo.local
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: demo-app-service
            port:
              number: 80
```

### 3. Access Application

```bash
# Apply manifests
kubectl apply -f demo-app.yaml

# Get VM IP
VM_IP=$(kina get clusters demo-cluster --format json | jq -r '.ip_address')

# Access via VM IP
curl -H "Host: demo.local" http://$VM_IP/

# Or add to /etc/hosts
echo "$VM_IP demo.local" >> /etc/hosts
curl http://demo.local/
```

## Development Workflow

### Quick Development Setup

```bash
# 1. Create development cluster
kina create dev

# 2. Install essential addons
kina install nginx-ingress --cluster dev
kina install metrics-server --cluster dev

# 3. Set kubeconfig
export KUBECONFIG=~/.kina/dev/kubeconfig

# 4. Develop and deploy
kubectl apply -f my-app.yaml

# 5. Access services via VM IP
kina get clusters dev  # Get VM IP
```

### Multi-Environment Management

```bash
# Different environments
kina create staging
kina create production

# Environment-specific addons
kina install nginx-ingress --cluster staging --version 5.1.1
kina install nginx-ingress --cluster production --version 5.0.0

# Switch between environments
export KUBECONFIG=~/.kina/staging/kubeconfig
export KUBECONFIG=~/.kina/production/kubeconfig
```

## Migration from kind

### Command Equivalence

| kind command | kina equivalent |
|-------------|-----------------|
| `kind create cluster` | `kina create cluster` |
| `kind delete cluster` | `kina delete cluster` |
| `kind get clusters` | `kina list` |
| `kind load docker-image` | `kina load` |
| `kind export kubeconfig` | `kina get kubeconfig` |
| Manual ingress setup | `kina install nginx-ingress` |

### Key Differences

1. **No port mapping**: Services accessible directly via VM IP
2. **Single-node only**: Until macOS 26 multi-VM support
3. **Modular addons**: Explicit addon installation vs bundled
4. **VM networking**: Each cluster gets dedicated IP address
5. **Native macOS**: No Docker Desktop dependency

## Future Enhancements (macOS 26+)

### Multi-Node Support

When Apple Container enables VM-to-VM communication:

```bash
# Future multi-node syntax
kina create production --nodes 3 --workers 2

# Node-specific addon installation
kina install nginx-ingress --cluster production --node control-plane-1
```

### Enhanced Networking

- Inter-cluster communication
- Load balancing across multiple VMs
- Advanced CNI configurations
- Service mesh support

This single-node architecture provides a solid foundation for local Kubernetes development while maintaining compatibility for future multi-node expansion.