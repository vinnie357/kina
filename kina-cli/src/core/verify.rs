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
/// The `container system dns list` command emits a header line (`DOMAIN`) before
/// the actual domain entries.  This function skips that header and any other
/// all-uppercase single-word header lines, then returns the first non-blank
/// trimmed data line.  Falls back to `"test"` when the output is empty, contains
/// only whitespace, or only the header — matching AC1/AC3.
///
/// # Example output from `container system dns list`
/// ```text
/// DOMAIN
/// local
/// ```
/// → returns `"local"`.
pub fn parse_dns_domain(dns_list_output: &str) -> String {
    dns_list_output
        .lines()
        .map(|l| l.trim())
        .find(|l| {
            // Skip blank lines and the "DOMAIN" column-header emitted by
            // `container system dns list`.  A header line is all-uppercase ASCII
            // (e.g. "DOMAIN"); real domain names contain lowercase letters.
            !l.is_empty() && !l.chars().all(|c| c.is_ascii_uppercase())
        })
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

/// Result of inspecting a cluster's CNI via its Cilium pods.
#[derive(Debug, PartialEq)]
pub enum CniReport {
    /// No Cilium pods — cluster uses PTP, which has no controller pods to poll.
    Ptp,
    /// Cilium is present; `ready`/`total` pod counts.
    Cilium { ready: usize, total: usize },
}

/// Decide CNI status from `kubectl get pods -l k8s-app=cilium --no-headers`
/// stdout. Empty output => PTP. Otherwise count pods whose READY column is `1/1`.
pub fn cni_report_from_cilium_pods(stdout: &str) -> CniReport {
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return CniReport::Ptp;
    }
    let total = lines.len();
    let ready = lines
        .iter()
        .filter(|l| {
            l.split_whitespace()
                .nth(1)
                .is_some_and(|s| s.starts_with("1/1"))
        })
        .count();
    CniReport::Cilium { ready, total }
}

/// Parse `kubectl get nodes -o custom-columns=NAME:.metadata.name,VERSION:.status.nodeInfo.kubeletVersion`
/// output into a name→version map, skipping the `NAME ...` header line.
pub fn parse_node_versions(stdout: &str) -> std::collections::HashMap<String, String> {
    stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("NAME"))
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            match (it.next(), it.next()) {
                (Some(name), Some(ver)) => Some((name.to_string(), ver.to_string())),
                _ => None,
            }
        })
        .collect()
}

/// Parse `kubectl get pods -n <ns> --no-headers` output into (ready, total).
/// A pod is "ready" when its READY column (col 1, "a/b") has a == b.
/// Returns `None` when there are no pod lines (namespace empty / absent).
pub fn pods_all_ready(no_headers_stdout: &str) -> Option<(usize, usize)> {
    let lines: Vec<&str> = no_headers_stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }
    let total = lines.len();
    let ready = lines
        .iter()
        .filter(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|col| col.split_once('/'))
                .map(|(a, b)| a == b)
                .unwrap_or(false)
        })
        .count();
    Some((ready, total))
}
