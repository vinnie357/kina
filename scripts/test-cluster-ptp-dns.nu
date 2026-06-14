#!/usr/bin/env nu

# Regression test for multi-node PTP cross-node DNS.
#
# PTP CNI is point-to-point, so a multi-node PTP cluster needs per-node pod
# subnets plus cross-node routes for pods on workers to reach CoreDNS on the
# control-plane. Without that fix this test FAILS: a pod scheduled on a worker
# cannot resolve `kubernetes.default` (connection times out).
#
# Self-contained: creates a 1 control-plane + 2 worker PTP cluster, schedules a
# probe pod pinned to a worker, asserts in-cluster DNS resolves, then cleans up.

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }
def banner [msg: string] {
    print $"(ansi purple)========================================(ansi reset)"
    print $"(ansi purple)($msg)(ansi reset)"
    print $"(ansi purple)========================================(ansi reset)"
}

let timestamp = (date now | format date "%Y%m%d-%H%M%S")
let cluster_name = $"demo-ptpdns-($timestamp)"
let kubeconfig_path = $"($env.HOME)/.kube/($cluster_name)"

banner "KINA MULTI-NODE PTP CROSS-NODE DNS TEST"
print $"(ansi cyan)Cluster Name:(ansi reset) ($cluster_name)"
print $"(ansi cyan)Topology:(ansi reset) 1 control-plane + 2 workers, CNI ptp"
print ""

if not ("Cargo.toml" | path exists) {
    err "Not in kina project root directory"
}

# Always clean up the cluster, even if an assertion fails.
def cleanup [name: string] {
    log $"Cleaning up cluster '($name)'..."
    do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- delete $name } | complete | ignore
}

# Step 1: Build
log "Step 1: Building kina CLI..."
let build = (do { ^cargo build --release --quiet --manifest-path kina-cli/Cargo.toml } | complete)
if $build.exit_code != 0 {
    print $build.stderr
    err "Failed to build kina CLI"
}
log "Build complete"

# Step 2: Create multi-node PTP cluster
log $"Step 2: Creating multi-node PTP cluster '($cluster_name)'..."
let create = (do { ^cargo run --release --manifest-path kina-cli/Cargo.toml -- create $cluster_name --workers 2 --wait 120 --cni ptp } | complete)
print $create.stdout
if $create.exit_code != 0 {
    print $create.stderr
    err "Failed to create multi-node cluster"
}
log $"Cluster '($cluster_name)' created"

# Step 3: Wait for all nodes Ready
log "Step 3: Waiting for all nodes to be Ready..."
let wait_nodes = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready nodes --all --timeout=180s } | complete)
if $wait_nodes.exit_code != 0 {
    cleanup $cluster_name
    err "Not all nodes became Ready in time"
}
log "All nodes are Ready"

# Step 4: Pick a worker node
let worker = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" get nodes -l '!node-role.kubernetes.io/control-plane' -o 'jsonpath={.items[0].metadata.name}' } | complete)
let worker_node = ($worker.stdout | str trim)
if ($worker_node | is-empty) {
    cleanup $cluster_name
    err "Could not find a worker node"
}
log $"Probing DNS from worker node: ($worker_node)"

# Step 5: Schedule a probe pod pinned to the worker (nodeName bypasses scheduler/taints)
let overrides = ({ spec: { nodeName: $worker_node } } | to json)
do { ^kubectl $"--kubeconfig=($kubeconfig_path)" delete pod ptp-dnstest --ignore-not-found } | complete | ignore
let run_pod = (do {
    ^kubectl $"--kubeconfig=($kubeconfig_path)" run ptp-dnstest --image=busybox:1.36 --restart=Never --overrides $overrides --command -- sleep 300
} | complete)
if $run_pod.exit_code != 0 {
    print $run_pod.stderr
    cleanup $cluster_name
    err "Failed to schedule probe pod"
}

let wait_pod = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready pod/ptp-dnstest --timeout=60s } | complete)
if $wait_pod.exit_code != 0 {
    cleanup $cluster_name
    err "Probe pod did not become Ready"
}

# Step 6: THE ASSERTION — resolve the kubernetes service FQDN from the worker pod.
# CoreDNS runs on the control-plane; without cross-node routing the query to the
# kube-dns ClusterIP times out ("no servers could be reached"). The FQDN is used
# (not the bare name) so a reachable CoreDNS returns an answer rather than NXDOMAIN.
log "Step 6: Resolving kubernetes.default.svc.cluster.local from the worker pod..."
let dns = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" exec ptp-dnstest -- nslookup kubernetes.default.svc.cluster.local } | complete)
print $dns.stdout
print $dns.stderr

cleanup $cluster_name

if $dns.exit_code != 0 {
    err "DNS resolution FAILED from worker pod — cross-node PTP routing is broken"
}
if not ($dns.stdout | str contains "10.96.0.1") {
    err "DNS resolved but did not return the expected kubernetes ClusterIP 10.96.0.1"
}

banner "PTP CROSS-NODE DNS TEST PASSED"
log "Worker pod resolved kubernetes.default via CoreDNS on the control-plane"
