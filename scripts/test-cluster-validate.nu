#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }

# Find the most recent demo cluster
let list_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet list } | complete)
let latest_demo = ($list_result.stdout | lines | where { |l| $l starts-with "demo-" } | last | default "")

if ($latest_demo | is-empty) {
    err "No demo clusters found. Run 'mise run test:cluster' first."
}

log $"Testing demo cluster: ($latest_demo)"

# Get cluster IP using kina status
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $latest_demo --output json } | complete)
let cluster_ip = if $status_result.exit_code == 0 {
    try { $status_result.stdout | from json | get nodes.0.ip_address } catch { "" }
} else {
    ""
}

if ($cluster_ip | is-empty) {
    err $"Could not find IP for cluster ($latest_demo)"
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

log $"Cluster IP: ($cluster_ip)"
log $"DNS Domain: ($dns_domain)"

# Test the ingress
log "Testing ingress routing..."
let kubeconfig_path = $"($env.HOME)/.kube/($latest_demo)"
let hostname = $"($latest_demo)-control-plane.($dns_domain)"

log $"Testing with hostname: ($hostname)"

let test_result = (do {
    ^kubectl $"--kubeconfig=($kubeconfig_path)" run test-curl --image=nginx:alpine --rm -i --restart=Never -- curl -s -H $"Host: ($hostname)" $"http://($cluster_ip)"
} | complete)

if ($test_result.stdout | str contains "Kina Demo Success") {
    log "Ingress test passed! Demo cluster is working correctly."
} else {
    err "Ingress test failed. Check cluster status."
}
