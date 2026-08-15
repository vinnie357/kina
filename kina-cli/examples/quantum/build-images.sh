#!/usr/bin/env bash
# Build each simulator image with Apple `container` and load it into the cluster.
# Usage: build-images.sh [simulator ...]  (default: all dirs under simulators/)
source "$(dirname "$0")/lib.sh"
need container "install Apple container: https://github.com/apple/container"
need kina "install kina, or run from a built binary"

SIM_DIR="$REPO_ROOT/simulators"
SIMS=("$@")
if [[ ${#SIMS[@]} -eq 0 ]]; then
  # Glob directories only: `$(ls)` would word-split and choke on stray
  # non-directory files (Finder's .DS_Store, a future README).
  for d in "$SIM_DIR"/*/; do SIMS+=("$(basename "$d")"); done
fi

for sim in "${SIMS[@]}"; do
  [[ -f "$SIM_DIR/$sim/Dockerfile" ]] || die "no Dockerfile for '$sim'"
  IMAGE="quantum-sims/${sim}:${SIM_IMAGE_TAG}"
  log "building $IMAGE"
  container build \
    --build-arg "PYTHON_BASE_IMAGE=${PYTHON_BASE_IMAGE}" \
    --build-arg "QISKIT_VERSION=${QISKIT_VERSION}" \
    --build-arg "QISKIT_AER_VERSION=${QISKIT_AER_VERSION}" \
    --build-arg "CIRQ_VERSION=${CIRQ_VERSION}" \
    --build-arg "QSIMCIRQ_VERSION=${QSIMCIRQ_VERSION}" \
    --build-arg "PENNYLANE_VERSION=${PENNYLANE_VERSION}" \
    --build-arg "PENNYLANE_QISKIT_VERSION=${PENNYLANE_QISKIT_VERSION}" \
    -t "$IMAGE" -f "$SIM_DIR/$sim/Dockerfile" "$SIM_DIR/$sim"
  log "loading $IMAGE into cluster '$CLUSTER_NAME'"
  kina load "$IMAGE" --cluster "$CLUSTER_NAME"
done
log "images built and loaded: ${SIMS[*]}"
