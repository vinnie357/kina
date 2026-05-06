#!/usr/bin/env nu

let platform_flags = [
    "-P" "macos-26=-self-hosted"
    "-P" "macos-15=-self-hosted"
]

act push ...$platform_flags
