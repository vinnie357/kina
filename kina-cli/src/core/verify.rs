//! Pure helper functions for the `kina verify` command.
//!
//! All functions in this module are pure (no side effects, no I/O, no subprocess calls).
//! They are unit-tested in kina-cli/tests/verify_cmd_tests.rs.

/// Substitute `${CLUSTER_NAME}` and `${DNS_DOMAIN}` placeholders in a manifest string.
///
/// Only these two placeholders are substituted at render time; pod-runtime variables
/// such as `${MY_POD_NAME}` are left untouched for the pod's own envsubst.
pub fn render_demo_manifest(manifest: &str, cluster: &str, domain: &str) -> String {
    manifest
        .replace("${CLUSTER_NAME}", cluster)
        .replace("${DNS_DOMAIN}", domain)
}

/// Build the Host-header value used when probing a worker node.
///
/// Returns `"<cluster>-control-plane.<domain>"`.
pub fn probe_host(cluster: &str, domain: &str) -> String {
    format!("{}-control-plane.{}", cluster, domain)
}

/// Build the probe URL for a given node IP.
///
/// Returns `"http://<node_ip>"`.  The Host header is carried separately via
/// [`probe_host`] so that the verify command can issue
/// `curl -H "Host: <probe_host>" <probe_url>`.
pub fn probe_url(node_ip: &str) -> String {
    format!("http://{}", node_ip)
}

/// Parse the DNS domain from the output of `container system dns list`.
///
/// Takes the first non-blank trimmed line.  Falls back to `"test"` when
/// the output is empty or contains only whitespace — matching AC1/AC3.
pub fn parse_dns_domain(dns_list_output: &str) -> String {
    dns_list_output
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("test")
        .to_string()
}

/// Return `true` iff the HTTP response body contains the demo success marker.
pub fn probe_passed(body: &str) -> bool {
    body.contains("Kina Demo Success")
}

/// A single probe result from a worker node.
pub struct ProbeResult {
    /// Node label (e.g. its IP or name) — used for display.
    pub node: String,
    /// Whether the probe succeeded.
    pub passed: bool,
}

/// Aggregate a slice of probe results into an overall pass/fail decision.
///
/// Returns `true` IFF the slice is non-empty AND every result passed.
/// An empty slice returns `false` (zero probes = no evidence = FAIL).
pub fn aggregate_verify(results: &[ProbeResult]) -> bool {
    !results.is_empty() && results.iter().all(|r| r.passed)
}

/// Decide the HTTP-layer pass/fail for the verify command.
///
/// Returns `false` when `node_ips` is empty — empty IPs means
/// `get_cluster_status` failed or produced no addresses, which is itself a
/// failure: zero HTTP evidence must never be reported as PASS.
///
/// When `node_ips` is non-empty the decision delegates to
/// [`aggregate_verify`] over the collected `results`.
pub fn http_layer_pass(node_ips: &[String], results: &[ProbeResult]) -> bool {
    if node_ips.is_empty() {
        return false;
    }
    aggregate_verify(results)
}
