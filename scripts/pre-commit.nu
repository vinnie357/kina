#!/usr/bin/env nu

print "Running pre-commit checks..."

let cli_steps = [
    { name: "fmt check", cmd: "cargo fmt -- --check" }
    { name: "clippy", cmd: "cargo clippy -- -D warnings" }
    { name: "test", cmd: "cargo test" }
]

cd kina-cli

for step in $cli_steps {
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

# audit runs from repo root where Cargo.lock lives
cd ..
print "  audit..."
let result = (do { nu -c "cargo audit" } | complete)
if $result.exit_code != 0 {
    print "  FAIL audit"
    print $result.stdout
    print $result.stderr
    exit $result.exit_code
}
print "  OK audit"

print "  gitleaks..."
let result = (do { nu -c "mise run gitleaks" } | complete)
if $result.exit_code != 0 {
    print "  FAIL gitleaks"
    print $result.stdout
    print $result.stderr
    exit $result.exit_code
}
print "  OK gitleaks"

print "pre-commit passed"
