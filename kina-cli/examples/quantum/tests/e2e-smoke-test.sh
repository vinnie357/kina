#!/usr/bin/env bash
# E2E: submit a Bell-state job for one simulator, require completion + sane counts.
# Usage: e2e-smoke-test.sh [pennylane|qiskit-aer|qsim]   (default: qiskit-aer)
source "$(dirname "$0")/../lib.sh"

SIMULATOR="${1:-qiskit-aer}"
NS=quantum-sims

JOB="$("$REPO_ROOT/submit-circuit.sh" \
  --simulator "$SIMULATOR" --circuit "$REPO_ROOT/circuits/bell.qasm" --shots 1024 | tail -1)"
[[ -n "$JOB" ]] || die "submit-circuit.sh produced no job name"
log "submitted $JOB, waiting for completion"

if ! kubectl -n "$NS" wait --for=condition=complete "job/$JOB" --timeout=300s; then
  kubectl -n "$NS" describe "job/$JOB" || true
  kubectl -n "$NS" logs "job/$JOB" --tail=50 || true
  die "job $JOB did not complete within 300s"
fi

LOGS="$(kubectl -n "$NS" logs "job/$JOB")"
echo "$LOGS"
echo "$LOGS" | grep -q "Measurement counts" || die "no measurement output in logs"
# Bell state: '00' and '11' keys both present in the counts dict.
# Scoped to the counts line and quoted keys — a bare grep over the whole log
# false-passes on digits in timestamps or count values (e.g. 09:00:15, 500).
COUNTS="$(echo "$LOGS" | grep "Measurement counts")"
echo "$COUNTS" | grep -q "'00'" && echo "$COUNTS" | grep -q "'11'" \
  || die "Bell-state counts missing 00/11 correlation: $COUNTS"

log "e2e smoke test PASS ($SIMULATOR)"
