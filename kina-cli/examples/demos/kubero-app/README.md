# Kubero App Demo — Self-Hosted PaaS

[Kubero](https://kubero.dev/) is a self-hosted Platform-as-a-Service that
runs on Kubernetes as an Operator + UI pair of CRDs (`Kubero`,
`KuberoPipeline`, `KuberoApp`). It is licensed GPL-3.0; it is **not** a
confirmed CNCF project, so this demo does not describe it as one. This demo
installs the Kubero operator and UI onto a kina cluster, then deploys a
public container image as a test app through the Kubero dashboard — no git
provider or container registry required. A PersistentVolume **is** required
for Kubero's own database; see Prerequisites below.

> **Status:** Not fully validated. Validation run 1 confirmed install → UI
> reachable → login: the operator reconciles, the `kubero` Service and its
> `nginx`-class Ingress serve traffic, JWT-based login succeeds with no
> setup wizard, and the WebSocket upgrade works through the
> `nginx.org/websocket-services` annotation. Run 2 confirmed the
> `kubero.database.storageClassName: local-path` fix works completely
> unaided — zero manual PV, zero manual chown, pod `1/1 Running` with 0
> restarts — retiring run 1's storage findings outright. Run 2 also found
> two new upstream bugs in deploying an app through the UI, both
> root-caused to kubero v3.1.1/operator v0.2.2 source and both worked
> around below: (1) the pipeline-creation form has no control for
> `deploymentstrategy` when "Builds" is off, and its hardcoded `"git"`
> default crash-loops the operator — this README now applies a vendored
> `pipeline-hello.yaml` via `kubectl` instead of the form; (2) the
> app-creation form's default `Read-only Root Filesystem` security switch
> crashes the documented test image — this README now documents turning
> that switch off via the form's own Advanced/Security panel. Neither fix
> has been exercised through an actual browser session yet (run 2 used
> source-verified API payloads, no browser tool was available) — that is
> the next validation run's job. Every command below is a plan, not a
> proven recipe, until that run confirms it and this note is updated.

## Prerequisites

- A kina cluster with room for the operator's burst ceiling. The operator
  pod's two containers total roughly 1.125Gi at their memory *limits* (1Gi
  manager + 128Mi `kube-rbac-proxy` sidecar), and workload *requests* on top
  of kina's own system pods come to about 768Mi (operator 128Mi, the
  nginx-ingress DaemonSet 128Mi, the Kubero UI 0Mi since its chart ships
  `resources: {}`, plus a 512Mi `small`-podSize test app). `--memory 4g` is
  kina's own default and is expected to cover this with headroom — escalate
  only if a live run shows node pressure:

  ```bash
  kina create kubero-demo --memory 4g
  ```

- [`nginx-ingress`](../../../manifests/nginx-ingress/), kina's first-party
  addon:

  ```bash
  kina install nginx-ingress --cluster kubero-demo
  ```

- **Mark the `nginx` IngressClass as cluster-default.** This step is
  load-bearing, not belt-and-suspenders: the Kubero UI's own app-creation
  form never sets an `ingressClassName` on the Ingress objects it creates
  for deployed apps, so the default-class annotation is the *only*
  mechanism that gets those app Ingresses served. Kubernetes assigns the
  default class **at Ingress creation time only** — annotating after an
  app already exists does not retro-class it. Run this **before** creating
  any pipeline or app in Kubero:

  ```bash
  kubectl annotate ingressclass nginx ingressclass.kubernetes.io/is-default-class=true
  ```

- **A StorageClass — required, not optional.** The Kubero chart
  unconditionally creates a `kubero-data` PersistentVolumeClaim (1Gi) for
  the app's own SQLite database at `/app/server/db`. This is independent of
  `kubero.auditLogs.enabled` (`false` in `kubero-cr.yaml`), which only gates
  a *separate*, optional audit-log PVC — `registry.enabled`/`registry.create`
  being `false` doesn't affect it either. Upstream's own chart leaves
  `kubero.database.storageClassName` unset by default, which resolves to an
  explicit `storageClassName: ""` on the PVC — opting it out of dynamic
  provisioning and default-class admission entirely, so installing a
  default StorageClass alone does not bind it (confirmed live in kina-53
  validation run 1). `kubero-cr.yaml` in this demo sets
  `kubero.database.storageClassName: local-path` to fix this directly.
  Install local-path-provisioner and mark it cluster-default before
  applying the CR — the same prerequisite the
  [cnpg-app](../cnpg-app/README.md#prerequisites) demo documents:

  ```bash
  kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.36/deploy/local-path-storage.yaml
  kubectl patch storageclass local-path \
    -p '{"metadata": {"annotations": {"storageclass.kubernetes.io/is-default-class": "true"}}}'
  ```

  Whether local-path-provisioner's own PVs also dissolve the separate
  `fsGroup`-does-not-apply-to-`hostPath` permission gap that run 1 hit under
  a raw `hostPath` PV is **not yet confirmed live** — the next validation
  run must confirm or refute this before this note is removed.

## Install (in order)

**1. Install the Kubero operator + CRDs, pinned at tag `v0.2.2`:**

```bash
kubectl apply -f https://raw.githubusercontent.com/kubero-dev/kubero-operator/v0.2.2/deploy/operator.yaml
kubectl get crd | grep kubero
```

**2. Patch the operator's `kube-rbac-proxy` sidecar image.** The manifest
from step 1 references `gcr.io/kubebuilder/kube-rbac-proxy:v0.11.0`, and
that GCR repository currently publishes no tags at all, so the image pull
fails. The sidecar shares the controller-manager pod, so an
ImagePullBackOff there keeps the whole operator un-Ready and no `Kubero`
CR ever reconciles:

```bash
kubectl -n kubero-operator-system set image \
  deployment/kubero-operator-controller-manager \
  kube-rbac-proxy=quay.io/brancz/kube-rbac-proxy:v0.11.0
kubectl -n kubero-operator-system get pods
# expect 2/2 Running
```

**3. Create the namespace and the UI's secret.** `KUBERO_WEBHOOK_SECRET` is
the only non-optional key; `KUBERO_SESSION_KEY` and the admin credentials
are optional but set explicitly here so the demo has a known, scriptable
login instead of a randomly generated password buried in pod logs. Choose
your own values — do not reuse example text verbatim:

```bash
kubectl create namespace kubero
kubectl create secret generic kubero-secrets \
  --from-literal=KUBERO_WEBHOOK_SECRET="$(openssl rand -hex 32)" \
  --from-literal=KUBERO_SESSION_KEY="$(openssl rand -hex 32)" \
  --from-literal=KUBERO_ADMIN_USERNAME=admin \
  --from-literal=KUBERO_ADMIN_PASSWORD="<choose-your-own-password>" \
  -n kubero
```

**4. Apply the Kubero CR**, substituting your node's IP:

```bash
NODE_IP=$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="InternalIP")].address}')
sed "s/<NODE_IP>/$NODE_IP/g" kubero-cr.yaml | kubectl apply -n kubero -f -
```

**5. Wait for reconciliation:**

```bash
kubectl -n kubero get pods -w
kubectl -n kubero get svc,ingress
```

## Access

```bash
curl -sS -o /dev/null -w "%{http_code}\n" "http://kubero.$NODE_IP.nip.io/"
```

Open `http://kubero.<NODE_IP>.nip.io` in a browser and log in as `admin`
with the password you chose in step 3 above.

Fallback, if the Ingress path isn't reachable:

```bash
kubectl port-forward svc/kubero -n kubero 2000:2000
# then open http://localhost:2000
```

## Deploy an app through Kubero

1. **Create the Pipeline via `kubectl`, not the dashboard's pipeline form.**
   The form has no control for the pipeline's deployment strategy when
   "Builds" (gitops) is left off — its hardcoded default (`git`) crash-loops
   the operator on a chart-default placeholder secret that isn't valid
   base64. Apply the vendored CR instead, which sets
   `deploymentstrategy: docker` directly (see
   [`pipeline-hello.yaml`](pipeline-hello.yaml) for the full root cause):

   ```bash
   kubectl apply -n kubero -f pipeline-hello.yaml
   ```

   Open the dashboard — the `hello` pipeline appears with its `production`
   stage already enabled; no "Add a stage" step is needed.
2. **Create an App** inside the `production` stage:
   - Deployment strategy: `docker`
   - Image: `nginxdemos/hello`
   - Tag: `0.4`
   - **Container Port: `80`** — the form defaults to `8080`. This field is
     not cosmetic: the kuberoapp chart wires the Service's `targetPort`
     directly to whatever the form submits, and `nginxdemos/hello:0.4`
     serves on port 80. Leaving the default routes `Service:80 →
     pod:8080` and the app returns a 502 no matter what else is correct.
   - Pod size: `small`
   - Domain: `hello.<NODE_IP>.nip.io`
   - **Turn on "Advanced App Config", open the Security panel, and switch
     off "Read-only Root Filesystem" before saving.** The form defaults
     this switch on, and `nginxdemos/hello:0.4` needs to `mkdir` under
     `/var/cache/nginx` at startup — with the default on, the pod
     permanently `CrashLoopBackOff`s. This switch is the exact field a
     `kubectl patch ... securityContext/readOnlyRootFilesystem` would
     otherwise flip by hand; using the form avoids the patch entirely.
3. **Verify:**

   ```bash
   curl -sS "http://hello.$NODE_IP.nip.io/"
   ```

   Expect the nginx hello-world page body.

## Notes

- The `small` pod-size preset applies a **1Gi memory limit** to every app
  deployed through Kubero. Kina node VMs OOM-kill a BEAM/Erlang workload
  instantly the moment any memory *limit* (not just a request) is applied —
  do not deploy a Phoenix/Elixir test app through Kubero without first
  reading the memory-limit note in the
  [cnpg-app](../cnpg-app/README.md#notes) demo.
- The upstream operator manifest's `kube-rbac-proxy` sidecar image
  (`gcr.io/kubebuilder/kube-rbac-proxy:v0.11.0`) currently has no published
  tags on GCR — step 2 above works around it with a maintained mirror.
- Kubero stamps community `nginx.ingress.kubernetes.io/*` annotations onto
  every app Ingress it creates. kina ships NGINX Inc's ingress controller,
  which ignores that annotation prefix entirely — these annotations are
  inert here, not a bug to "fix".
- `kina create` switches your shell's *default* kubectl context to the new
  cluster as a side effect. If you're running other clusters, check
  `kubectl config current-context` before and after, and switch back with
  `kubectl config use-context <previous-context>` when you're done.
- **Upstream bug (kubero v3.1.1 client + operator v0.2.2):** the
  pipeline-creation form never exposes `deploymentstrategy` when "Builds" is
  off, but its hardcoded `"git"` default makes the kuberopipeline chart
  render a git-deploy-key Secret from a chart-default placeholder string
  that isn't valid base64, permanently crash-looping the operator with
  `illegal base64 data at input byte 19` and never creating the pipeline's
  namespace. `deploymentstrategy: docker` is confirmed live (kina-53
  validation run 2) to avoid this entirely; `pipeline-hello.yaml` sets that
  field directly on the CR so the workaround doesn't need the form at all —
  see that file's header for the source citations and its
  [validate-live in run 3] note on the CR-apply mechanism itself.
- **Upstream bug (kubero v3.1.1 client):** the app-creation form defaults
  `Read-only Root Filesystem` to on for every new app, which crashes
  `nginxdemos/hello:0.4` (its nginx entrypoint needs to `mkdir` under
  `/var/cache/nginx` at startup) with a permanent `CrashLoopBackOff` —
  confirmed live via a `kubectl patch` to the same
  `securityContext.readOnlyRootFilesystem` field, kina-53 validation run 2.
  The form's own "Advanced App Config" → Security → "Read-only Root
  Filesystem" switch (documented above) sets this identical field and is
  sourced to the same v3.1.1 form code, but clicking it through an actual
  browser session is still [validate-live in run 3].

## Teardown

```bash
kubectl delete namespace kubero
kubectl delete -f https://raw.githubusercontent.com/kubero-dev/kubero-operator/v0.2.2/deploy/operator.yaml
kina delete kubero-demo
```
