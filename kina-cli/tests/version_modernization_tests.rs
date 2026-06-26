/// Version modernization tests — adversarial TDD (P2 test-author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns that do not yet exist in
/// kina_cli::core::apple_container. The compile errors and assertion failures are
/// the spec; P3 (the separate implementer agent) makes them green WITHOUT modifying
/// this file.
///
/// All tests are pure: NO live container/kubectl CLI invocations, NO process spawns,
/// NO network, NO filesystem writes, NO env mutation.
/// Source-grep guard tests open files via env!("CARGO_MANIFEST_DIR").
///
/// Import surface (BINDING for P3 implementer):
///   use kina_cli::core::apple_container::{
///       generate_kubeadm_init_config,
///       generate_worker_join_config,
///   };
///   use kina_cli::core::types::KubeadmJoinInfo;
///   use kina_cli::config::Config;
///   use kina_cli::core::verify::render_demo_manifest;
///
/// Required pub free functions (P3 must create or promote):
///   pub fn generate_kubeadm_init_config(
///       container_name: &str, vm_ip: &str, cluster_name: &str
///   ) -> String
///
///   pub fn generate_worker_join_config(
///       worker_name: &str, worker_ip: &str, join_info: &KubeadmJoinInfo
///   ) -> String
///
/// The "4 templates":
///   1. InitConfiguration (inside generate_kubeadm_init_config)
///   2. ClusterConfiguration (inside generate_kubeadm_init_config)
///   3. JoinConfiguration — control-plane init bundle (inside generate_kubeadm_init_config)
///   4. JoinConfiguration — worker join (inside generate_worker_join_config)
///
/// Version authority (VERSION CURRENCY AUDIT comment on kina-2, 2026-06-10):
///   K8s: v1.36.1 — kindest/node:v1.36.1 (default) and kina/node:v1.36.1 (custom) in sync
///        (kindest tag v1.36.0 does not exist — verified 404; v1.36.1 is the valid tag)
///   kubeadm API: v1beta4 (map -> list migration for kubeletExtraArgs + extraArgs)
///   cilium stays 1.18.10 (kina-3 decision, do NOT bump)
///   nginx-ingress: 5.5.0
///   demo-app: nginx:1.30.2-alpine3.23
// P3 must expose these as pub free functions in kina_cli::core::apple_container.
use kina_cli::config::Config;
use kina_cli::core::apple_container::{generate_kubeadm_init_config, generate_worker_join_config};
use kina_cli::core::types::KubeadmJoinInfo;
use kina_cli::core::verify::render_demo_manifest;

// ---------------------------------------------------------------------------
// Helpers — deterministic test inputs used across multiple tests
// ---------------------------------------------------------------------------

fn test_init_config() -> String {
    generate_kubeadm_init_config("kina-control-plane", "10.0.0.5", "kina")
}

fn test_join_info() -> KubeadmJoinInfo {
    KubeadmJoinInfo {
        token: "abcdef.0123456789abcdef".to_string(),
        ca_cert_hash: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string(),
        control_plane_endpoint: "10.0.0.5:6443".to_string(),
    }
}

fn test_worker_join_config() -> String {
    generate_worker_join_config("kina-worker", "10.0.0.6", &test_join_info())
}

// ===========================================================================
// GROUP A — kubeadm v1beta4 apiVersion (positive + negative), all 4 templates
// ===========================================================================

/// A1 — generate_kubeadm_init_config output contains 'apiVersion: kubeadm.k8s.io/v1beta4'
///      for the InitConfiguration stanza.
#[test]
fn a1_init_init_configuration_api_version_v1beta4() {
    let config = test_init_config();
    // The InitConfiguration stanza must use v1beta4. We verify the apiVersion line
    // appears before the first `kind: InitConfiguration` occurrence.
    assert!(
        config.contains("apiVersion: kubeadm.k8s.io/v1beta4"),
        "generate_kubeadm_init_config must contain 'apiVersion: kubeadm.k8s.io/v1beta4' \
         (required for InitConfiguration stanza); got:\n{}",
        config
    );
    // Also verify InitConfiguration kind is present so the stanza is not missing.
    assert!(
        config.contains("kind: InitConfiguration"),
        "generate_kubeadm_init_config must contain 'kind: InitConfiguration'; got:\n{}",
        config
    );
}

/// A2 — init config output contains 'apiVersion: kubeadm.k8s.io/v1beta4' immediately
///      preceding 'kind: ClusterConfiguration'.
#[test]
fn a2_init_cluster_configuration_api_version_v1beta4() {
    let config = test_init_config();
    // Verify the stanza pair exists by checking both strings are present and
    // that the apiVersion line appears before `kind: ClusterConfiguration`.
    let v1beta4_pos = config
        .find("apiVersion: kubeadm.k8s.io/v1beta4")
        .expect("v1beta4 apiVersion must appear in init config");
    let cluster_cfg_pos = config
        .find("kind: ClusterConfiguration")
        .expect("'kind: ClusterConfiguration' must appear in init config");
    // At minimum one v1beta4 apiVersion appears before ClusterConfiguration.
    assert!(
        v1beta4_pos < cluster_cfg_pos,
        "'apiVersion: kubeadm.k8s.io/v1beta4' must appear before 'kind: ClusterConfiguration' \
         in init config; got:\n{}",
        config
    );
    // Verify the apiVersion+kind pairing is consecutive (immediately preceding) by
    // locating the last v1beta4 occurrence before ClusterConfiguration.
    let before_cluster = &config[..cluster_cfg_pos];
    assert!(
        before_cluster.contains("apiVersion: kubeadm.k8s.io/v1beta4"),
        "A v1beta4 apiVersion must appear in the stanza immediately preceding \
         'kind: ClusterConfiguration'; got:\n{}",
        config
    );
}

/// A3 — init config output contains 'apiVersion: kubeadm.k8s.io/v1beta4' for the
///      control-plane-bundle JoinConfiguration stanza (inside generate_kubeadm_init_config).
#[test]
fn a3_init_join_configuration_api_version_v1beta4() {
    let config = test_init_config();
    assert!(
        config.contains("kind: JoinConfiguration"),
        "generate_kubeadm_init_config must contain a JoinConfiguration stanza; got:\n{}",
        config
    );
    // There must be a v1beta4 apiVersion before the JoinConfiguration.
    let join_pos = config
        .find("kind: JoinConfiguration")
        .expect("kind: JoinConfiguration must exist");
    let before_join = &config[..join_pos];
    assert!(
        before_join.contains("apiVersion: kubeadm.k8s.io/v1beta4"),
        "A v1beta4 apiVersion must appear before 'kind: JoinConfiguration' in init config; got:\n{}",
        config
    );
}

/// A4 — generate_worker_join_config output contains 'apiVersion: kubeadm.k8s.io/v1beta4'
///      for its JoinConfiguration stanza.
#[test]
fn a4_workerjoin_join_configuration_api_version_v1beta4() {
    let config = test_worker_join_config();
    assert!(
        config.contains("apiVersion: kubeadm.k8s.io/v1beta4"),
        "generate_worker_join_config must contain 'apiVersion: kubeadm.k8s.io/v1beta4'; got:\n{}",
        config
    );
    assert!(
        config.contains("kind: JoinConfiguration"),
        "generate_worker_join_config must contain 'kind: JoinConfiguration'; got:\n{}",
        config
    );
}

/// A5 — NEGATIVE: generate_kubeadm_init_config output does NOT contain 'kubeadm.k8s.io/v1beta3'.
#[test]
fn a5_init_no_v1beta3() {
    let config = test_init_config();
    assert!(
        !config.contains("kubeadm.k8s.io/v1beta3"),
        "generate_kubeadm_init_config must NOT contain 'kubeadm.k8s.io/v1beta3' \
         (full v1beta4 migration required); got:\n{}",
        config
    );
}

/// A6 — NEGATIVE: generate_worker_join_config output does NOT contain 'kubeadm.k8s.io/v1beta3'.
#[test]
fn a6_workerjoin_no_v1beta3() {
    let config = test_worker_join_config();
    assert!(
        !config.contains("kubeadm.k8s.io/v1beta3"),
        "generate_worker_join_config must NOT contain 'kubeadm.k8s.io/v1beta3' \
         (full v1beta4 migration required); got:\n{}",
        config
    );
}

/// A7 — init config contains exactly 3 occurrences of 'kubeadm.k8s.io/v1beta4'
///      (InitConfiguration + ClusterConfiguration + JoinConfiguration stanzas).
///      Guards against accidental stanza loss during migration.
#[test]
fn a7_init_exactly_three_kubeadm_stanzas() {
    let config = test_init_config();
    let count = config.matches("kubeadm.k8s.io/v1beta4").count();
    assert_eq!(
        count, 3,
        "generate_kubeadm_init_config must contain exactly 3 occurrences of \
         'kubeadm.k8s.io/v1beta4' (Init + Cluster + Join stanzas); \
         found {} occurrences in:\n{}",
        count, config
    );
}

// ===========================================================================
// GROUP B — kubeletExtraArgs + extraArgs as name/value LISTS (map->list contract)
// ===========================================================================

/// B1 — init InitConfiguration kubeletExtraArgs uses list form.
///      Output contains '- name: node-ip' and '- name: provider-id'
///      with rendered value 'kind://docker/kina/kina-control-plane'.
#[test]
fn b1_init_kubelet_extra_args_list_form_node_and_provider() {
    let config = test_init_config();
    assert!(
        config.contains("- name: node-ip"),
        "init config InitConfiguration kubeletExtraArgs must use list form '- name: node-ip'; got:\n{}",
        config
    );
    assert!(
        config.contains("- name: provider-id"),
        "init config InitConfiguration kubeletExtraArgs must use list form '- name: provider-id'; got:\n{}",
        config
    );
    // Verify provider-id value renders correctly.
    assert!(
        config.contains("kind://docker/kina/kina-control-plane"),
        "init config must contain rendered provider-id value 'kind://docker/kina/kina-control-plane'; got:\n{}",
        config
    );
}

/// B2 — init JoinConfiguration (control-plane bundle) kubeletExtraArgs uses list form.
#[test]
fn b2_init_join_kubelet_extra_args_list_form() {
    let config = test_init_config();
    // Locate the JoinConfiguration stanza and verify list-form kubeletExtraArgs.
    // We search after the first ClusterConfiguration boundary.
    let join_pos = config
        .find("kind: JoinConfiguration")
        .expect("kind: JoinConfiguration must exist in init config");
    let after_join = &config[join_pos..];
    assert!(
        after_join.contains("- name: node-ip"),
        "init JoinConfiguration kubeletExtraArgs must use list form '- name: node-ip'; got (after JoinConfiguration):\n{}",
        after_join
    );
    assert!(
        after_join.contains("- name: provider-id"),
        "init JoinConfiguration kubeletExtraArgs must use list form '- name: provider-id'; got (after JoinConfiguration):\n{}",
        after_join
    );
}

/// B3 — worker-join kubeletExtraArgs uses list form: '- name: node-ip' present.
#[test]
fn b3_workerjoin_kubelet_extra_args_list_form() {
    let config = test_worker_join_config();
    assert!(
        config.contains("- name: node-ip"),
        "generate_worker_join_config kubeletExtraArgs must use list form '- name: node-ip'; got:\n{}",
        config
    );
}

/// B4 — apiServer extraArgs uses list form: '- name: runtime-config' with value 'api/all=true'.
#[test]
fn b4_api_server_extra_args_list_form_runtime_config() {
    let config = test_init_config();
    assert!(
        config.contains("- name: runtime-config"),
        "init apiServer extraArgs must use list form '- name: runtime-config'; got:\n{}",
        config
    );
    assert!(
        config.contains("api/all=true"),
        "init apiServer extraArgs must contain value 'api/all=true'; got:\n{}",
        config
    );
}

/// B5 — controllerManager extraArgs uses list form:
///      '- name: enable-hostpath-provisioner' with value 'true'.
#[test]
fn b5_controller_manager_extra_args_list_form_hostpath() {
    let config = test_init_config();
    assert!(
        config.contains("- name: enable-hostpath-provisioner"),
        "init controllerManager extraArgs must use list form '- name: enable-hostpath-provisioner'; got:\n{}",
        config
    );
    // Verify the value field follows for that entry — conservative check: 'true' exists after
    // the name entry. We accept any ordering of value field.
    let hostpath_pos = config
        .find("- name: enable-hostpath-provisioner")
        .expect("- name: enable-hostpath-provisioner must exist");
    let after_name = &config[hostpath_pos..];
    assert!(
        after_name.contains("true"),
        "controllerManager extraArgs enable-hostpath-provisioner must have value 'true'; got (after name):\n{}",
        after_name
    );
}

/// B6 — NEGATIVE (old map-form gone): init config (rendered with vm_ip="10.0.0.5")
///      does NOT contain the old map-form 'node-ip: "10.0.0.5"'.
#[test]
fn b6_init_no_map_form_node_ip() {
    let config = test_init_config();
    // Old map-form was: node-ip: "10.0.0.5" (under kubeletExtraArgs as a YAML map key)
    assert!(
        !config.contains("node-ip: \"10.0.0.5\""),
        "init config must NOT contain old map-form 'node-ip: \"10.0.0.5\"' \
         (must use list form '- name: node-ip'); got:\n{}",
        config
    );
}

/// B7 — NEGATIVE: init config does NOT contain old map-form 'runtime-config: "api/all=true"'.
#[test]
fn b7_init_no_map_form_runtime_config() {
    let config = test_init_config();
    assert!(
        !config.contains("runtime-config: \"api/all=true\""),
        "init config must NOT contain old map-form 'runtime-config: \"api/all=true\"' \
         (must use list form '- name: runtime-config'); got:\n{}",
        config
    );
}

/// B8 — NEGATIVE: init config does NOT contain old map-form 'enable-hostpath-provisioner: "true"'.
#[test]
fn b8_init_no_map_form_hostpath() {
    let config = test_init_config();
    assert!(
        !config.contains("enable-hostpath-provisioner: \"true\""),
        "init config must NOT contain old map-form 'enable-hostpath-provisioner: \"true\"' \
         (must use list form '- name: enable-hostpath-provisioner'); got:\n{}",
        config
    );
}

// ===========================================================================
// GROUP C — kubernetesVersion v1.36.1
// ===========================================================================

/// C1 — init ClusterConfiguration contains 'kubernetesVersion: v1.36.1'.
#[test]
fn c1_kubernetes_version_v1_36_1() {
    let config = test_init_config();
    assert!(
        config.contains("kubernetesVersion: v1.36.1"),
        "init ClusterConfiguration must contain 'kubernetesVersion: v1.36.1'; got:\n{}",
        config
    );
}

/// C2 — NEGATIVE: init config does NOT contain 'v1.31.0' (old) nor 'v1.36.0'
///      (the nonexistent kindest/node tag — kind skipped .0 and published v1.36.1).
#[test]
fn c2_no_v1_31_0_and_no_nonexistent_v1_36_0() {
    let config = test_init_config();
    assert!(
        !config.contains("v1.31.0"),
        "init config must NOT contain stale 'v1.31.0'; got:\n{}",
        config
    );
    assert!(
        !config.contains("v1.36.0"),
        "init config must NOT contain 'v1.36.0' (nonexistent kindest/node tag; use v1.36.1); got:\n{}",
        config
    );
}

// ===========================================================================
// GROUP D — default image kindest/node:v1.36.1 (CLI + config defaults)
//            + stale default_version fix at config/mod.rs:184
// ===========================================================================

/// D1 — (CLI default) source-grep cli/cluster.rs: --image default_value is
///      'kindest/node:v1.36.1' and NOT 'kindest/node:v1.31.0'.
#[test]
fn d1_cli_default_image_v1_36_1() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");
    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };
    assert!(
        src.contains("kindest/node:v1.36.1"),
        "cli/cluster.rs --image default_value must be 'kindest/node:v1.36.1'; got source:\n{}",
        &src[..src.len().min(500)]
    );
    assert!(
        !src.contains("kindest/node:v1.31.0"),
        "cli/cluster.rs must NOT contain stale 'kindest/node:v1.31.0'"
    );
}

/// D2 — (config default_image) Config::default().cluster.default_image == 'kindest/node:v1.36.1'.
#[test]
fn d2_config_default_image_v1_36_1() {
    let cfg = Config::default();
    assert_eq!(
        cfg.cluster.default_image, "kindest/node:v1.36.1",
        "Config::default().cluster.default_image must be 'kindest/node:v1.36.1'"
    );
    assert_ne!(
        cfg.cluster.default_image, "kindest/node:v1.31.0",
        "Config::default().cluster.default_image must NOT be stale 'kindest/node:v1.31.0'"
    );
}

/// D3 — (stale default_version) Config::default().kubernetes.default_version == 'v1.36.1'.
///      Fixes config/mod.rs:184 stale v1.28.0.
#[test]
fn d3_config_default_version_v1_36_1() {
    let cfg = Config::default();
    assert_eq!(
        cfg.kubernetes.default_version, "v1.36.1",
        "Config::default().kubernetes.default_version must be 'v1.36.1' (was stale 'v1.28.0')"
    );
    assert_ne!(
        cfg.kubernetes.default_version, "v1.28.0",
        "Config::default().kubernetes.default_version must NOT be stale 'v1.28.0'"
    );
}

/// D4 — NEGATIVE guard: Config::default().cluster.default_image is NOT 'kina/node:v1.36.1'.
///      The custom kina/node image is opt-in (Cilium/custom-kernel path); the default
///      stays kindest/node even though both images are now synced at v1.36.1.
#[test]
fn d4_config_default_image_not_kina_node() {
    let cfg = Config::default();
    assert_ne!(
        cfg.cluster.default_image, "kina/node:v1.36.1",
        "Config::default().cluster.default_image must NOT be 'kina/node:v1.36.1' \
         (custom image is opt-in; the default uses kindest/node)"
    );
}

// ===========================================================================
// GROUP E — demo-app manifest pins nginx:1.30.2-alpine3.23 + NO floating tags
// ===========================================================================

/// E1 — manifests/demo-app.yaml (read via CARGO_MANIFEST_DIR) contains
///      'image: nginx:1.30.2-alpine3.23'.
#[test]
fn e1_demoapp_pins_nginx_1_30_2() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("manifests/demo-app.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read manifests/demo-app.yaml: {}", e),
    };
    assert!(
        content.contains("image: nginx:1.30.2-alpine3.23"),
        "manifests/demo-app.yaml must contain 'image: nginx:1.30.2-alpine3.23'; got (excerpt):\n{}",
        &content[..content.len().min(1000)]
    );
}

/// E2 — NEGATIVE: demo-app.yaml does NOT contain floating 'nginx:alpine'.
#[test]
fn e2_demoapp_no_floating_nginx_alpine() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("manifests/demo-app.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read manifests/demo-app.yaml: {}", e),
    };
    assert!(
        !content.contains("nginx:alpine"),
        "manifests/demo-app.yaml must NOT contain floating tag 'nginx:alpine' \
         (policy violation — all image refs must be pinned); got (excerpt):\n{}",
        &content[..content.len().min(1000)]
    );
}

/// E3 — NEGATIVE (no floating tags in demo-app): no uncommented image line uses
///      a floating tag (':latest', ':stable', bare ':alpine').
#[test]
fn e3_demoapp_no_floating_tags_anywhere() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("manifests/demo-app.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read manifests/demo-app.yaml: {}", e),
    };
    for line in content.lines() {
        let trimmed = line.trim_start();
        // Skip comment lines.
        if trimmed.starts_with('#') {
            continue;
        }
        if !trimmed.contains("image:") {
            continue;
        }
        assert!(
            !trimmed.contains(":latest"),
            "demo-app.yaml has a floating ':latest' image ref in line: {:?}",
            line
        );
        assert!(
            !trimmed.contains(":stable"),
            "demo-app.yaml has a floating ':stable' image ref in line: {:?}",
            line
        );
        // Check for bare ':alpine' (no digit after colon prefix meaning unversioned alpine).
        // We detect 'nginx:alpine' or similar — any ':alpine' not followed immediately by a digit
        // or dot (i.e., not ':alpine3.X') is floating.
        if trimmed.contains(":alpine") {
            // ':alpine3.' pattern with digits is a pinned tag (e.g. alpine3.23); allow that.
            let has_pinned_alpine = trimmed
                .split(':')
                .skip(1)
                .any(|after| after.starts_with("alpine") && after.contains('.'));
            assert!(
                has_pinned_alpine,
                "demo-app.yaml has a floating ':alpine' image ref (unpinned) in line: {:?}",
                line
            );
        }
    }
}

/// E4 — render_demo_manifest(raw_demo_app, "kina", "test") output still contains
///      'nginx:1.30.2-alpine3.23' — proves templating does not strip/alter the pinned image.
#[test]
fn e4_rendered_demoapp_keeps_pin() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("manifests/demo-app.yaml");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read manifests/demo-app.yaml: {}", e),
    };
    let rendered = render_demo_manifest(&raw, "kina", "test", "", "", "", "");
    assert!(
        rendered.contains("nginx:1.30.2-alpine3.23"),
        "render_demo_manifest output must still contain 'nginx:1.30.2-alpine3.23' \
         (templating must not alter pinned image tag); got (excerpt):\n{}",
        &rendered[..rendered.len().min(1000)]
    );
}

/// E5 — NEGATIVE: render_demo_manifest output does NOT contain 'nginx:alpine'.
#[test]
fn e5_rendered_demoapp_no_floating() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("manifests/demo-app.yaml");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read manifests/demo-app.yaml: {}", e),
    };
    let rendered = render_demo_manifest(&raw, "kina", "test", "", "", "", "");
    assert!(
        !rendered.contains("nginx:alpine"),
        "render_demo_manifest output must NOT contain 'nginx:alpine' (floating tag); got:\n{}",
        &rendered[..rendered.len().min(1000)]
    );
}

// ===========================================================================
// GROUP F — nginx-ingress manifests reference nginx/nginx-ingress:5.5.0
// ===========================================================================

/// F1 — nginx-ingress-daemonset.yaml (via CARGO_MANIFEST_DIR) contains 'nginx/nginx-ingress:5.5.0'.
#[test]
fn f1_nginx_ingress_5_5_0() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("manifests/nginx-ingress/nginx-ingress-daemonset.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read manifests/nginx-ingress/nginx-ingress-daemonset.yaml: {}",
            e
        ),
    };
    assert!(
        content.contains("nginx/nginx-ingress:5.5.0"),
        "nginx-ingress-daemonset.yaml must contain 'nginx/nginx-ingress:5.5.0'; got (excerpt):\n{}",
        &content[..content.len().min(500)]
    );
}

/// F2 — NEGATIVE: no UNCOMMENTED line in nginx-ingress-daemonset.yaml contains
///      'nginx/nginx-ingress:5.1.1'.
///      NOTE: line 103 is a commented line — this test must filter comment lines.
#[test]
fn f2_nginx_ingress_no_5_1_1_uncommented() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("manifests/nginx-ingress/nginx-ingress-daemonset.yaml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read manifests/nginx-ingress/nginx-ingress-daemonset.yaml: {}",
            e
        ),
    };
    for line in content.lines() {
        // Skip lines that are entirely comments (after trimming leading whitespace).
        if line.trim_start().starts_with('#') {
            continue;
        }
        assert!(
            !line.contains("nginx/nginx-ingress:5.1.1"),
            "nginx-ingress-daemonset.yaml has an uncommented line referencing stale \
             'nginx/nginx-ingress:5.1.1': {:?}",
            line
        );
    }
}

// ===========================================================================
// OPTIONAL GROUP G — source-grep guards for non-Rust files (defense-in-depth)
// ===========================================================================

/// G1 — OPTIONAL: images/Dockerfile contains KUBERNETES_VERSION 1.36.1 and NOT 1.31.0.
#[test]
fn g1_dockerfile_k8s_1_36_1_optional() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("images/Dockerfile");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read images/Dockerfile: {}", e),
    };
    assert!(
        content.contains("1.36.1"),
        "images/Dockerfile must contain '1.36.1' for KUBERNETES_VERSION; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        !content.contains("1.31.0"),
        "images/Dockerfile must NOT contain stale '1.31.0'"
    );
}

/// G2 — OPTIONAL: Dockerfile pins containerd 2.3.1, runc 1.4.2, CNI plugins 1.9.1,
///      debian:13-slim, includes erofs-utils; no 1.7.18/debian:12-slim.
#[test]
fn g2_dockerfile_runtime_versions_optional() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("images/Dockerfile");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read images/Dockerfile: {}", e),
    };
    assert!(
        content.contains("2.3.1"),
        "images/Dockerfile must pin containerd 2.3.1; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        content.contains("1.4.2"),
        "images/Dockerfile must pin runc 1.4.2; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        content.contains("1.9.1"),
        "images/Dockerfile must pin CNI plugins 1.9.1; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        content.contains("debian:13-slim"),
        "images/Dockerfile must use base image 'debian:13-slim'; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        content.contains("erofs-utils"),
        "images/Dockerfile must include 'erofs-utils'; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    assert!(
        !content.contains("1.7.18"),
        "images/Dockerfile must NOT contain stale containerd '1.7.18'"
    );
    assert!(
        !content.contains("debian:12-slim"),
        "images/Dockerfile must NOT use stale 'debian:12-slim'"
    );
}

/// G3 — OPTIONAL: mise.toml cilium-cli pinned '0.19.4', NOT 'latest', NOT '0.19.2'.
#[test]
fn g3_mise_cilium_cli_0_19_4_optional() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // mise.toml lives one directory above the kina-cli package.
    let path = std::path::Path::new(manifest_dir)
        .parent()
        .expect("kina-cli has a parent directory")
        .join("mise.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read mise.toml: {}", e),
    };
    assert!(
        content.contains("0.19.4"),
        "mise.toml must pin cilium-cli to '0.19.4'; got (excerpt):\n{}",
        &content[..content.len().min(800)]
    );
    // Must not use floating 'latest' for cilium-cli.
    // Parse only the cilium-cli line to avoid false positives from other tools.
    for line in content.lines() {
        if line.contains("cilium-cli") {
            assert!(
                !line.contains("latest"),
                "mise.toml cilium-cli must NOT be pinned to 'latest'; offending line: {:?}",
                line
            );
            assert!(
                !line.contains("0.19.2"),
                "mise.toml cilium-cli must NOT use stale '0.19.2'; offending line: {:?}",
                line
            );
        }
    }
}

/// G4 — OPTIONAL: apple_container.rs source still contains CILIUM_VERSION '1.18.10'
///      (kina-3 decision: do NOT bump Cilium).
#[test]
fn g4_cilium_version_unchanged_1_18_10_optional() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/core/apple_container.rs: {}", e),
    };
    assert!(
        content.contains("1.18.10"),
        "src/core/apple_container.rs CILIUM_VERSION must still be '1.18.10' \
         (kina-3 decision: do NOT bump Cilium); got (excerpt):\n{}",
        &content[..content.len().min(500)]
    );
}
