#!/usr/bin/env bash
# Prove Kueue controls admission: oversubmit vs cpu-heavy quota, assert >=1
# workload stays non-admitted, then assert it admits after capacity frees.
source "$(dirname "$0")/../lib.sh"
need jq "brew install jq"
NS=quantum-sims

QUOTA_M="$(kubectl get clusterqueue quantum-sim-cluster-queue -o json \
  | jq -r '.spec.resourceGroups[0].flavors[] | select(.name=="cpu-heavy") | .resources[] | select(.name=="cpu") | .nominalQuota' \
  | sed 's/m$//')"
REQUEST_M=500   # matches job template request
FITS=$(( QUOTA_M / REQUEST_M ))
SUBMIT=$(( FITS + 2 ))
log "cpu-heavy quota=${QUOTA_M}m, request=${REQUEST_M}m -> ${FITS} fit; submitting ${SUBMIT}"

JOBS=()
# Trap installed BEFORE the submit loop so a mid-loop failure still cleans up
# already-created jobs. The ${#JOBS[@]} guard matters: on bash 3.2 under
# set -u, expanding "${JOBS[@]}" on an empty array is an unbound-variable error.
cleanup() {
  if (( ${#JOBS[@]} > 0 )); then
    for j in "${JOBS[@]}"; do kubectl -n "$NS" delete job "$j" --ignore-not-found >/dev/null; done
  fi
}
trap cleanup EXIT

for i in $(seq 1 "$SUBMIT"); do
  JOBS+=("$("$REPO_ROOT/submit-circuit.sh" \
    --simulator qiskit-aer --circuit "$REPO_ROOT/circuits/bell.qasm" --shots 4096 | tail -1)")
done

# Poll immediately instead of sleep-then-check: the Bell jobs are small enough
# that the first wave can complete and free quota within a fixed wait, letting
# the overflow admit before a one-shot check sees it (false FAIL). Queued state
# must be caught while the first wave still holds the quota.
# Count only THIS run's non-admitted workloads (matched via ownerReferences to
# our job names) — a namespace-wide count would false-pass on stale workloads
# left by prior interrupted runs or a concurrent e2e test.
NAMES_JSON="$(printf '%s\n' "${JOBS[@]}" | jq -R . | jq -s .)"
pending_count() {
  kubectl -n "$NS" get workloads -o json \
    | jq --argjson names "$NAMES_JSON" \
      '[.items[]
        | select(any(.metadata.ownerReferences[]?; .kind=="Job" and (.name as $n | $names | index($n))))
        | select((.status.conditions // []) | map(select(.type=="Admitted" and .status=="True")) | length == 0)
       ] | length'
}
PENDING=0
for _ in $(seq 1 60); do
  PENDING="$(pending_count)"
  if (( PENDING >= 1 )); then break; fi
  sleep 1
done
log "non-admitted workloads (this run): $PENDING"

if (( PENDING < 1 )); then
  kubectl -n "$NS" get workloads
  die "FAIL: everything admitted immediately — quota is not constraining admission"
fi

log "PASS: $PENDING workload(s) queued behind quota; waiting for eventual drain"
for j in "${JOBS[@]}"; do
  kubectl -n "$NS" wait --for=condition=complete "job/$j" --timeout=600s >/dev/null \
    || die "job $j never completed after queueing"
done
log "quota exhaustion test PASS"
