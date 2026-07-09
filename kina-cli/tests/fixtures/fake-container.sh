#!/bin/sh
# Deterministic stand-in for the Apple Container CLI (kina-47).
#
# cli_tests spawn the real kina binary, and kina shells out to the container
# CLI — an external boundary. Pointing tests at this stub keeps them hermetic:
# results never depend on the host's container service state or whatever
# kina-labeled containers a developer has lying around.
case "$1" in
  --version)
    # Must match parse_version_output's "CLI version " prefix and pass
    # validate_version's minimum-version check.
    echo "container CLI version 1.0.0 (stub)"
    ;;
  *)
    # `list --format json --all` and friends: an empty container list.
    echo "[]"
    ;;
esac
