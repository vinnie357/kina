#!/usr/bin/env nu

if (which container | is-empty) {
    print "Apple Container CLI not found — install Apple Container before running tests"
    exit 1
}

let status = (do { ^container system status } | complete)
if $status.exit_code != 0 {
    print "Starting Apple Container..."
    let result = (do { ^container system start } | complete)
    if $result.exit_code != 0 {
        print "Failed to start Apple Container"
        print $result.stderr
        exit 1
    }
    print "Apple Container started"
} else {
    print "Apple Container already running"
}
