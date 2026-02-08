#!/usr/bin/env nu

# Only run on macOS
if $nu.os-info.name != "macos" {
    print "Colima is only available on macOS. Ubuntu/Linux has native Docker."
    exit 0
}

# Use mise exec to run colima with temporary tool activation
print "Checking colima status..."
let status = (mise exec lima@latest colima@latest -- colima status --profile default | complete)
if $status.exit_code != 0 {
    print "Starting colima..."
    mise exec lima@latest colima@latest -- colima start --vm-type vz --vz-rosetta --runtime docker
} else {
    print "Colima is already running"
}
