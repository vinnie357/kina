# kube-doom Demo

Kill Kubernetes pods by playing Doom. Each monster in the game represents a
running pod. Shooting a monster deletes the corresponding pod.

Based on [storax/kubedoom](https://github.com/storax/kubedoom) with arm64
support from [vinnie357/pi-kube-doom](https://github.com/vinnie357/pi-kube-doom).

---

## WARNINGS — Read Before Deploying

### DESTRUCTIVE

kube-doom **deletes real pods**. Every monster you kill triggers
`kubectl delete pod` against your cluster. Do NOT deploy in a cluster running
workloads you care about. Use a dedicated kina cluster for this demo.

### CVE-2025-10202 — Broad RBAC + Default VNC Password

The kubedoom RBAC grants **cluster-wide pod delete** permissions. Combined with
a default, unauthenticated VNC session (no password set), this creates a
significant security exposure:

- **RBAC**: `ClusterRole kubedoom-pods` allows `list/get/watch/delete` on all
  pods across all namespaces. Any process that can exec into the pod can delete
  any pod in the cluster.
- **Default VNC**: noVNC has no authentication by default. The service is
  ClusterIP only — but if you change it to NodePort or LoadBalancer, the VNC
  session becomes network-accessible without a password.

**Mitigations enforced in these manifests:**
- Service is `ClusterIP` only (no NodePort, no LoadBalancer).
- Access is via HTTPRoute/Ingress only (restricts to HTTP path; VNC protocol
  is not directly exposed).
- Tear down the namespace immediately after your demo session.

**DO NOT expose this demo on a shared network or leave it running unattended.**

---

## arm64 Build Required

Both containers in the pod require custom arm64 images. Reference fork:
[vinnie357/pi-kube-doom](https://github.com/vinnie357/pi-kube-doom) (see PR #1
for the arm64 patch).

There are no public arm64 images for these; you must build them.

### Build kube-doom (pi-kube-doom arm64)

```bash
git clone https://github.com/vinnie357/pi-kube-doom
cd pi-kube-doom
container build -t pi-kube-doom:arm64 .

# Push to in-cluster registry (kina-42)
container image push <REGISTRY_IP>:5000/pi-kube-doom:arm64
```

### Build noVNC (websockify)

```bash
cd kina-cli/examples/demos/kube-doom/
container build -t novnc-websockify:arm64 -f Dockerfile.novnc .

# Push to in-cluster registry
container image push <REGISTRY_IP>:5000/novnc-websockify:arm64
```

## Prerequisites

- Running kina cluster (`kina create demo-cluster`) — use a **dedicated** cluster
- Either `kina install nginx-gateway-fabric` or `kina install nginx-ingress`
- Custom arm64 images built and pushed (see above)
- `NODE_IP` and `REGISTRY_IP` from your cluster

## Deploy

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

# Gateway API variant
sed "s|REGISTRY/pi-kube-doom:arm64|<REGISTRY_IP>:5000/pi-kube-doom:arm64|g; \
     s|REGISTRY/novnc-websockify:arm64|<REGISTRY_IP>:5000/novnc-websockify:arm64|g; \
     s/<NODE_IP>/$NODE_IP/g" gatewayapi.yaml | kubectl apply -f -

# nginx-ingress variant
sed "s|REGISTRY/pi-kube-doom:arm64|<REGISTRY_IP>:5000/pi-kube-doom:arm64|g; \
     s|REGISTRY/novnc-websockify:arm64|<REGISTRY_IP>:5000/novnc-websockify:arm64|g; \
     s/<NODE_IP>/$NODE_IP/g" ingress.yaml | kubectl apply -f -
```

## Access

Open in your browser:

```
http://doom-killer.<NODE_IP>.nip.io
```

The noVNC web client appears. Click "Connect" and the Doom game starts.
Each monster represents a running pod. Shoot them to delete pods.

**WebSocket note (nginx-ingress):** The noVNC client uses WebSocket (`ws://`).
The nginx-ingress Ingress annotation `nginx.org/websocket-services: kubedoom-web`
is required for the proxy to upgrade the connection. See `ingress.yaml`.

## Watch the Carnage

```bash
# Watch pods being deleted in real time
kubectl get pods -A -w
```

## Teardown

```bash
kubectl delete namespace kubedoom

# ClusterRole and ClusterRoleBinding are cluster-scoped, delete separately:
kubectl delete clusterrole kubedoom-pods
kubectl delete clusterrolebinding kubedoom-pods
```

Verify nothing is left:

```bash
kubectl get clusterrole kubedoom-pods 2>&1 | grep -q "not found" && echo "RBAC cleaned up"
```

## Notes

- `kubedoom` uses `KUBECONFIG` auto-mounted via the service account token.
- The noVNC sidecar (`novnc` container) connects to the VNC server on port 5900
  inside the same pod and exposes a WebSocket HTTP server on port 6080.
- The Service routes port 80 to the noVNC container's port 6080.
- The kubedoom container (port 5900) is not exposed externally.
