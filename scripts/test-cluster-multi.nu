#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }
def banner [msg: string] {
    print $"(ansi purple)========================================(ansi reset)"
    print $"(ansi purple)($msg)(ansi reset)"
    print $"(ansi purple)========================================(ansi reset)"
}

# Generate cluster name
let timestamp = (date now | format date "%Y%m%d-%H%M%S")
let cluster_name = $"demo-multi-($timestamp)"
let worker_count = 2

banner "KINA MULTI-NODE INTEGRATION TEST"
print $"(ansi cyan)Cluster Name:(ansi reset) ($cluster_name)"
print $"(ansi cyan)Topology:(ansi reset) 1 control-plane + ($worker_count) workers"
print ""

if not ("Cargo.toml" | path exists) {
    err "Not in kina project root directory"
}

# Step 1: Build kina
log "Step 1: Building kina CLI..."
let build = (do { ^cargo build --release --quiet --manifest-path kina-cli/Cargo.toml } | complete)
if $build.exit_code != 0 {
    print $build.stderr
    err "Failed to build kina CLI"
}
log "Build complete"

# Step 2: Create multi-node cluster
log $"Step 2: Creating multi-node cluster '($cluster_name)' with ($worker_count) workers..."
let create = (do { ^cargo run --release --manifest-path kina-cli/Cargo.toml -- create $cluster_name --workers $worker_count --wait 120 } | complete)
print $create.stdout
if $create.exit_code != 0 {
    print $create.stderr
    err "Failed to create multi-node cluster"
}
log $"Cluster '($cluster_name)' created"

# Step 3: Verify node count
log "Step 3: Verifying cluster nodes..."
let kubeconfig_path = $"($env.HOME)/.kube/($cluster_name)"
let nodes_result = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" get nodes --no-headers } | complete)
if $nodes_result.exit_code != 0 {
    print $nodes_result.stderr
    err "Failed to list cluster nodes"
}

let node_lines = ($nodes_result.stdout | lines | where { |l| not ($l | is-empty) })
let node_count = ($node_lines | length)
let expected_count = ($worker_count + 1)
if $node_count != $expected_count {
    err $"Expected ($expected_count) nodes, found ($node_count)"
}
log $"Verified ($node_count) nodes in cluster"
for line in $node_lines {
    print $"   ($line)"
}

# Step 4: Wait for all nodes to be Ready
log "Step 4: Waiting for all nodes to be Ready..."
let wait_nodes = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready nodes --all --timeout=180s } | complete)
if $wait_nodes.exit_code != 0 {
    warn "Not all nodes became Ready in time"
    print $wait_nodes.stderr
} else {
    log "All nodes are Ready"
}

# Step 5: Check kina status shows all nodes
log "Step 5: Checking kina status..."
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $cluster_name --output json } | complete)
if $status_result.exit_code == 0 {
    let status = ($status_result.stdout | from json)
    let status_nodes = ($status | get nodes)
    log $"Kina reports ($status_nodes | length) nodes:"
    for node in $status_nodes {
        let ip = ($node | get ip_address | default "N/A")
        print $"   ($node.name) \(($node.role)\) - ($node.status) - IP: ($ip)"
    }
} else {
    warn "Could not get kina status"
}

# Step 6: Install nginx-ingress
log "Step 6: Installing nginx-ingress controller..."
let install_result = (do { ^cargo run --release --manifest-path kina-cli/Cargo.toml -- install nginx-ingress --cluster $cluster_name } | complete)
print $install_result.stdout
if $install_result.exit_code != 0 {
    print $install_result.stderr
    err "Failed to install nginx-ingress"
}
log "nginx-ingress installed"

# Step 7: Wait for nginx-ingress to be ready
log "Step 7: Waiting for nginx-ingress to be ready..."
let wait_result = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready pods -n nginx-ingress --all --timeout=180s } | complete)
if $wait_result.exit_code != 0 {
    warn "nginx-ingress did not become ready in time"
} else {
    log "nginx-ingress is ready"
}

# Step 8: Deploy demo application
log "Step 8: Deploying demo application..."

# Get DNS domain
let dns_raw = (do { ^container system dns list } | complete)
let dns_domain = if $dns_raw.exit_code == 0 {
    let first_line = ($dns_raw.stdout | lines | where { |l| not ($l | is-empty) } | first | default "")
    $first_line | str trim
} else {
    ""
}
let dns_domain = if ($dns_domain | is-empty) {
    warn "No DNS domain configured, using fallback: demo.local"
    "demo.local"
} else {
    $dns_domain
}

# Apply manifest with variable substitution
let manifest_path = "kina-cli/manifests/demo-app.yaml"
if not ($manifest_path | path exists) {
    err $"Demo app manifest not found: ($manifest_path)"
}

let manifest = (open $manifest_path --raw
    | str replace --all '${CLUSTER_NAME}' $cluster_name
    | str replace --all '${DNS_DOMAIN}' $dns_domain)

let apply = (do { $manifest | ^kubectl $"--kubeconfig=($kubeconfig_path)" apply -f - } | complete)
if $apply.exit_code != 0 {
    print $apply.stderr
    err "Failed to deploy demo application"
}

# Wait for pods to be ready
log "Waiting for demo app to be ready..."
let wait_app = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready pods -l app=kina-demo-app --timeout=60s } | complete)
if $wait_app.exit_code != 0 {
    warn "Demo app did not become ready in time"
} else {
    log "Demo application deployed and ready"
}

# Step 9: Verify pod scheduling across nodes
log "Step 9: Checking pod distribution across nodes..."
let pods_result = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" get pods -A -o wide --no-headers } | complete)
if $pods_result.exit_code == 0 {
    let pod_lines = ($pods_result.stdout | lines | where { |l| not ($l | is-empty) })
    let nodes_used = ($pod_lines | each { |l| $l | split row -r '\s+' | get 7 } | uniq)
    log $"Pods are scheduled on ($nodes_used | length) nodes"
}

# Step 10: Get cluster IP
let cluster_ip = if $status_result.exit_code == 0 {
    try { $status_result.stdout | from json | get nodes.0.ip_address } catch { "" }
} else {
    ""
}

# Success
banner "MULTI-NODE INTEGRATION TEST COMPLETE"
print ""
print $"(ansi green)Cluster Information:(ansi reset)"
print $"   Name:     (ansi cyan)($cluster_name)(ansi reset)"
print $"   Topology: (ansi cyan)1 control-plane + ($worker_count) workers(ansi reset)"
if not ($cluster_ip | is-empty) {
    print $"   CP IP:    (ansi cyan)($cluster_ip)(ansi reset)"
}
print $"   Kubeconfig: (ansi cyan)~/.kube/($cluster_name)(ansi reset)"
print ""
print $"(ansi green)Useful Commands:(ansi reset)"
print $"   Set kubectl:    (ansi cyan)mise run kina:use ($cluster_name)(ansi reset)"
print $"   Check status:   (ansi cyan)mise run kina -- status ($cluster_name)(ansi reset)"
print $"   View nodes:     (ansi cyan)kubectl --kubeconfig ~/.kube/($cluster_name) get nodes -o wide(ansi reset)"
print $"   View pods:      (ansi cyan)kubectl --kubeconfig ~/.kube/($cluster_name) get pods -A -o wide(ansi reset)"
print $"   Delete cluster: (ansi cyan)mise run kina -- delete ($cluster_name)(ansi reset)"
print $"   Cleanup all:    (ansi cyan)mise run test:cluster:cleanup(ansi reset)"
