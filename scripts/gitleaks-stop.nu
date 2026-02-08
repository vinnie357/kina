#!/usr/bin/env nu

print "Stopping container runtimes..."

# Stop Apple Container
if (which container | is-not-empty) {
    let status = (do { ^container system status } | complete)
    if $status.exit_code == 0 {
        print "  Stopping Apple Container..."
        do { ^container system stop } | complete
        print "  Apple Container stopped"
    }
}

# Stop Docker
if (which docker | is-not-empty) {
    let status = (do { ^docker info } | complete)
    if $status.exit_code == 0 {
        print "  Stopping Docker..."
        if $nu.os-info.name == "macos" {
            do { ^osascript -e 'quit app "Docker"' } | complete
            print "  Docker stopped"
        } else {
            print "  On Linux, stop Docker manually: sudo systemctl stop docker"
        }
    }
}

# Stop Colima
if $nu.os-info.name == "macos" and (which mise | is-not-empty) {
    let status = (do { ^mise exec lima@latest colima@latest -- colima status } | complete)
    if $status.exit_code == 0 {
        print "  Stopping Colima..."
        do { ^mise exec lima@latest colima@latest -- colima stop } | complete
        print "  Colima stopped"
    }
}

print "Done"
