#!/usr/bin/env bash
# Thin submission wrapper: render a simulator Job template for a QASM circuit
# and submit it. Not a CLI framework on purpose. Prints the created job name
# as its last line (stdout); everything else goes to stderr.
source "$(dirname "$0")/lib.sh"
need envsubst "brew install gettext"
need shasum "ships with macOS/perl"
need kubectl "brew install kubectl"

usage() {
  cat >&2 <<EOF
Usage: $0 --simulator {pennylane|qiskit-aer|qsim} --circuit FILE [--shots N] [--namespace NS] [--class {cpu-heavy|memory-heavy}]
EOF
  exit 1
}

CIRCUIT="" NAMESPACE=quantum-sims SHOTS=1024 SIMULATOR="" SIMULATOR_CLASS=cpu-heavy
while [[ $# -gt 0 ]]; do
  case "$1" in
    --simulator) SIMULATOR="$2"; shift 2;;
    --circuit)   CIRCUIT="$2"; shift 2;;
    --shots)     SHOTS="$2"; shift 2;;
    --namespace) NAMESPACE="$2"; shift 2;;
    --class)     SIMULATOR_CLASS="$2"; shift 2;;
    *) usage;;
  esac
done

case "$SIMULATOR" in pennylane|qiskit-aer|qsim) ;; *) die "unknown simulator '$SIMULATOR' (pennylane|qiskit-aer|qsim)";; esac
# Validate --class here: a typo'd class renders a nodeSelector matching no
# node/flavor and the workload pends forever with no error at submit time.
case "$SIMULATOR_CLASS" in cpu-heavy|memory-heavy) ;; *) die "unknown class '$SIMULATOR_CLASS' (cpu-heavy|memory-heavy)";; esac
[[ -f "$CIRCUIT" ]] || die "circuit file not found: $CIRCUIT"
TEMPLATE="$REPO_ROOT/simulators/$SIMULATOR/job-template.yaml"
[[ -f "$TEMPLATE" ]] || die "missing job template: $TEMPLATE"

export JOB_PREFIX="${SIMULATOR}-sim"
export IMAGE="quantum-sims/${SIMULATOR}:${SIM_IMAGE_TAG}"
# Content-hashed name: a shared per-simulator ConfigMap would let a queued or
# concurrent job read a circuit submitted AFTER it. Identical content reuses
# the same ConfigMap (apply is idempotent); no ownerReference cleanup because
# same-content submissions share one ConfigMap and tying it to a single Job
# would GC it out from under siblings. Stale ones are tiny and go with the ns.
CIRCUIT_HASH="$(shasum -a 256 "$CIRCUIT" | cut -c1-10)"
export CIRCUIT_CONFIGMAP="circuit-${SIMULATOR}-${CIRCUIT_HASH}"
CIRCUIT_FILENAME="$(basename "$CIRCUIT")"
export CIRCUIT_FILENAME
export SHOTS SIMULATOR_CLASS

kubectl create configmap "$CIRCUIT_CONFIGMAP" -n "$NAMESPACE" \
  --from-file="$CIRCUIT_FILENAME=$CIRCUIT" \
  --dry-run=client -o yaml | kubectl apply -f -

JOB_NAME="$(envsubst < "$TEMPLATE" | kubectl create -n "$NAMESPACE" -f - -o name)"
JOB_NAME="${JOB_NAME#job.batch/}"

log "watch queue:  kubectl -n $NAMESPACE get workloads"
log "watch job:    kubectl -n $NAMESPACE get job $JOB_NAME -w"
log "logs:         kubectl -n $NAMESPACE logs job/$JOB_NAME"
echo "$JOB_NAME"
