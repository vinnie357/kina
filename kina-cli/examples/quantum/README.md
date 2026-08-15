# Quantum Simulator Batch Jobs on Kina (with Kueue)

Run quantum-circuit **simulators** as Kubernetes `Job`s on a kina cluster, with
[Kueue](https://kueue.sigs.k8s.io) doing batch admission and quota control.
Three backends — `qiskit-aer`, `qsim`, and `pennylane` — each build a native
arm64 image, read an OpenQASM circuit, and print measurement counts. Kueue
`ClusterQueue`/`LocalQueue` quotas (derived live from your nodes) demonstrate
what happens when you submit more work than the cluster can run at once.

This example assumes you already have a running kina cluster. It does **not**
create or delete your cluster (except the opt-in `teardown.sh --cluster`).

## Prerequisites

- Apple Silicon Mac with the Apple [`container`](https://github.com/apple/container)
  runtime (for building the simulator images).
- `kubectl`, `helm` (>= 3.14, for the Kueue OCI chart), `jq`, and `envsubst`
  (`brew install kubectl helm jq gettext`).
- A kina cluster with **at least 2 workers** — each worker backs one Kueue
  flavor (`cpu-heavy` / `memory-heavy`).

```bash
# From the repo root (or with an installed `kina` binary):
kina create quantum --workers 2 --wait 300
kina export quantum --format kubeconfig --output ~/.kube/quantum
export KUBECONFIG=~/.kube/quantum

# metrics-server lets `kubectl top nodes` work (handy, not strictly required):
kina install metrics-server --cluster quantum
```

All the scripts below run against whatever `KUBECONFIG` / kubectl context is
active, and default to a cluster named `quantum` (override with `CLUSTER_NAME`).

## Setup

Run from this directory (`kina-cli/examples/quantum/`), in order:

```bash
./label-nodes.sh      # label the 2 workers cpu-heavy / memory-heavy
./install-kueue.sh    # helm-install Kueue, derive quotas, apply the queue objects
./build-images.sh     # build all 3 simulator images and `kina load` them
```

`build-images.sh qiskit-aer` builds just one backend. `install-kueue.sh` sizes
each flavor's quota at 85% of that worker's allocatable CPU/memory.

## Run a simulation

Submit the included 2-qubit Bell-state circuit:

```bash
./submit-circuit.sh --simulator qiskit-aer --circuit circuits/bell.qasm --shots 1024
```

It prints watch/log hints and the created job name as its last line. Watch
admission and completion, then read the counts:

```bash
kubectl -n quantum-sims get workloads -w
kubectl -n quantum-sims get job <JOB_NAME> -w
kubectl -n quantum-sims logs job/<JOB_NAME>     # look for: Measurement counts: {...}
```

Swap `--simulator qsim` or `--simulator pennylane` for the other backends, and
`--class memory-heavy` to target the other worker/flavor.

## Tests

```bash
./tests/e2e-smoke-test.sh qiskit-aer      # submit Bell circuit, assert 00/11 correlation
./tests/e2e-smoke-test.sh qsim
./tests/e2e-smoke-test.sh pennylane
./tests/quota-exhaustion-test.sh          # oversubscribe, prove Kueue queues >=1 workload
```

## Teardown

```bash
./teardown.sh             # remove only this example's namespace, queues, flavors, node labels
./teardown.sh --cluster   # also delete the whole `quantum` cluster (kina delete quantum)
```

## Limitations

- **PennyLane is fixed at 2 qubits.** `simulators/pennylane/run_circuit.py`
  hardcodes `qml.device("default.qubit", wires=2, ...)`, so only 2-qubit QASM
  circuits work on the `pennylane` backend. `qsim` and `qiskit-aer` size
  themselves from the circuit and handle arbitrary circuits within resource
  limits.
- **`--namespace` requires a matching `LocalQueue`.** Kueue only admits jobs
  whose `kueue.x-k8s.io/queue-name` label matches a `LocalQueue` in that
  namespace. Only `quantum-sims` gets one provisioned
  (`kueue/local-queue.yaml`); jobs submitted elsewhere sit unadmitted.

## arm64 note

The three images build **natively for arm64** on Apple Silicon (the pinned
`python:3.13.14-slim` base resolves to linux/arm64). Because the images are
built locally and `kina load`ed with `imagePullPolicy: Never`, no registry is
involved — unlike public amd64 images, which would run under emulation.

## Layout

```
.
├── versions.env                  tool/package pins (sourced by lib.sh)
├── circuits/bell.qasm            2-qubit Bell-state example circuit
├── kueue/                        namespace, ResourceFlavors, ClusterQueue template, LocalQueue
├── simulators/{qiskit-aer,qsim,pennylane}/   Dockerfile, run_circuit.py, job-template.yaml
├── lib.sh                        shared helpers (REPO_ROOT, CLUSTER_NAME, log/die/need)
├── label-nodes.sh               label workers cpu-heavy / memory-heavy
├── install-kueue.sh             install Kueue + apply queues with derived quotas
├── build-images.sh              build simulator images + kina load
├── submit-circuit.sh            render + submit a circuit job
├── teardown.sh                  remove example resources (or --cluster)
└── tests/                       e2e smoke test + quota-exhaustion test
```
