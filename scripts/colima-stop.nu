#!/usr/bin/env nu

# Only run on macOS
if $nu.os-info.name != "macos" {
    print "Colima is only available on macOS."
    exit 0
}

mise exec lima@latest colima@latest -- colima stop
