# Gateway API Examples for Kina

Practical examples for using NGINX Gateway Fabric (NGF) with Kina clusters.
Covers HTTP, TCP, and UDP routing using the Kubernetes Gateway API.

## Prerequisites

### 1. Create a Kina cluster with Cilium CNI

TCPRoute/UDPRoute require `--cni cilium`; the default PTP CNI does not support
the `protocol: TCP/UDP` listener mode that NGF uses for L4 routing.

```bash
kina create arena --cni cilium
```

### 2. Install NGINX Gateway Fabric

```bash
kina install nginx-gateway-fabric --cluster arena
```

This installs NGF with host-port bindings on port 80 (HTTP) and 443 (HTTPS).

### 3. Install experimental Gateway API CRDs (for TCPRoute / UDPRoute)

TCPRoute and UDPRoute are experimental resources not included in the standard
Gateway API CRD bundle. Install the experimental CRD set:

```bash
kubectl apply -f https://raw.githubusercontent.com/kubernetes-sigs/gateway-api/v1.5.1/config/crd/experimental/gateway.networking.k8s.io_tcproutes.yaml
kubectl apply -f https://raw.githubusercontent.com/kubernetes-sigs/gateway-api/v1.5.1/config/crd/experimental/gateway.networking.k8s.io_udproutes.yaml
```

The version must match the standard CRDs the NGF addon installs (v1.5.1).
Apply only the TCPRoute/UDPRoute CRDs, not the full `experimental-install.yaml`
bundle: since v1.5.1 a `safe-upgrades.gateway.networking.k8s.io`
ValidatingAdmissionPolicy denies replacing the installed standard-channel CRDs
(gateways, httproutes, ...) with experimental-channel versions, so the full
bundle apply fails. The standard Gateway CRD already accepts `protocol: TCP/UDP`
listeners — only the route CRDs are missing (verified: listener reports
`Programmed=True` in this state).

Then enable NGF's experimental features via the CLI flag (NGF only honors the
`--gateway-api-experimental-features` argument — setting an
`ENABLE_GATEWAY_API_EXPERIMENTAL_FEATURES` env var does nothing; verified
against NGF 2.6.5, where TCPRoutes were silently ignored until the flag was
added):

```bash
kubectl patch deployment nginx-gateway -n nginx-gateway --type=json \
  -p '[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--gateway-api-experimental-features"}]'
kubectl rollout status deployment/nginx-gateway -n nginx-gateway
```

### 4. Apply TCP/UDP RBAC for NGF (kina-43)

NGF's default ClusterRole does not grant list/watch on TCPRoute/UDPRoute.
Apply the supplemental RBAC (tracked in kina-43):

```bash
# Until kina-43 lands in `kina install nginx-gateway-fabric`:
kubectl apply -f - <<'EOF'
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: nginx-gateway-experimental
rules:
- apiGroups: ["gateway.networking.k8s.io"]
  resources: ["tcproutes", "udproutes"]
  verbs: ["list", "watch"]
- apiGroups: ["gateway.networking.k8s.io"]
  resources: ["tcproutes/status", "udproutes/status"]
  verbs: ["update"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: nginx-gateway-experimental
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: nginx-gateway-experimental
subjects:
- kind: ServiceAccount
  name: nginx-gateway
  namespace: nginx-gateway
EOF
```

### 5. Configure the Gateway with TCP/UDP listeners

The `nginx-gateway-fabric` addon creates the Gateway with only an HTTP listener.
To add TCP/UDPRoute support, patch in the extra listeners and expose the ports
via `NginxProxy` hostPorts:

```bash
kubectl patch gateway nginx -n nginx-gateway --type=merge -p '
spec:
  listeners:
  - name: http
    port: 80
    protocol: HTTP
    allowedRoutes:
      namespaces:
        from: All
  - name: tcp-mc
    port: 25565
    protocol: TCP
    allowedRoutes:
      namespaces:
        from: All
      kinds:
      - group: gateway.networking.k8s.io
        kind: TCPRoute
  - name: udp-game
    port: 7777
    protocol: UDP
    allowedRoutes:
      namespaces:
        from: All
      kinds:
      - group: gateway.networking.k8s.io
        kind: UDPRoute
'
```

Then expose those ports via the NginxProxy daemonSet hostPorts:

```bash
kubectl patch nginxproxy nginx-gateway-proxy-config -n nginx-gateway \
  --type=merge -p '
spec:
  kubernetes:
    daemonSet:
      container:
        hostPorts:
        - containerPort: 80
          port: 80
        - containerPort: 443
          port: 443
        - containerPort: 25565
          port: 25565
        - containerPort: 7777
          port: 7777
'
```

## How Gateway Listeners Work

A single `Gateway` object acts as the traffic entry point for the cluster.
Each `listener` defines a protocol+port combination and the kinds of routes
allowed to attach to it:

| Listener name | Port  | Protocol | Accepts        |
|---------------|-------|----------|----------------|
| `http`        | 80    | HTTP     | HTTPRoute       |
| `tcp-mc`      | 25565 | TCP      | TCPRoute        |
| `udp-game`    | 7777  | UDP      | UDPRoute        |

Routes (HTTPRoute, TCPRoute, UDPRoute) reference the Gateway by name and
`sectionName`, and point to backend Services. NGF programs the NGINX proxy
pods accordingly.

## nip.io Local Routing

**No `/etc/hosts` needed.** [nip.io](https://nip.io) is a public wildcard DNS
service: any hostname of the form `<label>.<ip>.nip.io` resolves to `<ip>`.

```
demo.192.168.65.127.nip.io  →  192.168.65.127
myapp.10.0.0.5.nip.io       →  10.0.0.5
```

Use the node's Apple Container VM IP in your hostnames and they resolve
automatically on your Mac — no DNS configuration required.

**macOS `.local` caveat:** macOS intercepts `.local` domains for mDNS/Bonjour
resolution before consulting `/etc/hosts`. Hostnames like `myapp.local` that
you add to `/etc/hosts` will not reliably resolve. Use nip.io instead, or the
`.flame.local` name that the `container system dns` command registers for the
control-plane container (tracked in kina-40).

## Finding Your Node IP

```bash
# From kubectl (works after kina create)
kubectl get nodes -o wide
# The INTERNAL-IP column is the Apple Container VM IP

# Alternatively, from the container CLI
container list
# Look for your cluster's control-plane container; the IP is in the last column
```

For single-node clusters (`kina create` default `--workers 0`), the control-plane
node serves all workloads. Its IP is both the node IP and the cluster entry point.

## Examples

| File | What it shows |
|------|---------------|
| `http-basic.yaml` | Single Deployment + Service + HTTPRoute |
| `http-virtual-hosts.yaml` | Two apps on different hostnames, one Gateway |
| `tcp-route.yaml` | TCP service exposed via TCPRoute |
| `udp-route.yaml` | UDP service exposed via UDPRoute |
| `cross-namespace-referencegrant.yaml` | HTTPRoute in a different namespace from the backend Service |

Database-backed Gateway API demos live under `examples/demos/`:
[cnpg-app](../demos/cnpg-app/) (Phoenix app + CloudNativePG Postgres via
HTTPRoute) and [cnpg-service](../demos/cnpg-service/) (Postgres exposed to the
Mac via TCPRoute).

## Quick Start

```bash
# Deploy all examples (auto-detects node IP)
./deploy-examples.sh deploy all

# Deploy a single example
./deploy-examples.sh deploy http-basic

# Test an example
./deploy-examples.sh test http-basic

# Clean up
./deploy-examples.sh cleanup all
```

## Manual Deploy

```bash
# Get your node IP
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

# Substitute and apply
sed "s/<NODE_IP>/$NODE_IP/g" http-basic.yaml | kubectl apply -f -

# Test — should return 200
curl -s -o /dev/null -w "%{http_code}" http://demo.$NODE_IP.nip.io
```
