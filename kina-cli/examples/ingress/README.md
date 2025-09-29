# NGINX Ingress Examples for Kina

This directory contains practical examples of using NGINX Ingress Controller with Kina clusters. These examples demonstrate common ingress patterns and configurations.

## Prerequisites

1. **Running Kina cluster with nginx-ingress**:
   ```bash
   # Create a cluster (if not already created)
   kina create test-cluster

   # Install nginx-ingress controller
   kina install nginx-ingress --cluster test-cluster
   ```

2. **Verify nginx-ingress is running**:
   ```bash
   kubectl --kubeconfig ~/.kube/test-cluster get pods -n nginx-ingress
   ```

## Examples Overview

### 1. Basic Web App (`basic-web-app.yaml`)

**Purpose**: Simple single-service web application with host-based routing.

**Features**:
- Single nginx deployment with custom HTML content
- ClusterIP service
- Simple ingress rule with host-based routing
- Custom HTML served from ConfigMap

**Deploy**:
```bash
kubectl --kubeconfig ~/.kube/test-cluster apply -f basic-web-app.yaml
```

**Test**:
```bash
# From within a pod (since kina uses container networking)
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: myapp.local" http://192.168.64.178
```

**DNS Setup** (optional):
Add to your `/etc/hosts` for local testing:
```
192.168.64.178 myapp.local
```

### 2. Multi-Service Routing (`multi-service-routing.yaml`)

**Purpose**: Demonstrates path-based routing to multiple services behind a single host.

**Features**:
- Multiple deployments (frontend, API, admin)
- Path-based routing (`/app`, `/api`, `/admin`)
- URL rewriting annotations
- Different content types (HTML, JSON)

**Deploy**:
```bash
kubectl --kubeconfig ~/.kube/test-cluster apply -f multi-service-routing.yaml
```

**Test**:
```bash
# Test different paths
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: platform.local" http://192.168.64.178/app
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: platform.local" http://192.168.64.178/api
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: platform.local" http://192.168.64.178/admin
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: platform.local" http://192.168.64.178/api/health.html
```

### 3. Virtual Hosts (`virtual-hosts.yaml`)

**Purpose**: Multiple applications using different hostnames on the same ingress.

**Features**:
- Host-based routing with multiple domains
- Different applications per host
- Shared ingress configuration
- Custom styling per application

**Deploy**:
```bash
kubectl --kubeconfig ~/.kube/test-cluster apply -f virtual-hosts.yaml
```

**Test**:
```bash
# Test different hosts
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: webapp.local" http://192.168.64.178
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: api.local" http://192.168.64.178
kubectl --kubeconfig ~/.kube/test-cluster exec -it <any-pod> -- curl -H "Host: blog.local" http://192.168.64.178
```

## Common Patterns

### Testing with Container Networking

Since Kina uses Apple Container networking, the easiest way to test ingress is from within the cluster:

1. **Deploy a test pod**:
   ```bash
   kubectl --kubeconfig ~/.kube/test-cluster run test-pod --image=nginx:alpine --rm -it -- sh
   ```

2. **Test from within the pod**:
   ```bash
   # Inside the pod
   curl -H "Host: myapp.local" http://192.168.64.178
   ```

### Container DNS Integration

Kina automatically configures DNS for test domains:
```bash
# Check configured DNS entries
container system dns list
```

### Finding Your Cluster IP

```bash
# Get the cluster container IP
container list
# Look for your cluster name (e.g., test-cluster-control-plane)
```

### Common Troubleshooting

1. **502 Bad Gateway**: Usually indicates service port mismatch
   ```bash
   # Check service endpoints
   kubectl --kubeconfig ~/.kube/test-cluster get endpoints

   # Check service configuration
   kubectl --kubeconfig ~/.kube/test-cluster describe svc <service-name>
   ```

2. **404 Not Found**: Ingress is working but path/content not found
   ```bash
   # Check ingress configuration
   kubectl --kubeconfig ~/.kube/test-cluster describe ingress <ingress-name>
   ```

3. **Connection refused**: nginx-ingress controller not running
   ```bash
   # Check ingress controller status
   kubectl --kubeconfig ~/.kube/test-cluster get pods -n nginx-ingress
   ```

## NGINX Ingress Annotations

Common annotations used in these examples:

- `nginx.org/ssl-redirect: "false"` - Disable automatic HTTPS redirect
- `nginx.org/server-tokens: "false"` - Hide nginx version in headers
- `nginx.org/rewrites` - URL rewriting rules
- `nginx.org/server-snippets` - Custom nginx configuration

For more annotations, see the [NGINX Ingress Controller documentation](https://docs.nginx.com/nginx-ingress-controller/configuration/ingress-resources/advanced-configuration-with-annotations/).

## Clean Up

To remove all examples:
```bash
kubectl --kubeconfig ~/.kube/test-cluster delete -f basic-web-app.yaml
kubectl --kubeconfig ~/.kube/test-cluster delete -f multi-service-routing.yaml
kubectl --kubeconfig ~/.kube/test-cluster delete -f virtual-hosts.yaml
```

## Advanced Examples

For more advanced examples including:
- SSL/TLS termination
- Rate limiting
- Authentication
- Load balancing strategies

Check the official [NGINX Ingress examples repository](https://github.com/nginxinc/kubernetes-ingress/tree/main/examples).