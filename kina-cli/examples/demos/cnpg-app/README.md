# CNPG App Demo — Phoenix + HA CloudNativePG Postgres

A minimal [Phoenix LiveView app](https://github.com/vinnie357/cnpg-phoenix-demo)
backed by **two** [CloudNativePG](https://cloudnative-pg.io/) clusters, served
over HTTP via the Gateway API:

- **demo-db** — a 3-instance HA cluster for the app's notes. The app writes via
  the `demo-db-rw` service and reads via `demo-db-ro` (hot standbys).
- **metrics-db** — a single-instance cluster holding a second data domain
  (page-visit events).

The home page shows the live topology (which CNPG instance each connection pool
landed on, primary vs hot standby, streaming standbys from
`pg_stat_replication`), a read-only table browser for both databases, visit
counts, and the notes form.

Image: `ghcr.io/vinnie357/cnpg-phoenix-demo` (public, multi-arch incl. arm64)

## Prerequisites

- Running kina cluster with extra memory — four Postgres instances plus the app
  do not fit the 4g default. Default CNI is fine for this HTTP-only demo; use
  `--cni cilium` only if you also plan the [cnpg-service](../cnpg-service/)
  TCPRoute demo:

  ```bash
  kina create <cluster> --memory 8g
  ```

- `kina install nginx-gateway-fabric --cluster <cluster>`
- A default StorageClass — kina clusters ship without one, and CNPG needs a PVC
  per Postgres instance (4 here). Install local-path-provisioner and mark it
  default:

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

**1. Create both Postgres clusters and wait for them to be healthy:**

```bash
kubectl apply -f cluster.yaml
kubectl get cluster -n cnpg-app -w
# Wait for BOTH demo-db (3/3) and metrics-db (1/1) to report
# "Cluster in healthy state" — demo-db bootstraps the primary first, then
# joins each standby via pg_basebackup (allow a few minutes on first run).
```

If a cluster sits in "Setting up primary", check `kubectl get events -n cnpg-app`
for webhook timeouts or Pending PVCs before changing anything.

> Upgrading an existing single-instance demo-db? `kubectl apply -f cluster.yaml`
> scales it 1→3 in place, but `postInitApplicationSQL` only runs at bootstrap —
> grant pg_monitor manually so the standby list isn't NULL-masked:
>
> ```bash
> kubectl exec -n cnpg-app demo-db-1 -c postgres -- psql -U postgres -c 'GRANT pg_monitor TO app'
> ```

**2. Create the Phoenix secret and deploy the app:**

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')

kubectl -n cnpg-app create secret generic phoenix-demo-secret \
  --from-literal=SECRET_KEY_BASE="$(openssl rand -base64 48)"

sed "s/<NODE_IP>/$NODE_IP/g" app.yaml | kubectl apply -f -
sed "s/<NODE_IP>/$NODE_IP/g" route.yaml | kubectl apply -f -
```

The app runs migrations for both databases on boot. If it starts before the
databases are reachable it exits and Kubernetes restarts it — a few early
restarts are expected, not a problem.

## Verify

```bash
curl -s -o /dev/null -w "%{http_code}" http://phoenix.$NODE_IP.nip.io
# 200
```

Open `http://phoenix.<NODE_IP>.nip.io` in a browser:

- **Topology** — three pools: the primary pool (`demo-db-rw`, role *primary*),
  the read pool (`demo-db-ro`, role *hot standby*, a **different server IP**
  than the primary — that's the read/write split), and analytics
  (`metrics-db-rw`). The primary row also lists the streaming standbys
  (`demo-db-2`, `demo-db-3`) straight from `pg_stat_replication`.
- **Analytics** — every page load records a visit in the metrics-db cluster.
- **Databases** — click a table (notes, visits, schema_migrations) to browse
  its rows; main-database reads go through the replica pool.
- **Save a note** — inserts over the LiveView websocket into the primary.
- **LiveView showcase** — open the page in TWO browser windows. Click
  **Simulate traffic** in the analytics card: 20 visit rows stream into the
  metrics-db cluster and the visit count climbs live in BOTH windows
  (Phoenix.PubSub pushes every insert to every connected client). Notes work
  the same way — type in one window, it appears in the other.

## Failover walkthrough (the HA payoff)

```bash
# 1. Find the current primary
kubectl get cluster -n cnpg-app demo-db   # PRIMARY column, e.g. demo-db-1

# 2. Kill it
kubectl delete pod -n cnpg-app demo-db-1

# 3. Watch CNPG promote a standby (seconds)
kubectl get cluster -n cnpg-app demo-db -w
```

Reload the page: the primary pool's server IP has changed to the promoted
instance, your notes are intact, and the standby list re-forms as the old
primary rejoins as a replica. The page may flash a database error for a few
seconds mid-promotion — that's the failover window.

## How the wiring works

CNPG generates a secret per cluster with ready-made connection details. The
Deployment consumes `demo-db-app`'s `uri` as `DATABASE_URL` and
`metrics-db-app`'s `uri` as `ANALYTICS_DATABASE_URL`. The read-only URL is
composed from the same secret's discrete keys against the `demo-db-ro` service
(CNPG publishes no `-ro` URI). No credentials appear in any manifest.

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
