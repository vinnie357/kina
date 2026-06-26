//! Adversarial-TDD tests for Traefik addon followup issues: kina-34, kina-35, kina-36.
//!
//! ## P2 contract — these tests are INTENTIONALLY RED
//!
//! Every test in this file references a pub fn or type that does NOT yet exist in
//! `kina_cli::core::verify`.  The compile errors ARE the spec.  The separate P3
//! implementer makes them green WITHOUT modifying this file.
//!
//! All tests are pure: no live kubectl, no process spawns, no network, no filesystem.
//!
//! ## Implementer binding (kina-cli/src/core/verify.rs)
//!
//! Add the following items to `kina_cli::core::verify` — exact signatures are binding:
//!
//! ### kina-34 — controller label
//! ```rust
//! /// Return the active-controller label string for `${CONTROLLER}` substitution
//! /// in demo-app.yaml templates.
//! ///   gateway_present = true  → "traefik"
//! ///   gateway_present = false → "nginx-ingress"
//! pub fn select_controller_label(gateway_present: bool) -> &'static str;
//! ```
//! Also: add a `${CONTROLLER}` placeholder in `kina-cli/manifests/demo-app.yaml`
//! at line 198 (the hardcoded `nginx-ingress` info-value) and update
//! `render_demo_manifest` (or the `install_demo_app` call-site) to substitute it.
//!
//! ### kina-35 — ingress readiness classification
//! ```rust
//! #[derive(Debug, PartialEq)]
//! pub enum IngressReadiness {
//!     /// Namespace absent or empty — controller is optional, not an error.
//!     NotInstalled,
//!     /// Namespace has pods; ready/total counts from the pods stdout.
//!     Ready { ready: usize, total: usize },
//!     /// kubectl process failed for a reason other than "namespace not found"
//!     /// (e.g. API server unreachable).  Must NOT be silently collapsed to
//!     /// NotInstalled — callers surface this to the user.
//!     CommandFailure(String),
//! }
//!
//! /// Map raw kubectl output to a typed readiness status.
//! ///
//! /// Decision table:
//! ///   exit_ok=false, stderr contains "not found"          → NotInstalled
//! ///   exit_ok=false, stderr does NOT contain "not found"  → CommandFailure(stderr)
//! ///   exit_ok=true, pods_stdout is empty                  → NotInstalled
//! ///   exit_ok=true, pods_stdout has lines                 → Ready { ready, total }
//! ///     (uses the same ready-column logic as pods_all_ready)
//! pub fn classify_ingress_kubectl_result(
//!     exit_ok: bool,
//!     pods_stdout: &str,
//!     stderr: &str,
//! ) -> IngressReadiness;
//! ```
//!
//! ### kina-36 — route type selection
//! ```rust
//! #[derive(Debug, PartialEq)]
//! pub enum DemoRouteType {
//!     /// Apply demo-app-route.yaml (HTTPRoute targeting the Traefik Gateway).
//!     HttpRoute,
//!     /// Apply demo-app-ingress.yaml (nginx Ingress).
//!     NginxIngress,
//! }
//!
//! /// Decide which routing object to apply for the demo app.
//! ///   gateway_present = true  → HttpRoute
//! ///   gateway_present = false → NginxIngress
//! pub fn select_demo_route_type(gateway_present: bool) -> DemoRouteType;
//! ```
//!
//! ### kina-36 — one-controller conflict guard
//! ```rust
//! /// Return `Some(message)` when installing `installing_controller` is blocked
//! /// because the conflicting controller's namespace is already present.
//! /// Return `None` when installation is allowed.
//! ///
//! /// installing_controller: "nginx-ingress" or "traefik"
//! /// conflicting_ns_present: true = the OTHER controller's namespace exists
//! ///
//! /// When installing "nginx-ingress" and blocked, message must mention "traefik".
//! /// When installing "traefik" and blocked, message must mention "nginx-ingress".
//! pub fn controller_conflict_message(
//!     installing_controller: &str,
//!     conflicting_ns_present: bool,
//! ) -> Option<String>;
//! ```
//!
//! ## kina-33 note (out of scope for unit tests)
//!
//! kina-33 (replace fixed `tokio::time::sleep` in install_* with bounded readiness polling)
//! requires process-level integration: the sleep is inside `install_nginx_ingress`,
//! `install_traefik`, and `install_demo_app` and drives real kubectl pod-wait logic.
//! No pure seam can be isolated for it at this time.  The implementer must replace
//! the 5-second fixed `tokio::time::sleep` calls with a bounded readiness poll that
//! calls `kubectl wait --for=condition=Ready` (or equivalent) with a configurable
//! timeout, without adding a unit-testable pure function.

use kina_cli::core::verify::{
    classify_ingress_kubectl_result, controller_conflict_message, select_controller_label,
    select_demo_route_type, DemoRouteType, IngressReadiness,
};

// ===========================================================================
// kina-34: select_controller_label
// The demo-app HTML page must reflect the ACTIVE controller; currently the
// template has a hardcoded "nginx-ingress" at manifests/demo-app.yaml:198.
// ===========================================================================

/// kina-34/T1 — gateway present → controller label is "traefik".
/// When a Traefik Gateway exists the branding must say "traefik", not "nginx-ingress".
#[test]
fn controller_label_is_traefik_when_gateway_present() {
    let label = select_controller_label(true);
    assert_eq!(
        label, "traefik",
        "When a Gateway is present the controller label must be \"traefik\", got \"{}\"",
        label,
    );
}

/// kina-34/T2 — no gateway → controller label is "nginx-ingress".
/// When nginx-ingress is active, the branding must say "nginx-ingress".
#[test]
fn controller_label_is_nginx_ingress_when_no_gateway() {
    let label = select_controller_label(false);
    assert_eq!(
        label, "nginx-ingress",
        "When no Gateway is present the controller label must be \"nginx-ingress\", got \"{}\"",
        label,
    );
}

// ===========================================================================
// kina-35: classify_ingress_kubectl_result
// Must distinguish controller-absent (NotInstalled) from API failure (CommandFailure).
// ===========================================================================

/// kina-35/T1 — exit_ok=false, stderr mentions "not found" → NotInstalled (optional absence).
/// Namespace doesn't exist = controller never installed = not an error.
#[test]
fn ingress_readiness_namespace_not_found_is_not_installed() {
    let result = classify_ingress_kubectl_result(
        false,
        "",
        "Error from server (NotFound): namespaces \"traefik\" not found",
    );
    assert_eq!(
        result,
        IngressReadiness::NotInstalled,
        "Namespace-not-found stderr must map to NotInstalled, got {:?}",
        result,
    );
}

/// kina-35/T2 — exit_ok=false, stderr is a connectivity failure → CommandFailure, NOT NotInstalled.
/// Silently collapsing an API failure into "not installed" hides real cluster problems.
#[test]
fn ingress_readiness_connection_failure_is_command_failure() {
    let stderr = "Unable to connect to the server: dial tcp 127.0.0.1:6443: connection refused";
    let result = classify_ingress_kubectl_result(false, "", stderr);
    assert!(
        matches!(result, IngressReadiness::CommandFailure(_)),
        "API/connection failure must map to CommandFailure, got {:?}",
        result,
    );
}

/// kina-35/T3 — exit_ok=true, empty pods stdout → NotInstalled (namespace exists but is empty).
#[test]
fn ingress_readiness_empty_pods_is_not_installed() {
    let result = classify_ingress_kubectl_result(true, "", "");
    assert_eq!(
        result,
        IngressReadiness::NotInstalled,
        "Empty pods stdout with exit_ok=true must map to NotInstalled, got {:?}",
        result,
    );
}

/// kina-35/T4 — exit_ok=true, all pods ready → Ready { ready: 2, total: 2 }.
#[test]
fn ingress_readiness_all_pods_ready_is_ready() {
    let stdout = "traefik-abc   1/1   Running   0   2m\ntraefik-def   1/1   Running   0   2m\n";
    let result = classify_ingress_kubectl_result(true, stdout, "");
    assert_eq!(
        result,
        IngressReadiness::Ready { ready: 2, total: 2 },
        "All-ready pods must map to Ready {{ready:2, total:2}}, got {:?}",
        result,
    );
}

/// kina-35/T5 — exit_ok=true, 1 of 2 pods ready → Ready { ready: 1, total: 2 }.
/// Partial-ready is still classified Ready (with accurate counts), not NotInstalled.
#[test]
fn ingress_readiness_partial_ready_reports_counts() {
    let stdout = "traefik-abc   1/1   Running   0   2m\ntraefik-def   0/1   Pending   0   2m\n";
    let result = classify_ingress_kubectl_result(true, stdout, "");
    assert_eq!(
        result,
        IngressReadiness::Ready { ready: 1, total: 2 },
        "Partial-ready pods must map to Ready {{ready:1, total:2}}, got {:?}",
        result,
    );
}

// ===========================================================================
// kina-36: select_demo_route_type — route auto-selection
// ===========================================================================

/// kina-36/T1 — gateway present → select HTTPRoute (Traefik Gateway API).
#[test]
fn route_type_is_httproute_when_gateway_present() {
    let route = select_demo_route_type(true);
    assert_eq!(
        route,
        DemoRouteType::HttpRoute,
        "Gateway present must select HttpRoute, got {:?}",
        route,
    );
}

/// kina-36/T2 — no gateway → select NginxIngress.
#[test]
fn route_type_is_nginx_ingress_when_no_gateway() {
    let route = select_demo_route_type(false);
    assert_eq!(
        route,
        DemoRouteType::NginxIngress,
        "No gateway must select NginxIngress, got {:?}",
        route,
    );
}

// ===========================================================================
// kina-36: controller_conflict_message — one-controller guard (both directions)
// ===========================================================================

/// kina-36/T3 — traefik namespace present → nginx-ingress install blocked.
/// Exercises the traefik-present-blocks-nginx direction of the guard.
#[test]
fn nginx_install_blocked_when_traefik_namespace_present() {
    let msg = controller_conflict_message("nginx-ingress", true);
    assert!(
        msg.is_some(),
        "nginx-ingress install must be blocked when traefik namespace is present",
    );
    let msg = msg.unwrap();
    assert!(
        msg.to_lowercase().contains("traefik"),
        "Block message for nginx-ingress must mention \"traefik\"; got: {}",
        msg,
    );
}

/// kina-36/T4 — traefik namespace absent → nginx-ingress install allowed.
#[test]
fn nginx_install_allowed_when_traefik_namespace_absent() {
    let msg = controller_conflict_message("nginx-ingress", false);
    assert!(
        msg.is_none(),
        "nginx-ingress install must be allowed when traefik is absent, got: {:?}",
        msg,
    );
}

/// kina-36/T5 — nginx-ingress namespace present → traefik install blocked.
/// Exercises the nginx-present-blocks-traefik direction of the guard.
#[test]
fn traefik_install_blocked_when_nginx_ingress_namespace_present() {
    let msg = controller_conflict_message("traefik", true);
    assert!(
        msg.is_some(),
        "traefik install must be blocked when nginx-ingress namespace is present",
    );
    let msg = msg.unwrap();
    assert!(
        msg.to_lowercase().contains("nginx-ingress"),
        "Block message for traefik must mention \"nginx-ingress\"; got: {}",
        msg,
    );
}

/// kina-36/T6 — nginx-ingress namespace absent → traefik install allowed.
#[test]
fn traefik_install_allowed_when_nginx_ingress_namespace_absent() {
    let msg = controller_conflict_message("traefik", false);
    assert!(
        msg.is_none(),
        "traefik install must be allowed when nginx-ingress is absent, got: {:?}",
        msg,
    );
}
