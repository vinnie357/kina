# Kina Demo Apps

Fun, arm64-verified workloads for kina clusters. All demos run on single-node
or multi-node clusters and include both nginx-ingress (Ingress) and
Gateway API (HTTPRoute/TCPRoute/UDPRoute) routing manifests.

## nip.io Local Routing

**No `/etc/hosts` edits required.** These demos use [nip.io](https://nip.io) —
a public wildcard DNS service that maps `<label>.<ip>.nip.io` to `<ip>`:

```
minecraft.192.168.65.127.nip.io  →  192.168.65.127
doom.10.0.0.5.nip.io             →  10.0.0.5
```

**Finding your node IP:**

```bash
kubectl get nodes -o wide
# INTERNAL-IP column is the Apple Container VM IP

# Or via the container CLI
container list
# Look for your cluster's control-plane container
```

**macOS `.local` caveat (kina-40):** macOS intercepts `.local` domains via
mDNS/Bonjour before consulting `/etc/hosts`. Hostnames like `doom.local` added
to `/etc/hosts` will not reliably resolve on macOS. Use nip.io instead.

## Single-Node Default

`kina create` defaults to `--workers 0` — a single-node cluster where the
control-plane also runs workloads. All demos work on single-node clusters.
For multi-node clusters, add `--workers N` to spread pods across dedicated
worker nodes.

## Custom arm64 Images (kina-42)

Demos with custom images (js-dos-doom, kube-doom, noVNC) require building
arm64 images and pushing them to a registry accessible from inside the cluster.
The recommended approach is to use an in-cluster registry (kina-42 tracks the
`kina install registry` addon):

```bash
# Once kina-42 lands:
kina install registry --cluster <cluster>
# Then build + push inside the cluster or via the node VM
```

Until kina-42 lands, build images on your Mac and load them into the node
containers directly. See each demo's README for instructions.

## Demo Index

| Demo | Protocol | Port | Routing | Status |
|------|----------|------|---------|--------|
| [minecraft](minecraft/) | TCP | 25565 | TCPRoute + TransportServer | Public image (itzg/minecraft-server) |
| [ut99](ut99/) | UDP | 7777 | UDPRoute | Public image (phasecorex/ut99-server) |
| [js-dos-doom](js-dos-doom/) | HTTP | 80 | HTTPRoute + Ingress | Custom arm64 image required |
| [kube-doom](kube-doom/) | HTTP (noVNC) | 80→6080 | HTTPRoute + Ingress | Custom arm64 image required; **DESTRUCTIVE** |
| [cnpg-app](cnpg-app/) | HTTP | 80 | HTTPRoute | HA 3-instance CNPG + analytics cluster, read/write split, failover demo, table browser; needs CNPG operator + StorageClass |
| [cnpg-service](cnpg-service/) | TCP | 5432 | TCPRoute (NodePort fallback) | Postgres as a service; needs CNPG operator + StorageClass |

## Routing Prerequisites

### nginx-ingress (Ingress manifests)

```bash
kina install nginx-ingress --cluster <cluster>
```

For L4 demos (minecraft TCP, ut99 UDP), nginx-ingress uses `TransportServer`
resources (nginx.org/v1), which are included in the demo manifests.

### Gateway API (gatewayapi manifests)

```bash
kina install nginx-gateway-fabric --cluster <cluster>
```

For L4 TCPRoute/UDPRoute, see `examples/gatewayapi/README.md` for the extra
experimental CRDs and kina-43 RBAC setup.

### CloudNativePG (cnpg-* demos)

The cnpg demos additionally need a default StorageClass (kina clusters ship
without one) and the CNPG operator:

```bash
kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.36/deploy/local-path-storage.yaml
kubectl patch storageclass local-path \
  -p '{"metadata": {"annotations": {"storageclass.kubernetes.io/is-default-class": "true"}}}'

kubectl apply --server-side -f \
  https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/release-1.30/releases/cnpg-1.30.0.yaml
```

kina-46 tracks making the storage provisioner a proper `kina install` addon.
