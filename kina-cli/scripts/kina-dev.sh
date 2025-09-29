#!/bin/bash
set -e

# Build the debug binary (always rebuild for development)
echo "Building kina CLI (debug mode)..."
cargo build

# Run the CLI with all arguments
exec ./target/debug/kina "$@"