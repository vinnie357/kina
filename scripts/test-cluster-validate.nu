#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }

# Find the most recent demo cluster
let list_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet list } | complete)
let demo_clusters = ($list_result.stdout | lines | where { |l| $l starts-with "demo-" })

if ($demo_clusters | is-empty) {
    log "No demo clusters found. Run 'mise run test:cluster' or 'mise run test:cluster:multi' first."
    exit 0
}

let latest_demo = ($demo_clusters | last)

log $"Testing demo cluster: ($latest_demo)"

# Get cluster status JSON
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $latest_demo --output json } | complete)
if $status_result.exit_code != 0 {
    err $"Could not get status for cluster ($latest_demo)"
}

let status = ($status_result.stdout | from json)
let nodes = ($status | get nodes)

# Determine which nodes to test:
# - Multi-node: test all worker IPs (nginx-ingress runs on workers, control-plane is tainted)
# - Single-node: test the only node's IP
let worker_nodes = ($nodes | where { |n| $n.role == "worker" })
let test_nodes = if ($worker_nodes | is-not-empty) {
    $worker_nodes
} else {
    $nodes
}

# Get DNS domain
let dns_raw = (do { ^container system dns list } | complete)
let dns_domain = if $dns_raw.exit_code == 0 {
    $dns_raw.stdout | lines | where { |l| not ($l | is-empty) } | first | default "" | str trim
} else {
    ""
}

if ($dns_domain | is-empty) {
    err "Could not detect DNS domain from Apple Container"
}

let hostname = $"($latest_demo)-control-plane.($dns_domain)"
log $"DNS Domain: ($dns_domain)"
let node_count = ($test_nodes | length)
log $"Testing ($node_count) nodes for ingress..."

# Helper to extract pod name from response HTML
def extract-pod-name [html: string]: nothing -> string {
    let result = ($html | parse --regex 'Pod Name</div>\s*<div class="info-value">([^<]+)</div>' | get -o 0 | get -o capture0 | default "unknown")
    $result
}

def extract-pod-ip [html: string]: nothing -> string {
    let result = ($html | parse --regex 'Pod IP</div>\s*<div class="info-value">([^<]+)</div>' | get -o 0 | get -o capture0 | default "unknown")
    $result
}

def extract-node-name [html: string]: nothing -> string {
    let result = ($html | parse --regex 'Node</div>\s*<div class="info-value">([^<]+)</div>' | get -o 0 | get -o capture0 | default "unknown")
    $result
}

# === Phase 1: Test each node has a working ingress ===
print ""
log "Phase 1: Testing ingress on each node..."

mut passed = 0
mut failed = 0

for node in $test_nodes {
    let ip = ($node | get ip_address | default "")
    if ($ip | is-empty) {
        warn $"No IP for node ($node.name), skipping"
        $failed = $failed + 1
        continue
    }

    log $"  ($node.name) at ($ip)..."
    let result = (do { ^curl -s --max-time 10 -H $"Host: ($hostname)" $"http://($ip)" } | complete)

    if ($result.stdout | str contains "Kina Demo Success") {
        $passed = $passed + 1
        let pod = (extract-pod-name $result.stdout)
        let pod_ip = (extract-pod-ip $result.stdout)
        let on_node = (extract-node-name $result.stdout)
        log $"    Served by: ($pod) [($pod_ip)] on ($on_node)"
    } else {
        $failed = $failed + 1
        warn $"    Ingress not responding on ($node.name)"
    }
}

if $passed == 0 {
    err "All ingress tests failed. Check cluster status."
}

if $failed > 0 {
    warn $"($passed)/($passed + $failed) nodes passed ingress test"
} else {
    log $"All ($passed) nodes passed ingress test"
}

# === Phase 2: Test load balancing by hitting the same node multiple times ===
print ""
log "Phase 2: Testing load balancing..."

# Pick the first working node
let target_node = ($test_nodes | first)
let target_ip = ($target_node | get ip_address | default "")

if ($target_ip | is-empty) {
    warn "Cannot test load balancing - no target IP"
} else {
    let request_count = 10
    log $"  Sending ($request_count) requests to ($target_node.name) at ($target_ip)..."

    mut all_pods = []
    for i in 1..$request_count {
        let result = (do { ^curl -s --max-time 5 -H $"Host: ($hostname)" $"http://($target_ip)" } | complete)
        if ($result.stdout | str contains "Kina Demo Success") {
            let pod = (extract-pod-name $result.stdout)
            $all_pods = ($all_pods | append $pod)
        }
    }

    let unique_pods = ($all_pods | uniq)
    let unique_count = ($unique_pods | length)
    let total_hits = ($all_pods | length)

    if $unique_count > 1 {
        log $"  Load balancing confirmed: ($unique_count) different pods responded out of ($total_hits) requests"
        for pod in $unique_pods {
            let count = ($all_pods | where { |p| $p == $pod } | length)
            log $"    ($pod): ($count) responses"
        }
    } else if $unique_count == 1 {
        let pod = ($unique_pods | first)
        warn $"  No load balancing observed: all ($total_hits) requests served by ($pod)"
        warn "  This may indicate kube-proxy is not routing across pods"
    } else {
        warn "  No successful responses for load balancing test"
    }
}

print ""
log "Validation complete."
