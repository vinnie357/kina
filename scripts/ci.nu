#!/usr/bin/env nu

print "Running CI pipeline..."

let steps = [
    { name: "fmt check", cmd: "cargo fmt -- --check" }
    { name: "clippy", cmd: "cargo clippy -- -D warnings" }
    { name: "test", cmd: "cargo test" }
]

cd kina-cli

for step in $steps {
    print $"  ($step.name)..."
    let result = (do { nu -c $step.cmd } | complete)
    if $result.exit_code != 0 {
        print $"  FAIL ($step.name)"
        print $result.stdout
        print $result.stderr
        exit $result.exit_code
    }
    print $"  OK ($step.name)"
}

print "CI passed"
