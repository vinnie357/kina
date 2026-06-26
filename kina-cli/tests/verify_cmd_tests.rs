/// Verify command tests — adversarial TDD (P2 test-author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns / types that do not yet
/// exist in kina_cli::core::verify. The compile errors are the spec; P3 (the
/// separate implementer agent) makes them green WITHOUT modifying this file.
///
/// All tests are pure: NO live container/kubectl CLI invocations, NO process spawns,
/// NO network, NO filesystem writes, NO env mutation.
/// Source-grep guard tests open the source file via CARGO_MANIFEST_DIR.
///
/// Import surface (binding for P3):
///   use kina_cli::core::verify::{
///       render_demo_manifest, probe_host, probe_url, parse_dns_domain,
///       probe_passed, aggregate_verify, ProbeResult,
///   };
use kina_cli::core::verify::{
    aggregate_verify, cni_report_from_cilium_pods, parse_dns_domain, parse_node_versions,
    probe_host, probe_passed, probe_url, render_demo_manifest, CniReport, ProbeResult,
};

// ===========================================================================
// Group A — render_demo_manifest (demo-app substitution) — simplest first
// ===========================================================================

/// T1 — render replaces ${CLUSTER_NAME} with the given cluster name
///       and the result contains the expected interpolated substring.
#[test]
fn render_substitutes_cluster_name() {
    let fixture = "host: ${CLUSTER_NAME}-control-plane.${DNS_DOMAIN}";
    let result = render_demo_manifest(fixture, "kina-test", "test", "", "", "", "");
    assert!(
        result.contains("kina-test-control-plane.test"),
        "render_demo_manifest must produce \"kina-test-control-plane.test\"; got:\n{}",
        result
    );
    assert!(
        !result.contains("${CLUSTER_NAME}"),
        "render_demo_manifest must leave NO literal \"${{CLUSTER_NAME}}\" in output; got:\n{}",
        result
    );
}

/// T2 — render replaces ${DNS_DOMAIN} with the given domain
///       and no literal ${DNS_DOMAIN} remains.
#[test]
fn render_substitutes_dns_domain() {
    let fixture = "ingress: ${CLUSTER_NAME}-control-plane.${DNS_DOMAIN}";
    let result = render_demo_manifest(fixture, "c", "mydomain", "", "", "", "");
    assert!(
        result.contains("mydomain"),
        "render_demo_manifest must produce output containing \"mydomain\"; got:\n{}",
        result
    );
    assert!(
        !result.contains("${DNS_DOMAIN}"),
        "render_demo_manifest must leave NO literal \"${{DNS_DOMAIN}}\" in output; got:\n{}",
        result
    );
}

/// T3 — triangulation: distinct cluster+domain values produce distinct interpolated output
///       and the result does NOT contain any hardcoded "kina-test" default.
#[test]
fn render_triangulation_distinct_values() {
    let fixture = "host: ${CLUSTER_NAME}-control-plane.${DNS_DOMAIN}";
    let result = render_demo_manifest(fixture, "alpha", "beta", "", "", "", "");
    assert!(
        result.contains("alpha-control-plane.beta"),
        "render_demo_manifest(fixture, \"alpha\", \"beta\") must produce \
         \"alpha-control-plane.beta\"; got:\n{}",
        result
    );
    assert!(
        !result.contains("kina-test"),
        "render_demo_manifest(fixture, \"alpha\", \"beta\") must NOT contain \"kina-test\" — \
         values must be interpolated, not hardcoded; got:\n{}",
        result
    );
}

/// T4 — render leaves pod-runtime env vars (MY_POD_NAME, MY_POD_IP, MY_NODE_NAME) untouched;
///       only CLUSTER_NAME and DNS_DOMAIN are substituted at render time.
#[test]
fn render_leaves_pod_runtime_vars_untouched() {
    let fixture = "x ${MY_POD_NAME} ${CLUSTER_NAME}";
    let result = render_demo_manifest(fixture, "c", "d", "", "", "", "");
    assert!(
        result.contains("${MY_POD_NAME}"),
        "render_demo_manifest must leave \"${{MY_POD_NAME}}\" as a literal — pod runtime vars \
         are NOT substituted at render time; got:\n{}",
        result
    );
    assert!(
        result.contains("c"),
        "render_demo_manifest must still substitute CLUSTER_NAME with \"c\"; got:\n{}",
        result
    );
}

/// T5 — render replaces ALL occurrences of ${CLUSTER_NAME}, not just the first.
#[test]
fn render_substitutes_all_occurrences() {
    let fixture = "name: ${CLUSTER_NAME}\nother: ${CLUSTER_NAME}";
    let result = render_demo_manifest(fixture, "mycluster", "test", "", "", "", "");
    assert!(
        !result.contains("${CLUSTER_NAME}"),
        "render_demo_manifest must replace ALL occurrences of \"${{CLUSTER_NAME}}\" (global replace, \
         not first-only); remaining literal found in:\n{}",
        result
    );
    let count = result.matches("mycluster").count();
    assert!(
        count >= 2,
        "render_demo_manifest must replace both occurrences; found \"mycluster\" {} times in:\n{}",
        count,
        result
    );
}

// ===========================================================================
// Group B — probe_host construction
// ===========================================================================

/// T6 — probe_host("kina-test", "test") returns the expected Host header value.
#[test]
fn probe_host_basic() {
    let host = probe_host("kina-test", "test");
    assert_eq!(
        host, "kina-test-control-plane.test",
        "probe_host(\"kina-test\", \"test\") must return \
         \"kina-test-control-plane.test\"; got \"{}\"",
        host
    );
}

/// T7 — triangulation: different cluster+domain yield a different host value;
///       proves both fields are interpolated, not hardcoded.
#[test]
fn probe_host_triangulation() {
    let host = probe_host("prod", "local");
    assert_eq!(
        host, "prod-control-plane.local",
        "probe_host(\"prod\", \"local\") must return \
         \"prod-control-plane.local\"; got \"{}\"",
        host
    );
    assert_ne!(
        host, "kina-test-control-plane.test",
        "probe_host(\"prod\", \"local\") must differ from the basic value — \
         cluster AND domain must both be templated"
    );
}

// ===========================================================================
// Group C — probe_url construction
// ===========================================================================

/// T8 — probe_url prefixes the IP with "http://".
#[test]
fn probe_url_prefixes_http() {
    let url = probe_url("10.0.0.5");
    assert_eq!(
        url, "http://10.0.0.5",
        "probe_url(\"10.0.0.5\") must return \"http://10.0.0.5\"; got \"{}\"",
        url
    );
}

/// T9 — triangulation: different IP yields a different URL; proves IP is interpolated.
#[test]
fn probe_url_triangulation() {
    let url = probe_url("192.168.65.3");
    assert_eq!(
        url, "http://192.168.65.3",
        "probe_url(\"192.168.65.3\") must return \"http://192.168.65.3\"; got \"{}\"",
        url
    );
    assert_ne!(
        url, "http://10.0.0.5",
        "probe_url(\"192.168.65.3\") must differ from the T8 value — IP must be interpolated"
    );
}

// ===========================================================================
// Group D — parse_dns_domain (with fallback "test")
// ===========================================================================

/// T10 — single non-empty line with trailing newline returns the trimmed domain.
#[test]
fn parse_dns_domain_single_line() {
    let result = parse_dns_domain("test\n");
    assert_eq!(
        result, "test",
        "parse_dns_domain(\"test\\n\") must return \"test\"; got \"{}\"",
        result
    );
}

/// T11 — triangulation: a distinct domain value is returned correctly.
#[test]
fn parse_dns_domain_distinct_value() {
    let result = parse_dns_domain("mydomain\n");
    assert_eq!(
        result, "mydomain",
        "parse_dns_domain(\"mydomain\\n\") must return \"mydomain\" \
         (triangulation — not hardcoded \"test\"); got \"{}\"",
        result
    );
}

/// T12 — leading and trailing whitespace is trimmed from the parsed domain.
#[test]
fn parse_dns_domain_trims_whitespace() {
    let result = parse_dns_domain("  example  \n");
    assert_eq!(
        result, "example",
        "parse_dns_domain(\"  example  \\n\") must return \"example\" \
         (leading/trailing trimmed); got \"{}\"",
        result
    );
}

/// T13 — empty string falls back to "test" (AC1/AC3 fallback).
#[test]
fn parse_dns_domain_empty_falls_back_to_test() {
    let result = parse_dns_domain("");
    assert_eq!(
        result, "test",
        "parse_dns_domain(\"\") must return \"test\" (AC1/AC3 fallback); got \"{}\"",
        result
    );
}

/// T14 — all-blank output (newlines and spaces only) falls back to "test".
#[test]
fn parse_dns_domain_blank_lines_fall_back_to_test() {
    let result = parse_dns_domain("\n  \n");
    assert_eq!(
        result, "test",
        "parse_dns_domain(\"\\n  \\n\") must return \"test\" \
         (all-blank output → fallback, not empty string); got \"{}\"",
        result
    );
}

/// T15 — skips leading blank lines; returns the first non-empty trimmed line.
#[test]
fn parse_dns_domain_first_nonblank_line() {
    let result = parse_dns_domain("\nactualdomain\nignored\n");
    assert_eq!(
        result, "actualdomain",
        "parse_dns_domain(\"\\nactualdomain\\nignored\\n\") must return \"actualdomain\" \
         (skips leading blank, takes first non-empty line); got \"{}\"",
        result
    );
}

// ===========================================================================
// Group E — probe_passed (demo marker detection)
// ===========================================================================

/// T16 — body containing the demo success marker returns true.
#[test]
fn probe_passed_true_on_marker() {
    let body = "...<title>Kina Demo Success!</title>...";
    assert!(
        probe_passed(body),
        "probe_passed must return true when body contains \
         \"Kina Demo Success\"; body was:\n{}",
        body
    );
}

/// T17 — body without the marker returns false.
#[test]
fn probe_passed_false_without_marker() {
    let body = "<html>404 not found</html>";
    assert!(
        !probe_passed(body),
        "probe_passed must return false when body does NOT contain \
         \"Kina Demo Success\"; body was:\n{}",
        body
    );
}

/// T18 — empty body returns false (no-response body is a FAIL).
#[test]
fn probe_passed_false_on_empty_body() {
    assert!(
        !probe_passed(""),
        "probe_passed(\"\") must return false — empty body is a FAIL, not a pass"
    );
}

// ===========================================================================
// Group F — aggregate_verify (pass/fail aggregation)
// ===========================================================================

/// T19 — all probes passing → aggregate returns true.
#[test]
fn aggregate_all_pass_is_true() {
    let results = [
        ProbeResult {
            node: "node-0".to_string(),
            passed: true,
        },
        ProbeResult {
            node: "node-1".to_string(),
            passed: true,
        },
    ];
    assert!(
        aggregate_verify(&results),
        "aggregate_verify must return true when every ProbeResult has passed=true"
    );
}

/// T20 — one failing probe → aggregate returns false (non-zero exit semantics).
#[test]
fn aggregate_any_fail_is_false() {
    let results = [
        ProbeResult {
            node: "node-0".to_string(),
            passed: true,
        },
        ProbeResult {
            node: "node-1".to_string(),
            passed: false,
        },
    ];
    assert!(
        !aggregate_verify(&results),
        "aggregate_verify must return false when any ProbeResult has passed=false \
         (one FAIL fails the whole verify)"
    );
}

/// T21 — empty slice → aggregate returns false (zero probes = FAIL; never green on no evidence).
#[test]
fn aggregate_empty_is_false() {
    let results: &[ProbeResult] = &[];
    assert!(
        !aggregate_verify(results),
        "aggregate_verify(&[]) must return false — \
         zero probes is a FAIL, never green on no evidence"
    );
}

/// T22 — single passing result → aggregate returns true (boundary case).
#[test]
fn aggregate_single_pass_is_true() {
    let results = [ProbeResult {
        node: "node-0".to_string(),
        passed: true,
    }];
    assert!(
        aggregate_verify(&results),
        "aggregate_verify must return true for a single passing ProbeResult (boundary)"
    );
}

/// T23 — all failing → aggregate returns false.
#[test]
fn aggregate_all_fail_is_false() {
    let results = [
        ProbeResult {
            node: "node-0".to_string(),
            passed: false,
        },
        ProbeResult {
            node: "node-1".to_string(),
            passed: false,
        },
    ];
    assert!(
        !aggregate_verify(&results),
        "aggregate_verify must return false when all ProbeResults have passed=false"
    );
}

// ===========================================================================
// Group G — CLI wiring source-grep guards (cli/cluster.rs)
// Live behavior is review-gated; these tests guard the structural wiring only.
// No bang fns — uses match/panic per project rule (mirrors cilium_install_tests.rs L45-48).
// ===========================================================================

/// T24 — AddonType gains a DemoApp variant with clap value name "demo-app" (AC1).
#[test]
fn source_addontype_has_demo_app_variant() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("\"demo-app\""),
        "cli/cluster.rs must contain clap value name \"demo-app\" \
         (AddonType gains DemoApp variant for AC1 `kina install demo-app`)"
    );
}

/// T25 — A "verify" subcommand is wired in cli/cluster.rs (or the command-enum source) (AC3).
#[test]
fn source_has_verify_subcommand() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("verify"),
        "cli/cluster.rs must contain a \"verify\" subcommand wiring \
         (AC3 `kina verify [cluster]`)"
    );
}

/// T26 — cli/cluster.rs uses include_str! referencing "demo-app.yaml" (AC1 embedded manifest).
#[test]
fn source_demo_manifest_embedded_via_include_str() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("include_str!") && src.contains("demo-app.yaml"),
        "cli/cluster.rs must contain include_str! referencing \"demo-app.yaml\" \
         (AC1 embedded manifest, not CWD-relative file open)"
    );
}

/// T27 — cli/cluster.rs uses include_str! referencing an nginx-ingress manifest (AC2 embedded).
#[test]
fn source_nginx_manifests_embedded_via_include_str() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("include_str!") && src.contains("nginx-ingress"),
        "cli/cluster.rs must contain include_str! referencing an nginx-ingress manifest \
         (AC2 — nginx manifests embedded in binary)"
    );
}

/// T28 — cli/cluster.rs does NOT contain the CWD-relative manifest join `.join("kina-cli")`
///        (the L508-520 current_dir()-derived path is removed; manifests resolve from binary).
#[test]
fn source_no_cwd_relative_manifest_resolution() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        !src.contains(".join(\"kina-cli\")"),
        "cli/cluster.rs must NOT contain the CWD-relative manifest join \
         \".join(\\\"kina-cli\\\")\" — the L508-520 current_dir()-derived \
         manifest path must be removed (AC2 works from any directory)"
    );
}

/// T29 — cli/cluster.rs references render_demo_manifest (proves the fn is wired, not dead code).
#[test]
fn source_install_calls_render_demo_manifest() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("render_demo_manifest"),
        "cli/cluster.rs must reference render_demo_manifest — \
         the install demo-app path must render the embedded manifest \
         through the pure fn (AC1 substitution wired, not dead code)"
    );
}

/// T30 — cli/cluster.rs references aggregate_verify (verify command routes through the
///        aggregation fn for its PASS/FAIL/exit-code decision — AC3 non-zero exit on any FAIL).
#[test]
fn source_verify_uses_aggregate() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("aggregate_verify"),
        "cli/cluster.rs must reference aggregate_verify — \
         the verify command's PASS/FAIL/exit-code decision must route \
         through the pure aggregation fn (AC3 non-zero exit on any FAIL)"
    );
}

// ===========================================================================
// Group H — CNI runtime detection (cni_report_from_cilium_pods)
// ===========================================================================

#[test]
fn cni_report_empty_means_ptp() {
    assert_eq!(cni_report_from_cilium_pods("   \n"), CniReport::Ptp);
}

#[test]
fn cni_report_counts_ready_cilium() {
    let out = "cilium-abc 1/1 Running 0 1m\ncilium-def 0/1 Pending 0 1m\n";
    assert_eq!(
        cni_report_from_cilium_pods(out),
        CniReport::Cilium { ready: 1, total: 2 }
    );
}

// ===========================================================================
// Group I — node version parsing (parse_node_versions)
// ===========================================================================

#[test]
fn parse_node_versions_skips_header() {
    let out = "NAME                 VERSION\ncp-node              v1.36.1\nwk-node              v1.36.1\n";
    let m = parse_node_versions(out);
    assert_eq!(m.get("cp-node").map(String::as_str), Some("v1.36.1"));
    assert_eq!(m.get("wk-node").map(String::as_str), Some("v1.36.1"));
    assert_eq!(m.len(), 2);
}
