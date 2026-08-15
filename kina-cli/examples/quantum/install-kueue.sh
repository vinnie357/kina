#!/usr/bin/env bash
# Install Kueue via its pinned Helm chart, then apply the queue objects with
# quotas derived live from worker allocatable (85%, headroom rule).
# Prereq: workers labeled (run ./label-nodes.sh first).
source "$(dirname "$0")/lib.sh"
need helm "brew install helm"; need envsubst "part of gettext: brew install gettext"
need jq "brew install jq"; need kubectl "brew install kubectl"

if helm status kueue -n kueue-system -o json 2>/dev/null | jq -e '.info.status == "deployed"' >/dev/null; then
  log "Kueue helm release already deployed"
else
  log "installing Kueue ${KUEUE_VERSION} (upgrade --install recovers from failed prior attempts)"
  helm upgrade --install kueue "${KUEUE_CHART}" --version="${KUEUE_VERSION}" \
    --namespace kueue-system --create-namespace \
    --set enablePrometheus=true \
    --wait --timeout 300s
fi

kubectl wait --for=condition=Established crd/clusterqueues.kueue.x-k8s.io --timeout=120s
kubectl -n kueue-system wait --for=condition=Available deployment --all --timeout=180s

# Derive quota from each flavor's worker node: 85% of allocatable.
quota_for() { # $1=simulator-class label value  $2=cpu|memory
  local nodes count node
  nodes="$(kubectl get nodes -l "simulator-class=$1" -o name)"
  count="$(printf '%s\n' "$nodes" | grep -c . || true)"
  [[ "$count" -eq 1 ]] || die "expected exactly 1 node labeled simulator-class=$1, found ${count}.
Run ./label-nodes.sh first. Quota derivation assumes one node per flavor (v1
topology); aggregate in quota_for() if the topology grew."
  node="$nodes"
  if [[ "$2" == cpu ]]; then
    local mc; mc="$(kubectl get "$node" -o jsonpath='{.status.allocatable.cpu}')"
    [[ "$mc" == *m ]] || mc="$((mc * 1000))m"
    echo "$(( ${mc%m} * 85 / 100 ))m"
  else
    local ki; ki="$(kubectl get "$node" -o jsonpath='{.status.allocatable.memory}')"
    [[ "$ki" == *Ki ]] || die "unexpected allocatable.memory format '${ki}' on ${node} (expected Ki suffix).
Refusing to guess units — a wrong quota would be silent. Adjust quota_for() for this format."
    echo "$(( ${ki%Ki} * 85 / 100 / 1024 ))Mi"
  fi
}
# Assign before export: `export VAR="$(...)"` returns export's status (0),
# which would swallow die's exit under set -e.
CPU_HEAVY_CPU_QUOTA="$(quota_for cpu-heavy cpu)"
CPU_HEAVY_MEM_QUOTA="$(quota_for cpu-heavy memory)"
MEM_HEAVY_CPU_QUOTA="$(quota_for memory-heavy cpu)"
MEM_HEAVY_MEM_QUOTA="$(quota_for memory-heavy memory)"
export CPU_HEAVY_CPU_QUOTA CPU_HEAVY_MEM_QUOTA MEM_HEAVY_CPU_QUOTA MEM_HEAVY_MEM_QUOTA
log "quotas: cpu-heavy ${CPU_HEAVY_CPU_QUOTA}/${CPU_HEAVY_MEM_QUOTA}, memory-heavy ${MEM_HEAVY_CPU_QUOTA}/${MEM_HEAVY_MEM_QUOTA}"

kubectl apply -f "$REPO_ROOT/kueue/namespace.yaml"
kubectl apply -f "$REPO_ROOT/kueue/resource-flavors.yaml"
envsubst < "$REPO_ROOT/kueue/cluster-queue.yaml.tmpl" | kubectl apply -f -
kubectl apply -f "$REPO_ROOT/kueue/local-queue.yaml"

kubectl get clusterqueue quantum-sim-cluster-queue
kubectl -n quantum-sims get localqueue quantum-sims-queue
log "Kueue OK"
