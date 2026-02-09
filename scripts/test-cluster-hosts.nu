#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }
def err [msg: string] { print $"(ansi red)[ERROR](ansi reset) ($msg)"; exit 1 }

let marker_begin = "# BEGIN kina demo cluster"
let marker_end = "# END kina demo cluster"

# Check for --clean flag
let clean_mode = ($env | get -o KINA_HOSTS_CLEAN | default "" | str trim) != ""

# Read current /etc/hosts
let hosts_content = (open /etc/hosts)

# Remove any existing kina block
let cleaned = if ($hosts_content | str contains $marker_begin) {
    let before = ($hosts_content | split row $marker_begin | first | str trim)
    let after_parts = ($hosts_content | split row $marker_end)
    let after = if ($after_parts | length) > 1 { $after_parts | last | str trim } else { "" }
    [$before $after] | str join "\n" | str trim
} else {
    $hosts_content | str trim
}

# Write /etc/hosts via temp file + sudo cp
def write-hosts [content: string] {
    let tmp = $"/tmp/kina-hosts-(random chars -l 8)"
    $content | save -f $tmp
    let result = (do { ^sudo cp $tmp /etc/hosts } | complete)
    rm -f $tmp
    if $result.exit_code != 0 {
        err "Failed to update /etc/hosts - sudo required"
    }
}

if $clean_mode {
    if ($hosts_content | str contains $marker_begin) {
        log "Removing kina entries from /etc/hosts..."
        write-hosts $"($cleaned)\n"
        log "Cleaned /etc/hosts"
    } else {
        log "No kina entries found in /etc/hosts"
    }
    exit 0
}

# Find the most recent demo cluster
let list_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet list } | complete)
let demo_clusters = ($list_result.stdout | lines | where { |l| $l starts-with "demo-" })

if ($demo_clusters | is-empty) {
    log "No demo clusters found. Run 'mise run test:cluster' or 'mise run test:cluster:multi' first."
    exit 0
}

let latest_demo = ($demo_clusters | last)
log $"Found cluster: ($latest_demo)"

# Get cluster status JSON
let status_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet status $latest_demo --output json } | complete)
if $status_result.exit_code != 0 {
    err $"Could not get status for cluster ($latest_demo)"
}

let status = ($status_result.stdout | from json)
let nodes = ($status | get nodes)

# Use worker IPs for multi-node, all node IPs for single-node
let worker_nodes = ($nodes | where { |n| $n.role == "worker" })
let target_nodes = if ($worker_nodes | is-not-empty) {
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

# Pick the first worker with a valid IP (/etc/hosts only resolves the first match)
let target = ($target_nodes | where { |n| ($n | get ip_address | default "") != "" } | first)
let target_ip = ($target | get ip_address)

let entry = $"($target_ip)\t($hostname)"

# Build the new hosts block
let hosts_block = $"($marker_begin)\n($entry)\n($marker_end)"
let new_hosts = $"($cleaned)\n\n($hosts_block)\n"

log "Adding to /etc/hosts:"
log $"  ($entry)"

write-hosts $new_hosts

print ""
log $"Browse to: http://($hostname)"
log "Run 'mise run test:cluster:hosts:clean' to remove entries"
