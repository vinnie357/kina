//! Adversarial-TDD tests for the nginx-gateway-fabric addon (kina-32).
//!
//! ## P2 contract — these tests are INTENTIONALLY RED
//!
//! Every test in this file references either:
//!   (a) a pub fn / type that does NOT yet exist in `kina_cli::core::verify`, OR
//!   (b) a manifest file that does NOT yet exist under
//!       `kina-cli/manifests/nginx-gateway-fabric/`.
//!
//! The compile errors and test failures ARE the spec.  The separate P3 implementer
//! makes them green WITHOUT modifying this file.
//!
//! All pure-function tests: no live kubectl, no process spawns, no network, no filesystem
//! writes.  Manifest embedding tests use `include_str!` (compile-time); YAML validity
//! is checked with `serde_yaml`.
//!
//! ## Implementer binding (kina-cli/src/core/verify.rs)
//!
//! Add the following items to `kina_cli::core::verify` — exact signatures are binding:
//!
//! ```rust
//! /// Active ingress/gateway controller installed in the cluster.
//! #[derive(Debug, PartialEq)]
//! pub enum ActiveController { Traefik, NginxGatewayFabric, NginxIngress, None }
//!
//! /// Label string used in ${CONTROLLER} substitution and log messages.
//! pub fn controller_label(c: ActiveController) -> &'static str;
//! // Traefik → "traefik"
//! // NginxGatewayFabric → "nginx-gateway-fabric"
//! // NginxIngress → "nginx-ingress"
//! // None → "nginx-ingress"  (sensible fallback, asserted in tests)
//!
//! /// Route type to apply for the demo app.
//! pub fn demo_route_type(c: ActiveController) -> DemoRouteType;
//! // Traefik / NginxGatewayFabric → HttpRoute
//! // NginxIngress / None          → NginxIngress
//!
//! /// Gateway parentRef triple (name, namespace, sectionName); None for non-gateway
//! /// controllers.
//! pub fn gateway_parent_ref(c: ActiveController) -> Option<(&'static str, &'static str, &'static str)>;
//! // Traefik           → Some(("traefik",  "traefik",       "web"))
//! // NginxGatewayFabric → Some(("nginx",   "nginx-gateway", "http"))
//! // NginxIngress / None → None
//!
//! /// All known controller (id, namespace) pairs — id may differ from namespace (NGF).
//! pub fn ingress_controllers() -> [(&'static str, &'static str); 3];
//! // [("nginx-ingress","nginx-ingress"), ("traefik","traefik"), ("nginx-gateway-fabric","nginx-gateway")]
//!
//! /// Block-guard: Some(msg) iff any OTHER controller id is in `present`.
//! pub fn controller_conflict_message_multi(
//!     installing: &str,
//!     present: &[&str],
//! ) -> Option<String>;
//!
//! /// (namespace, display-label) targets probed to detect existing controllers.
//! pub fn ingress_probe_targets() -> [(&'static str, &'static str); 3];
//! // [("traefik", ...), ("nginx-ingress", ...), ("nginx-gateway", "Gateway (nginx-gateway-fabric)")]
//!
//! /// Extended render_demo_manifest — adds ${CONTROLLER}, ${GATEWAY_NAME},
//! /// ${GATEWAY_NAMESPACE}, ${GATEWAY_SECTION} placeholders alongside the existing
//! /// ${CLUSTER_NAME} / ${DNS_DOMAIN} ones.
//! pub fn render_demo_manifest(
//!     manifest: &str,
//!     cluster: &str,
//!     domain: &str,
//!     controller: &str,
//!     gateway_name: &str,
//!     gateway_ns: &str,
//!     gateway_section: &str,
//! ) -> String;
//! ```
//!
//! ## Implementer binding (kina-cli/manifests/nginx-gateway-fabric/)
//!
//! Create the following manifest files (exact paths):
//!   deploy.yaml      — Namespace + RBAC + Deployment + Job + GatewayClass + NginxGateway config
//!                      Control-plane image: ghcr.io/nginx/nginx-gateway-fabric:2.6.5@sha256:8fba8e3ea9e1050cda4d3a47e8c5c1814b688674ac3d52d55511e30adc5f4e5f
//!   nginxproxy.yaml  — NginxProxy (gateway.nginx.org/v1alpha2) + DaemonSet hostPorts 80/443 + ClusterIP Service
//!                      Data-plane image:    ghcr.io/nginx/nginx-gateway-fabric/nginx:2.6.5@sha256:bc2676fca1b3e24a1f560a22b60fef56e140ac38dfe4b5b4f202a4c9db41e9b9
//!   gateway.yaml     — Gateway `nginx` in ns nginx-gateway (gatewayClassName: nginx)
//!   ngf-crds.yaml    — 11 CRDs, all group gateway.nginx.org

// ---------------------------------------------------------------------------
// Manifest embedding — compile-time include_str! (red until files are created)
// ---------------------------------------------------------------------------

const DEPLOY_YAML: &str = include_str!("../manifests/nginx-gateway-fabric/deploy.yaml");

const NGINXPROXY_YAML: &str = include_str!("../manifests/nginx-gateway-fabric/nginxproxy.yaml");

const GATEWAY_YAML: &str = include_str!("../manifests/nginx-gateway-fabric/gateway.yaml");

const NGF_CRDS_YAML: &str = include_str!("../manifests/nginx-gateway-fabric/ngf-crds.yaml");

// ---------------------------------------------------------------------------
// API imports — compile-time (red until implementer adds these to verify.rs)
// ---------------------------------------------------------------------------

use kina_cli::core::verify::{
    controller_conflict_message_multi, controller_label, demo_route_type, gateway_parent_ref,
    ingress_controllers, ingress_probe_targets, render_demo_manifest, ActiveController,
    DemoRouteType,
};

// ===========================================================================
// Section 1 — ActiveController::controller_label
// ===========================================================================

/// NGF/T01 — Traefik controller label is "traefik".
#[test]
fn controller_label_traefik_is_traefik() {
    assert_eq!(
        controller_label(ActiveController::Traefik),
        "traefik",
        "controller_label(Traefik) must return \"traefik\"",
    );
}

/// NGF/T02 — NginxGatewayFabric controller label is the kebab-case addon id.
#[test]
fn controller_label_ngf_is_nginx_gateway_fabric() {
    assert_eq!(
        controller_label(ActiveController::NginxGatewayFabric),
        "nginx-gateway-fabric",
        "controller_label(NginxGatewayFabric) must return \"nginx-gateway-fabric\"",
    );
}

/// NGF/T03 — NginxIngress controller label is "nginx-ingress".
#[test]
fn controller_label_nginx_ingress_is_nginx_ingress() {
    assert_eq!(
        controller_label(ActiveController::NginxIngress),
        "nginx-ingress",
        "controller_label(NginxIngress) must return \"nginx-ingress\"",
    );
}

/// NGF/T04 — None controller label must be a non-empty sensible default.
/// Specifying "nginx-ingress" as the fall-through default (matching existing
/// behaviour when no gateway is active).
#[test]
fn controller_label_none_defaults_to_nginx_ingress() {
    let label = controller_label(ActiveController::None);
    assert_eq!(
        label, "nginx-ingress",
        "controller_label(None) must default to \"nginx-ingress\"; got \"{}\"",
        label,
    );
}

// ===========================================================================
// Section 2 — demo_route_type
// ===========================================================================

/// NGF/T05 — Traefik → HttpRoute (existing behaviour preserved after migration).
#[test]
fn demo_route_type_traefik_is_httproute() {
    assert_eq!(
        demo_route_type(ActiveController::Traefik),
        DemoRouteType::HttpRoute,
        "demo_route_type(Traefik) must be HttpRoute",
    );
}

/// NGF/T06 — NginxGatewayFabric → HttpRoute (NGF also uses Gateway API HTTPRoute).
#[test]
fn demo_route_type_ngf_is_httproute() {
    assert_eq!(
        demo_route_type(ActiveController::NginxGatewayFabric),
        DemoRouteType::HttpRoute,
        "demo_route_type(NginxGatewayFabric) must be HttpRoute",
    );
}

/// NGF/T07 — NginxIngress → NginxIngress (legacy Ingress object).
#[test]
fn demo_route_type_nginx_ingress_is_nginx_ingress() {
    assert_eq!(
        demo_route_type(ActiveController::NginxIngress),
        DemoRouteType::NginxIngress,
        "demo_route_type(NginxIngress) must be NginxIngress",
    );
}

/// NGF/T08 — None → NginxIngress (fallback: no gateway active).
#[test]
fn demo_route_type_none_is_nginx_ingress() {
    assert_eq!(
        demo_route_type(ActiveController::None),
        DemoRouteType::NginxIngress,
        "demo_route_type(None) must fall back to NginxIngress",
    );
}

// ===========================================================================
// Section 3 — gateway_parent_ref (name, namespace, sectionName)
// ===========================================================================

/// NGF/T09 — Traefik parentRef: name=traefik ns=traefik sectionName=web.
#[test]
fn gateway_parent_ref_traefik() {
    assert_eq!(
        gateway_parent_ref(ActiveController::Traefik),
        Some(("traefik", "traefik", "web")),
        "gateway_parent_ref(Traefik) must return Some((\"traefik\",\"traefik\",\"web\"))",
    );
}

/// NGF/T10 — NGF parentRef: name=nginx ns=nginx-gateway sectionName=http.
/// Listener name "http" is read from spike-gateway.yaml → listeners[0].name.
#[test]
fn gateway_parent_ref_ngf() {
    assert_eq!(
        gateway_parent_ref(ActiveController::NginxGatewayFabric),
        Some(("nginx", "nginx-gateway", "http")),
        "gateway_parent_ref(NginxGatewayFabric) must return \
         Some((\"nginx\",\"nginx-gateway\",\"http\"))",
    );
}

/// NGF/T11 — NginxIngress has no Gateway parentRef (Ingress object, not HTTPRoute).
#[test]
fn gateway_parent_ref_nginx_ingress_is_none() {
    assert_eq!(
        gateway_parent_ref(ActiveController::NginxIngress),
        None,
        "gateway_parent_ref(NginxIngress) must be None",
    );
}

/// NGF/T12 — None has no Gateway parentRef.
#[test]
fn gateway_parent_ref_none_is_none() {
    assert_eq!(
        gateway_parent_ref(ActiveController::None),
        None,
        "gateway_parent_ref(None) must be None",
    );
}

// ===========================================================================
// Section 4 — ingress_controllers (id → namespace mapping)
// ===========================================================================

/// NGF/T13 — Three controllers registered.
#[test]
fn ingress_controllers_has_three_entries() {
    assert_eq!(ingress_controllers().len(), 3);
}

/// NGF/T14 — nginx-ingress maps to its own namespace (id == ns).
#[test]
fn ingress_controllers_nginx_ingress_maps_self() {
    assert!(
        ingress_controllers().contains(&("nginx-ingress", "nginx-ingress")),
        "ingress_controllers() must include (\"nginx-ingress\",\"nginx-ingress\")",
    );
}

/// NGF/T15 — traefik maps to its own namespace (id == ns).
#[test]
fn ingress_controllers_traefik_maps_self() {
    assert!(
        ingress_controllers().contains(&("traefik", "traefik")),
        "ingress_controllers() must include (\"traefik\",\"traefik\")",
    );
}

/// NGF/T16 — NGF id ≠ namespace: id="nginx-gateway-fabric", ns="nginx-gateway".
/// This asymmetry is load-bearing: the CLI detects NGF by its addon id but the
/// namespace where pods live is "nginx-gateway".
#[test]
fn ingress_controllers_ngf_id_differs_from_namespace() {
    let controllers = ingress_controllers();
    assert!(
        controllers.contains(&("nginx-gateway-fabric", "nginx-gateway")),
        "ingress_controllers() must include (\"nginx-gateway-fabric\",\"nginx-gateway\") \
         (id ≠ namespace); got {:?}",
        controllers,
    );
    // Negative: the pair ("nginx-gateway-fabric","nginx-gateway-fabric") must NOT appear.
    assert!(
        !controllers.contains(&("nginx-gateway-fabric", "nginx-gateway-fabric")),
        "ingress_controllers() must NOT map NGF id to itself; namespace is \"nginx-gateway\"",
    );
}

// ===========================================================================
// Section 5 — controller_conflict_message_multi
// ===========================================================================

/// NGF/T17 — Empty present list → no conflict for any controller.
#[test]
fn conflict_no_present_allows_all() {
    for id in &["nginx-gateway-fabric", "traefik", "nginx-ingress"] {
        assert!(
            controller_conflict_message_multi(id, &[]).is_none(),
            "controller_conflict_message_multi({id:?}, &[]) must be None when nothing is present",
        );
    }
}

/// NGF/T18 — NGF blocked when traefik is present.
#[test]
fn ngf_blocked_by_traefik() {
    let msg = controller_conflict_message_multi("nginx-gateway-fabric", &["traefik"]);
    assert!(
        msg.is_some(),
        "NGF install must be blocked when traefik is present",
    );
    assert!(
        msg.unwrap().contains("traefik"),
        "Block message must name the conflicting controller \"traefik\"",
    );
}

/// NGF/T19 — NGF blocked when nginx-ingress is present.
#[test]
fn ngf_blocked_by_nginx_ingress() {
    let msg = controller_conflict_message_multi("nginx-gateway-fabric", &["nginx-ingress"]);
    assert!(
        msg.is_some(),
        "NGF install must be blocked when nginx-ingress is present",
    );
    assert!(
        msg.unwrap().contains("nginx-ingress"),
        "Block message must name the conflicting controller \"nginx-ingress\"",
    );
}

/// NGF/T20 — traefik blocked when NGF is present.
#[test]
fn traefik_blocked_by_ngf() {
    let msg = controller_conflict_message_multi("traefik", &["nginx-gateway-fabric"]);
    assert!(
        msg.is_some(),
        "traefik install must be blocked when nginx-gateway-fabric is present",
    );
    assert!(
        msg.unwrap().contains("nginx-gateway-fabric"),
        "Block message must name the conflicting controller \"nginx-gateway-fabric\"",
    );
}

/// NGF/T21 — nginx-ingress blocked when NGF is present.
#[test]
fn nginx_ingress_blocked_by_ngf() {
    let msg = controller_conflict_message_multi("nginx-ingress", &["nginx-gateway-fabric"]);
    assert!(
        msg.is_some(),
        "nginx-ingress install must be blocked when nginx-gateway-fabric is present",
    );
    assert!(
        msg.unwrap().contains("nginx-gateway-fabric"),
        "Block message must name the conflicting controller \"nginx-gateway-fabric\"",
    );
}

/// NGF/T22 — A controller is NOT blocked by its OWN id in present
/// (idempotent re-install must not self-block).
#[test]
fn controller_not_blocked_by_itself() {
    assert!(
        controller_conflict_message_multi("nginx-gateway-fabric", &["nginx-gateway-fabric"])
            .is_none(),
        "NGF must not block itself when its own id appears in present",
    );
    assert!(
        controller_conflict_message_multi("traefik", &["traefik"]).is_none(),
        "traefik must not block itself when its own id appears in present",
    );
}

/// NGF/T23 — Message names ALL conflicting controllers when multiple are present.
#[test]
fn conflict_message_names_multiple_conflicts() {
    let msg =
        controller_conflict_message_multi("nginx-gateway-fabric", &["traefik", "nginx-ingress"]);
    assert!(
        msg.is_some(),
        "NGF must be blocked when both traefik and nginx-ingress are present"
    );
    let text = msg.unwrap();
    assert!(
        text.contains("traefik"),
        "Block message must mention \"traefik\"; got: {text}",
    );
    assert!(
        text.contains("nginx-ingress"),
        "Block message must mention \"nginx-ingress\"; got: {text}",
    );
}

// ===========================================================================
// Section 6 — ingress_probe_targets
// ===========================================================================

/// NGF/T24 — Three probe targets registered.
#[test]
fn ingress_probe_targets_has_three_entries() {
    assert_eq!(ingress_probe_targets().len(), 3);
}

/// NGF/T25 — NGF probe target is ("nginx-gateway", "Gateway (nginx-gateway-fabric)").
/// The namespace key is "nginx-gateway" (where the Gateway object lives).
#[test]
fn ingress_probe_targets_includes_ngf() {
    assert!(
        ingress_probe_targets().contains(&("nginx-gateway", "Gateway (nginx-gateway-fabric)")),
        "ingress_probe_targets() must include \
         (\"nginx-gateway\", \"Gateway (nginx-gateway-fabric)\"); \
         got {:?}",
        ingress_probe_targets(),
    );
}

/// NGF/T26 — traefik probe target is present.
#[test]
fn ingress_probe_targets_includes_traefik() {
    let targets = ingress_probe_targets();
    assert!(
        targets.iter().any(|(ns, _)| *ns == "traefik"),
        "ingress_probe_targets() must include a traefik namespace entry; got {:?}",
        targets,
    );
}

/// NGF/T27 — nginx-ingress probe target is present.
#[test]
fn ingress_probe_targets_includes_nginx_ingress() {
    let targets = ingress_probe_targets();
    assert!(
        targets.iter().any(|(ns, _)| *ns == "nginx-ingress"),
        "ingress_probe_targets() must include a nginx-ingress namespace entry; got {:?}",
        targets,
    );
}

// ===========================================================================
// Section 7 — render_demo_manifest — new GATEWAY_* and ${CONTROLLER} placeholders
// ===========================================================================

/// NGF/T28 — ${GATEWAY_NAME} is substituted.
#[test]
fn render_substitutes_gateway_name() {
    let manifest = "parentRef:\n  name: ${GATEWAY_NAME}";
    let result = render_demo_manifest(
        manifest, "c", "local", "traefik", "traefik", "traefik", "web",
    );
    assert_eq!(
        result, "parentRef:\n  name: traefik",
        "render_demo_manifest must substitute ${{GATEWAY_NAME}}; got: {result}",
    );
    assert!(
        !result.contains("${GATEWAY_NAME}"),
        "render_demo_manifest must leave no literal ${{GATEWAY_NAME}}; got: {result}",
    );
}

/// NGF/T29 — ${GATEWAY_NAMESPACE} is substituted.
#[test]
fn render_substitutes_gateway_namespace() {
    let manifest = "namespace: ${GATEWAY_NAMESPACE}";
    let result = render_demo_manifest(
        manifest, "c", "local", "traefik", "traefik", "traefik", "web",
    );
    assert_eq!(
        result, "namespace: traefik",
        "render_demo_manifest must substitute ${{GATEWAY_NAMESPACE}}; got: {result}",
    );
}

/// NGF/T30 — ${GATEWAY_SECTION} (sectionName) is substituted.
#[test]
fn render_substitutes_gateway_section() {
    let manifest = "sectionName: ${GATEWAY_SECTION}";
    let result = render_demo_manifest(
        manifest, "c", "local", "traefik", "traefik", "traefik", "web",
    );
    assert_eq!(
        result, "sectionName: web",
        "render_demo_manifest must substitute ${{GATEWAY_SECTION}}; got: {result}",
    );
}

/// NGF/T31 — All three GATEWAY_* placeholders work together for NGF.
#[test]
fn render_ngf_gateway_parent_ref_substitution() {
    let manifest =
        "  parentRef:\n    name: ${GATEWAY_NAME}\n    namespace: ${GATEWAY_NAMESPACE}\n    sectionName: ${GATEWAY_SECTION}";
    let result = render_demo_manifest(
        manifest,
        "kina",
        "local",
        "nginx-gateway-fabric",
        "nginx",
        "nginx-gateway",
        "http",
    );
    assert_eq!(
        result,
        "  parentRef:\n    name: nginx\n    namespace: nginx-gateway\n    sectionName: http",
        "NGF parentRef substitution must produce exact YAML; got:\n{result}",
    );
}

/// NGF/T32 — Existing ${CLUSTER_NAME} and ${DNS_DOMAIN} placeholders are preserved.
#[test]
fn render_existing_placeholders_still_substituted() {
    let manifest = "host: ${CLUSTER_NAME}-control-plane.${DNS_DOMAIN}";
    let result = render_demo_manifest(
        manifest,
        "mycluster",
        "example.local",
        "nginx-gateway-fabric",
        "nginx",
        "nginx-gateway",
        "http",
    );
    assert!(
        result.contains("mycluster-control-plane.example.local"),
        "render_demo_manifest must still substitute ${{CLUSTER_NAME}} and ${{DNS_DOMAIN}}; got: {result}",
    );
    assert!(
        !result.contains("${CLUSTER_NAME}"),
        "No literal ${{CLUSTER_NAME}} must remain; got: {result}",
    );
    assert!(
        !result.contains("${DNS_DOMAIN}"),
        "No literal ${{DNS_DOMAIN}} must remain; got: {result}",
    );
}

/// NGF/T33 — ${CONTROLLER} placeholder is substituted (kina-34 scope, now via
/// the extended render_demo_manifest parameter).
#[test]
fn render_substitutes_controller() {
    let manifest = "info: controller=${CONTROLLER}";
    let result = render_demo_manifest(
        manifest,
        "c",
        "local",
        "nginx-gateway-fabric",
        "nginx",
        "nginx-gateway",
        "http",
    );
    assert_eq!(
        result, "info: controller=nginx-gateway-fabric",
        "render_demo_manifest must substitute ${{CONTROLLER}}; got: {result}",
    );
    assert!(
        !result.contains("${CONTROLLER}"),
        "No literal ${{CONTROLLER}} must remain; got: {result}",
    );
}

// ===========================================================================
// Section 8 — Manifest content tests (depend on implementer adding the files)
// ===========================================================================

/// NGF/M01 — deploy.yaml must be non-empty.
#[test]
fn deploy_yaml_is_non_empty() {
    assert!(
        !DEPLOY_YAML.is_empty(),
        "manifests/nginx-gateway-fabric/deploy.yaml must not be empty"
    );
}

/// NGF/M02 — deploy.yaml must be valid YAML.
#[test]
fn deploy_yaml_is_valid_yaml() {
    serde_yaml::from_str::<serde_yaml::Value>(DEPLOY_YAML)
        .expect("manifests/nginx-gateway-fabric/deploy.yaml must be valid YAML");
}

/// NGF/M03 — deploy.yaml control-plane image must be digest-pinned.
/// Exact digest: sha256:8fba8e3ea9e1050cda4d3a47e8c5c1814b688674ac3d52d55511e30adc5f4e5f
#[test]
fn deploy_yaml_control_plane_image_is_digest_pinned() {
    assert!(
        DEPLOY_YAML
            .contains("@sha256:8fba8e3ea9e1050cda4d3a47e8c5c1814b688674ac3d52d55511e30adc5f4e5f"),
        "deploy.yaml must pin the control-plane image with digest \
         sha256:8fba8e3ea9e1050cda4d3a47e8c5c1814b688674ac3d52d55511e30adc5f4e5f",
    );
}

/// NGF/M04 — deploy.yaml GatewayClass name must be "nginx".
#[test]
fn deploy_yaml_gatewayclass_name_is_nginx() {
    assert!(
        DEPLOY_YAML.contains("name: nginx"),
        "deploy.yaml must define a GatewayClass named \"nginx\"",
    );
}

/// NGF/M05 — deploy.yaml controllerName must be the canonical NGF value.
#[test]
fn deploy_yaml_controller_name_is_canonical() {
    assert!(
        DEPLOY_YAML.contains("gateway.nginx.org/nginx-gateway-controller"),
        "deploy.yaml must specify controllerName \
         \"gateway.nginx.org/nginx-gateway-controller\"",
    );
}

/// NGF/M06 — deploy.yaml must not bind 127.0.0.1 (no localhost hardcoding).
#[test]
fn deploy_yaml_no_localhost_bind() {
    assert!(
        !DEPLOY_YAML.contains("127.0.0.1"),
        "deploy.yaml must not contain a 127.0.0.1 bind address",
    );
}

/// NGF/M07 — nginxproxy.yaml must be non-empty.
#[test]
fn nginxproxy_yaml_is_non_empty() {
    assert!(
        !NGINXPROXY_YAML.is_empty(),
        "manifests/nginx-gateway-fabric/nginxproxy.yaml must not be empty"
    );
}

/// NGF/M08 — nginxproxy.yaml must be valid YAML.
#[test]
fn nginxproxy_yaml_is_valid_yaml() {
    serde_yaml::from_str::<serde_yaml::Value>(NGINXPROXY_YAML)
        .expect("manifests/nginx-gateway-fabric/nginxproxy.yaml must be valid YAML");
}

/// NGF/M09 — nginxproxy.yaml data-plane image must be digest-pinned.
/// Exact digest: sha256:bc2676fca1b3e24a1f560a22b60fef56e140ac38dfe4b5b4f202a4c9db41e9b9
#[test]
fn nginxproxy_yaml_data_plane_image_is_digest_pinned() {
    assert!(
        NGINXPROXY_YAML
            .contains("@sha256:bc2676fca1b3e24a1f560a22b60fef56e140ac38dfe4b5b4f202a4c9db41e9b9"),
        "nginxproxy.yaml must pin the data-plane image with digest \
         sha256:bc2676fca1b3e24a1f560a22b60fef56e140ac38dfe4b5b4f202a4c9db41e9b9",
    );
}

/// NGF/M10 — nginxproxy.yaml must have hostPort 80.
#[test]
fn nginxproxy_yaml_has_host_port_80() {
    assert!(
        NGINXPROXY_YAML.contains("port: 80"),
        "nginxproxy.yaml must declare hostPort 80 for HTTP traffic",
    );
}

/// NGF/M11 — nginxproxy.yaml must have hostPort 443.
#[test]
fn nginxproxy_yaml_has_host_port_443() {
    assert!(
        NGINXPROXY_YAML.contains("port: 443"),
        "nginxproxy.yaml must declare hostPort 443 for HTTPS traffic",
    );
}

/// NGF/M12 — nginxproxy.yaml Service must be ClusterIP (not NodePort/LoadBalancer).
#[test]
fn nginxproxy_yaml_service_is_cluster_ip() {
    assert!(
        NGINXPROXY_YAML.contains("ClusterIP"),
        "nginxproxy.yaml Service must be type ClusterIP",
    );
    assert!(
        !NGINXPROXY_YAML.contains("NodePort"),
        "nginxproxy.yaml Service must NOT be NodePort",
    );
    assert!(
        !NGINXPROXY_YAML.contains("LoadBalancer"),
        "nginxproxy.yaml Service must NOT be LoadBalancer",
    );
}

/// NGF/M13 — nginxproxy.yaml must not bind 127.0.0.1.
#[test]
fn nginxproxy_yaml_no_localhost_bind() {
    assert!(
        !NGINXPROXY_YAML.contains("127.0.0.1"),
        "nginxproxy.yaml must not contain a 127.0.0.1 bind address",
    );
}

/// NGF/M14 — nginxproxy.yaml must use the gateway.nginx.org/v1alpha2 API version.
#[test]
fn nginxproxy_yaml_uses_v1alpha2() {
    assert!(
        NGINXPROXY_YAML.contains("gateway.nginx.org/v1alpha2"),
        "nginxproxy.yaml must use apiVersion gateway.nginx.org/v1alpha2 for NginxProxy",
    );
}

/// NGF/M15 — gateway.yaml must be non-empty.
#[test]
fn gateway_yaml_is_non_empty() {
    assert!(
        !GATEWAY_YAML.is_empty(),
        "manifests/nginx-gateway-fabric/gateway.yaml must not be empty"
    );
}

/// NGF/M16 — gateway.yaml must be valid YAML.
#[test]
fn gateway_yaml_is_valid_yaml() {
    serde_yaml::from_str::<serde_yaml::Value>(GATEWAY_YAML)
        .expect("manifests/nginx-gateway-fabric/gateway.yaml must be valid YAML");
}

/// NGF/M17 — gateway.yaml Gateway must be named "nginx".
#[test]
fn gateway_yaml_gateway_name_is_nginx() {
    assert!(
        GATEWAY_YAML.contains("name: nginx"),
        "gateway.yaml must define a Gateway named \"nginx\"",
    );
}

/// NGF/M18 — gateway.yaml Gateway must be in namespace nginx-gateway.
#[test]
fn gateway_yaml_namespace_is_nginx_gateway() {
    assert!(
        GATEWAY_YAML.contains("namespace: nginx-gateway"),
        "gateway.yaml Gateway must be in namespace \"nginx-gateway\"",
    );
}

/// NGF/M19 — gateway.yaml listener name must match the sectionName in the HTTPRoute
/// parentRef ("http" — from spike-gateway.yaml).
#[test]
fn gateway_yaml_listener_name_is_http() {
    assert!(
        GATEWAY_YAML.contains("name: http"),
        "gateway.yaml listener name must be \"http\" \
         (matches HTTPRoute parentRef sectionName)",
    );
}

/// NGF/M20 — ngf-crds.yaml must be non-empty.
#[test]
fn ngf_crds_yaml_is_non_empty() {
    assert!(
        !NGF_CRDS_YAML.is_empty(),
        "manifests/nginx-gateway-fabric/ngf-crds.yaml must not be empty"
    );
}

/// NGF/M21 — ngf-crds.yaml must be valid YAML.
#[test]
fn ngf_crds_yaml_is_valid_yaml() {
    serde_yaml::from_str::<serde_yaml::Value>(NGF_CRDS_YAML)
        .expect("manifests/nginx-gateway-fabric/ngf-crds.yaml must be valid YAML");
}

/// NGF/M22 — ngf-crds.yaml must contain 11 CRDs (one `kind: CustomResourceDefinition`
/// line per CRD).
#[test]
fn ngf_crds_yaml_has_eleven_crds() {
    let count = NGF_CRDS_YAML
        .lines()
        .filter(|l| l.contains("kind: CustomResourceDefinition"))
        .count();
    assert_eq!(
        count, 11,
        "ngf-crds.yaml must define exactly 11 CRDs; found {count}",
    );
}

/// NGF/M23 — ngf-crds.yaml CRDs must all be in the gateway.nginx.org group.
#[test]
fn ngf_crds_yaml_group_is_gateway_nginx_org() {
    assert!(
        NGF_CRDS_YAML.contains("gateway.nginx.org"),
        "ngf-crds.yaml CRDs must use group \"gateway.nginx.org\"",
    );
}
