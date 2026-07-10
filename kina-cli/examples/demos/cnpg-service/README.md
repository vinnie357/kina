# CNPG Service Demo — Postgres as a Service

Runs a [CloudNativePG](https://cloudnative-pg.io/) (CNPG) Postgres cluster and
exposes it OUTSIDE the kina cluster, so `psql` on your Mac connects straight to
a "kina Postgres cluster as a service".

Two exposure paths:

- **TCPRoute (primary)** — Postgres traffic flows through the NGF Gateway on
  port 5432, consistent with the other Gateway API demos.
- **CNPG managed NodePort (fallback)** — no NGF involvement; CNPG itself creates
  a NodePort service from the Cluster spec.

> **Security note** — CNPG's docs warn: "Allowing access to a database from the
> public network could expose your database to potential attacks from malicious
> users. Make sure you secure your database before granting external access, or
> that your Kubernetes cluster is only reachable from a private network."
> A kina cluster's node IP is only reachable from your Mac, and this is a demo:
> do not reuse this pattern to expose a real database.

## Prerequisites

- Running kina cluster with `--cni cilium` (required for L4 TCPRoute routing)
  and extra memory (`--memory 8g` — the 4g default cannot hold CNPG + Postgres
  alongside the cluster components):

  ```bash
  kina create <cluster> --cni cilium --memory 8g
  ```
- `kina install nginx-gateway-fabric --cluster <cluster>`
- Experimental TCPRoute CRD (**v1.5.1** — match the CRD version the NGF addon
  installs), the NGF `--gateway-api-experimental-features` flag, and kina-43
  RBAC — follow `examples/gatewayapi/README.md` (Prerequisites steps 3–4)
- A default StorageClass and the CNPG operator — follow the
  [cnpg-app README](../cnpg-app/README.md) Prerequisites (local-path-provisioner
  v0.0.36 + CNPG 1.30.0)

## Deploy

**1. Create the Postgres cluster and wait for it to be healthy:**

```bash
kubectl apply -f cluster.yaml
kubectl get cluster -n cnpg-service pg -w
# Wait for STATUS: Cluster in healthy state
```

**2. Add a `tcp-pg` listener to the Gateway and expose port 5432.**

These patches follow the same pattern as `examples/gatewayapi/README.md` step 5
(merge patches replace the whole list — include every listener/hostPort you use):

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
  - name: tcp-pg
    port: 5432
    protocol: TCP
    allowedRoutes:
      namespaces:
        from: All
      kinds:
      - group: gateway.networking.k8s.io
        kind: TCPRoute
'

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
        - containerPort: 5432
          port: 5432
'
```

**3. Attach the TCPRoute:**

```bash
kubectl apply -f tcp-route.yaml
```

## Connect from your Mac

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

# The CNPG-generated pg-app secret holds the app user's password
PGPASSWORD=$(kubectl get secret -n cnpg-service pg-app \
  -o jsonpath='{.data.password}' | base64 -d)

psql "postgresql://app:$PGPASSWORD@$NODE_IP:5432/app" -c 'SELECT version()'
```

## Fallback: CNPG managed NodePort (no Gateway)

CNPG can create the external service itself. Add this to `cluster.yaml` under
`spec` and re-apply (do NOT set a `selector` — the operator manages it):

```yaml
  managed:
    services:
      additional:
      - selectorType: rw
        serviceTemplate:
          metadata:
            name: pg-external
          spec:
            type: NodePort
            ports:
            - name: postgres
              port: 5432
              targetPort: 5432
              nodePort: 30432
```

Then connect via the pinned NodePort:

```bash
psql "postgresql://app:$PGPASSWORD@$NODE_IP:30432/app" -c 'SELECT 1'
```

## Teardown

```bash
kubectl delete namespace cnpg-service
```

To remove the Gateway listener/hostPort, re-run the two patches from step 2
without the `tcp-pg` / `5432` entries.

## Notes

- `pg-rw` always points at the primary; connect writes there. CNPG also creates
  `pg-ro`/`pg-r` for read traffic.
- External clients authenticate with password auth as the `app` user against
  the `app` database (CNPG's generated credentials). The superuser is not
  exposed (`enableSuperuserAccess` defaults to off).
- Future work: an external-secrets/OpenBao demo for production-grade credential
  distribution is out of scope here.
