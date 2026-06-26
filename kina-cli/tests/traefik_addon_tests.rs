//! Adversarial-TDD tests for Traefik addon followup issues: kina-34, kina-35, kina-36.
//!
//! ## Migration note (kina-32 / nginx-gateway-fabric addon)
//!
//! This file has been updated from the two-arg bool API to the `ActiveController` enum
//! API.  The old functions (`select_controller_label`, `select_demo_route_type`,
//! `controller_conflict_message`) are REMOVED by the implementer; this file now calls:
//!
//!   `controller_label(ActiveController)`
//!   `demo_route_type(ActiveController)`
//!   `gateway_parent_ref(ActiveController)`
//!   `controller_conflict_message_multi(installing, present)`
//!
//! Coverage is equivalent to the previous version.  One new Traefik-specific test
//! (T36/Traefik-gateway-parent-ref) is added to exercise the parentRef accessor.
//!
//! ## P2 contract — these tests are INTENTIONALLY RED until the implementer
//! adds the new API to `kina_cli::core::verify`.
//!
//! All tests are pure: no live kubectl, no process spawns, no network, no filesystem.
//!
//! ## Implementer binding (kina-cli/src/core/verify.rs)
//!
//! Remove the following (no longer tested here, superseded by the enum API):
//!   `pub fn select_controller_label(gateway_present: bool) -> &'static str`
//!   `pub fn select_demo_route_type(gateway_present: bool) -> DemoRouteType`
//!   `pub fn controller_conflict_message(installing: &str, conflicting_ns_present: bool) -> Option<String>`
//!
//! Add (exact signatures — binding for this file and nginx_gateway_fabric_addon_tests.rs):
//!   `pub enum ActiveController { Traefik, NginxGatewayFabric, NginxIngress, None }`
//!   `pub fn controller_label(c: ActiveController) -> &'static str`
//!   `pub fn demo_route_type(c: ActiveController) -> DemoRouteType`
//!   `pub fn gateway_parent_ref(c: ActiveController) -> Option<(&'static str, &'static str, &'static str)>`
//!   `pub fn controller_conflict_message_multi(installing: &str, present: &[&str]) -> Option<String>`

use kina_cli::core::verify::{
    classify_ingress_kubectl_result, controller_conflict_message_multi, controller_label,
    demo_route_type, gateway_parent_ref, ActiveController, DemoRouteType, IngressReadiness,
};

// ===========================================================================
// kina-34 (migrated): controller_label — gateway label for ${CONTROLLER}
// ===========================================================================

/// kina-34/T1 — Traefik controller label is "traefik".
/// When the Traefik Gateway is active the template must say "traefik".
#[test]
fn controller_label_is_traefik_when_gateway_present() {
    let label = controller_label(ActiveController::Traefik);
    assert_eq!(
        label, "traefik",
        "controller_label(Traefik) must return \"traefik\"; got \"{}\"",
        label,
    );
}

/// kina-34/T2 — NginxIngress controller label is "nginx-ingress".
/// When no gateway is active the label must revert to "nginx-ingress".
#[test]
fn controller_label_is_nginx_ingress_when_no_gateway() {
    let label = controller_label(ActiveController::NginxIngress);
    assert_eq!(
        label, "nginx-ingress",
        "controller_label(NginxIngress) must return \"nginx-ingress\"; got \"{}\"",
        label,
    );
}

// ===========================================================================
// kina-35: classify_ingress_kubectl_result (unchanged — no API migration)
// Must distinguish controller-absent (NotInstalled) from API failure (CommandFailure).
// ===========================================================================

/// kina-35/T1 — exit_ok=false, stderr mentions "not found" → NotInstalled.
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

/// kina-35/T2 — exit_ok=false, stderr is a connectivity failure → CommandFailure.
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

/// kina-35/T3 — exit_ok=true, empty pods stdout → NotInstalled.
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
// kina-36 (migrated): demo_route_type — route auto-selection
// ===========================================================================

/// kina-36/T1 — Traefik → HttpRoute (Gateway API HTTPRoute).
#[test]
fn route_type_is_httproute_when_traefik_active() {
    let route = demo_route_type(ActiveController::Traefik);
    assert_eq!(
        route,
        DemoRouteType::HttpRoute,
        "demo_route_type(Traefik) must be HttpRoute, got {:?}",
        route,
    );
}

/// kina-36/T2 — NginxIngress → NginxIngress (classic Ingress object).
#[test]
fn route_type_is_nginx_ingress_when_no_gateway() {
    let route = demo_route_type(ActiveController::NginxIngress);
    assert_eq!(
        route,
        DemoRouteType::NginxIngress,
        "demo_route_type(NginxIngress) must be NginxIngress, got {:?}",
        route,
    );
}

// ===========================================================================
// kina-36 (migrated): controller_conflict_message_multi — one-controller guard
// The old two-arg bool API is replaced by the multi-controller list API.
// Traefik-specific cases: blocking nginx-ingress ↔ traefik, plus NGF blocking.
// ===========================================================================

/// kina-36/T3 — traefik present → nginx-ingress install blocked.
#[test]
fn nginx_install_blocked_when_traefik_namespace_present() {
    let msg = controller_conflict_message_multi("nginx-ingress", &["traefik"]);
    assert!(
        msg.is_some(),
        "nginx-ingress install must be blocked when traefik namespace is present",
    );
    assert!(
        msg.unwrap().to_lowercase().contains("traefik"),
        "Block message for nginx-ingress install must mention \"traefik\"",
    );
}

/// kina-36/T4 — traefik absent → nginx-ingress install allowed.
#[test]
fn nginx_install_allowed_when_no_conflict_present() {
    let msg = controller_conflict_message_multi("nginx-ingress", &[]);
    assert!(
        msg.is_none(),
        "nginx-ingress install must be allowed when no other controller is present; got: {:?}",
        msg,
    );
}

/// kina-36/T5 — nginx-ingress present → traefik install blocked.
#[test]
fn traefik_install_blocked_when_nginx_ingress_namespace_present() {
    let msg = controller_conflict_message_multi("traefik", &["nginx-ingress"]);
    assert!(
        msg.is_some(),
        "traefik install must be blocked when nginx-ingress namespace is present",
    );
    assert!(
        msg.unwrap().to_lowercase().contains("nginx-ingress"),
        "Block message for traefik install must mention \"nginx-ingress\"",
    );
}

/// kina-36/T6 — nginx-ingress absent → traefik install allowed.
#[test]
fn traefik_install_allowed_when_no_conflict_present() {
    let msg = controller_conflict_message_multi("traefik", &[]);
    assert!(
        msg.is_none(),
        "traefik install must be allowed when no other controller is present; got: {:?}",
        msg,
    );
}

// ===========================================================================
// kina-36 + kina-32: Traefik gateway_parent_ref (new test)
// ===========================================================================

/// kina-36+32/T7 — Traefik parentRef is (name="traefik", ns="traefik", sectionName="web").
/// Verifies Traefik Gateway listener sectionName is preserved in the enum API.
#[test]
fn traefik_gateway_parent_ref_correct() {
    let parent = gateway_parent_ref(ActiveController::Traefik);
    assert_eq!(
        parent,
        Some(("traefik", "traefik", "web")),
        "gateway_parent_ref(Traefik) must return \
         Some((\"traefik\",\"traefik\",\"web\")); got {:?}",
        parent,
    );
}
