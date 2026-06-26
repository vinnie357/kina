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

// ===========================================================================
// kina-34 — controller label
// ===========================================================================

/// Return the active-controller label string for `${CONTROLLER}` substitution
/// in demo-app.yaml templates.
///   gateway_present = true  → "traefik"
///   gateway_present = false → "nginx-ingress"
pub fn select_controller_label(gateway_present: bool) -> &'static str {
    if gateway_present {
        "traefik"
    } else {
        "nginx-ingress"
    }
}

// ===========================================================================
// kina-35 — ingress readiness classification
// ===========================================================================

/// Ingress controller readiness as classified from raw kubectl output.
#[derive(Debug, PartialEq)]
pub enum IngressReadiness {
    /// Namespace absent or empty — controller is optional, not an error.
    NotInstalled,
    /// Namespace has pods; ready/total counts from the pods stdout.
    Ready { ready: usize, total: usize },
    /// kubectl process failed for a reason other than "namespace not found"
    /// (e.g. API server unreachable). Must NOT be silently collapsed to
    /// NotInstalled — callers surface this to the user.
    CommandFailure(String),
}

/// Map raw kubectl output to a typed readiness status.
///
/// Decision table:
///   exit_ok=false, stderr contains "not found"          → NotInstalled
///   exit_ok=false, stderr does NOT contain "not found"  → CommandFailure(stderr)
///   exit_ok=true, pods_stdout is empty                  → NotInstalled
///   exit_ok=true, pods_stdout has lines                 → Ready { ready, total }
///     (uses the same ready-column logic as pods_all_ready)
pub fn classify_ingress_kubectl_result(
    exit_ok: bool,
    pods_stdout: &str,
    stderr: &str,
) -> IngressReadiness {
    if !exit_ok {
        if stderr.to_lowercase().contains("not found") {
            return IngressReadiness::NotInstalled;
        }
        return IngressReadiness::CommandFailure(stderr.to_string());
    }
    // exit_ok = true
    match pods_all_ready(pods_stdout) {
        None => IngressReadiness::NotInstalled,
        Some((ready, total)) => IngressReadiness::Ready { ready, total },
    }
}

// ===========================================================================
// kina-36 — route type selection and conflict guard
// ===========================================================================

/// Routing object type to apply for the demo app.
#[derive(Debug, PartialEq)]
pub enum DemoRouteType {
    /// Apply demo-app-route.yaml (HTTPRoute targeting the Traefik Gateway).
    HttpRoute,
    /// Apply demo-app-ingress.yaml (nginx Ingress).
    NginxIngress,
}

/// Decide which routing object to apply for the demo app.
///   gateway_present = true  → HttpRoute
///   gateway_present = false → NginxIngress
pub fn select_demo_route_type(gateway_present: bool) -> DemoRouteType {
    if gateway_present {
        DemoRouteType::HttpRoute
    } else {
        DemoRouteType::NginxIngress
    }
}

/// Return `Some(message)` when installing `installing_controller` is blocked
/// because the conflicting controller's namespace is already present.
/// Return `None` when installation is allowed.
///
/// installing_controller: "nginx-ingress" or "traefik"
/// conflicting_ns_present: true = the OTHER controller's namespace exists
///
/// When installing "nginx-ingress" and blocked, message must mention "traefik".
/// When installing "traefik" and blocked, message must mention "nginx-ingress".
pub fn controller_conflict_message(
    installing_controller: &str,
    conflicting_ns_present: bool,
) -> Option<String> {
    if !conflicting_ns_present {
        return None;
    }
    let conflicting = match installing_controller {
        "nginx-ingress" => "traefik",
        "traefik" => "nginx-ingress",
        other => {
            return Some(format!(
                "A conflicting ingress controller is already installed; \
                 cannot install {}.",
                other
            ))
        }
    };
    Some(format!(
        "{} is already installed. Only one ingress controller can bind host \
         ports 80/443. Remove it before installing {}.",
        conflicting, installing_controller
    ))
}
