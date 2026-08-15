#!/usr/bin/env bash
# Label the two worker nodes with simulator-class so Kueue's ResourceFlavors
# (cpu-heavy / memory-heavy) have nodes to bind to. Run once after creating a
# cluster with >=2 workers (`kina create quantum --workers 2`).
source "$(dirname "$0")/lib.sh"
need kubectl "brew install kubectl"

# Select workers by the kubeadm control-plane role label (kindest/node images
# are kubeadm-based) instead of a name-substring heuristic. Sorted for
# deterministic flavor assignment.
WORKER_LIST="$(kubectl get nodes --selector '!node-role.kubernetes.io/control-plane' \
  -o custom-columns=NAME:.metadata.name --no-headers | sort || true)"
WORKERS=()
while IFS= read -r node; do
  [[ -n "$node" ]] && WORKERS+=("$node")
done <<< "$WORKER_LIST"
TOTAL_NODES="$(kubectl get nodes --no-headers | wc -l | tr -d ' ')"
(( ${#WORKERS[@]} >= 2 )) || die "expected >=2 worker nodes, found ${#WORKERS[@]} of ${TOTAL_NODES} total.
Recreate the cluster with more workers: kina create ${CLUSTER_NAME} --workers 2
Check: kubectl get nodes --show-labels — if kina's control-plane node lacks the
node-role.kubernetes.io/control-plane label, worker selection needs adjusting."
(( ${#WORKERS[@]} < TOTAL_NODES )) || die "role selector excluded no nodes (${#WORKERS[@]} of ${TOTAL_NODES}) — refusing to label: the control-plane node would be mislabeled as a worker.
Check: kubectl get nodes --show-labels for node-role.kubernetes.io/control-plane."

kubectl label node "${WORKERS[0]}" simulator-class=cpu-heavy --overwrite
kubectl label node "${WORKERS[1]}" simulator-class=memory-heavy --overwrite

kubectl get nodes -L simulator-class
log "workers labeled: ${WORKERS[0]}=cpu-heavy, ${WORKERS[1]}=memory-heavy"
