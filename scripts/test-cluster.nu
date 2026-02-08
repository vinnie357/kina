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
let cluster_name = $"demo-($timestamp)"

banner "KINA INTEGRATION TEST CLUSTER"
print $"(ansi cyan)Cluster Name:(ansi reset) ($cluster_name)"
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

# Step 2: Create cluster
log $"Step 2: Creating cluster '($cluster_name)'..."
let create = (do { ^cargo run --release --manifest-path kina-cli/Cargo.toml -- create $cluster_name --wait 60 } | complete)
print $create.stdout
if $create.exit_code != 0 {
    print $create.stderr
    err "Failed to create cluster"
}
log $"Cluster '($cluster_name)' created"

# Step 3: Install nginx-ingress
log "Step 3: Installing nginx-ingress controller..."
let install_result = (do { ^cargo run --release --manifest-path kina-cli/Cargo.toml -- install nginx-ingress --cluster $cluster_name } | complete)
print $install_result.stdout
if $install_result.exit_code != 0 {
    print $install_result.stderr
    err "Failed to install nginx-ingress"
}
log "nginx-ingress installed"

# Step 4: Wait for nginx-ingress to be ready
log "Step 4: Waiting for nginx-ingress to be ready..."
let kubeconfig_path = $"($env.HOME)/.kube/($cluster_name)"
let wait_result = (do { ^kubectl $"--kubeconfig=($kubeconfig_path)" wait --for=condition=Ready pods -n nginx-ingress --all --timeout=180s } | complete)
if $wait_result.exit_code != 0 {
    err "nginx-ingress did not become ready in time"
}
log "nginx-ingress is ready"

# Step 5: Deploy demo application
log "Step 5: Deploying demo application..."

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
    err "Demo app did not become ready in time"
}
log "Demo application deployed and ready"

# Step 6: Get cluster information
log "Step 6: Getting cluster information..."
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $cluster_name --output json } | complete)
let cluster_ip = if $status_result.exit_code == 0 {
    try { $status_result.stdout | from json | get nodes.0.ip_address } catch { "" }
} else {
    ""
}

if ($cluster_ip | is-empty) {
    err "Could not determine cluster IP address"
}

log $"Cluster IP: ($cluster_ip)"

# Step 7: Test the demo application
log "Step 7: Testing demo application..."
if not ($dns_domain | is-empty) and $dns_domain != "demo.local" {
    let test_host = $"($cluster_name)-control-plane.($dns_domain)"
    log $"Testing with hostname: ($test_host)"

    let curl_result = (do { ^curl -s -H $"Host: ($test_host)" $"http://($cluster_ip)" } | complete)
    if ($curl_result.stdout | str contains "Kina Demo") {
        log "Demo application is accessible and working!"
    } else {
        warn "Demo application test failed, but cluster is ready for manual testing"
    }
} else {
    log "No DNS domain configured, skipping automated test"
}

# Success
banner "INTEGRATION TEST CLUSTER READY"
print ""
print $"(ansi green)Cluster Information:(ansi reset)"
print $"   Name: (ansi cyan)($cluster_name)(ansi reset)"
print $"   IP: (ansi cyan)($cluster_ip)(ansi reset)"
print $"   Kubeconfig: (ansi cyan)~/.kube/($cluster_name)(ansi reset)"
print ""
print $"(ansi green)Access Your Demo App:(ansi reset)"
print ""
print $"(ansi yellow)Direct curl test:(ansi reset)"
print $"   curl -H \"Host: ($cluster_name)-control-plane.($dns_domain)\" http://($cluster_ip)"
print ""
print $"(ansi green)Useful Commands:(ansi reset)"
print $"   Set kubectl:    (ansi cyan)mise run kina:use ($cluster_name)(ansi reset)"
print $"   Check status:   (ansi cyan)mise run kina -- status ($cluster_name)(ansi reset)"
print $"   View pods:      (ansi cyan)kubectl --kubeconfig ~/.kube/($cluster_name) get pods -A(ansi reset)"
print $"   Delete cluster: (ansi cyan)mise run kina -- delete ($cluster_name)(ansi reset)"
