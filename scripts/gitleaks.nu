#!/usr/bin/env nu

let repo_root = (git rev-parse --show-toplevel | str trim)
let image = "zricethezav/gitleaks"

# Build gitleaks arguments
mut args = ["detect" "--source=/code" "-v"]

# Auto-detect baseline file
let baseline_path = ($repo_root | path join ".gitleaks-baseline.json")
if ($baseline_path | path exists) {
    let baseline_name = ($baseline_path | path basename)
    $args = ($args | append $"--baseline-path=/code/($baseline_name)")
    print $"Using baseline: ($baseline_name)"
}

# Ensure Apple Container is running
if (which container | is-empty) {
    print "Apple Container CLI not found. Use mise run gitleaks:docker or mise run gitleaks:colima"
    exit 1
}

let status = (do { ^container system status } | complete)
if $status.exit_code != 0 {
    print "Starting Apple Container..."
    do { ^container system start } | complete
}

let final_args = $args
print $"Scanning: ($repo_root)"
let result = (do { ^container run --rm -v $"($repo_root):/code" $image ...$final_args } | complete)
print $result.stdout

if $result.exit_code == 0 {
    print "No secrets detected"
} else if $result.exit_code == 1 {
    print "Secrets detected!"
    print $result.stderr
} else {
    print $"Error: ($result.stderr)"
}

exit $result.exit_code
