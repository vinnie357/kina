#!/bin/bash
set -e

# Build the release binary if it doesn't exist or if source files are newer
if [ ! -f "./target/release/kina" ] || [ "src/main.rs" -nt "./target/release/kina" ]; then
    echo "Building kina CLI..."
    cargo build --release
fi

# Run the CLI with all arguments
exec ./target/release/kina "$@"