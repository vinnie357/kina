#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }

mut selected_cluster = ($env | get -i usage_cluster | default "")

if ($selected_cluster | is-empty) {
    # Auto-detect running clusters
    let list_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet list } | complete)
    let clusters = ($list_result.stdout | lines | where { |l| not ($l starts-with "No clusters") and not ($l | is-empty) })

    if ($clusters | is-empty) {
        err "No kina clusters found. Create one with: mise run test:cluster"
    }

    let count = ($clusters | length)

    if $count == 1 {
        $selected_cluster = ($clusters | first)
        log $"Auto-selected cluster: (ansi cyan)($selected_cluster)(ansi reset)"
    } else {
        print $"(ansi yellow)Multiple clusters found:(ansi reset)"
        for c in $clusters { print $"  - ($c)" }
        err "Specify a cluster name: mise run kina:use <cluster-name>"
    }
}

# Verify cluster exists
let kubeconfig_path = $"($env.HOME)/.kube/($selected_cluster)"
if not ($kubeconfig_path | path exists) {
    err $"Kubeconfig not found: ($kubeconfig_path) — is cluster '($selected_cluster)' running?"
}

# Set kubectl context
let ctx = (do { ^kubectl config use-context $selected_cluster } | complete)
if $ctx.exit_code != 0 {
    warn $"Context '($selected_cluster)' not in merged config, using individual kubeconfig"
}

# Get cluster IP via kina status
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $selected_cluster --output json } | complete)
let cluster_ip = if $status_result.exit_code == 0 {
    try { $status_result.stdout | from json | get nodes.0.ip_address } catch { "" }
} else {
    ""
}

# Show confirmation
let current_ctx = (do { ^kubectl config current-context } | complete)
print ""
print $"(ansi green)kubectl configured for cluster: (ansi cyan)($selected_cluster)(ansi reset)"
print $"   Context:    (ansi cyan)($current_ctx.stdout | str trim)(ansi reset)"
print $"   Kubeconfig: (ansi cyan)($kubeconfig_path)(ansi reset)"
if not ($cluster_ip | is-empty) {
    print $"   Cluster IP: (ansi cyan)($cluster_ip)(ansi reset)"
}
print ""
print $"(ansi green)Try:(ansi reset) kubectl get nodes"
