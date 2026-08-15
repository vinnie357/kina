#!/usr/bin/env bash
# Remove this example's resources from your cluster. By default it leaves the
# cluster (and the Kueue install) intact and only deletes what this example
# created: the quantum-sims namespace, the cluster-scoped Kueue objects, and
# the simulator-class node labels.
#
# Usage:
#   ./teardown.sh              # remove example resources only (default)
#   ./teardown.sh --cluster    # also delete the whole kina cluster ($CLUSTER_NAME)
source "$(dirname "$0")/lib.sh"
need kubectl "brew install kubectl"

DELETE_CLUSTER=0
[[ "${1:-}" == "--cluster" ]] && DELETE_CLUSTER=1

if (( DELETE_CLUSTER )); then
  command -v kina >/dev/null 2>&1 || die "kina not found — cannot delete cluster"
  if kina status "$CLUSTER_NAME" >/dev/null 2>&1; then
    log "deleting cluster '$CLUSTER_NAME'"
    kina delete "$CLUSTER_NAME"
  else
    log "no cluster named '$CLUSTER_NAME' — nothing to delete"
  fi
  exit 0
fi

log "removing example resources (namespace, queues, flavors, node labels)"
# Namespace deletion also removes the LocalQueue, Jobs, and circuit ConfigMaps.
kubectl delete namespace quantum-sims --ignore-not-found
kubectl delete clusterqueue quantum-sim-cluster-queue --ignore-not-found
kubectl delete resourceflavor cpu-heavy memory-heavy --ignore-not-found
for node in $(kubectl get nodes -l simulator-class -o name); do
  kubectl label "$node" simulator-class- >/dev/null 2>&1 || true
done
log "example resources removed (cluster and Kueue install left in place)"
log "to remove the whole cluster: ./teardown.sh --cluster  (or: kina delete $CLUSTER_NAME)"
