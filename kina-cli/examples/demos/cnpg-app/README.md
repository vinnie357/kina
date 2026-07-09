# CNPG App Demo — Phoenix + CloudNativePG Postgres

A minimal [Phoenix LiveView app](https://github.com/vinnie357/cnpg-phoenix-demo)
backed by a [CloudNativePG](https://cloudnative-pg.io/) (CNPG) Postgres cluster,
served over HTTP via the Gateway API. The app's home page shows the CNPG wiring
(Postgres version, database, user, pod, node) and stores notes in the database.

Image: `ghcr.io/vinnie357/cnpg-phoenix-demo` (public, multi-arch incl. arm64)

## Prerequisites

- Running kina cluster with extra memory — the default 4g node cannot hold
  CNPG + Postgres + the app alongside the cluster components (verified: the
  node thrashes and probes time out). Default CNI is fine for this HTTP-only
  demo; use `--cni cilium` only if you also plan to run the
  [cnpg-service](../cnpg-service/) TCPRoute demo:

  ```bash
  kina create <cluster> --memory 8g
  ```
- `kina install nginx-gateway-fabric --cluster <cluster>`
- A default StorageClass — kina clusters ship without one, and CNPG needs a PVC
  per Postgres instance. Install local-path-provisioner and mark it default:

  ```bash
  kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.36/deploy/local-path-storage.yaml
  kubectl patch storageclass local-path \
    -p '{"metadata": {"annotations": {"storageclass.kubernetes.io/is-default-class": "true"}}}'
  ```

- The CNPG operator:

  ```bash
  kubectl apply --server-side -f \
    https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/release-1.30/releases/cnpg-1.30.0.yaml
  kubectl rollout status deployment -n cnpg-system cnpg-controller-manager
  ```

## Deploy (in order)

**1. Create the Postgres cluster and wait for it to be healthy:**

```bash
kubectl apply -f cluster.yaml
kubectl get cluster -n cnpg-app demo-db -w
# Wait for STATUS: Cluster in healthy state (first boot pulls the postgres
# image and runs initdb — allow a couple of minutes)
```

If the cluster sits in "Setting up primary", check `kubectl get events -n cnpg-app`
for webhook timeouts or Pending PVCs before changing anything.

**2. Create the Phoenix secret and deploy the app:**

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

kubectl -n cnpg-app create secret generic phoenix-demo-secret \
  --from-literal=SECRET_KEY_BASE="$(openssl rand -base64 48)"

sed "s/<NODE_IP>/$NODE_IP/g" app.yaml | kubectl apply -f -
sed "s/<NODE_IP>/$NODE_IP/g" route.yaml | kubectl apply -f -
```

The app runs its database migrations on boot. If it starts before the database
is reachable it exits and Kubernetes restarts it — a few early restarts are
expected, not a problem.

## Verify

```bash
curl -s -o /dev/null -w "%{http_code}" http://phoenix.$NODE_IP.nip.io
# 200
```

Then open `http://phoenix.<NODE_IP>.nip.io` in a browser:

- The **Cluster wiring** panel shows the Postgres server version, the `app`
  database/user from the CNPG-generated secret, and the pod/node the app runs on.
- **Save a note** — the insert goes over the LiveView websocket through the
  Gateway, into Postgres via the `demo-db-rw` service. If notes persist after
  `kubectl delete pod -n cnpg-app -l app=phoenix-demo`, the full CNPG wiring
  (PVC + credentials + service) is working.

## How the wiring works

CNPG generates a `demo-db-app` secret containing ready-made connection details
for the `app` database owner. The Deployment consumes its `uri` key directly as
`DATABASE_URL` — no credentials appear in any manifest:

```yaml
- name: DATABASE_URL
  valueFrom:
    secretKeyRef:
      name: demo-db-app
      key: uri
```

The `uri` points at the `demo-db-rw` service (always the primary). CNPG also
creates `demo-db-ro`/`demo-db-r` services for read-only traffic.

## Teardown

```bash
kubectl delete namespace cnpg-app
```

## Notes

- The CNPG operator and postgres operand images are multi-arch (arm64 native).
- The app Deployment sets resource requests but no memory limit: on kina node
  VMs a memory limit OOM-kills the BEAM instantly at boot (the VM kernel's
  cgroup accounting counts the runtime's large virtual reservations against
  `memory.max`).
- First deploy pulls the ~50 MB app image inside the node VM (about 2 minutes);
  later restarts reuse it.
- For exposing Postgres itself outside the cluster, see the
  [cnpg-service](../cnpg-service/) demo.
