//! Adversarial-TDD tests for kina-39 / GitHub issue #46:
//! host kubectl reachability — kubeconfig server rewrite and unreachable diagnostics.
//!
//! ## P2 contract — these tests are INTENTIONALLY RED until the implementer
//! adds the two pure fns to `kina_cli::core::verify`:
//!
//!   `pub fn rewrite_kubeconfig_server(kubeconfig_yaml: &str, vm_ip: &str) -> String`
//!   `pub fn build_unreachable_diagnostic(
//!       cluster: &str, vm_ip: &str, port: u16,
//!       bridge: Option<&str>, gateway: Option<&str>,
//!   ) -> String`
//!
//! ## Implementer binding (kina-cli/src/core/verify.rs)
//!
//! `rewrite_kubeconfig_server`:
//!   - Accepts a raw kubeconfig YAML string and a target VM IP.
//!   - Rewrites the `server:` value inside every cluster stanza to
//!     `https://<vm_ip>:6443` (port 6443 fixed).
//!   - Generalises the ad-hoc localhost→VM-IP replace that lives in
//!     apple_container.rs ~line 1774:
//!       kubeconfig.replace("https://127.0.0.1:6443", &format!("https://{}:6443", vm_ip))
//!       kubeconfig.replace("https://localhost:6443", &format!("https://{}:6443", vm_ip))
//!     The new pure fn handles ANY host that currently appears after `server: https://`.
//!   - Idempotent: if the server is already `https://<vm_ip>:6443`, output == input.
//!   - Must NOT touch any other line (cluster names, certificate-authority-data,
//!     user names, client-certificate-data, client-key-data, current-context).
//!
//! `build_unreachable_diagnostic`:
//!   - Returns a human-readable multi-line string.
//!   - Always includes the cluster name, vm_ip, and port.
//!   - When bridge and/or gateway are `Some`, includes those values in the output.
//!   - When bridge/gateway are `None`, must NOT panic; includes a generic network hint.
//!   - Must name the repair command `kina kubeconfig <cluster>` using the actual
//!     cluster name passed in (not a placeholder).
//!
//! ## Out of scope for this test file (integration-only, wired by the implementer)
//!
//!   - TCP reachability probe (TcpStream::connect / tokio::net::TcpStream) with
//!     bounded retry: I/O boundary, not unit-testable.
//!   - `container network inspect <cluster>` output parsing for bridge/gateway:
//!     subprocess boundary, not unit-testable.
//!   - Wiring into the `kina create` flow and the new `kina kubeconfig <cluster>`
//!     subcommand: CLI integration tested with a live cluster only.
//!   - The verify-then-repair loop (probe → rewrite → re-probe): integration only.
//!
//! All tests in this file are pure: no subprocess spawns, no network, no filesystem.

use kina_cli::core::verify::{build_unreachable_diagnostic, rewrite_kubeconfig_server};

// ===========================================================================
// Shared fixture — a realistic minimal kubeconfig.
//
// The server initially points at 127.0.0.1 (the typical in-cluster localhost
// address that makes kubectl work from inside the container but not from the
// macOS host).  The tests rewrite it to a VM IP.
// ===========================================================================

fn kubeconfig_localhost() -> &'static str {
    "apiVersion: v1\n\
clusters:\n\
- cluster:\n\
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t\n\
    server: https://127.0.0.1:6443\n\
  name: kina-test\n\
contexts:\n\
- context:\n\
    cluster: kina-test\n\
    user: kina-test-admin\n\
  name: kina-test\n\
current-context: kina-test\n\
kind: Config\n\
preferences: {}\n\
users:\n\
- name: kina-test-admin\n\
  user:\n\
    client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t\n\
    client-key-data: LS0tLS1CRUdJTiBFQyBQUklWQVRFIEtFWS0tLS0t\n"
}

fn kubeconfig_old_ip() -> &'static str {
    "apiVersion: v1\n\
clusters:\n\
- cluster:\n\
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t\n\
    server: https://192.168.65.5:6443\n\
  name: kina-test\n\
contexts:\n\
- context:\n\
    cluster: kina-test\n\
    user: kina-test-admin\n\
  name: kina-test\n\
current-context: kina-test\n\
kind: Config\n\
preferences: {}\n\
users:\n\
- name: kina-test-admin\n\
  user:\n\
    client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t\n\
    client-key-data: LS0tLS1CRUdJTiBFQyBQUklWQVRFIEtFWS0tLS0t\n"
}

// ===========================================================================
// Group A — rewrite_kubeconfig_server: localhost → vm_ip
// ===========================================================================

/// kina-39/A1 — localhost server URL is rewritten to the vm_ip.
///
/// The most common create-flow shape: the kubeconfig extracted from inside the
/// container says `server: https://127.0.0.1:6443`; the host cannot reach that
/// address from macOS.  The fn must replace 127.0.0.1 with the VM's IP.
#[test]
fn rewrite_replaces_localhost_ip_with_vm_ip() {
    let vm_ip = "10.211.55.4";
    let result = rewrite_kubeconfig_server(kubeconfig_localhost(), vm_ip);

    assert!(
        result.contains(&format!("server: https://{}:6443", vm_ip)),
        "rewrite_kubeconfig_server must produce \"server: https://{}:6443\" \
         when input has \"server: https://127.0.0.1:6443\"; got:\n{}",
        vm_ip,
        result
    );
    assert!(
        !result.contains("127.0.0.1"),
        "rewrite_kubeconfig_server must remove 127.0.0.1 from the output; \
         stale localhost still present in:\n{}",
        result
    );
}

/// kina-39/A2 — an old stale VM IP is replaced with the new vm_ip.
///
/// The `kina kubeconfig <cluster>` repair command runs after the node's IP
/// changes.  The kubeconfig may already have a non-localhost server from a
/// previous repair attempt but with the WRONG IP — the fn must overwrite it.
#[test]
fn rewrite_replaces_old_vm_ip_with_new_vm_ip() {
    let new_ip = "10.211.55.7";
    let result = rewrite_kubeconfig_server(kubeconfig_old_ip(), new_ip);

    assert!(
        result.contains(&format!("server: https://{}:6443", new_ip)),
        "rewrite_kubeconfig_server must replace an old VM IP (192.168.65.5) \
         with the new vm_ip ({}); got:\n{}",
        new_ip,
        result
    );
    assert!(
        !result.contains("192.168.65.5"),
        "rewrite_kubeconfig_server must remove the old VM IP 192.168.65.5; \
         stale IP still present in:\n{}",
        result
    );
}

/// kina-39/A3 — idempotent: server already correct → output equals input.
///
/// If the kubeconfig was already repaired (e.g. `kina kubeconfig` ran twice),
/// calling the fn again must NOT mutate the YAML — enables safe unconditional
/// invocation in the create flow.
#[test]
fn rewrite_is_idempotent_when_server_already_correct() {
    let vm_ip = "10.211.55.4";
    // Build a kubeconfig that already points at the correct IP.
    let already_correct = format!(
        "apiVersion: v1\n\
clusters:\n\
- cluster:\n\
    certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t\n\
    server: https://{}:6443\n\
  name: kina-test\n\
current-context: kina-test\n\
kind: Config\n",
        vm_ip
    );

    let result = rewrite_kubeconfig_server(&already_correct, vm_ip);

    assert_eq!(
        result, already_correct,
        "rewrite_kubeconfig_server must be idempotent: when the server is \
         already https://{}:6443 the output must equal the input exactly;\n\
         input:\n{}\noutput:\n{}",
        vm_ip, already_correct, result
    );
}

/// kina-39/A4 — non-server lines are preserved verbatim.
///
/// The fn must be surgical: only the server URL host changes.  All other
/// YAML content (cluster name, certificate-authority-data, user names,
/// client certs, current-context, apiVersion) must survive untouched.
#[test]
fn rewrite_preserves_non_server_lines_verbatim() {
    let vm_ip = "10.211.55.4";
    let result = rewrite_kubeconfig_server(kubeconfig_localhost(), vm_ip);

    let preserved = [
        "apiVersion: v1",
        "certificate-authority-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t",
        "name: kina-test",
        "current-context: kina-test",
        "kind: Config",
        "preferences: {}",
        "kina-test-admin",
        "client-certificate-data: LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0t",
        "client-key-data: LS0tLS1CRUdJTiBFQyBQUklWQVRFIEtFWS0tLS0t",
    ];

    for expected_line in &preserved {
        assert!(
            result.contains(expected_line),
            "rewrite_kubeconfig_server must preserve non-server line {:?} verbatim; \
             it was not found in the output:\n{}",
            expected_line,
            result
        );
    }
}

/// kina-39/A5 — port 6443 is preserved; only the host portion changes.
///
/// Triangulation: two distinct vm_ip values both produce :6443 — proves
/// the fn is not hardcoding the port alongside the host replacement.
#[test]
fn rewrite_preserves_port_6443_only_host_changes() {
    let ip_a = "10.211.55.4";
    let ip_b = "172.16.0.10";

    let result_a = rewrite_kubeconfig_server(kubeconfig_localhost(), ip_a);
    let result_b = rewrite_kubeconfig_server(kubeconfig_old_ip(), ip_b);

    assert!(
        result_a.contains(&format!("https://{}:6443", ip_a)),
        "rewrite_kubeconfig_server with vm_ip={} must produce https://{}:6443 \
         (port 6443 preserved); got:\n{}",
        ip_a,
        ip_a,
        result_a
    );
    assert!(
        result_b.contains(&format!("https://{}:6443", ip_b)),
        "rewrite_kubeconfig_server with vm_ip={} must produce https://{}:6443 \
         (port 6443 preserved); got:\n{}",
        ip_b,
        ip_b,
        result_b
    );
}

// ===========================================================================
// Group B — build_unreachable_diagnostic
// ===========================================================================

/// kina-39/B1 — diagnostic contains cluster name, vm_ip, and port.
///
/// The operator must be able to tell from the message alone which cluster
/// is unreachable and at what address to investigate.
#[test]
fn diagnostic_contains_cluster_ip_and_port() {
    let msg = build_unreachable_diagnostic("kina-test", "10.211.55.4", 6443, None, None);

    assert!(
        msg.contains("kina-test"),
        "build_unreachable_diagnostic must name the cluster \"kina-test\"; got:\n{}",
        msg
    );
    assert!(
        msg.contains("10.211.55.4"),
        "build_unreachable_diagnostic must include the VM IP \"10.211.55.4\"; got:\n{}",
        msg
    );
    assert!(
        msg.contains("6443"),
        "build_unreachable_diagnostic must include port \"6443\"; got:\n{}",
        msg
    );
}

/// kina-39/B2 — diagnostic includes bridge and gateway when provided.
///
/// When `container network inspect` succeeds, the caller passes the bridge
/// name and default gateway so the operator can identify the routing path.
/// Both values must appear in the output.
#[test]
fn diagnostic_includes_bridge_and_gateway_when_some() {
    let msg = build_unreachable_diagnostic(
        "my-cluster",
        "192.168.64.5",
        6443,
        Some("bridge100"),
        Some("192.168.64.1"),
    );

    assert!(
        msg.contains("bridge100"),
        "build_unreachable_diagnostic must include bridge name \"bridge100\" \
         when provided; got:\n{}",
        msg
    );
    assert!(
        msg.contains("192.168.64.1"),
        "build_unreachable_diagnostic must include gateway \"192.168.64.1\" \
         when provided; got:\n{}",
        msg
    );
}

/// kina-39/B3 — bridge/gateway None → no panic; still mentions vm_ip:port.
///
/// When `container network inspect` fails or is unavailable (e.g. during
/// the create flow before the network is fully up), the fn must not panic
/// and must still emit the minimum useful information.
#[test]
fn diagnostic_with_none_bridge_gateway_does_not_panic() {
    let msg = build_unreachable_diagnostic("kina-test", "10.211.55.4", 6443, None, None);

    // Must survive with None args (no panic is the gate; the assertion proves it returned)
    assert!(
        !msg.is_empty(),
        "build_unreachable_diagnostic must return a non-empty string even \
         when bridge and gateway are None"
    );
    // Must still name the unreachable address so the operator knows what to check
    assert!(
        msg.contains("10.211.55.4"),
        "build_unreachable_diagnostic must include the VM IP even with \
         None bridge/gateway; got:\n{}",
        msg
    );
    assert!(
        msg.contains("6443"),
        "build_unreachable_diagnostic must include the port even with \
         None bridge/gateway; got:\n{}",
        msg
    );
}

/// kina-39/B4 — diagnostic names the repair command `kina kubeconfig <cluster>`.
///
/// The message must point the operator at the repair path using the actual
/// cluster name passed in — not a placeholder like `<cluster>` or `kina-cluster`.
#[test]
fn diagnostic_names_repair_command_with_cluster() {
    let cluster = "prod-cluster";
    let msg = build_unreachable_diagnostic(cluster, "10.211.55.4", 6443, None, None);

    let repair_cmd = format!("kina kubeconfig {}", cluster);
    assert!(
        msg.contains(&repair_cmd),
        "build_unreachable_diagnostic must include the repair command \
         \"kina kubeconfig {}\" (cluster name interpolated, not a placeholder); got:\n{}",
        cluster,
        msg
    );
}

/// kina-39/B5 — triangulation: different cluster names produce different repair commands.
///
/// Proves that the repair command interpolates the cluster argument rather
/// than hardcoding a single name from the B4 test.
#[test]
fn diagnostic_repair_command_uses_given_cluster_name() {
    let msg_a = build_unreachable_diagnostic("alpha", "10.0.0.1", 6443, None, None);
    let msg_b = build_unreachable_diagnostic("beta", "10.0.0.2", 6443, None, None);

    assert!(
        msg_a.contains("kina kubeconfig alpha"),
        "build_unreachable_diagnostic(\"alpha\", ...) must say \
         \"kina kubeconfig alpha\"; got:\n{}",
        msg_a
    );
    assert!(
        msg_b.contains("kina kubeconfig beta"),
        "build_unreachable_diagnostic(\"beta\", ...) must say \
         \"kina kubeconfig beta\"; got:\n{}",
        msg_b
    );
}

/// kina-39/B6 — diagnostic is multi-line and human-readable.
///
/// A single-line error message forces the operator to parse a long string.
/// The diagnostic must contain at least one newline and a recognisable header
/// (e.g. "unreachable", "cannot reach", "API server", or similar) so that it
/// reads as an actionable message, not a log token.
#[test]
fn diagnostic_is_multiline_with_recognisable_header() {
    let msg = build_unreachable_diagnostic("kina-test", "10.211.55.4", 6443, None, None);

    assert!(
        msg.contains('\n'),
        "build_unreachable_diagnostic must return a multi-line string \
         (contains at least one '\\n'); got a single-line:\n{}",
        msg
    );

    // At least one of these human-readable signals must appear somewhere in the output.
    let readable_signals = [
        "unreachable",
        "cannot reach",
        "API server",
        "api server",
        "6443",
    ];
    let has_readable = readable_signals
        .iter()
        .any(|s| msg.to_lowercase().contains(s));
    assert!(
        has_readable,
        "build_unreachable_diagnostic must contain a recognisable header \
         (one of {:?}); got:\n{}",
        readable_signals, msg
    );
}
