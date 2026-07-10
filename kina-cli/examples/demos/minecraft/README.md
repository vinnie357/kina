# Minecraft Demo

Runs a Minecraft Java Edition server in the cluster and exposes TCP port 25565
via TCPRoute (Gateway API) or TransportServer (nginx-ingress).

Image: `itzg/minecraft-server` (public, arm64-native)

## Prerequisites

- Running kina cluster with `--cni cilium` (required for L4 routing)
- Either `kina install nginx-gateway-fabric` (for Gateway API) or
  `kina install nginx-ingress` (for nginx-ingress TransportServer)
- For Gateway API: experimental CRDs + kina-43 RBAC (see `examples/gatewayapi/README.md`)

## Deploy

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

# Gateway API variant
kubectl apply -f gatewayapi.yaml

# nginx-ingress variant
kubectl apply -f ingress.yaml
```

## Connect

Connect with any **Minecraft Java Edition** client (1.20+) to:

```
Host: <NODE_IP>
Port: 25565
```

Or, if your cluster uses the Gateway API TCPRoute on port 25565:

```
# From your Mac's terminal — verify the port is reachable
nc -zv <NODE_IP> 25565
```

The server may take 60-90 seconds to start (JVM startup + world generation).
Watch the logs:

```bash
kubectl logs -n games deploy/minecraft -f
```

Look for `Done!` in the output to confirm the server is ready.

## Resource Usage

The manifests request 500m CPU / 1.2 Gi RAM and limit at 2 CPU / 2 Gi RAM.
Adjust to taste for your Mac's available resources.

## Teardown

```bash
kubectl delete namespace games
```

## Notes

- `EULA=TRUE` is set automatically in the manifest (required by Mojang).
- The `MEMORY=1G` env var sets the JVM heap. Increase if you see OOM kills.
- World data is not persisted across pod restarts (no PVC in this demo).
  Add a PersistentVolumeClaim to the Deployment for persistence.
- `itzg/minecraft-server` builds natively for linux/arm64; no emulation needed.
