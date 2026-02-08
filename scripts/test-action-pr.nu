#!/usr/bin/env nu

let colima_socket = $"($env.HOME)/.colima/docker.sock"
let use_colima = ($colima_socket | path exists)

# Set DOCKER_HOST for colima
if $use_colima {
    $env.DOCKER_HOST = $"unix://($colima_socket)"
}

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
