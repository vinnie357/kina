//! Hardening tests for `kina verify` — empty-node-IP FAIL path.
//!
//! These tests are INTENTIONALLY RED until the implementation in
//! kina_cli::core::verify and kina-cli/src/cli/cluster.rs is updated.
//!
//! Contract:
//! - `http_layer_pass(node_ips, results)` must return `false` when `node_ips` is empty,
//!   regardless of what `results` contains.
//! - `cli/cluster.rs` must NOT guard the HTTP-aggregation block with
//!   `if !node_ips.is_empty()` — that guard silently skips the FAIL path.
//! - `cli/cluster.rs` must use `http_layer_pass` to produce the HTTP layer decision.
//!
//! All tests are pure: no CLI invocations, no network, no filesystem writes.
use kina_cli::core::verify::{http_layer_pass, ProbeResult};

// ===========================================================================
// Group A — http_layer_pass: empty node_ips always returns false
// ===========================================================================

/// H1 — empty node_ips with empty results returns false.
///
/// `aggregate_verify(&[])` already returns false; `http_layer_pass` wraps
/// the same invariant but is the entry point the CLI must call so the guard
/// `if !node_ips.is_empty()` can be removed.
#[test]
fn http_layer_pass_empty_ips_no_results_is_false() {
    let node_ips: Vec<String> = vec![];
    let results: Vec<ProbeResult> = vec![];
    assert!(
        !http_layer_pass(&node_ips, &results),
        "http_layer_pass with empty node_ips must return false \
         (no IPs = no evidence = FAIL; never green on no evidence)"
    );
}

/// H2 — empty node_ips even when results is non-empty still returns false.
///
/// Defensive: if node_ips is empty the caller cannot have valid probe results
/// anyway, but the function must be unconditionally false on empty IPs.
#[test]
fn http_layer_pass_empty_ips_with_phantom_results_is_false() {
    let node_ips: Vec<String> = vec![];
    // Phantom passing result — should not flip the outcome.
    let results = vec![ProbeResult {
        node: "10.0.0.1".to_string(),
        passed: true,
    }];
    assert!(
        !http_layer_pass(&node_ips, &results),
        "http_layer_pass must return false when node_ips is empty, \
         even if results contains passing entries — empty IPs means no evidence"
    );
}

/// H3 — non-empty node_ips with all probes passing returns true.
#[test]
fn http_layer_pass_all_passing_is_true() {
    let node_ips = vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()];
    let results = vec![
        ProbeResult {
            node: "10.0.0.1".to_string(),
            passed: true,
        },
        ProbeResult {
            node: "10.0.0.2".to_string(),
            passed: true,
        },
    ];
    assert!(
        http_layer_pass(&node_ips, &results),
        "http_layer_pass must return true when node_ips is non-empty \
         and all ProbeResults have passed=true"
    );
}

/// H4 — non-empty node_ips with one failing probe returns false.
#[test]
fn http_layer_pass_any_failing_is_false() {
    let node_ips = vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()];
    let results = vec![
        ProbeResult {
            node: "10.0.0.1".to_string(),
            passed: true,
        },
        ProbeResult {
            node: "10.0.0.2".to_string(),
            passed: false,
        },
    ];
    assert!(
        !http_layer_pass(&node_ips, &results),
        "http_layer_pass must return false when any ProbeResult has passed=false"
    );
}

// ===========================================================================
// Group B — source-grep guards: cluster.rs wiring
// ===========================================================================

/// H5 — cli/cluster.rs must NOT contain the bypassing guard
///      `if !node_ips.is_empty()` around the HTTP-aggregation block.
///
/// That guard causes `all_pass` to stay `true` when no node IPs are found,
/// producing a false-positive PASS with zero HTTP evidence.
#[test]
fn source_cluster_no_bypass_guard_on_node_ips() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");
    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };
    assert!(
        !src.contains("if !node_ips.is_empty()"),
        "cli/cluster.rs must NOT contain 'if !node_ips.is_empty()' — \
         that guard silently skips all_pass=false when no node IPs are found, \
         producing a false-positive PASS with zero HTTP evidence. \
         Use http_layer_pass(node_ips, results) unconditionally instead."
    );
}

/// H6 — cli/cluster.rs must call http_layer_pass for the HTTP-layer decision.
#[test]
fn source_cluster_uses_http_layer_pass() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");
    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };
    assert!(
        src.contains("http_layer_pass"),
        "cli/cluster.rs must call http_layer_pass for the HTTP-layer PASS/FAIL decision — \
         this ensures empty node_ips always produces all_pass=false with an explicit FAIL line"
    );
}
