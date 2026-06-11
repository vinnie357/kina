/// Apple Container 1.0.0 compatibility tests — adversarial TDD (P2 test author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns / consts / types that do not yet
/// exist in kina_cli::core::apple_container. The compile errors are the spec; P3 (the
/// separate implementer agent) makes them green without modifying this file.
///
/// All tests are pure: NO live `container` CLI invocations, NO process spawns, NO network.
/// Source-grep guard tests (T20, T23, T24) open the source file via CARGO_MANIFEST_DIR.
use kina_cli::core::apple_container::{
    cli_path_candidates, node_cap_args, parse_container_list, parse_version_output,
    validate_version, CliPathStrategy, MIN_VERSION,
};
#[allow(unused_imports)]
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// VERBATIM live-confirmed 1.0.0 fixture (single-element array)
// Source: "LIVE 1.0.0 SHAPE CONFIRMED" bees comment on kina-1, posted by Probe stage.
// The status.networks[0].ipv4Address value is "192.168.65.2/24" — CIDR form, exact.
// ---------------------------------------------------------------------------
const FIXTURE_1_0_0: &str = r#"[{"configuration":{"capAdd":[],"capDrop":[],"creationDate":"2026-06-10T23:03:47Z","dns":{"nameservers":[],"options":[],"searchDomains":[]},"id":"kina-probe-100","image":{"descriptor":{"digest":"sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659","mediaType":"application/vnd.oci.image.index.v1+json","size":9218},"reference":"docker.io/library/alpine:latest"},"initProcess":{"arguments":["120"],"environment":["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],"executable":"sleep","rlimits":[],"supplementalGroups":[],"terminal":false,"user":{"id":{"gid":0,"uid":0}},"workingDirectory":"/"},"labels":{},"mounts":[],"networks":[{"network":"default","options":{"hostname":"kina-probe-100","mtu":1280}}],"platform":{"architecture":"arm64","os":"linux"},"publishedPorts":[],"publishedSockets":[],"readOnly":false,"resources":{"cpuOverhead":1,"cpus":4,"memoryInBytes":1073741824},"rosetta":false,"runtimeHandler":"container-runtime-linux","ssh":false,"sysctls":{},"useInit":false,"virtualization":false},"id":"kina-probe-100","status":{"networks":[{"hostname":"kina-probe-100","ipv4Address":"192.168.65.2/24","ipv4Gateway":"192.168.65.1","ipv6Address":"fd68:e17d:9fc2:4b0e:f035:48ff:fe88:75d2/64","macAddress":"f2:35:48:88:75:d2","mtu":1280,"network":"default"}],"startedDate":"2026-06-10T23:03:49Z","state":"running"}}]"#;

// ---------------------------------------------------------------------------
// Derived fixture: same 1.0.0 shape with populated kina labels (element 0 =
// running, element 1 = stopped clone with distinct state) so that
// configuration.labels and multi-element grouping can be asserted.
// The status.networks subtree is byte-for-byte identical to the live fixture.
// ---------------------------------------------------------------------------
const FIXTURE_1_0_0_KINA: &str = r#"[{"configuration":{"capAdd":[],"capDrop":[],"creationDate":"2026-06-10T23:03:47Z","dns":{"nameservers":[],"options":[],"searchDomains":[]},"id":"kina-test-control-plane","image":{"descriptor":{"digest":"sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659","mediaType":"application/vnd.oci.image.index.v1+json","size":9218},"reference":"docker.io/library/alpine:latest"},"initProcess":{"arguments":["120"],"environment":["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],"executable":"sleep","rlimits":[],"supplementalGroups":[],"terminal":false,"user":{"id":{"gid":0,"uid":0}},"workingDirectory":"/"},"labels":{"io.kina.cluster":"test-cluster","io.kina.role":"control-plane","io.kina.image":"kindest/node:v1.29.0"},"mounts":[],"networks":[{"network":"default","options":{"hostname":"kina-test-control-plane","mtu":1280}}],"platform":{"architecture":"arm64","os":"linux"},"publishedPorts":[],"publishedSockets":[],"readOnly":false,"resources":{"cpuOverhead":1,"cpus":4,"memoryInBytes":1073741824},"rosetta":false,"runtimeHandler":"container-runtime-linux","ssh":false,"sysctls":{},"useInit":false,"virtualization":false},"id":"kina-test-control-plane","status":{"networks":[{"hostname":"kina-test-control-plane","ipv4Address":"192.168.65.2/24","ipv4Gateway":"192.168.65.1","ipv6Address":"fd68:e17d:9fc2:4b0e:f035:48ff:fe88:75d2/64","macAddress":"f2:35:48:88:75:d2","mtu":1280,"network":"default"}],"startedDate":"2026-06-10T23:03:49Z","state":"running"}},{"configuration":{"capAdd":[],"capDrop":[],"creationDate":"2026-06-10T23:04:00Z","dns":{"nameservers":[],"options":[],"searchDomains":[]},"id":"kina-test-worker","image":{"descriptor":{"digest":"sha256:25109184c71bdad752c8312a8623239686a9a2071e8825f20acb8f2198c3f659","mediaType":"application/vnd.oci.image.index.v1+json","size":9218},"reference":"docker.io/library/alpine:latest"},"initProcess":{"arguments":["120"],"environment":["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],"executable":"sleep","rlimits":[],"supplementalGroups":[],"terminal":false,"user":{"id":{"gid":0,"uid":0}},"workingDirectory":"/"},"labels":{"io.kina.cluster":"test-cluster","io.kina.role":"worker","io.kina.image":"kindest/node:v1.29.0"},"mounts":[],"networks":[{"network":"default","options":{"hostname":"kina-test-worker","mtu":1280}}],"platform":{"architecture":"arm64","os":"linux"},"publishedPorts":[],"publishedSockets":[],"readOnly":false,"resources":{"cpuOverhead":1,"cpus":4,"memoryInBytes":1073741824},"rosetta":false,"runtimeHandler":"container-runtime-linux","ssh":false,"sysctls":{},"useInit":false,"virtualization":false},"id":"kina-test-worker","status":{"networks":[{"hostname":"kina-test-worker","ipv4Address":"192.168.65.3/24","ipv4Gateway":"192.168.65.1","ipv6Address":"fd68:e17d:9fc2:4b0e:f035:48ff:fe88:cafe/64","macAddress":"f2:35:48:88:75:ca","mtu":1280,"network":"default"}],"startedDate":"2026-06-10T23:04:02Z","state":"stopped"}}]"#;

// ===========================================================================
// Group A: version string parsing
// ===========================================================================

/// T1 — garbage input (no "version " token) must return Err
#[test]
fn parse_version_output_garbage_errs() {
    let result = parse_version_output("totally not a version line");
    assert!(
        result.is_err(),
        "expected Err for garbage input, got {:?}",
        result
    );
}

/// T2 — empty input must return Err
#[test]
fn parse_version_output_empty_errs() {
    let result = parse_version_output("");
    assert!(
        result.is_err(),
        "expected Err for empty input, got {:?}",
        result
    );
}

/// T3 — exact live 1.0.0 version string parses to "1.0.0"
#[test]
fn parse_version_output_extracts_1_0_0() {
    let raw = "container CLI version 1.0.0 (build: release, commit: ee848e3)";
    let version = parse_version_output(raw).expect("should parse 1.0.0 version string");
    assert_eq!(version, "1.0.0");
}

/// T4 — future version string "1.1.0" parses correctly (triangulation / generalization)
#[test]
fn parse_version_output_extracts_future_1_1_0() {
    let raw = "container CLI version 1.1.0 (build: release, commit: abc1234)";
    let version = parse_version_output(raw).expect("should parse 1.1.0 version string");
    assert_eq!(version, "1.1.0");
}

// ===========================================================================
// Group B: version floor — MIN_VERSION=(1,0,0), reject <1.0.0, accept >=1.0.0
// ===========================================================================

/// T5 — MIN_VERSION constant is exactly (1,0,0)
#[test]
fn min_version_const_is_1_0_0() {
    assert_eq!(MIN_VERSION, (1, 0, 0));
}

/// T6 — validate_version rejects 0.12.3
#[test]
fn validate_version_rejects_0_12_3() {
    let result = validate_version("0.12.3");
    assert!(
        result.is_err(),
        "expected Err for version 0.12.3, got {:?}",
        result
    );
}

/// T7 — error message from rejecting 0.12.3 must name 1.0.0, config.toml,
///       caps change, and output/shape change
#[test]
fn validate_version_error_names_1_0_0_and_migration() {
    let err = validate_version("0.12.3").expect_err("expected Err for 0.12.3");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("1.0.0"),
        "error message must contain '1.0.0'; got: {}",
        msg
    );
    assert!(
        msg.contains("config.toml"),
        "error message must mention 'config.toml'; got: {}",
        msg
    );
    // caps reduced since 0.12.0 — message should mention cap or capab
    assert!(
        msg.contains("cap"),
        "error message must mention capabilities ('cap'); got: {}",
        msg
    );
    // output shape changed — message should mention shape or output
    assert!(
        msg.contains("shape") || msg.contains("output"),
        "error message must mention output shape change ('shape' or 'output'); got: {}",
        msg
    );
}

/// T8 — error message must NOT contain stale 0.5.0 text
#[test]
fn validate_version_error_omits_stale_0_5_0_text() {
    let err = validate_version("0.12.3").expect_err("expected Err for 0.12.3");
    let msg = err.to_string();
    assert!(
        !msg.contains("container images"),
        "error message must NOT contain stale 'container images' text; got: {}",
        msg
    );
    assert!(
        !msg.contains("0.5.0"),
        "error message must NOT contain stale '0.5.0' text; got: {}",
        msg
    );
    assert!(
        !msg.contains("com.apple.container.registry"),
        "error message must NOT contain stale 'com.apple.container.registry' text; got: {}",
        msg
    );
}

/// T9 — exact floor boundary 1.0.0 accepted
#[test]
fn validate_version_accepts_1_0_0() {
    validate_version("1.0.0").expect("1.0.0 should be accepted as the minimum version");
}

/// T10 — 1.1.0 accepted (above floor)
#[test]
fn validate_version_accepts_1_1_0() {
    validate_version("1.1.0").expect("1.1.0 should be accepted");
}

// ===========================================================================
// Group C: CLI path ordering — PATH resolution before hardcoded fallback
// ===========================================================================

/// T11 — first candidate is a Which(_) strategy; at least one Hardcoded(_) appears after
#[test]
fn cli_path_candidates_path_resolution_first() {
    let candidates = cli_path_candidates();
    assert!(
        !candidates.is_empty(),
        "cli_path_candidates() must not be empty"
    );

    // First element must be a Which(_) (PATH/which resolution)
    assert!(
        matches!(&candidates[0], CliPathStrategy::Which(_)),
        "first candidate must be CliPathStrategy::Which(_), got a Hardcoded variant"
    );

    // At least one Hardcoded(_) must exist somewhere after index 0
    let last_which_idx = candidates
        .iter()
        .rposition(|c| matches!(c, CliPathStrategy::Which(_)))
        .unwrap_or(0);

    let has_hardcoded_after = candidates[last_which_idx + 1..]
        .iter()
        .any(|c| matches!(c, CliPathStrategy::Hardcoded(_)));

    assert!(
        has_hardcoded_after,
        "at least one Hardcoded(_) candidate must appear after the last Which(_) candidate"
    );
}

/// T12 — first Which(_) targets the binary named "container"
#[test]
fn cli_path_candidates_which_targets_container() {
    let candidates = cli_path_candidates();

    let first_which_name = candidates
        .iter()
        .find_map(|c| {
            if let CliPathStrategy::Which(name) = c {
                Some(name.clone())
            } else {
                None
            }
        })
        .expect("at least one Which(_) candidate expected");

    assert_eq!(
        first_which_name, "container",
        "first Which(_) must target 'container' (the brew/PATH binary), got '{}'",
        first_which_name
    );
}

// ===========================================================================
// Group D: container-list JSON parsing — degenerate and happy paths
// ===========================================================================

/// T13 — empty / whitespace input parses to empty Vec (no error)
#[test]
fn parse_container_list_empty_is_empty() {
    let result_empty = parse_container_list("").expect("empty string should return Ok(empty vec)");
    assert!(result_empty.is_empty(), "empty input must yield empty Vec");

    let result_ws =
        parse_container_list("   ").expect("whitespace-only should return Ok(empty vec)");
    assert!(
        result_ws.is_empty(),
        "whitespace-only input must yield empty Vec"
    );
}

/// T14 — VERBATIM live fixture parses: len==1, id=="kina-probe-100", state=="running"
#[test]
fn parse_container_list_live_fixture_id_and_state() {
    let containers =
        parse_container_list(FIXTURE_1_0_0).expect("FIXTURE_1_0_0 should parse without error");
    assert_eq!(
        containers.len(),
        1,
        "expected exactly 1 container from fixture"
    );
    assert_eq!(containers[0].id, "kina-probe-100");
    assert_eq!(containers[0].state, "running");
}

/// T15 — CIDR suffix stripped: ipv4 is Some("192.168.65.2"), NOT "192.168.65.2/24"
#[test]
fn parse_container_list_live_fixture_ipv4_cidr_stripped() {
    let containers =
        parse_container_list(FIXTURE_1_0_0).expect("FIXTURE_1_0_0 should parse without error");
    assert_eq!(containers.len(), 1);
    assert_eq!(
        containers[0].ipv4,
        Some("192.168.65.2".to_string()),
        "ipv4 must be bare IP without CIDR suffix"
    );
}

// ===========================================================================
// Group E: container-list JSON parsing — labels and multi-element
// ===========================================================================

/// T16 — labels read from configuration.labels (not top-level)
#[test]
fn parse_container_list_labels_from_configuration() {
    let containers = parse_container_list(FIXTURE_1_0_0_KINA)
        .expect("FIXTURE_1_0_0_KINA should parse without error");
    assert!(!containers.is_empty(), "expected at least one container");

    // Find the control-plane element by id
    let cp = containers
        .iter()
        .find(|c| c.id == "kina-test-control-plane")
        .expect("kina-test-control-plane element not found");

    assert_eq!(
        cp.labels.get("io.kina.cluster"),
        Some(&"test-cluster".to_string()),
        "io.kina.cluster label must be read from configuration.labels"
    );
}

/// T17 — 2-element fixture yields 2 ParsedContainers with distinct states
#[test]
fn parse_container_list_multi_element_states() {
    let containers = parse_container_list(FIXTURE_1_0_0_KINA)
        .expect("FIXTURE_1_0_0_KINA should parse without error");
    assert_eq!(
        containers.len(),
        2,
        "expected exactly 2 containers from KINA fixture"
    );

    let states: Vec<&str> = containers.iter().map(|c| c.state.as_str()).collect();
    assert!(
        states.contains(&"running"),
        "expected a 'running' state in results; got: {:?}",
        states
    );
    assert!(
        states.contains(&"stopped"),
        "expected a 'stopped' state in results; got: {:?}",
        states
    );
}

// ===========================================================================
// Group F: error / malformed JSON
// ===========================================================================

/// T18 — malformed JSON returns Err (must not panic)
#[test]
fn parse_container_list_malformed_json_errs() {
    let result = parse_container_list("{not json");
    assert!(
        result.is_err(),
        "malformed JSON must return Err, not panic; got {:?}",
        result
    );
}

/// T19 — JSON object (not top-level array) returns Err
#[test]
fn parse_container_list_object_not_array_errs() {
    let result = parse_container_list(r#"{"id":"x"}"#);
    assert!(
        result.is_err(),
        "top-level JSON object must return Err (array expected); got {:?}",
        result
    );
}

// ===========================================================================
// Group G: source-grep guards — no stale shape reads in apple_container.rs
// ===========================================================================

/// T20 — apple_container.rs must not contain legacy .get("address") or
///        .get("status").and_then(|v| v.as_str()) patterns
#[test]
fn apple_container_source_has_no_legacy_address_key() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        !src.contains(r#"network.get("address")"#),
        "apple_container.rs must not reference legacy network.get(\"address\") key"
    );
    assert!(
        !src.contains(r#".get("status").and_then(|v| v.as_str())"#),
        "apple_container.rs must not read top-level .status as a string via .and_then(|v| v.as_str())"
    );
}

// ===========================================================================
// Group H: capabilities on node creation
// ===========================================================================

/// T21 — node_cap_args() returns exactly ["--cap-add", "ALL"]
#[test]
fn node_cap_args_is_cap_add_all() {
    let args = node_cap_args();
    assert_eq!(
        args,
        vec!["--cap-add", "ALL"],
        "node_cap_args() must return [\"--cap-add\", \"ALL\"]"
    );
}

/// T22 — node_cap_args() joined string does NOT contain "--privileged"
#[test]
fn node_cap_args_has_no_privileged() {
    let joined = node_cap_args().join(" ");
    assert!(
        !joined.contains("--privileged"),
        "node_cap_args() must not produce --privileged (flag does not exist in Apple Container)"
    );
}

/// T23 — source-grep guard: apple_container.rs does NOT contain "--privileged"
#[test]
fn apple_container_source_has_no_privileged_flag() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        !src.contains("--privileged"),
        "apple_container.rs must not contain \"--privileged\" (flag does not exist in Apple Container)"
    );
}

/// T24 — source-grep guard: apple_container.rs wires --cap-add ALL in all three creators
///        (at least 3 occurrences of node_cap_args — one per create_single_node,
///        create_control_plane_node, create_worker_node)
#[test]
fn apple_container_source_wires_cap_add_in_all_three_creators() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("--cap-add"),
        "apple_container.rs must contain \"--cap-add\" for node capability wiring"
    );
    assert!(
        src.contains("ALL"),
        "apple_container.rs must contain \"ALL\" (cap-add target for k8s nodes)"
    );

    let node_cap_args_count = src.matches("node_cap_args").count();
    assert!(
        node_cap_args_count >= 3,
        "apple_container.rs must call node_cap_args() at least 3 times (once per create_single_node, \
         create_control_plane_node, create_worker_node); found {} occurrences",
        node_cap_args_count
    );
}
