#!/usr/bin/env nu

# Detect OS and set flags for Mac
let act_flags = if $nu.os-info.name == "macos" {
    [
        "--container-architecture" "linux/amd64"
        "--container-daemon-socket" "-"
        "-P" "ubuntu-latest=catthehacker/ubuntu:act-latest"
    ]
} else {
    [
        "-P" "ubuntu-latest=catthehacker/ubuntu:act-latest"
    ]
}

act pull_request ...$act_flags
