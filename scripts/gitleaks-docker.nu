#!/usr/bin/env nu

let repo_root = (git rev-parse --show-toplevel | str trim)
let image = "zricethezav/gitleaks"

mut args = ["detect" "--source=/code" "-v"]

let baseline_path = ($repo_root | path join ".gitleaks-baseline.json")
if ($baseline_path | path exists) {
    let baseline_name = ($baseline_path | path basename)
    $args = ($args | append $"--baseline-path=/code/($baseline_name)")
    print $"Using baseline: ($baseline_name)"
}

if (which docker | is-empty) {
    print "Docker CLI not found"
    exit 1
}

let status = (do { ^docker info } | complete)
if $status.exit_code != 0 {
    print "Starting Docker..."
    if $nu.os-info.name == "macos" {
        do { ^open -a Docker } | complete
        print "Waiting for Docker to start..."
        mut attempts = 0
        loop {
            sleep 2sec
            $attempts = $attempts + 1
            let check = (do { ^docker info } | complete)
            if $check.exit_code == 0 { break }
            if $attempts >= 30 {
                print "Docker failed to start"
                exit 1
            }
        }
    } else {
        print "Docker is not running. Start it manually."
        exit 1
    }
}

let final_args = $args
print $"Scanning: ($repo_root)"
let result = (do { ^docker run --rm -v $"($repo_root):/code" $image ...$final_args } | complete)
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
