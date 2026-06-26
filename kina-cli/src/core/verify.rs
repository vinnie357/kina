//! Pure helper functions for the `kina verify` command.
//!
//! All functions in this module are pure (no side effects, no I/O, no subprocess calls).
//! They are unit-tested in kina-cli/tests/verify_cmd_tests.rs.

/// Substitute manifest placeholders for the demo app.
///
/// Replaced placeholders:
///   `${CLUSTER_NAME}`    — cluster name
///   `${DNS_DOMAIN}`      — DNS domain
///   `${CONTROLLER}`      — active controller label (e.g. "traefik", "nginx-ingress")
///   `${GATEWAY_NAME}`    — Gateway object name for HTTPRoute parentRef
///   `${GATEWAY_NAMESPACE}` — Gateway namespace for HTTPRoute parentRef
///   `${GATEWAY_SECTION}` — Gateway listener sectionName for HTTPRoute parentRef
///
/// Pod-runtime variables such as `${MY_POD_NAME}` are left untouched for the
/// pod's own envsubst.
pub fn render_demo_manifest(
    manifest: &str,
    cluster: &str,
    domain: &str,
    controller: &str,
    gateway_name: &str,
    gateway_ns: &str,
    gateway_section: &str,
) -> String {
    manifest
        .replace("${CLUSTER_NAME}", cluster)
        .replace("${DNS_DOMAIN}", domain)
        .replace("${CONTROLLER}", controller)
        .replace("${GATEWAY_NAME}", gateway_name)
        .replace("${GATEWAY_NAMESPACE}", gateway_ns)
        .replace("${GATEWAY_SECTION}", gateway_section)
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
// kina-32 — ActiveController enum and associated pure helpers
// ===========================================================================

/// Active ingress/gateway controller installed in the cluster.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ActiveController {
    Traefik,
    NginxGatewayFabric,
    NginxIngress,
    None,
}

/// Label string used in `${CONTROLLER}` substitution and log messages.
///
/// - `Traefik`             → `"traefik"`
/// - `NginxGatewayFabric`  → `"nginx-gateway-fabric"`
/// - `NginxIngress`        → `"nginx-ingress"`
/// - `None`                → `"nginx-ingress"` (sensible fallback)
pub fn controller_label(c: ActiveController) -> &'static str {
    match c {
        ActiveController::Traefik => "traefik",
        ActiveController::NginxGatewayFabric => "nginx-gateway-fabric",
        ActiveController::NginxIngress | ActiveController::None => "nginx-ingress",
    }
}

/// Gateway parentRef triple (name, namespace, sectionName); `None` for
/// non-gateway controllers.
///
/// - `Traefik`             → `Some(("traefik",  "traefik",       "web"))`
/// - `NginxGatewayFabric`  → `Some(("nginx",    "nginx-gateway", "http"))`
/// - `NginxIngress` / `None` → `None`
pub fn gateway_parent_ref(
    c: ActiveController,
) -> Option<(&'static str, &'static str, &'static str)> {
    match c {
        ActiveController::Traefik => Some(("traefik", "traefik", "web")),
        ActiveController::NginxGatewayFabric => Some(("nginx", "nginx-gateway", "http")),
        ActiveController::NginxIngress | ActiveController::None => None,
    }
}

/// All known controller (id, namespace) pairs.
///
/// Note that NGF's id (`"nginx-gateway-fabric"`) differs from its namespace
/// (`"nginx-gateway"`); this asymmetry is intentional and load-bearing.
pub fn ingress_controllers() -> [(&'static str, &'static str); 3] {
    [
        ("nginx-ingress", "nginx-ingress"),
        ("traefik", "traefik"),
        ("nginx-gateway-fabric", "nginx-gateway"),
    ]
}

/// Block-guard: `Some(message)` iff any controller id in `present` is different
/// from `installing`.  Returns `None` when installation is allowed.
///
/// The message names all conflicting controllers so the user knows exactly what
/// to remove.  A controller does NOT block itself (idempotent re-install).
pub fn controller_conflict_message_multi(installing: &str, present: &[&str]) -> Option<String> {
    let conflicts: Vec<&str> = present
        .iter()
        .filter(|&&id| id != installing)
        .copied()
        .collect();
    if conflicts.is_empty() {
        return None;
    }
    Some(format!(
        "Cannot install {}: conflicting ingress controller(s) already installed: {}. \
         Only one ingress controller can bind host ports 80/443. \
         Remove it before installing {}.",
        installing,
        conflicts.join(", "),
        installing,
    ))
}

/// (namespace, display-label) targets probed by `check_ingress_ready` to detect
/// installed controllers.
pub fn ingress_probe_targets() -> [(&'static str, &'static str); 3] {
    [
        ("traefik", "Gateway (traefik)"),
        ("nginx-ingress", "Ingress (nginx)"),
        ("nginx-gateway", "Gateway (nginx-gateway-fabric)"),
    ]
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
// kina-36 — route type selection
// ===========================================================================

/// Routing object type to apply for the demo app.
#[derive(Debug, PartialEq)]
pub enum DemoRouteType {
    /// Apply demo-app-route.yaml (HTTPRoute via Gateway API).
    HttpRoute,
    /// Apply demo-app-ingress.yaml (nginx Ingress object).
    NginxIngress,
}

/// Decide which routing object to apply for the demo app.
///
/// - `Traefik` / `NginxGatewayFabric` → `HttpRoute` (Gateway API HTTPRoute)
/// - `NginxIngress` / `None`           → `NginxIngress` (legacy Ingress object)
pub fn demo_route_type(c: ActiveController) -> DemoRouteType {
    match c {
        ActiveController::Traefik | ActiveController::NginxGatewayFabric => {
            DemoRouteType::HttpRoute
        }
        ActiveController::NginxIngress | ActiveController::None => DemoRouteType::NginxIngress,
    }
}
