#!/usr/bin/env nu

def log [msg: string] { print $"(ansi green)[INFO](ansi reset) ($msg)" }
def warn [msg: string] { print $"(ansi yellow)[WARN](ansi reset) ($msg)" }

log "Finding demo clusters to clean up..."

let list_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet list } | complete)
let demo_clusters = ($list_result.stdout | lines | where { |l| $l starts-with "demo-" })

if ($demo_clusters | is-empty) {
    log "No demo clusters found to clean up"

    # Check for orphaned kubeconfig files
    let kube_dir = $"($env.HOME)/.kube"
    let demo_configs = (glob $"($kube_dir)/demo-*")
    if not ($demo_configs | is-empty) {
        log "Cleaning up orphaned kubeconfig files..."
        for f in $demo_configs { rm $f }
        log "Cleaned up kubeconfig files"
    }
    exit 0
}

print ""
print $"(ansi yellow)Found demo clusters:(ansi reset)"
for c in $demo_clusters { print $"  ($c)" }
print ""

for cluster_name in $demo_clusters {
    log $"Deleting cluster: ($cluster_name)"

    let delete_result = (do { ^cargo run --release --quiet --manifest-path kina-cli/Cargo.toml -- --quiet delete $cluster_name } | complete)
    if $delete_result.exit_code == 0 {
        log $"Deleted cluster ($cluster_name)"
    } else {
        warn $"Failed to delete ($cluster_name), trying manual cleanup..."
        do { ^container stop $"($cluster_name)-control-plane" } | complete
        do { ^container delete $"($cluster_name)-control-plane" } | complete
    }

    let kubeconfig = $"($env.HOME)/.kube/($cluster_name)"
    if ($kubeconfig | path exists) {
        rm $kubeconfig
        log $"Removed kubeconfig for ($cluster_name)"
    }
}

log "Demo cleanup completed"
