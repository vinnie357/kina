# kagent App Demo — Hello-World Agent on a Local Ollama Model

[kagent](https://kagent.dev/) is a CNCF Sandbox project that runs AI agents
as Kubernetes-native custom resources (`Agent`, `ModelConfig`, `ToolServer`).
This demo installs kagent on a kina cluster, points it at an
[Ollama](https://ollama.com/) server running on the macOS host (no cloud API
key required), and applies a minimal hello-world `Agent`.

> **Status: not yet validated on a live cluster.** Validation run 1 found
> storage and values-key issues (a missing default-StorageClass prerequisite,
> and values field names originally sourced from the chart's `main` branch
> instead of the pinned release tag); both are fixed in this revision. The
> pod→host networking path and whether kina's documented memory-limit OOM
> class (see the note in [cnpg-app](../cnpg-app/README.md)) reproduces for
> kagent's Go/Python components remain **unverified**. Re-validation is
> pending — treat every command below as a documented starting point, not a
> proven recipe, until someone runs it end to end and updates this note.

## Prerequisites

- A kina cluster (see memory sizing below).
- [Helm](https://helm.sh/) 3.x.
- A default StorageClass — kina clusters ship without one, and kagent's
  bundled Postgres unconditionally claims a PVC (`database.postgres.bundled`,
  500Mi by default; see `values-kina.yaml`'s header comment for the exact
  chart template reference — there is no persistence-disabled/emptyDir
  escape hatch short of pointing at an external database). Install
  local-path-provisioner and mark it default, the same way the
  [cnpg-app](../cnpg-app/README.md#prerequisites) demo does:

  ```bash
  kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/v0.0.36/deploy/local-path-storage.yaml
  kubectl patch storageclass local-path \
    -p '{"metadata": {"annotations": {"storageclass.kubernetes.io/is-default-class": "true"}}}'
  ```

  As with the cnpg demos, this is a manual step until kina-46 lands a proper
  `kina install <addon>` for the storage provisioner (see the "CloudNativePG"
  note in [../README.md](../README.md)).
- [Ollama](https://ollama.com/) running on the macOS host (not in the
  cluster) with a tool-calling-capable model pulled. This demo uses
  `llama3.1:8b` — confirmed on Ollama's own model library page to carry the
  `tools` capability badge (`ollama.com/library/llama3.1`). kagent's own
  docs are explicit that the model matters, not the Ollama integration
  itself: *"As kagent relies on calling tools, make sure you're using a
  model that allows function calling."*
  ([kagent.dev/docs/kagent/supported-providers/ollama](https://www.kagent.dev/docs/kagent/supported-providers/ollama))

  ```bash
  ollama pull llama3.1:8b
  ollama serve   # if not already running as a background service
  ```

## 1. Create the cluster

The `kagent` chart is a 14-dependency Helm umbrella chart. All ten built-in
agent subcharts (`k8s-agent`, `kgateway-agent`, `istio-agent`, `promql-agent`,
`observability-agent`, `argo-rollouts-agent`, `helm-agent`,
`cilium-policy-agent`, `cilium-manager-agent`, `cilium-debug-agent`) plus
`grafana-mcp`, `querydoc`, and the `kmcp` subchart default to `enabled: true`
— none needed here. Left enabled, their combined memory *requests* alone
total ~2.25Gi (recomputed from all 16 resource-bearing blocks in the chart's
`values.yaml` at the pinned `v0.10.0-beta7` tag) — already past kina's 2GB
default cluster memory budget before any hello-world `Agent` pod runs.

The values overlay in this directory (`values-kina.yaml`) disables all of
them and strips memory *limits* (not requests) from what's left, per kina's
documented OOM class. What remains after disabling the extras:

| Component | Memory request |
|---|---|
| controller | 128Mi |
| ui | 256Mi |
| bundled postgres | 256Mi |
| kagent-tools | 128Mi |

That's ~768Mi of requests before the hello-world `Agent`'s own engine pod
and kina's system pods (CNI, CoreDNS, kube-proxy) are counted — recomputed
directly from the four kept components' `resources` blocks at the pinned
tag, not copied from the chart's all-16-block total above. kina's default
cluster memory budget is 2GB — too tight a margin even at the reduced
footprint once system pods and the demo agent are counted. Create the
cluster with headroom, following the same pattern as the
[cnpg-app](../cnpg-app/README.md) demo:

```bash
kina create kagent-demo --memory 4g
```

## 2. Install kagent

Two OCI Helm charts, CRDs first, both pinned to `0.10.0-beta7` — the latest
release as of this writing. Note the version-string mismatch between
sources: the GitHub release **git tag** is `v0.10.0-beta7` (with a `v`
prefix), but the **chart version and container image tags** on
`ghcr.io/kagent-dev/kagent/*` have no `v` prefix (`0.10.0-beta7`). Use the
unprefixed form for `--version`.

```bash
helm install kagent-crds oci://ghcr.io/kagent-dev/kagent/helm/kagent-crds \
  --version 0.10.0-beta7 \
  --namespace kagent \
  --create-namespace

helm install kagent oci://ghcr.io/kagent-dev/kagent/helm/kagent \
  --version 0.10.0-beta7 \
  --namespace kagent \
  -f values-kina.yaml

kubectl rollout status deployment -n kagent kagent-controller
```

`values-kina.yaml`'s field names (agent enable toggles, `querydoc`,
`grafana-mcp`, `kmcp`, and the `resources.limits` paths for `controller`,
`ui`, `kagent-tools`, and `database.postgres.bundled`) are verified against
the chart's `values.yaml` and `Chart-template.yaml` at the pinned
`v0.10.0-beta7` release tag itself:
<https://raw.githubusercontent.com/kagent-dev/kagent/v0.10.0-beta7/helm/kagent/values.yaml>
<https://raw.githubusercontent.com/kagent-dev/kagent/v0.10.0-beta7/helm/kagent/Chart-template.yaml>
An earlier revision of this overlay was sourced from the `main` branch,
which is missing the ten built-in agent subcharts — main's thinner fetch
never surfaced them, so that revision left those pods unstripped. Disabling
all ten agents plus `kmcp`/`grafana-mcp`/`querydoc` together (rather than
leaving at least one agent enabled) has not been tested against a live
install — it's possible the controller has a CRD/webhook dependency on at
least one agent existing. **Requires validation.**

## 3. Find the address of the host's Ollama server

Pods inside a kina cluster run in their own CNI namespace, nested inside the
node VM. Apple Container's `host.docker.internal` DNS name (documented for
Apple Container 0.9.0+) is the first thing to try, but whether it
propagates through kina's CNI into pod-level DNS resolution is unconfirmed.

**Try `host.docker.internal` first:**

```bash
kubectl run -n kagent netprobe --rm -it --image=busybox --restart=Never -- \
  wget -qO- http://host.docker.internal:11434/api/tags
```

If that resolves and returns Ollama's model list JSON, use
`host.docker.internal` as the `ModelConfig` host and skip the fallback.

**Fallback — the cluster's network gateway:**

kina itself inspects this address for host↔node-VM reachability
(`kina-cli/src/core/apple_container.rs`, `inspect_network_bridge`). In
Apple Container's shared/NAT networking mode, the gateway IP is
conventionally the macOS host:

```bash
container network inspect kagent-demo --format json | jq -r '.[0].ipv4Gateway // .[0].gateway'
```

Then re-probe with that IP in place of `host.docker.internal`:

```bash
kubectl run -n kagent netprobe --rm -it --image=busybox --restart=Never -- \
  wget -qO- http://<GATEWAY_IP>:11434/api/tags
```

Whichever address responds with Ollama's `{"models": [...]}` JSON is the
value to substitute into `manifests/modelconfig-ollama.yaml`'s
`HOST_PLACEHOLDER`.

## 4. Apply the ModelConfig and hello-world Agent

```bash
sed -i '' "s/HOST_PLACEHOLDER/<the address from step 3>/" manifests/modelconfig-ollama.yaml
kubectl apply -f manifests/modelconfig-ollama.yaml
kubectl apply -f manifests/agent-hello.yaml
```

`manifests/modelconfig-ollama.yaml` also creates a Secret with a dummy API
key. kagent's own Ollama documentation example
(<https://www.kagent.dev/docs/kagent/supported-providers/ollama>) sets
`apiKeySecret`/`apiKeySecretKey` even for Ollama, which reads as a
copy-paste artifact from the OpenAI example — Ollama itself performs no key
check. Whether the kagent controller actually requires this field to
resolve for a non-OpenAI provider is **unconfirmed**; the manifest includes
it defensively. If a live run shows the `ModelConfig` reconciles fine
without it, delete the Secret and the two `apiKeySecret*` fields.

CRD field names (`ModelConfig.spec.provider`, `.spec.ollama.host`,
`.spec.model`, `Agent.spec.type`, `.spec.declarative.systemMessage`,
`.spec.declarative.modelConfig`) were verified against
<https://kagent.dev/docs/kagent/resources/api-ref>.

## 5. Verify

```bash
kubectl get modelconfig -n kagent
kubectl get agents -n kagent
kubectl get pods -n kagent -w   # watch for CrashLoopBackOff / OOMKilled
```

Once the `hello-world` agent reports ready, port-forward to the kagent UI
(kagent's documented access path is port-forward, not ingress):

```bash
kubectl port-forward -n kagent svc/kagent-ui 8080:80
```

Open `http://localhost:8080`, select the `hello-world` agent, and send it a
message. A plain response confirms the ModelConfig → Ollama wiring works.

This scaffold does **not** wire an explicit tool call — no `ToolServer` is
attached to `hello-world`, since the exact schema for referencing a working
built-in or remote MCP tool server wasn't confirmed in this session (the
`Agent.spec.declarative.tools[].mcpServer` field shape is documented, but no
pre-installed `ToolServer` ships with the chart by default per the API
reference). Exercising an actual tool call is a follow-up: install a
`ToolServer` (or reference kagent's `kagent-tools` component, left enabled
in `values-kina.yaml` for this reason) and add a `tools:` entry to
`manifests/agent-hello.yaml`. The tool-calling-capable model choice above is
forward-looking preparation for that follow-up, not something this scaffold
currently exercises end to end.

## Alternative: OpenAI-compatible local servers (MLX, LM Studio)

kagent's `OpenAI` provider accepts a `baseUrl` override
(`ModelConfig.spec.openAI.baseUrl`), which is the generic path for any
OpenAI-compatible local inference server — not a distinct provider type.
Swap `manifests/modelconfig-ollama.yaml`'s `spec` for:

```yaml
spec:
  provider: OpenAI
  model: <model-name-your-server-exposes>
  openAI:
    baseUrl: "http://<HOST_ADDRESS>:<PORT>/v1"
  apiKeySecret: ollama-hello-dummy-key
  apiKeySecretKey: DUMMY_API_KEY
```

Tool-calling support varies a lot by server:

- **`mlx_lm.server`** (the reference MLX server) — as of the research behind
  this demo, native OpenAI-style tool-calling was not yet merged upstream.
  **Requires validation** against the current `mlx-lm` release before
  relying on it.
- **LM Studio** — documents OpenAI-compatible tool-use support
  (`lmstudio.ai/docs/developer/openai-compat/tools`); not independently
  verified against kagent in this session.
- Community MLX servers claiming tool-calling (`mlx-openai-server`,
  `vllm-mlx`) exist but are unverified against kagent — treat as candidates
  to evaluate, not confirmed-working.

The pod→host discovery steps in section 3 apply identically regardless of
which local server backs the `OpenAI` provider's `baseUrl`.

## Cleanup

```bash
kubectl delete namespace kagent
```

## Notes

- kagent's controller, UI, and app images publish `linux/arm64` manifests at
  `0.10.0-beta7` on `ghcr.io` (confirmed by direct manifest query during
  this demo's research — see the bundled kagent research doc referenced in
  bees issue kina-51).
- v0.10.0-beta7 is a pre-1.0 beta release; CRD fields may still shift
  between releases. Re-check the API reference before reusing this demo
  against a newer kagent version.
- The bundled Postgres image (`docker.io/library/postgres:18.3-alpine`) is
  pulled from Docker Hub, not `ghcr.io` — worth knowing if you're behind a
  registry mirror.
