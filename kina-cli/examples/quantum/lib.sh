#!/usr/bin/env bash
# Shared helpers for the quantum example scripts. Source this; do not execute.
set -euo pipefail

# Files sit at the example root (not under a scripts/ subdir).
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export REPO_ROOT

# versions.env is the single source of truth for all pins.
if [[ -f "$REPO_ROOT/versions.env" ]]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/versions.env"
fi

# Runs against your existing kina cluster. These scripts do NOT manage a
# repo-local kubeconfig — set KUBECONFIG (or use your active kubectl context)
# to point at the cluster before running. CLUSTER_NAME is used by the scripts
# that call `kina` (build-images.sh load, optional teardown --cluster).
export KUBECONFIG="${KUBECONFIG:-$HOME/.kube/config}"
CLUSTER_NAME="${CLUSTER_NAME:-quantum}"
export CLUSTER_NAME

# log goes to stderr: several callers capture stdout (tail -1 job-name
# contract in the tests), so stdout is reserved for machine-readable output.
log()  { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >&2; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "required command not found: $1 ($2)"; }
