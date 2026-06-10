/// Cilium install tests — adversarial TDD (P2 test author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns / consts / types that do not yet
/// exist in kina_cli::core::apple_container. The compile errors are the spec; P3 (the
/// separate implementer agent) makes them green without modifying this file.
///
/// All tests are pure: NO live `container` CLI invocations, NO process spawns, NO network.
/// Source-grep guard tests open the source file via CARGO_MANIFEST_DIR.
use kina_cli::config::CniPlugin;
use kina_cli::core::apple_container::{
    build_cilium_cli_install_script, build_cilium_install_cmd, select_cni, CILIUM_CLI_VERSION,
    CILIUM_VERSION,
};

// ===========================================================================
// Group A: pinned version consts (AC1) — simplest, just equality
// ===========================================================================

/// T1 — CILIUM_CLI_VERSION const must be pinned to "v0.19.4" (not curled from stable.txt)
#[test]
fn cilium_cli_version_const_is_pinned() {
    assert_eq!(
        CILIUM_CLI_VERSION, "v0.19.4",
        "CILIUM_CLI_VERSION must be pinned to \"v0.19.4\"; got \"{}\"",
        CILIUM_CLI_VERSION
    );
}

/// T2 — CILIUM_VERSION const must be pinned to "1.18.10"
#[test]
fn cilium_version_const_is_pinned() {
    assert_eq!(
        CILIUM_VERSION, "1.18.10",
        "CILIUM_VERSION must be pinned to \"1.18.10\"; got \"{}\"",
        CILIUM_VERSION
    );
}

/// T3 — source-grep guard: apple_container.rs must NOT contain "stable.txt" (runtime curl removed)
#[test]
fn source_has_no_runtime_stable_txt_curl() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        !src.contains("stable.txt"),
        "apple_container.rs must not contain \"stable.txt\" — runtime curl of stable.txt must be replaced with pinned CILIUM_CLI_VERSION const"
    );
}

// ===========================================================================
// Group B: cilium-cli download/install script builder (AC4)
// ===========================================================================

/// T4 — build_cilium_cli_install_script(CILIUM_CLI_VERSION) output contains literal "v0.19.4"
#[test]
fn build_cli_script_embeds_pinned_version() {
    let script = build_cilium_cli_install_script(CILIUM_CLI_VERSION);
    assert!(
        script.contains("v0.19.4"),
        "build_cilium_cli_install_script output must contain the pinned version \"v0.19.4\"; got:\n{}",
        script
    );
}

/// T5 — script output does NOT contain "stable.txt" nor the stable.txt raw.githubusercontent URL
#[test]
fn build_cli_script_no_stable_txt() {
    let script = build_cilium_cli_install_script(CILIUM_CLI_VERSION);
    assert!(
        !script.contains("stable.txt"),
        "build_cilium_cli_install_script output must not contain \"stable.txt\"; got:\n{}",
        script
    );
    assert!(
        !script.contains("raw.githubusercontent.com/cilium/cilium-cli/main/stable.txt"),
        "build_cilium_cli_install_script output must not contain the stable.txt URL; got:\n{}",
        script
    );
}

/// T6 — script output contains "arm64" (CLI_ARCH=arm64 / cilium-linux-arm64)
#[test]
fn build_cli_script_is_arm64() {
    let script = build_cilium_cli_install_script(CILIUM_CLI_VERSION);
    assert!(
        script.contains("arm64"),
        "build_cilium_cli_install_script output must contain \"arm64\" for Apple Silicon; got:\n{}",
        script
    );
}

/// T7 — script output contains "sha256sum --check" (integrity check retained)
#[test]
fn build_cli_script_retains_sha256_check() {
    let script = build_cilium_cli_install_script(CILIUM_CLI_VERSION);
    assert!(
        script.contains("sha256sum --check"),
        "build_cilium_cli_install_script output must contain \"sha256sum --check\"; got:\n{}",
        script
    );
}

/// T8 — script output contains the release download URL incorporating the pinned version
#[test]
fn build_cli_script_download_url_from_version() {
    let script = build_cilium_cli_install_script(CILIUM_CLI_VERSION);
    assert!(
        script.contains("releases/download/v0.19.4/cilium-linux-arm64.tar.gz"),
        "build_cilium_cli_install_script output must contain \
         \"releases/download/v0.19.4/cilium-linux-arm64.tar.gz\" built from the pinned const; got:\n{}",
        script
    );
}

// ===========================================================================
// Group C: cilium install command builder — exact --set flags (AC2)
// ===========================================================================

/// T9 — build_cilium_install_cmd("1.18.10","192.168.65.2") contains "cilium install --version 1.18.10"
#[test]
fn install_cmd_pins_version() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("cilium install --version 1.18.10"),
        "build_cilium_install_cmd must contain \"cilium install --version 1.18.10\"; got:\n{}",
        cmd
    );
}

/// T10 — contains "--set kubeProxyReplacement=false"
#[test]
fn install_cmd_kube_proxy_replacement_false() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set kubeProxyReplacement=false"),
        "build_cilium_install_cmd must contain \"--set kubeProxyReplacement=false\"; got:\n{}",
        cmd
    );
}

/// T11 — contains "--set ipam.mode=kubernetes"
#[test]
fn install_cmd_ipam_mode_kubernetes() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set ipam.mode=kubernetes"),
        "build_cilium_install_cmd must contain \"--set ipam.mode=kubernetes\"; got:\n{}",
        cmd
    );
}

/// T12 — contains "--set ipv4NativeRoutingCIDR=10.244.0.0/16"
#[test]
fn install_cmd_ipv4_native_routing_cidr() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set ipv4NativeRoutingCIDR=10.244.0.0/16"),
        "build_cilium_install_cmd must contain \"--set ipv4NativeRoutingCIDR=10.244.0.0/16\"; got:\n{}",
        cmd
    );
}

/// T13 — contains "--set routingMode=tunnel"
#[test]
fn install_cmd_routing_mode_tunnel() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set routingMode=tunnel"),
        "build_cilium_install_cmd must contain \"--set routingMode=tunnel\"; got:\n{}",
        cmd
    );
}

/// T14 — contains "--set tunnelProtocol=vxlan"
#[test]
fn install_cmd_tunnel_protocol_vxlan() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set tunnelProtocol=vxlan"),
        "build_cilium_install_cmd must contain \"--set tunnelProtocol=vxlan\"; got:\n{}",
        cmd
    );
}

/// T15 — contains "--set ipv6.enabled=false"
#[test]
fn install_cmd_ipv6_disabled() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set ipv6.enabled=false"),
        "build_cilium_install_cmd must contain \"--set ipv6.enabled=false\"; got:\n{}",
        cmd
    );
}

/// T16 — contains "--set enableLocalNodeRoute=false"
#[test]
fn install_cmd_enable_local_node_route_false() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set enableLocalNodeRoute=false"),
        "build_cilium_install_cmd must contain \"--set enableLocalNodeRoute=false\"; got:\n{}",
        cmd
    );
}

/// T17 — contains "--set nodePort.enabled=true"
#[test]
fn install_cmd_nodeport_enabled() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set nodePort.enabled=true"),
        "build_cilium_install_cmd must contain \"--set nodePort.enabled=true\"; got:\n{}",
        cmd
    );
}

/// T18 — contains "--set hostPort.enabled=true"
#[test]
fn install_cmd_hostport_enabled() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set hostPort.enabled=true"),
        "build_cilium_install_cmd must contain \"--set hostPort.enabled=true\"; got:\n{}",
        cmd
    );
}

/// T19 — contains "--set k8sServiceHost=192.168.65.2" (templated from cp_ip input)
#[test]
fn install_cmd_k8s_service_host_uses_cp_ip() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set k8sServiceHost=192.168.65.2"),
        "build_cilium_install_cmd must contain \"--set k8sServiceHost=192.168.65.2\"; got:\n{}",
        cmd
    );
}

/// T20 — contains "--set k8sServicePort=6443"
#[test]
fn install_cmd_k8s_service_port_6443() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set k8sServicePort=6443"),
        "build_cilium_install_cmd must contain \"--set k8sServicePort=6443\"; got:\n{}",
        cmd
    );
}

/// T21 — contains "--set operator.replicas=1"
#[test]
fn install_cmd_operator_replicas_1() {
    let cmd = build_cilium_install_cmd("1.18.10", "192.168.65.2");
    assert!(
        cmd.contains("--set operator.replicas=1"),
        "build_cilium_install_cmd must contain \"--set operator.replicas=1\"; got:\n{}",
        cmd
    );
}

/// T22 — triangulation: build with a different ip "10.0.0.5" yields k8sServiceHost=10.0.0.5
///        and does NOT contain "192.168.65.2" (proves cp_ip is interpolated, not hardcoded)
#[test]
fn install_cmd_distinct_cp_ip_templated() {
    let cmd = build_cilium_install_cmd("1.18.10", "10.0.0.5");
    assert!(
        cmd.contains("--set k8sServiceHost=10.0.0.5"),
        "build_cilium_install_cmd with ip \"10.0.0.5\" must contain \
         \"--set k8sServiceHost=10.0.0.5\"; got:\n{}",
        cmd
    );
    assert!(
        !cmd.contains("192.168.65.2"),
        "build_cilium_install_cmd with ip \"10.0.0.5\" must NOT contain the hardcoded \
         \"192.168.65.2\" — proves cp_ip is interpolated; got:\n{}",
        cmd
    );
}

// ===========================================================================
// Group D: rationale comments in source (AC3) — source-grep guards
// ===========================================================================

/// T23 — apple_container.rs src contains "cilium/cilium#32448" (enableLocalNodeRoute comment)
#[test]
fn source_cites_cilium_32448() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("cilium/cilium#32448"),
        "apple_container.rs must contain \"cilium/cilium#32448\" as the rationale comment \
         for enableLocalNodeRoute=false"
    );
}

/// T24 — src contains "kubernetes/minikube#18851" (kata-kernel EAFNOSUPPORT citation)
#[test]
fn source_cites_minikube_18851() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("kubernetes/minikube#18851"),
        "apple_container.rs must contain \"kubernetes/minikube#18851\" as the kata-kernel \
         EAFNOSUPPORT workaround citation"
    );
}

/// T25 — src contains "cilium/cilium#31168" (nodePort+hostPort pairing comment)
#[test]
fn source_cites_cilium_31168() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("cilium/cilium#31168"),
        "apple_container.rs must contain \"cilium/cilium#31168\" as the rationale comment \
         for requiring both nodePort.enabled and hostPort.enabled"
    );
}

// ===========================================================================
// Group E: readiness gating + diagnostics in install path (AC5, AC6) — source-grep guards
// ===========================================================================

/// T26 — src contains "cilium status --wait --wait-duration 5m" (readiness gate after install)
#[test]
fn source_has_cilium_status_wait_gate() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("cilium status --wait --wait-duration 5m"),
        "apple_container.rs must contain \"cilium status --wait --wait-duration 5m\" \
         as the readiness gate after Cilium install"
    );
}

/// T27 — src contains "KUBECONFIG=/etc/kubernetes/admin.conf" in cilium install path
#[test]
fn source_install_uses_admin_conf_kubeconfig() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("KUBECONFIG=/etc/kubernetes/admin.conf"),
        "apple_container.rs must contain \"KUBECONFIG=/etc/kubernetes/admin.conf\" \
         in the Cilium install path"
    );
}

/// T28 — src contains "kubectl wait --for=condition=Ready node --all --timeout=300s" (final gate)
#[test]
fn source_final_gate_kubectl_wait_nodes() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("kubectl wait --for=condition=Ready node --all --timeout=300s"),
        "apple_container.rs must contain \"kubectl wait --for=condition=Ready node --all --timeout=300s\" \
         as the final readiness gate after all workers join"
    );
}

/// T29 — src contains a bare "cilium status" used for diagnostics (distinct from T26 which has --wait)
#[test]
fn source_diag_cilium_status_no_wait() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    // The source must contain a "cilium status" occurrence that is NOT followed immediately by " --wait"
    // (i.e., the diagnostics branch captures bare status, separate from the gating invocation)
    let has_bare_cilium_status = src
        .split('\n')
        .any(|line| line.contains("cilium status") && !line.contains("--wait"));

    assert!(
        has_bare_cilium_status,
        "apple_container.rs must contain a bare \"cilium status\" (without --wait) in the \
         diagnostics capture path, distinct from the readiness gate in T26"
    );
}

/// T30 — src contains "kubectl -n kube-system get pods -o wide" (diagnostics on failure)
#[test]
fn source_diag_get_pods_wide() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("kubectl -n kube-system get pods -o wide"),
        "apple_container.rs must contain \"kubectl -n kube-system get pods -o wide\" \
         in the diagnostics capture on Cilium install failure"
    );
}

/// T31 — src contains "kubectl -n kube-system logs ds/cilium --tail=100" (diagnostics on failure)
#[test]
fn source_diag_cilium_logs_tail() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("kubectl -n kube-system logs ds/cilium --tail=100"),
        "apple_container.rs must contain \"kubectl -n kube-system logs ds/cilium --tail=100\" \
         in the diagnostics capture on Cilium install failure"
    );
}

// ===========================================================================
// Group F: --cni flag wiring + selection fn (AC7)
// ===========================================================================

/// T32 — select_cni(Some(Cilium), Ptp) == Cilium (cli flag overrides config default)
#[test]
fn select_cni_flag_overrides_config_default() {
    let result = select_cni(Some(CniPlugin::Cilium), CniPlugin::Ptp);
    assert_eq!(
        result,
        CniPlugin::Cilium,
        "select_cni(Some(Cilium), Ptp) must return Cilium — cli flag overrides config default"
    );
}

/// T33 — select_cni(None, Ptp) == Ptp (no flag → config default)
#[test]
fn select_cni_none_falls_back_to_config() {
    let result = select_cni(None, CniPlugin::Ptp);
    assert_eq!(
        result,
        CniPlugin::Ptp,
        "select_cni(None, Ptp) must return Ptp — no cli flag means use config default"
    );
}

/// T34 — triangulation: select_cni(None, Cilium) == Cilium (fallback honors config, not hardcoded Ptp)
#[test]
fn select_cni_none_falls_back_to_cilium_config() {
    let result = select_cni(None, CniPlugin::Cilium);
    assert_eq!(
        result,
        CniPlugin::Cilium,
        "select_cni(None, Cilium) must return Cilium — fallback must honor the config value, \
         not be hardcoded to Ptp"
    );
}

/// T35 — select_cni(Some(Ptp), Cilium) == Ptp (override works both directions)
#[test]
fn select_cni_ptp_flag_overrides_cilium_config() {
    let result = select_cni(Some(CniPlugin::Ptp), CniPlugin::Cilium);
    assert_eq!(
        result,
        CniPlugin::Ptp,
        "select_cni(Some(Ptp), Cilium) must return Ptp — override must work in both directions"
    );
}

/// T36 — source-grep guard: install_cni_plugin path consumes options.cni_plugin
///        (not only config.cluster.default_cni)
#[test]
fn source_install_cni_plugin_reads_options_cni() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    assert!(
        src.contains("options.cni_plugin"),
        "apple_container.rs must contain \"options.cni_plugin\" — the create_cluster / \
         install_cni_plugin path must consume the CLI-flag-driven cni_plugin field from \
         CreateClusterOptions, not only config.cluster.default_cni"
    );
}

/// T37 — source-grep guard: per-worker PTP gate in create_multi_node_cluster keys off the
///        resolved/selected cni, not only self.config.cluster.default_cni
#[test]
fn source_per_worker_ptp_gate_uses_selected_cni() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read source for guard test: {}", e),
    };

    // The per-worker PTP gate must use a resolved cni variable (from select_cni),
    // not the raw config default. We assert the source uses select_cni (the resolution fn)
    // somewhere in the multi-node path, ensuring the per-worker gate is driven by the
    // resolved value rather than only self.config.cluster.default_cni.
    assert!(
        src.contains("select_cni"),
        "apple_container.rs must contain \"select_cni\" — the per-worker PTP gate in \
         create_multi_node_cluster must key off the resolved CNI plugin (via select_cni), \
         not only self.config.cluster.default_cni"
    );
}
