use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use tracing::{debug, info, warn};

use super::types::{
    ClusterInfo, ClusterStatus, CreateClusterOptions, KubeadmJoinInfo, LoadImageOptions, NodeInfo,
    NodeRole,
};
use crate::config::{CniPlugin, Config};

/// Minimum supported Apple Container version (major, minor, patch).
/// Raised to 1.0.0: config.toml replaces system property get/set/clear,
/// structured ls/inspect output shape changed, and container cp is required
/// for image loading.
pub const MIN_VERSION: (u32, u32, u32) = (1, 0, 0);

/// Pinned cilium-cli version (2026-05-20 release).
/// Replacing the runtime curl-based version discovery with a pinned const ensures
/// reproducible installs and removes the dependency on an outbound HTTP call
/// during cluster bootstrap.
pub const CILIUM_CLI_VERSION: &str = "v0.19.4";

/// Pinned Cilium CNI version (latest 1.18.x patch, 2026-05-13).
/// Stays on the validated 1.18 minor; treat 1.19.x as a separate upgrade.
pub const CILIUM_VERSION: &str = "1.18.10";

/// Build the shell snippet that downloads and installs the cilium-cli binary
/// inside a node container. All values are derived from the pinned `cli_version`
/// const — no runtime version discovery via external HTTP.
///
/// The snippet is ARM64-only (Apple Silicon / Apple Container VMs) and retains
/// the sha256sum integrity check.
pub fn build_cilium_cli_install_script(cli_version: &str) -> String {
    format!(
        r#"CLI_ARCH=arm64
CILIUM_CLI_VERSION={cli_version}
curl -L --fail --remote-name-all https://github.com/cilium/cilium-cli/releases/download/{cli_version}/cilium-linux-arm64.tar.gz{{,.sha256sum}}
sha256sum --check cilium-linux-arm64.tar.gz.sha256sum
tar xzvfC cilium-linux-arm64.tar.gz /usr/local/bin
rm -f cilium-linux-arm64.tar.gz cilium-linux-arm64.tar.gz.sha256sum"#,
        cli_version = cli_version,
    )
}

/// Build the `cilium install` command string for the given Cilium version and
/// control-plane VM IP.
///
/// All `--set` values are topology-correct for Apple Container clusters using the
/// stock kata-kernel (workaround profile — custom kernel switches to the full-eBPF profile):
/// - kubeadm deploys kube-proxy, so `kubeProxyReplacement=false` makes the hybrid explicit
/// - `ipam.mode=kubernetes` uses per-node PodCIDRs from `podSubnet 10.244.0.0/16`
///   (the cilium-cli default cluster-pool 10.0.0.0/8 mismatches the kube-proxy clusterCIDR)
/// - `routingMode=tunnel` + `tunnelProtocol=vxlan` — outer packets use node VM IPs, vmnet-safe
/// - `enableLocalNodeRoute=false` — kata-kernel EAFNOSUPPORT workaround,
///   see cilium/cilium#32448 and kubernetes/minikube#18851
/// - `l7Proxy=false` — disables Cilium's L7 proxy (Envoy-based). The kata-kernel used by
///   Apple Container VMs has `CONFIG_IP_ADVANCED_ROUTER` / `CONFIG_IP_MULTIPLE_TABLES` unset,
///   which means IPv4 policy routing (`ip rule`) is absent. Cilium's L7 proxy init path
///   calls `NodeEnsureLocalRoutingRule()` (daemon/cmd/daemon.go) unconditionally when
///   `EnableL7Proxy=true`; that function issues `ip rule replace … table local` with AF_INET
///   which the kernel rejects with EAFNOSUPPORT, crashing the cilium-agent before it starts.
///   Network policy enforcement (L3/L4) and pod-to-pod connectivity are unaffected.
/// - `--set-string extraConfig.dnsproxy-enable-transparent-mode=false` — injects the key
///   `dnsproxy-enable-transparent-mode: "false"` (string, not boolean) verbatim into the
///   `cilium-config` ConfigMap. The `extraConfig` Helm key merges arbitrary keys directly
///   into the ConfigMap and is immune to chart path renames. `--set-string` is required because
///   the ConfigMap `data` field is `map<string,string>`; using plain `--set` passes a YAML
///   boolean `false` which the Go JSON unmarshaler rejects with "cannot unmarshal bool into Go
///   struct field ConfigMap.data of type string". This disables the dnsproxy transparent mode.
///   NOTE: Do NOT also set a chart-native `dnsProxy.*` path simultaneously — that would produce
///   a duplicate key in the rendered ConfigMap. See cilium/cilium#32448 and
///   kubernetes/minikube#18851 for the broader kata-kernel xt_socket / ip-rule context.
/// - `nodePort.enabled=true` AND `hostPort.enabled=true` — BOTH required: without nodePort,
///   hostPort is silently ignored, breaking nginx-ingress DaemonSet on 80/443,
///   see cilium/cilium#31168
/// - `k8sServiceHost` / `k8sServicePort` remove the kube-proxy bootstrap dependency for
///   joining workers
pub fn build_cilium_install_cmd(version: &str, cp_ip: &str) -> String {
    format!(
        "KUBECONFIG=/etc/kubernetes/admin.conf cilium install --version {version} \
         --set kubeProxyReplacement=false \
         --set ipam.mode=kubernetes \
         --set ipv4NativeRoutingCIDR=10.244.0.0/16 \
         --set routingMode=tunnel \
         --set tunnelProtocol=vxlan \
         --set ipv6.enabled=false \
         --set enableLocalNodeRoute=false \
         --set l7Proxy=false \
         --set-string extraConfig.dnsproxy-enable-transparent-mode=false \
         --set nodePort.enabled=true \
         --set hostPort.enabled=true \
         --set k8sServiceHost={cp_ip} \
         --set k8sServicePort=6443 \
         --set operator.replicas=1",
        version = version,
        cp_ip = cp_ip,
    )
}

/// Build the `cilium install` command string for the full-eBPF profile.
///
/// Used when kina nodes are booted on the custom kernel (6.18.5+kina.1) which has
/// all required BPF options compiled in (CONFIG_BPF_JIT=y, CONFIG_BPF_EVENTS=y,
/// CONFIG_NETFILTER_XT_MATCH_SOCKET=y, CONFIG_IP_MULTIPLE_TABLES=y, etc.).
///
/// Full-eBPF values (all kata-kernel workarounds retired):
/// - `kubeProxyReplacement=true` — Cilium replaces kube-proxy; kubeadm must skip
///   the addon/kube-proxy phase (see `build_kubeadm_init_args(true)`).
/// - `bpf.masquerade=true` — eBPF-native masquerade instead of iptables MASQUERADE.
/// - `bpf.hostLegacyRouting=false` — BPF host routing (requires BPF_JIT + multiple-tables).
/// - `hubble.enabled=true` — Hubble observability plane (requires BPF_EVENTS + KPROBES).
/// - `ipam.mode=kubernetes` — per-node PodCIDRs from kubeadm podSubnet 10.244.0.0/16.
/// - `k8sServiceHost` / `k8sServicePort` — remove kube-proxy bootstrap dependency.
///
/// Retired workarounds (vs stock profile):
/// - enableLocalNodeRoute=false — retired; IP_MULTIPLE_TABLES + FIB_RULES present in kernel.
/// - l7Proxy=false — retired; L7 proxy enabled (omitted-default-true).
/// - extraConfig.dnsproxy-enable-transparent-mode=false — retired; XT_MATCH_SOCKET + TPROXY present.
/// - kubeProxyReplacement=false — retired; full replacement enabled.
///
/// Note: l7Proxy is intentionally omitted (default true) rather than set explicitly.
/// See cilium/cilium#32448 and kubernetes/minikube#18851 for the stock-kernel context.
pub fn build_cilium_install_cmd_ebpf(version: &str, cp_ip: &str) -> String {
    format!(
        "KUBECONFIG=/etc/kubernetes/admin.conf cilium install --version {version} \
         --set kubeProxyReplacement=true \
         --set ipam.mode=kubernetes \
         --set bpf.masquerade=true \
         --set bpf.hostLegacyRouting=false \
         --set hubble.enabled=true \
         --set k8sServiceHost={cp_ip} \
         --set k8sServicePort=6443 \
         --set operator.replicas=1",
        version = version,
        cp_ip = cp_ip,
    )
}

/// Returns the `--kernel <path>` arguments to pass to `container run` when a custom
/// kernel path is set, or an empty Vec when using the system default (stock) kernel.
///
/// Used by all three node-creation fns (create_single_node, create_control_plane_node,
/// create_worker_node) to inject the custom kernel into the node container run args.
pub fn node_kernel_args(kernel_path: Option<&std::path::Path>) -> Vec<String> {
    match kernel_path {
        Some(path) => vec!["--kernel".to_string(), path.to_string_lossy().into_owned()],
        None => vec![],
    }
}

/// Built-in default CPU count for node containers.
///
/// Matches the prior hardcoded value ("--cpus", "4") so that changing nothing in the
/// config preserves existing cluster creation behaviour.
pub const DEFAULT_NODE_CPUS: u32 = 4;

/// Built-in default memory for node containers.
///
/// Matches the prior hardcoded value ("--memory", "4g") so that changing nothing in the
/// config preserves existing cluster creation behaviour.
pub const DEFAULT_NODE_MEMORY: &str = "4g";

/// Returns the `--cpus <n> --memory <size>` arguments to pass to `container run`.
///
/// Produces an owned-String Vec (like `node_kernel_args`) because the values are dynamic.
/// Used by all three node-creation fns. Callers build the owned Vec, then turn it into
/// `&str` refs for `args.extend_from_slice`:
///
/// ```ignore
/// let res_owned = node_resource_args(cpus, &memory);
/// let res_refs: Vec<&str> = res_owned.iter().map(|s| s.as_str()).collect();
/// args.extend_from_slice(&res_refs);
/// ```
pub fn node_resource_args(cpus: u32, memory: &str) -> Vec<String> {
    vec![
        "--cpus".to_string(),
        cpus.to_string(),
        "--memory".to_string(),
        memory.to_string(),
    ]
}

/// Validate that `cpus` and `memory` are usable by `container run`.
///
/// Errors include the field name ("cpus" or "memory") in their message so that
/// CLI error output is unambiguous:
/// - `cpus == 0` → `Err("cpus: ...")`
/// - empty memory → `Err("memory: ...")`
/// - memory with no m/g suffix → `Err("memory: ...")`
/// - memory with non-numeric prefix → `Err("memory: ...")`
/// - memory numeric prefix is 0 → `Err("memory: ...")`
///
/// Mirrors `validate_version`'s `Result<()>` + `anyhow::anyhow!` style.
pub fn validate_resources(cpus: u32, memory: &str) -> anyhow::Result<()> {
    if cpus == 0 {
        return Err(anyhow::anyhow!(
            "cpus: value must be at least 1; got 0 (a 0-CPU node cannot boot)"
        ));
    }

    if memory.is_empty() {
        return Err(anyhow::anyhow!(
            "memory: value must not be empty; expected format <positive-int><m|g> (e.g. \"512m\", \"4g\")"
        ));
    }

    // Accept <positive-int><m|M|g|G>
    let lower = memory.to_lowercase();
    let suffix = lower.chars().last().unwrap_or('\0');
    if suffix != 'm' && suffix != 'g' {
        return Err(anyhow::anyhow!(
            "memory: value \"{}\" has no valid unit suffix; \
             expected format <positive-int><m|g> (e.g. \"512m\", \"4g\")",
            memory
        ));
    }

    let numeric_part = &lower[..lower.len() - 1];
    let n: u64 = numeric_part.parse().map_err(|_| {
        anyhow::anyhow!(
            "memory: numeric prefix \"{}\" in \"{}\" is not a valid integer; \
             expected format <positive-int><m|g> (e.g. \"512m\", \"4g\")",
            numeric_part,
            memory
        )
    })?;

    if n == 0 {
        return Err(anyhow::anyhow!(
            "memory: value \"{}\" has a zero numeric prefix; \
             must be a positive integer followed by m or g",
            memory
        ));
    }

    Ok(())
}

/// Resolve the effective CPU count using the three-tier precedence model.
///
/// Precedence (highest to lowest):
///   1. CLI `--cpus` flag (`cli`)
///   2. Per-role config default (`config`)
///   3. Built-in default (`builtin` — callers pass `DEFAULT_NODE_CPUS`)
pub fn resolve_cpus(cli: Option<u32>, config: Option<u32>, builtin: u32) -> u32 {
    cli.or(config).unwrap_or(builtin)
}

/// Resolve the effective memory string using the three-tier precedence model.
///
/// Precedence (highest to lowest):
///   1. CLI `--memory` flag (`cli`)
///   2. Per-role config default (`config`)
///   3. Built-in default (`builtin` — callers pass `DEFAULT_NODE_MEMORY`)
///
/// Returns an owned `String` so callers can store it in structs without lifetime issues.
pub fn resolve_memory(cli: Option<&str>, config: Option<&str>, builtin: &str) -> String {
    cli.or(config).unwrap_or(builtin).to_string()
}

/// Resolve the effective kernel path: CLI flag takes precedence over the config default.
///
/// Mirrors the `select_cni` precedence model — CLI flag always wins over config default.
/// Returns `None` when neither is set (stock kernel; no `--kernel` flag injected).
pub fn select_kernel_path(
    cli_flag: Option<std::path::PathBuf>,
    config_default: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    cli_flag.or(config_default)
}

/// Build the kubeadm init argument list for node initialization.
///
/// When `full_ebpf` is true (custom kernel with kubeProxyReplacement=true), the
/// `addon/kube-proxy` phase is skipped so kubeadm does not deploy kube-proxy.
/// When `full_ebpf` is false (stock kernel), kube-proxy is retained.
///
/// The `--skip-phases=preflight` flag is always present (existing baseline);
/// `--skip-phases=addon/kube-proxy` is added as a comma-joined extension under
/// full-eBPF mode.
pub fn build_kubeadm_init_args(full_ebpf: bool) -> Vec<String> {
    if full_ebpf {
        vec![
            "--config=/kind/kubeadm.conf".to_string(),
            "--skip-phases=preflight,addon/kube-proxy".to_string(),
            "--v=1".to_string(),
        ]
    } else {
        vec![
            "--config=/kind/kubeadm.conf".to_string(),
            "--skip-phases=preflight".to_string(),
            "--v=1".to_string(),
        ]
    }
}

/// Resolve the effective CNI plugin: CLI flag takes precedence over the
/// config file default when present.
pub fn select_cni(cli_flag: Option<CniPlugin>, config_default: CniPlugin) -> CniPlugin {
    cli_flag.unwrap_or(config_default)
}

/// Strategy for resolving the Apple Container CLI binary path.
///
/// `Which(name)` asks the shell PATH resolver (`which <name>`) — preferred
/// because it respects the user's PATH ordering (e.g., brew-managed 1.0.0
/// binary at /opt/homebrew/bin/container before a stale package-installer
/// binary at /usr/local/bin/container).
///
/// `Hardcoded(path)` tries a known absolute path as a fallback for systems
/// where the binary is not on PATH.
pub enum CliPathStrategy {
    /// Resolve via PATH (e.g., `which container`). The inner String is the
    /// binary name to pass to `which`.
    Which(String),
    /// Try this absolute path directly.
    Hardcoded(String),
}

/// Returns the ordered list of strategies detect_cli_path() uses to locate
/// the Apple Container CLI binary.
///
/// ORDER CONTRACT: all Which(_) entries come before any Hardcoded(_) entry.
/// This ensures brew/PATH-managed binaries (including the 1.0.0 release) are
/// found before any stale package-installer binary that may still reside at
/// /usr/local/bin/container on upgraded hosts.
pub fn cli_path_candidates() -> Vec<CliPathStrategy> {
    vec![
        // PATH resolution first — respects the user's PATH, so brew/nix/mise-managed
        // 1.0.0 binaries win over any stale /usr/local/bin/container left by older
        // package installers.
        CliPathStrategy::Which("container".to_string()),
        CliPathStrategy::Which("apple-container".to_string()),
        // Hardcoded fallbacks — tried only when PATH resolution finds nothing.
        CliPathStrategy::Hardcoded("/opt/homebrew/bin/container".to_string()),
        CliPathStrategy::Hardcoded("/opt/homebrew/bin/apple-container".to_string()),
        CliPathStrategy::Hardcoded("/usr/local/bin/container".to_string()),
        CliPathStrategy::Hardcoded("/usr/local/bin/apple-container".to_string()),
        CliPathStrategy::Hardcoded(
            "/System/Library/PrivateFrameworks/ContainerManager.framework/Versions/A/Resources/apple-container".to_string(),
        ),
    ]
}

/// Parse the version string from Apple Container CLI output.
///
/// Expects the format `container CLI version <version> (build: ..., commit: ...)`.
/// The token after `"CLI version "` must start with a digit (semver major).
/// Returns `Ok(version_string)` on success, `Err` if the expected token is absent
/// or does not start with a digit.
pub fn parse_version_output(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    // Require the specific "CLI version " prefix used by Apple Container so that
    // arbitrary strings containing the word "version" do not accidentally parse.
    if let Some(pos) = trimmed.find("CLI version ") {
        let after_version = &trimmed[pos + "CLI version ".len()..];
        let version = after_version
            .split_whitespace()
            .next()
            .unwrap_or(after_version);
        if version.is_empty() || !version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!(
                "Could not parse Apple Container CLI version from output: {}",
                trimmed
            ));
        }
        debug!("Detected Apple Container CLI version: {}", version);
        return Ok(version.to_string());
    }
    Err(anyhow::anyhow!(
        "Could not parse Apple Container CLI version from output: {}",
        trimmed
    ))
}

/// Validate that `version` meets MIN_VERSION (1.0.0).
///
/// Returns `Err` with migration guidance if the version is too old;
/// returns `Ok(())` if the version is at or above the minimum.
pub fn validate_version(version: &str) -> Result<()> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "Invalid Apple Container CLI version format: {}",
            version
        ));
    }

    let major = parts[0]
        .parse::<u32>()
        .context("Invalid major version number")?;
    let minor = parts[1]
        .parse::<u32>()
        .context("Invalid minor version number")?;
    let patch = parts
        .get(2)
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);

    let (min_major, min_minor, min_patch) = MIN_VERSION;
    let meets_minimum = (major, minor, patch) >= (min_major, min_minor, min_patch);

    if !meets_minimum {
        return Err(anyhow::anyhow!(
            "Apple Container CLI version {} is not supported. \
             Minimum required version is {}.{}.{}.\n\
             Migration to 1.0.0 requires these changes:\n\
             - config.toml replaces 'container system property get/set/clear' \
               (UserDefaults-backed properties removed; write values to \
               ~/.config/container/config.toml and restart the service)\n\
             - structured 'container ls/inspect' output shape changed \
               (.status is now an object; networks moved to .status.networks; \
               ipv4Address replaces address)\n\
             - default Linux capability set (cap) reduced since 0.12.0; \
               use --cap-add to restore capabilities needed by workloads\n\
             Please upgrade Apple Container to 1.0.0 or later to continue.\n\
             See: https://github.com/apple/container/releases",
            version,
            min_major,
            min_minor,
            min_patch
        ));
    }

    info!(
        "Apple Container CLI version {} meets minimum requirement ({}.{}.{})",
        version, min_major, min_minor, min_patch
    );
    Ok(())
}

/// Format Apple Container's `creationDate` (ISO-8601, e.g. "2026-06-14T21:52:43Z")
/// into a friendly "YYYY-MM-DD HH:MM UTC" string. Returns the raw input on parse
/// failure and "unknown" when empty. (Apple Container 1.0.0 emits ISO strings,
/// not Mac-absolute-time floats.)
fn format_created(iso: &str) -> String {
    if iso.is_empty() {
        return "unknown".to_string();
    }
    match chrono::DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt
            .with_timezone(&chrono::Utc)
            .format("%Y-%m-%d %H:%M UTC")
            .to_string(),
        Err(_) => iso.to_string(),
    }
}

/// A container entry parsed from `container list --format json` (1.0.0 shape).
#[derive(Debug)]
pub struct ParsedContainer {
    /// Top-level `id` field (same as `configuration.id`).
    pub id: String,
    /// Labels from `configuration.labels`.
    pub labels: HashMap<String, String>,
    /// State string from `status.state` (e.g., `"running"`, `"stopped"`).
    pub state: String,
    /// Bare IPv4 address from `status.networks[0].ipv4Address` with the CIDR
    /// suffix stripped (e.g., `"192.168.65.2"` not `"192.168.65.2/24"`).
    /// `None` when the container has no network attachment.
    pub ipv4: Option<String>,
    /// `configuration.creationDate` (ISO-8601 string), if present.
    pub created: Option<String>,
}

/// Parse the JSON output of `container list --format json` using the 1.0.0 shape.
///
/// Returns `Ok(vec![])` for empty or whitespace-only input.
/// Returns `Err` for malformed JSON or if the top-level value is not an array.
pub fn parse_container_list(json: &str) -> Result<Vec<ParsedContainer>> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value =
        serde_json::from_str(json).context("Failed to parse container list JSON")?;

    let array = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Expected a JSON array at top level"))?;

    let mut result = Vec::with_capacity(array.len());

    for elem in array {
        // Top-level "id"
        let id = elem
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // configuration.labels -> HashMap
        let labels: HashMap<String, String> = elem
            .get("configuration")
            .and_then(|c| c.get("labels"))
            .and_then(|l| l.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        // status.state
        let state = elem
            .get("status")
            .and_then(|s| s.get("state"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // status.networks[0].ipv4Address with CIDR stripped
        let ipv4 = elem
            .get("status")
            .and_then(|s| s.get("networks"))
            .and_then(|n| n.as_array())
            .and_then(|arr| arr.first())
            .and_then(|net| net.get("ipv4Address"))
            .and_then(|v| v.as_str())
            .map(|cidr| cidr.split('/').next().unwrap_or(cidr).to_string());

        // configuration.creationDate (ISO-8601 string)
        let created = elem
            .get("configuration")
            .and_then(|c| c.get("creationDate"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        result.push(ParsedContainer {
            id,
            labels,
            state,
            ipv4,
            created,
        });
    }

    Ok(result)
}

/// Returns the capability arguments required for Kubernetes node containers.
///
/// Apple Container has no privileged-mode flag. Since 0.12.0 the default
/// capability set is reduced to 14 Docker-style caps — insufficient for
/// systemd, kubeadm, kubelet, containerd, and Cilium eBPF. Use `--cap-add ALL`
/// (equivalent to KIND's privileged mode) so the node can run the full k8s stack.
pub fn node_cap_args() -> Vec<&'static str> {
    vec!["--cap-add", "ALL"]
}

/// Generate kubeadm init configuration YAML (v1beta4, K8s v1.36.1).
///
/// Emits three stanzas separated by "---":
///   1. InitConfiguration  — advertise address, criSocket, kubeletExtraArgs (list form)
///   2. ClusterConfiguration — kubernetesVersion v1.36.1, apiServer/controllerManager extraArgs (list form)
///   3. JoinConfiguration  — criSocket, kubeletExtraArgs (list form)
///
/// Followed by KubeletConfiguration and KubeProxyConfiguration stanzas.
///
/// The map→list migration for kubeletExtraArgs and extraArgs mirrors
/// PR #14 (vinnie357/kina) which adopted the kubeadm v1beta4 list form.
pub fn generate_kubeadm_init_config(
    container_name: &str,
    vm_ip: &str,
    cluster_name: &str,
) -> String {
    format!(
        r#"apiVersion: kubeadm.k8s.io/v1beta4
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: "{vm_ip}"
  bindPort: 6443
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
  - name: node-ip
    value: "{vm_ip}"
  - name: provider-id
    value: "kind://docker/{cluster_name}/{container_name}"
---
apiVersion: kubeadm.k8s.io/v1beta4
kind: ClusterConfiguration
kubernetesVersion: v1.36.1
clusterName: "{cluster_name}"
controlPlaneEndpoint: "{vm_ip}:6443"
apiServer:
  certSANs:
  - "{vm_ip}"
  - "{container_name}"
  - "localhost"
  - "127.0.0.1"
  extraArgs:
  - name: runtime-config
    value: "api/all=true"
networking:
  serviceSubnet: "10.96.0.0/16"
  podSubnet: "10.244.0.0/16"
  dnsDomain: "cluster.local"
controllerManager:
  extraArgs:
  - name: enable-hostpath-provisioner
    value: "true"
scheduler: {{}}
etcd:
  local:
    dataDir: "/var/lib/etcd"
---
apiVersion: kubeadm.k8s.io/v1beta4
kind: JoinConfiguration
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
  - name: node-ip
    value: "{vm_ip}"
  - name: provider-id
    value: "kind://docker/{cluster_name}/{container_name}"
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
failSwapOn: false
authentication:
  anonymous:
    enabled: false
  webhook:
    enabled: true
authorization:
  mode: Webhook
serverTLSBootstrap: true
---
apiVersion: kubeproxy.config.k8s.io/v1alpha1
kind: KubeProxyConfiguration
bindAddress: "0.0.0.0"
healthzBindAddress: "0.0.0.0:10256"
metricsBindAddress: "0.0.0.0:10249"
clusterCIDR: "10.244.0.0/16"
"#,
    )
}

/// Generate kubeadm worker-join configuration YAML (v1beta4).
///
/// Emits a JoinConfiguration stanza with kubeletExtraArgs in list form,
/// followed by a KubeletConfiguration stanza.
///
/// The map→list migration mirrors PR #14 (vinnie357/kina).
pub fn generate_worker_join_config(
    _worker_name: &str,
    worker_ip: &str,
    join_info: &KubeadmJoinInfo,
) -> String {
    format!(
        r#"apiVersion: kubeadm.k8s.io/v1beta4
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: "{endpoint}"
    token: "{token}"
    caCertHashes:
    - "{hash}"
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
  - name: node-ip
    value: "{worker_ip}"
---
apiVersion: kubelet.config.k8s.io/v1beta1
kind: KubeletConfiguration
cgroupDriver: systemd
failSwapOn: false
"#,
        endpoint = join_info.control_plane_endpoint,
        token = join_info.token,
        hash = join_info.ca_cert_hash,
        worker_ip = worker_ip,
    )
}

// ---------------------------------------------------------------------------
// Image injection helpers (kina-12)
//
// The `container cp` subcommand silently exits 0 without transferring the
// file in some container 1.0.0 configurations.  These pure functions
// build the exec-stdin injection pattern and post-copy verification
// commands so the caller can detect and abort on a silent no-op.
// ---------------------------------------------------------------------------

/// Build the exec-stdin argument vector that streams tar bytes into a running
/// container.  The caller pipes the local tar file to the child's stdin:
///
/// ```text
/// container exec -i <container_id> sh -c 'cat > /tmp/image.tar'
/// ```
///
/// The returned slice starts with `"exec"` (never `"cp"`), includes the `-i`
/// flag so the child inherits a writable stdin, and embeds the `cat >` shell
/// redirect so the piped bytes land at `dest_path`.
#[allow(dead_code)]
pub fn build_inject_tar_args(container_id: &str, dest_path: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        "-i".to_string(),
        container_id.to_string(),
        "sh".to_string(),
        "-c".to_string(),
        format!("cat > {dest_path}"),
    ]
}

/// Build the argument vector that prints the byte count of a remote file.
///
/// ```text
/// container exec <container_id> sh -c 'wc -c < /tmp/image.tar'
/// ```
///
/// The output (trimmed) is a single integer string suitable for
/// [`parse_remote_size_output`].
#[allow(dead_code)]
pub fn build_remote_size_args(container_id: &str, dest_path: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        container_id.to_string(),
        "sh".to_string(),
        "-c".to_string(),
        format!("wc -c < {dest_path}"),
    ]
}

/// Build the argument vector that prints the sha256 digest of a remote file.
///
/// ```text
/// container exec <container_id> sh -c 'sha256sum /tmp/image.tar'
/// ```
///
/// The output is in `sha256sum` format (`<hex>  <path>\n`) and is parsed by
/// [`parse_remote_sha256_output`].
#[allow(dead_code)]
pub fn build_remote_sha256_args(container_id: &str, dest_path: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        container_id.to_string(),
        "sh".to_string(),
        "-c".to_string(),
        format!("sha256sum {dest_path}"),
    ]
}

/// Return the lowercase hex-encoded SHA-256 digest of `bytes`.
#[allow(dead_code)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Parse the output of `wc -c < <path>` (or equivalent) into a `u64` byte
/// count.  Returns `Err` for empty, whitespace-only, or non-numeric output so
/// that a missing remote file (which produces no output or `0`) is surfaced as
/// an explicit error by [`verify_injection`].
#[allow(dead_code)]
pub fn parse_remote_size_output(raw: &str) -> anyhow::Result<u64> {
    let token = raw
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("remote size output is empty or unparseable: {:?}", raw))?;
    token
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("remote size output {:?} is not a valid u64: {}", raw, e))
}

/// Parse the output of `sha256sum <path>` into the leading hex digest token.
/// `sha256sum` prints `<hex>  <path>\n`; this function returns only the hex
/// part.
///
/// Returns `Err` for empty, whitespace-only, or malformed output (including
/// tokens that are not exactly 64 lowercase hex characters).  This ensures
/// that non-`sha256sum` output (e.g. error messages, empty output) is
/// treated as a verification failure rather than a silently accepted digest.
#[allow(dead_code)]
pub fn parse_remote_sha256_output(raw: &str) -> anyhow::Result<String> {
    let token = raw.split_whitespace().next().ok_or_else(|| {
        anyhow::anyhow!("remote sha256 output is empty or unparseable: {:?}", raw)
    })?;
    // A SHA-256 hex digest is exactly 64 lowercase hex characters.
    if token.len() != 64 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!(
            "remote sha256 output {:?} does not look like a 64-character hex digest (got {:?})",
            raw,
            token,
        ));
    }
    Ok(token.to_string())
}

/// Gate function: compare local file metadata against remote measurements and
/// return `Err` on any mismatch.
///
/// Checks are applied in this order:
/// 1. Parse `remote_size_raw`; if remote size ≠ `local_len` → hard error.
/// 2. Parse `remote_sha_raw`; if remote sha ≠ `local_sha` → hard error.
/// 3. Both match → `Ok(())`.
///
/// Passing `remote_size_raw = "0"` against a non-zero `local_len` simulates
/// the silent `container cp` no-op and causes the function to return `Err`
/// before `ctr images import` is attempted.
#[allow(dead_code)]
pub fn verify_injection(
    local_len: u64,
    local_sha: &str,
    remote_size_raw: &str,
    remote_sha_raw: &str,
) -> anyhow::Result<()> {
    let remote_size = parse_remote_size_output(remote_size_raw)?;
    if remote_size != local_len {
        return Err(anyhow::anyhow!(
            "post-injection size mismatch: expected {} bytes locally but remote reports {} bytes; \
             the file may not have been transferred (silent exec no-op?)",
            local_len,
            remote_size,
        ));
    }

    let remote_sha = parse_remote_sha256_output(remote_sha_raw)?;
    if remote_sha != local_sha {
        return Err(anyhow::anyhow!(
            "post-injection sha256 mismatch: expected {} but remote reports {}; \
             the transferred file is corrupt or incomplete",
            local_sha,
            remote_sha,
        ));
    }

    Ok(())
}

/// Normalize a Docker image reference into the fully-qualified,
/// registry-host-prefixed form that containerd's CRI plugin uses as its
/// exact lookup key.
///
/// `ctr images import` tags the imported image under whatever short name
/// was embedded in the source tar. If kubelet later requests the image by
/// its canonical reference (Docker's own normalization — the form
/// `docker.io/library/<name>:<tag>` for an unqualified short name), the CRI
/// plugin's exact-key lookup misses even though the image is present,
/// surfacing as `ImagePullBackOff`. This function computes that canonical
/// form so the caller can `ctr images tag` the import under the name
/// kubelet will actually ask for.
///
/// Rules (mirrors Docker reference normalization):
/// - A reference already has a registry host when its first `/`-separated
///   component contains `.` or `:`, or is exactly `localhost` — left as-is,
///   only gaining a `:latest` suffix if it has no tag.
/// - Otherwise it's a Docker Hub short name: prefix `docker.io/`, adding
///   `library/` too when there's no `/` at all (the official-images
///   namespace). A `:latest` suffix is appended if no tag is present.
#[allow(dead_code)]
pub fn normalize_image_ref(image: &str) -> String {
    let mut components = image.splitn(2, '/');
    let first = components.next().unwrap_or("");
    let rest = components.next();

    // A bare single-segment reference (no `/` at all) can never carry a
    // registry host — any `:` present there is a tag colon, not a
    // host:port colon, so `rest.is_some()` gates the host check.
    let has_host =
        rest.is_some() && (first.contains('.') || first.contains(':') || first == "localhost");

    if has_host {
        return ensure_tag(image);
    }

    let qualified = match rest {
        Some(_) => image.to_string(),
        None => format!("library/{image}"),
    };
    ensure_tag(&format!("docker.io/{qualified}"))
}

/// Append `:latest` to `reference` if its last path segment carries no tag.
/// The tag colon is looked for after the final `/` so a host:port colon
/// earlier in the reference is never mistaken for one.
fn ensure_tag(reference: &str) -> String {
    let last_segment = reference.rsplit('/').next().unwrap_or(reference);
    if last_segment.contains(':') {
        reference.to_string()
    } else {
        format!("{reference}:latest")
    }
}

/// Client for interacting with Apple Container
pub struct AppleContainerClient {
    config: Config,
    cli_path: String,
    container_version: String,
}

impl AppleContainerClient {
    /// Create a new Apple Container client
    pub fn new(config: &Config) -> Result<Self> {
        let cli_path = if let Some(path) = &config.apple_container.cli_path {
            path.to_string_lossy().to_string()
        } else {
            // Try to detect Apple Container CLI in PATH
            Self::detect_cli_path()?
        };

        let container_version = Self::detect_version(&cli_path)?;
        validate_version(&container_version)?;

        Ok(Self {
            config: config.clone(),
            cli_path,
            container_version,
        })
    }

    /// Get the detected Apple Container CLI version
    pub fn version(&self) -> &str {
        &self.container_version
    }

    /// Detect the Apple Container CLI version by running `<cli_path> --version`
    fn detect_version(cli_path: &str) -> Result<String> {
        let output = std::process::Command::new(cli_path)
            .arg("--version")
            .output()
            .context("Failed to execute Apple Container CLI for version detection")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Apple Container CLI version check failed: {}",
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_version_output(stdout.trim())
    }

    /// Detect Apple Container CLI path.
    ///
    /// Delegates to `cli_path_candidates()` to enforce the PATH-first ordering
    /// contract: brew/nix/mise-managed binaries resolve before any stale
    /// package-installer binary that may remain at /usr/local/bin/container.
    fn detect_cli_path() -> Result<String> {
        for strategy in cli_path_candidates() {
            match strategy {
                CliPathStrategy::Which(name) => {
                    if let Ok(output) = std::process::Command::new("which").arg(&name).output() {
                        if output.status.success() {
                            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if !path.is_empty() {
                                info!("Found Apple Container CLI via PATH: {}", path);
                                return Ok(path);
                            }
                        }
                    }
                }
                CliPathStrategy::Hardcoded(path) => {
                    if std::path::Path::new(&path).exists() {
                        info!("Found Apple Container CLI at hardcoded path: {}", path);
                        return Ok(path);
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Apple Container CLI not found. Please install Apple Container or specify the path in configuration."
        ))
    }

    /// Create a cluster using Apple Container
    pub async fn create_cluster(&self, options: &CreateClusterOptions) -> Result<()> {
        info!(
            "Creating cluster '{}' with image '{}'",
            options.name, options.image
        );

        let worker_count = options.workers.unwrap_or(0);
        // Resolve the effective CNI plugin: CLI flag overrides config default.
        let cni = select_cni(
            Some(options.cni_plugin.clone()),
            self.config.cluster.default_cni.clone(),
        );

        if worker_count == 0 {
            // Single-node cluster with combined control-plane/worker roles
            let node_name = format!("{}-control-plane", options.name);
            info!(
                "Creating single-node cluster with combined roles: {}",
                node_name
            );
            self.create_single_node(
                &options.name,
                &node_name,
                &options.image,
                cni,
                options.node_kernel_path.as_deref(),
                options.control_plane_cpus,
                &options.control_plane_memory,
            )
            .await?;
        } else {
            // Multi-node cluster: 1 control-plane + N workers
            info!(
                "Creating multi-node cluster with 1 control-plane + {} workers",
                worker_count
            );
            self.create_multi_node_cluster(options, worker_count)
                .await?;
        }

        info!("Cluster '{}' created successfully", options.name);
        Ok(())
    }

    /// Create a multi-node cluster with separate control-plane and worker nodes
    async fn create_multi_node_cluster(
        &self,
        options: &CreateClusterOptions,
        worker_count: u32,
    ) -> Result<()> {
        let cp_name = format!("{}-control-plane", options.name);

        // Resolve the effective CNI plugin: CLI flag (options.cni_plugin) overrides config default.
        let cni = select_cni(
            Some(options.cni_plugin.clone()),
            self.config.cluster.default_cni.clone(),
        );

        // 1. Create control-plane container
        self.create_control_plane_node(
            &options.name,
            &cp_name,
            &options.image,
            true,
            options.node_kernel_path.as_deref(),
            options.control_plane_cpus,
            &options.control_plane_memory,
        )
        .await?;

        // 2. Wait for control-plane container to be ready and get IP
        self.wait_for_container_ready(&cp_name).await?;
        let cp_ip = self.get_container_ip(&cp_name).await?;
        info!("Control-plane '{}' running at IP: {}", cp_name, cp_ip);

        // 3. Initialize Kubernetes on control-plane and get join info
        let join_info = self
            .initialize_kubernetes_cluster_with_join_info(
                &cp_name,
                &cp_ip,
                &options.name,
                options.node_kernel_path.as_deref(),
            )
            .await?;

        // 4. Setup kubeconfig early (user gets kubectl access even if workers fail)
        self.setup_kubeconfig(&options.name, &cp_name, &cp_ip)
            .await?;

        // 5. Install CNI on control-plane (must be before workers join)
        // Pass kernel_path so Cilium selects the full-eBPF or stock workaround profile.
        self.install_cni_plugin(&cp_name, cni.clone(), options.node_kernel_path.as_deref())
            .await?;

        // Track every node and its VM IP so PTP cross-node routing can be set up
        // once all workers have joined and been assigned pod CIDRs.
        let mut all_nodes: Vec<(String, String)> = vec![(cp_name.clone(), cp_ip.clone())];

        // 6. Create and join worker nodes sequentially
        for i in 0..worker_count {
            // KIND convention: first worker is {name}-worker, subsequent are {name}-worker-N
            let worker_name = if i == 0 {
                format!("{}-worker", options.name)
            } else {
                format!("{}-worker-{}", options.name, i + 1)
            };

            info!(
                "Creating worker node {}/{}: {}",
                i + 1,
                worker_count,
                worker_name
            );

            self.create_worker_node(
                &options.name,
                &worker_name,
                &options.image,
                options.node_kernel_path.as_deref(),
                options.worker_cpus,
                &options.worker_memory,
            )
            .await?;

            self.wait_for_container_ready(&worker_name).await?;
            let worker_ip = self.get_container_ip(&worker_name).await?;
            info!("Worker '{}' running at IP: {}", worker_name, worker_ip);

            self.join_worker_node(&worker_name, &worker_ip, &join_info)
                .await?;

            // PTP CNI requires the config file on each node (it's not a DaemonSet).
            // Cilium deploys as a DaemonSet from the control-plane and auto-rolls to workers.
            // Use the resolved cni (via select_cni) so --cni flag is honoured per-worker too.
            if matches!(cni, CniPlugin::Ptp) {
                self.install_ptp_cni(&worker_name).await?;
            }

            all_nodes.push((worker_name.clone(), worker_ip.clone()));
        }

        // For PTP, reconfigure each node with its per-node pod CIDR and add cross-node
        // routes so pods on workers can reach CoreDNS (and other cross-node pods). This
        // must run after all workers have joined so every node has an assigned podCIDR.
        // A routing failure is non-fatal: the cluster is usable for same-node workloads.
        if matches!(cni, CniPlugin::Ptp) && all_nodes.len() > 1 {
            if let Err(e) = self
                .configure_ptp_cross_node_routing(&cp_name, &all_nodes)
                .await
            {
                warn!(
                    "PTP cross-node routing setup failed (DNS may not work on workers): {}",
                    e
                );
            }
        }

        // After all workers have joined, re-run the Cilium readiness gate and then
        // wait for all nodes to be Ready before reporting success.
        if matches!(cni, CniPlugin::Cilium) {
            // Issue #43: worker kubelets submit their serving CSRs only after they join,
            // so the approval during control-plane Cilium install does not cover them.
            // Re-approve now so `cilium status` can exec into the worker cilium pods;
            // otherwise the gate fails with "tls: internal error" on those pods.
            self.approve_pending_kubelet_csrs(&cp_name);
            info!("Re-running Cilium readiness gate after all workers joined");
            let readiness_cmd =
                "KUBECONFIG=/etc/kubernetes/admin.conf cilium status --wait --wait-duration 5m";
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["exec", &cp_name, "sh", "-c", readiness_cmd]);
            let output = cmd
                .output()
                .context("Failed to run post-join Cilium readiness gate")?;
            if !output.status.success() {
                let diag = self.collect_cilium_diagnostics(&cp_name);
                return Err(anyhow::anyhow!(
                    "Cilium post-join readiness gate failed: {}\nDiagnostics:\n{}",
                    String::from_utf8_lossy(&output.stderr),
                    diag
                ));
            }
        }

        // Final gate: wait for all nodes to be Ready before reporting success.
        info!("Waiting for all nodes to be Ready");
        let node_wait_cmd =
            "kubectl wait --for=condition=Ready node --all --timeout=300s --kubeconfig=/etc/kubernetes/admin.conf";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", &cp_name, "sh", "-c", node_wait_cmd]);
        let output = cmd.output().context("Failed to run node readiness gate")?;
        if !output.status.success() {
            warn!(
                "Node readiness gate timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Note: Do NOT remove control-plane taint in multi-node mode.
        // Workers handle workloads; control-plane stays dedicated.

        info!(
            "Multi-node Kubernetes cluster '{}' initialized successfully",
            options.name
        );
        Ok(())
    }

    /// Collect Cilium diagnostics for error reporting on install/readiness failure.
    ///
    /// Runs three diagnostic commands inside the container and returns their output
    /// concatenated for inclusion in the error message:
    /// - `cilium status` (bare, no --wait — captures current state snapshot)
    /// - `kubectl -n kube-system get pods -o wide`
    /// - `kubectl -n kube-system logs ds/cilium --tail=100`
    fn collect_cilium_diagnostics(&self, container_name: &str) -> String {
        let mut diag = String::new();

        // 1. Bare cilium status (no --wait) for current state snapshot.
        let status_cmd = "KUBECONFIG=/etc/kubernetes/admin.conf cilium status 2>&1 || true";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", status_cmd]);
        if let Ok(out) = cmd.output() {
            diag.push_str("=== cilium status ===\n");
            diag.push_str(&String::from_utf8_lossy(&out.stdout));
            diag.push_str(&String::from_utf8_lossy(&out.stderr));
            diag.push('\n');
        }

        // 2. Pod list for kube-system namespace.
        let pods_cmd =
            "KUBECONFIG=/etc/kubernetes/admin.conf kubectl -n kube-system get pods -o wide 2>&1 || true";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", pods_cmd]);
        if let Ok(out) = cmd.output() {
            diag.push_str("=== kubectl -n kube-system get pods -o wide ===\n");
            diag.push_str(&String::from_utf8_lossy(&out.stdout));
            diag.push_str(&String::from_utf8_lossy(&out.stderr));
            diag.push('\n');
        }

        // 3. Cilium DaemonSet logs.
        let logs_cmd =
            "KUBECONFIG=/etc/kubernetes/admin.conf kubectl -n kube-system logs ds/cilium --tail=100 2>&1 || true";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", logs_cmd]);
        if let Ok(out) = cmd.output() {
            diag.push_str("=== kubectl -n kube-system logs ds/cilium --tail=100 ===\n");
            diag.push_str(&String::from_utf8_lossy(&out.stdout));
            diag.push_str(&String::from_utf8_lossy(&out.stderr));
            diag.push('\n');
        }

        diag
    }

    /// Create a single node with combined control-plane and worker roles
    /// Note: Required due to Apple Container VM communication limitation until macOS 26
    #[allow(clippy::too_many_arguments)]
    async fn create_single_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        cni: CniPlugin,
        kernel_path: Option<&std::path::Path>,
        cpus: u32,
        memory: &str,
    ) -> Result<()> {
        info!("Creating single Kubernetes node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Create container with appropriate labels for single-node cluster
        let mut cmd = std::process::Command::new(&self.cli_path);

        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=control-plane,worker", // Combined roles
            "--label",
            "io.kina.primary=true",
            "--label",
            "io.kina.single-node=true",
            "--label",
            &image_label,
        ];

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Add required capabilities for Kubernetes node workloads.
        // Apple Container has no privileged-mode flag; since 0.12.0 the default cap set
        // is insufficient for systemd, kubeadm, kubelet, containerd, and Cilium eBPF.
        args.extend_from_slice(&node_cap_args());

        // Resource allocation: resolved CPUs and memory per node.
        // The Apple Container default (4 vCPUs / 1024 MB) is insufficient for a full
        // kube-system stack (etcd + apiserver + KCM + scheduler + Cilium agent +
        // cilium-operator + Envoy DaemonSet + Hubble). OOM kills cascade into
        // control-plane component crashes that look like TLS / leader-election failures.
        // 4 GB is the minimum for a stable full-eBPF Cilium cluster.
        let res_owned = node_resource_args(cpus, memory);
        let res_refs: Vec<&str> = res_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&res_refs);

        // Inject custom kernel args when a kernel path is configured.
        // node_kernel_args returns ["--kernel", "<path>"] or [] for stock kernel.
        let kernel_args_owned = node_kernel_args(kernel_path);
        let kernel_args_refs: Vec<&str> = kernel_args_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&kernel_args_refs);

        // Note: No port mapping needed - Apple Container VM gets its own IP
        // Kubernetes API server will be accessible at <vm-ip>:6443
        // Ingress controllers will be accessible at <vm-ip>:80, <vm-ip>:443
        // Services can be reached directly at VM IP address

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            "--env",
            "KINA_NODE_TYPE=single-node",
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        // Debug: Print the exact command being executed
        debug!(
            "Executing Apple Container command: {} {:?}",
            self.cli_path, args
        );

        let output = cmd
            .output()
            .context("Failed to create single-node cluster")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create single-node cluster '{}': {}",
                node_name,
                stderr
            ));
        }

        info!("Container '{}' created, waiting for startup...", node_name);

        // Wait for container to be fully running
        self.wait_for_container_ready(node_name).await?;

        // Get the VM IP address
        let vm_ip = self.get_container_ip(node_name).await?;
        info!("Container '{}' running at IP: {}", node_name, vm_ip);

        // Initialize Kubernetes cluster (kernel_path determines full-eBPF vs stock kubeadm profile)
        self.initialize_kubernetes_cluster(node_name, &vm_ip, kernel_path)
            .await?;

        // Generate and save kubeconfig immediately after cluster init
        // This ensures user has kubectl access even if CNI installation fails
        self.setup_kubeconfig(cluster_name, node_name, &vm_ip)
            .await?;

        // Remove control-plane taint for single-node scheduling
        self.remove_control_plane_taint(node_name).await?;

        // Install CNI plugin (now user has kubectl access if this fails)
        // Use the resolved CNI plugin (CLI flag overrides config default).
        // Pass kernel_path so Cilium selects the full-eBPF or stock workaround profile.
        self.install_cni_plugin(node_name, cni, kernel_path).await?;

        info!(
            "Kubernetes cluster '{}' initialized successfully",
            cluster_name
        );
        Ok(())
    }

    /// Create a control plane node
    #[allow(clippy::too_many_arguments)]
    async fn create_control_plane_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        is_primary: bool,
        kernel_path: Option<&std::path::Path>,
        cpus: u32,
        memory: &str,
    ) -> Result<()> {
        info!("Creating control plane node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Create container with appropriate labels and configuration
        // Apple Container automatically assigns VM and IP address - no explicit network needed
        let mut cmd = std::process::Command::new(&self.cli_path);

        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=control-plane",
            "--label",
            &image_label,
        ];

        if is_primary {
            args.extend_from_slice(&["--label", "io.kina.primary=true"]);
        }

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Add required capabilities for Kubernetes node workloads.
        // Apple Container has no privileged-mode flag; since 0.12.0 the default cap set
        // is insufficient for systemd, kubeadm, kubelet, containerd, and Cilium eBPF.
        args.extend_from_slice(&node_cap_args());

        // Resource allocation: resolved CPUs and memory per node.
        // The Apple Container default (4 vCPUs / 1024 MB) is insufficient for a full
        // kube-system stack. See single-node creation comment for rationale.
        let cp_res_owned = node_resource_args(cpus, memory);
        let cp_res_refs: Vec<&str> = cp_res_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&cp_res_refs);

        // Inject custom kernel args when a kernel path is configured.
        let cp_kernel_args_owned = node_kernel_args(kernel_path);
        let cp_kernel_args_refs: Vec<&str> =
            cp_kernel_args_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&cp_kernel_args_refs);

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            "--env",
            "KINA_NODE_TYPE=control-plane",
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        let output = cmd
            .output()
            .context("Failed to create control plane node")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create control plane node '{}': {}",
                node_name,
                stderr
            ));
        }

        debug!("Created control plane node '{}'", node_name);
        Ok(())
    }

    /// Create a worker node
    async fn create_worker_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        kernel_path: Option<&std::path::Path>,
        cpus: u32,
        memory: &str,
    ) -> Result<()> {
        info!("Creating worker node '{}'", node_name);

        let cluster_label = format!("io.kina.cluster={}", cluster_name);
        let image_label = format!("io.kina.image={}", image);

        // Apple Container automatically assigns VM and IP address - no explicit network needed
        let mut cmd = std::process::Command::new(&self.cli_path);
        let mut args = vec![
            "run",
            "-d", // Run in detached mode
            "--name",
            node_name,
            "--label",
            &cluster_label,
            "--label",
            "io.kina.role=worker",
            "--label",
            &image_label,
        ];

        // Add tmpfs mounts for systemd in VM
        args.extend_from_slice(&["--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/run/lock"]);

        // Add required capabilities for Kubernetes node workloads.
        // Apple Container has no privileged-mode flag; since 0.12.0 the default cap set
        // is insufficient for systemd, kubeadm, kubelet, containerd, and Cilium eBPF.
        args.extend_from_slice(&node_cap_args());

        // Resource allocation: resolved CPUs and memory per node.
        // The Apple Container default (4 vCPUs / 1024 MB) is insufficient for a full
        // kube-system stack. See single-node creation comment for rationale.
        let wk_res_owned = node_resource_args(cpus, memory);
        let wk_res_refs: Vec<&str> = wk_res_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&wk_res_refs);

        // Inject custom kernel args when a kernel path is configured.
        let wk_kernel_args_owned = node_kernel_args(kernel_path);
        let wk_kernel_args_refs: Vec<&str> =
            wk_kernel_args_owned.iter().map(|s| s.as_str()).collect();
        args.extend_from_slice(&wk_kernel_args_refs);

        // Set up environment for containerized systemd in VM
        let hostname_env = format!("HOSTNAME={}", node_name);
        args.extend_from_slice(&[
            "--env",
            "container=docker",
            "--env",
            &hostname_env,
            "--env",
            "KINA_NODE_TYPE=worker",
            image,
            "/sbin/init", // Start systemd in VM
        ]);

        cmd.args(&args);

        let output = cmd.output().context("Failed to create worker node")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to create worker node '{}': {}",
                node_name,
                stderr
            ));
        }

        debug!("Created worker node '{}'", node_name);
        Ok(())
    }

    /// Delete a cluster
    pub async fn delete_cluster(&self, name: &str) -> Result<()> {
        info!("Deleting cluster '{}'", name);

        // Find all containers belonging to this cluster
        let clusters = self.list_clusters().await?;
        let cluster = clusters.iter().find(|c| c.name == name);

        if let Some(cluster) = cluster {
            // Delete all containers in the cluster
            for node in &cluster.nodes {
                if let Some(container_id) = &node.container_id {
                    self.delete_container(container_id).await?;
                }
            }

            // Remove kubeconfig context
            self.remove_kubeconfig_context(name).await?;

            // Note: No explicit network cleanup needed for Apple Container
            // VM-per-container architecture handles networking automatically
        } else {
            warn!("Cluster '{}' not found", name);
        }

        info!("Cluster '{}' deleted successfully", name);
        Ok(())
    }

    /// Delete a container
    async fn delete_container(&self, container_id: &str) -> Result<()> {
        debug!("Deleting container '{}'", container_id);

        // Try graceful stop, then force stop with SIGKILL
        let stop_attempts = [
            ("stop", None, None),                  // Graceful stop (SIGTERM, 5s default)
            ("stop", Some("SIGKILL"), Some("10")), // Force stop with SIGKILL and 10s timeout
        ];

        for (attempt, (cmd_name, signal, timeout)) in stop_attempts.iter().enumerate() {
            let signal_desc = signal.map(|s| format!(" with {}", s)).unwrap_or_default();
            info!(
                "Attempting to {} container '{}'{} (attempt {})",
                cmd_name,
                container_id,
                signal_desc,
                attempt + 1
            );

            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.arg(cmd_name);

            if let Some(sig) = signal {
                cmd.args(["--signal", sig]);
            }
            if let Some(time) = timeout {
                cmd.args(["--time", time]);
            }

            cmd.arg(container_id);

            let output = cmd
                .output()
                .context("Failed to execute container command")?;

            if output.status.success() {
                let stop_desc = if let Some(sig) = signal {
                    format!("{} ({})", cmd_name, sig)
                } else {
                    cmd_name.to_string()
                };
                info!("Successfully {} container '{}'", stop_desc, container_id);

                // Wait for container to fully stop - increased timeout for Apple Container VMs
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                break;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("is not running")
                    || stderr.contains("No such container")
                    || stderr.contains("cannot kill: container is not running")
                {
                    debug!(
                        "Container '{}' already stopped or doesn't exist",
                        container_id
                    );
                    break;
                } else if attempt == 0 {
                    warn!(
                        "Graceful stop failed for '{}': {}, trying force stop",
                        container_id, stderr
                    );
                    continue;
                } else {
                    warn!(
                        "All stop attempts failed for '{}': {}, proceeding with removal anyway",
                        container_id, stderr
                    );
                    break; // Try removal anyway - Apple Container might have state sync issues
                }
            }
        }

        // Remove the container - try normal removal first, then force if needed
        for (attempt, use_force) in [false, true].iter().enumerate() {
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.arg("delete"); // Use 'delete' not 'rm'

            if *use_force {
                cmd.arg("--force");
                info!("Attempting force removal of container '{}'", container_id);
            } else {
                info!("Attempting removal of container '{}'", container_id);
            }

            cmd.arg(container_id);

            let output = cmd.output().context("Failed to remove container")?;

            if output.status.success() {
                info!("Successfully removed container '{}'", container_id);
                break;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("No such container") {
                    debug!("Container '{}' already removed", container_id);
                    break;
                } else if attempt == 0 && stderr.contains("container is running") {
                    warn!("Normal removal failed (container still running), trying force removal");
                    continue;
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to remove container '{}': {}",
                        container_id,
                        stderr
                    ));
                }
            }
        }

        info!("Successfully deleted container '{}'", container_id);
        Ok(())
    }

    /// List clusters
    pub async fn list_clusters(&self) -> Result<Vec<ClusterInfo>> {
        debug!("Listing clusters from Apple Container");

        // Run 'container list' to list all containers and filter by kina labels
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["list", "--format", "json", "--all"]);

        let output = cmd
            .output()
            .context("Failed to execute Apple Container CLI")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Apple Container CLI failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Apple Container ps output: {}", stdout);

        // Parse JSON output using the 1.0.0 shape helper
        let parsed =
            parse_container_list(&stdout).context("Failed to parse Apple Container JSON output")?;

        let mut clusters: HashMap<String, ClusterInfo> = HashMap::new();

        for container in parsed {
            // Only include containers managed by kina (must have io.kina.cluster label)
            if let Some(cluster_name) = container.labels.get("io.kina.cluster") {
                let role = container
                    .labels
                    .get("io.kina.role")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");

                let image = container
                    .labels
                    .get("io.kina.image")
                    .map(|s| s.as_str())
                    .unwrap_or("kindest/node:latest");

                let state = &container.state;
                let ip_address = container.ipv4.clone();
                let container_name = container.id.clone();

                // Group containers by cluster name
                let cluster_info =
                    clusters
                        .entry(cluster_name.clone())
                        .or_insert_with(|| ClusterInfo {
                            name: cluster_name.clone(),
                            image: image.to_string(),
                            status: if state == "running" {
                                ClusterStatus::Running
                            } else {
                                ClusterStatus::Stopped
                            },
                            nodes: Vec::new(),
                            created: container
                                .created
                                .as_deref()
                                .map(format_created)
                                .unwrap_or_else(|| "unknown".to_string()),
                            kubeconfig_path: None,
                        });

                // Add node information
                cluster_info.nodes.push(NodeInfo {
                    name: container_name.clone(),
                    role: if role.contains("control-plane") {
                        NodeRole::ControlPlane
                    } else {
                        NodeRole::Worker
                    },
                    status: state.clone(),
                    version: "unknown".to_string(),
                    container_id: Some(container_name),
                    ip_address,
                });

                // Update cluster status based on all containers
                if state != "running" {
                    cluster_info.status = ClusterStatus::Stopped;
                }
            }
        }

        let result: Vec<ClusterInfo> = clusters.into_values().collect();
        debug!("Found {} kina clusters", result.len());

        Ok(result)
    }

    /// Get kubeconfig for a cluster
    pub async fn get_kubeconfig(&self, name: &str) -> Result<String> {
        debug!("Getting kubeconfig for cluster '{}'", name);

        // First try to read from standard kubectl location: ~/.kube/<cluster-name>
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir).join(".kube").join(name);

        if kubeconfig_path.exists() {
            return fs::read_to_string(kubeconfig_path).context("Failed to read kubeconfig file");
        }

        // If no local kubeconfig, try to generate one from the cluster
        self.generate_kubeconfig(name).await
    }

    /// Generate kubeconfig from a running cluster
    /// Note: Apple Container provides DNS resolution for container names,
    /// so kubeconfig can use container hostnames directly instead of IP addresses
    async fn generate_kubeconfig(&self, name: &str) -> Result<String> {
        info!("Generating kubeconfig for cluster '{}'", name);

        // Find the primary control plane node
        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' not found", name))?;

        let control_plane_node = cluster
            .nodes
            .iter()
            .find(|n| n.role == NodeRole::ControlPlane)
            .ok_or_else(|| anyhow::anyhow!("No control plane node found for cluster '{}'", name))?;

        if let Some(container_id) = &control_plane_node.container_id {
            // Try to execute kubectl config view in the control plane container
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["exec", container_id, "kubectl", "config", "view", "--raw"]);

            let output = cmd
                .output()
                .context("Failed to get kubeconfig from cluster")?;

            if output.status.success() {
                let kubeconfig_raw = String::from_utf8_lossy(&output.stdout).to_string();

                // Rewrite server URL to the live VM IP (pure fn, idempotent, handles any host).
                let kubeconfig = if let Some(vm_ip) = &control_plane_node.ip_address {
                    info!("Updating kubeconfig server URL to use VM IP: {}", vm_ip);
                    crate::core::verify::rewrite_kubeconfig_server(&kubeconfig_raw, vm_ip)
                } else {
                    kubeconfig_raw
                };

                // Make user names cluster-specific to prevent conflicts in merged config
                let kubeconfig = self.make_user_names_cluster_specific(&kubeconfig, name)?;

                // Save the kubeconfig locally for future use
                self.save_kubeconfig(name, &kubeconfig).await?;

                return Ok(kubeconfig);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to get kubeconfig from cluster: {}", stderr);
            }
        }

        Err(anyhow::anyhow!(
            "Could not retrieve kubeconfig for cluster '{}'. The cluster may not be fully initialized.",
            name
        ))
    }

    /// Make user names cluster-specific to prevent conflicts in merged kubeconfig
    fn make_user_names_cluster_specific(
        &self,
        kubeconfig: &str,
        cluster_name: &str,
    ) -> Result<String> {
        let mut config: serde_yaml::Value =
            serde_yaml::from_str(kubeconfig).context("Failed to parse kubeconfig")?;

        // Create cluster-specific user name
        let cluster_specific_user = format!("{}-admin", cluster_name);

        // Update user name in users section
        if let Some(users) = config.get_mut("users").and_then(|u| u.as_sequence_mut()) {
            for user in users.iter_mut() {
                if let Some(name) = user.get("name").and_then(|n| n.as_str()) {
                    if name == "kubernetes-admin" {
                        user["name"] = serde_yaml::Value::String(cluster_specific_user.clone());
                    }
                }
            }
        }

        // Update user reference in contexts section
        if let Some(contexts) = config.get_mut("contexts").and_then(|c| c.as_sequence_mut()) {
            for context in contexts.iter_mut() {
                if let Some(context_obj) = context.get_mut("context") {
                    if let Some(user) = context_obj.get("user").and_then(|u| u.as_str()) {
                        if user == "kubernetes-admin" {
                            context_obj["user"] =
                                serde_yaml::Value::String(cluster_specific_user.clone());
                        }
                    }
                }
            }
        }

        let updated_kubeconfig =
            serde_yaml::to_string(&config).context("Failed to serialize updated kubeconfig")?;

        Ok(updated_kubeconfig)
    }

    /// Save kubeconfig using kubectl-native merging for proper context management
    async fn save_kubeconfig(&self, name: &str, kubeconfig: &str) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = std::path::Path::new(&home_dir).join(".kube");
        let global_config_path = kube_dir.join("config");
        let individual_path = kube_dir.join(name);

        // Ensure the .kube directory exists
        fs::create_dir_all(&kube_dir).context("Failed to create .kube directory")?;

        // Save individual file for backward compatibility and direct access
        fs::write(&individual_path, kubeconfig)
            .context("Failed to write individual kubeconfig file")?;

        // Use kubectl config merge approach by setting KUBECONFIG environment variable
        // This leverages kubectl's built-in merging logic which is much more reliable
        let existing_kubeconfig = if global_config_path.exists() {
            format!(
                "{}:{}",
                global_config_path.display(),
                individual_path.display()
            )
        } else {
            individual_path.display().to_string()
        };

        // Use kubectl config view --flatten to merge configs
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.env("KUBECONFIG", &existing_kubeconfig)
            .arg("config")
            .arg("view")
            .arg("--flatten");

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config view")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("kubectl config view failed: {}", stderr));
        }

        // Write the merged config back to the global config file
        let merged_config = String::from_utf8_lossy(&output.stdout);
        fs::write(&global_config_path, merged_config.as_bytes())
            .context("Failed to write merged kubeconfig")?;

        // Set the current context to the new cluster
        let context_name = name; // Use cluster name as context name
        let mut use_context_cmd = tokio::process::Command::new("kubectl");
        use_context_cmd
            .arg("config")
            .arg("use-context")
            .arg(context_name);

        let use_output = use_context_cmd
            .output()
            .await
            .context("Failed to set current context")?;
        if !use_output.status.success() {
            let stderr = String::from_utf8_lossy(&use_output.stderr);
            warn!(
                "Failed to set current context to '{}': {}",
                context_name, stderr
            );
        }

        info!(
            "Saved kubeconfig to: {} and merged into: {} using kubectl",
            individual_path.display(),
            global_config_path.display()
        );
        Ok(())
    }

    /// Remove kubeconfig context for deleted cluster using kubectl commands
    async fn remove_kubeconfig_context(&self, cluster_name: &str) -> Result<()> {
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = std::path::Path::new(&home_dir).join(".kube");
        let individual_path = kube_dir.join(cluster_name);

        // Remove individual kubeconfig file
        if individual_path.exists() {
            fs::remove_file(&individual_path)
                .context("Failed to remove individual kubeconfig file")?;
            info!(
                "Removed individual kubeconfig: {}",
                individual_path.display()
            );
        }

        // Use kubectl to remove cluster components
        let user_name = format!("{}-admin", cluster_name);

        // Remove context (ignore errors if context doesn't exist)
        let _ = self.kubectl_delete_context(cluster_name).await;

        // Remove cluster (ignore errors if cluster doesn't exist)
        let _ = self.kubectl_delete_cluster(cluster_name).await;

        // Remove user (ignore errors if user doesn't exist)
        let _ = self.kubectl_delete_user(&user_name).await;

        info!(
            "Removed cluster '{}' from kubeconfig using kubectl commands",
            cluster_name
        );
        Ok(())
    }

    /// Delete context using kubectl config delete-context
    async fn kubectl_delete_context(&self, context_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-context").arg(context_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-context")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-context failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Delete cluster using kubectl config delete-cluster
    async fn kubectl_delete_cluster(&self, cluster_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-cluster").arg(cluster_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-cluster")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-cluster failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Delete user using kubectl config delete-user
    async fn kubectl_delete_user(&self, user_name: &str) -> Result<()> {
        let mut cmd = tokio::process::Command::new("kubectl");
        cmd.arg("config").arg("delete-user").arg(user_name);

        let output = cmd
            .output()
            .await
            .context("Failed to run kubectl config delete-user")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "kubectl config delete-user failed: {}",
                stderr
            ));
        }

        Ok(())
    }

    /// Load image into cluster
    pub async fn load_image(&self, options: &LoadImageOptions) -> Result<()> {
        info!(
            "Loading image '{}' into cluster '{}'",
            options.image, options.cluster
        );

        // Find all nodes in the cluster
        let clusters = self.list_clusters().await?;
        let cluster = clusters
            .iter()
            .find(|c| c.name == options.cluster)
            .ok_or_else(|| anyhow::anyhow!("Cluster '{}' not found", options.cluster))?;

        if cluster.nodes.is_empty() {
            return Err(anyhow::anyhow!(
                "No nodes found in cluster '{}'",
                options.cluster
            ));
        }

        // Load image into each node container
        for node in &cluster.nodes {
            if let Some(container_id) = &node.container_id {
                self.load_image_into_container(container_id, &options.image)
                    .await?;
            }
        }

        info!(
            "Image '{}' loaded successfully into cluster '{}'",
            options.image, options.cluster
        );
        Ok(())
    }

    /// Load image into a specific container
    async fn load_image_into_container(&self, container_id: &str, image: &str) -> Result<()> {
        debug!(
            "Loading image '{}' into container '{}'",
            image, container_id
        );

        // First, save the image to a tar file using Apple Container's native image save
        let temp_dir = std::env::temp_dir();
        let image_tar = temp_dir.join(format!("{}.tar", image.replace(['/', ':'], "_")));

        // Export the image using container image save
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["image", "save", image, "-o", &image_tar.to_string_lossy()]);

        let output = cmd
            .output()
            .context("Failed to export image with container image save")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to export image: {}", stderr));
        }

        // Read the tar bytes to compute local size and digest for post-injection
        // verification.  Reading into memory is acceptable for node images (~100–500 MB)
        // on an Apple Silicon dev machine; this also allows a single fs read to compute
        // both size and sha256 before streaming.
        let tar_bytes = fs::read(&image_tar)
            .with_context(|| format!("Failed to read image tar from {}", image_tar.display()));
        let tar_bytes = match tar_bytes {
            Ok(b) => b,
            Err(e) => {
                let _ = fs::remove_file(&image_tar);
                return Err(e);
            }
        };
        let local_len = tar_bytes.len() as u64;
        let local_sha = sha256_hex(&tar_bytes);

        // Inject tar bytes into the container via exec-stdin (`container exec -i
        // <id> sh -c 'cat > /path'`).  This replaces `container cp` which silently
        // exits 0 without transferring the file in some container 1.0.0 configurations.
        let dest_path = "/tmp/image.tar";
        let inject_args = build_inject_tar_args(container_id, dest_path);
        let spawn_result = std::process::Command::new(&self.cli_path)
            .args(&inject_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to spawn exec-stdin injection for container '{}'",
                    container_id
                )
            });
        let mut child = match spawn_result {
            Ok(c) => c,
            Err(e) => {
                let _ = fs::remove_file(&image_tar);
                return Err(e);
            }
        };

        // Write tar bytes to child stdin; drop the handle to close stdin so the
        // `cat >` shell command sees EOF and terminates.
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(&tar_bytes) {
                let _ = child.wait();
                let _ = fs::remove_file(&image_tar);
                return Err(anyhow::anyhow!(
                    "Failed to write tar bytes to container stdin: {}",
                    e
                ));
            }
        }
        // Drop tar_bytes after writing to free memory before waiting.
        drop(tar_bytes);

        let inject_status = child
            .wait()
            .context("Failed to wait for exec-stdin injection")?;
        if !inject_status.success() {
            let _ = fs::remove_file(&image_tar);
            return Err(anyhow::anyhow!(
                "exec-stdin injection into container '{}' failed (exit {:?})",
                container_id,
                inject_status.code(),
            ));
        }

        // Post-injection verification: compare remote size and sha256 against the
        // local tarball before running ctr images import.  A size mismatch of 0
        // (or any value ≠ local_len) indicates a silent no-op and aborts with a
        // hard error so the import step is never attempted on a corrupt/missing file.
        let size_args = build_remote_size_args(container_id, dest_path);
        let size_output = std::process::Command::new(&self.cli_path)
            .args(&size_args)
            .output()
            .context("Failed to run remote size check")?;
        let remote_size_raw = String::from_utf8_lossy(&size_output.stdout).into_owned();

        let sha_args = build_remote_sha256_args(container_id, dest_path);
        let sha_output = std::process::Command::new(&self.cli_path)
            .args(&sha_args)
            .output()
            .context("Failed to run remote sha256 check")?;
        let remote_sha_raw = String::from_utf8_lossy(&sha_output.stdout).into_owned();

        if let Err(e) = verify_injection(local_len, &local_sha, &remote_size_raw, &remote_sha_raw) {
            let _ = fs::remove_file(&image_tar);
            return Err(e.context(format!(
                "post-injection verification failed for container '{}'; \
                 the exec-stdin transfer may have produced an incomplete file",
                container_id,
            )));
        }

        // Import the image inside the container using ctr (containerd CLI),
        // into the `k8s.io` namespace — that's the namespace kubelet's CRI
        // plugin reads images from, not the default `ctr` namespace, so
        // importing without `-n k8s.io` leaves the image invisible to kubelet.
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_id,
            "ctr",
            "-n",
            "k8s.io",
            "images",
            "import",
            dest_path,
        ]);
        let output = cmd.output().context("Failed to load image in container")?;

        // Clean up the temp file on all paths.
        let _ = fs::remove_file(&image_tar);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to load image in container: {}",
                stderr
            ));
        }

        // containerd's CRI plugin looks up images by their exact,
        // registry-qualified key. `ctr images import` preserves whatever
        // short name was embedded in the tar, so if kubelet requests the
        // canonical form (e.g. `docker.io/library/alpine:latest` for a bare
        // `alpine`), the image is present in containerd but invisible to
        // that lookup — surfacing as ImagePullBackOff. Tag the imported
        // image under its canonical name so the CRI lookup succeeds.
        let normalized = normalize_image_ref(image);
        if normalized != image {
            let mut tag_cmd = std::process::Command::new(&self.cli_path);
            tag_cmd.args([
                "exec",
                container_id,
                "ctr",
                "-n",
                "k8s.io",
                "images",
                "tag",
                image,
                &normalized,
            ]);
            let tag_output = tag_cmd
                .output()
                .context("Failed to tag imported image under its registry-qualified name")?;
            if !tag_output.status.success() {
                let stderr = String::from_utf8_lossy(&tag_output.stderr);
                return Err(anyhow::anyhow!(
                    "Failed to tag image '{}' as '{}': {}",
                    image,
                    normalized,
                    stderr
                ));
            }
        }

        debug!(
            "Successfully loaded image '{}' into container '{}'",
            image, container_id
        );
        Ok(())
    }

    /// Wait for container to be ready
    async fn wait_for_container_ready(&self, container_name: &str) -> Result<()> {
        info!("Waiting for container '{}' to be ready...", container_name);

        for attempt in 1..=30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["list", "--format", "json"]);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Ok(containers) = parse_container_list(&stdout) {
                        for container in containers {
                            if container.id == container_name && container.state == "running" {
                                debug!(
                                    "Container '{}' is running after {} attempts",
                                    container_name, attempt
                                );
                                return Ok(());
                            }
                        }
                    }
                }
            }

            debug!(
                "Container '{}' not ready yet, attempt {}/30",
                container_name, attempt
            );
        }

        Err(anyhow::anyhow!(
            "Container '{}' failed to become ready within 60 seconds",
            container_name
        ))
    }

    /// Get container IP address
    async fn get_container_ip(&self, container_name: &str) -> Result<String> {
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["list", "--format", "json"]);

        let output = cmd
            .output()
            .context("Failed to get container information")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to list containers: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers =
            parse_container_list(&stdout).context("Failed to parse container list JSON")?;

        for container in containers {
            if container.id == container_name {
                if let Some(ip) = container.ipv4 {
                    return Ok(ip);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not find IP address for container '{}'",
            container_name
        ))
    }

    /// Generate kubeadm init configuration YAML (method wrapper for compatibility)
    fn generate_kubeadm_init_config(
        &self,
        container_name: &str,
        vm_ip: &str,
        cluster_name: &str,
    ) -> String {
        generate_kubeadm_init_config(container_name, vm_ip, cluster_name)
    }

    /// Write kubeadm config and run kubeadm init in a container.
    ///
    /// When `full_ebpf` is true (custom kernel + kubeProxyReplacement=true), passes
    /// `--skip-phases=preflight,addon/kube-proxy` so kubeadm does not deploy kube-proxy.
    /// When false (stock kernel), only `--skip-phases=preflight` is passed.
    ///
    /// Uses `build_kubeadm_init_args(full_ebpf)` to build the argument list.
    fn run_kubeadm_init(
        &self,
        container_name: &str,
        kubeadm_config: &str,
        full_ebpf: bool,
    ) -> Result<std::process::Output> {
        // Write kubeadm config to container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "sh",
            "-c",
            &format!("cat > /kind/kubeadm.conf << 'EOF'\n{}\nEOF", kubeadm_config),
        ]);

        let output = cmd.output().context("Failed to write kubeadm config")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to write kubeadm config: {}",
                stderr
            ));
        }

        // Initialize cluster with kubeadm using the computed arg list.
        // build_kubeadm_init_args handles the kube-proxy skip for full-eBPF mode.
        let kubeadm_args = build_kubeadm_init_args(full_ebpf);

        let mut cmd = std::process::Command::new(&self.cli_path);
        let mut exec_args = vec!["exec", container_name, "kubeadm", "init"];
        let arg_refs: Vec<&str> = kubeadm_args.iter().map(|s| s.as_str()).collect();
        exec_args.extend_from_slice(&arg_refs);
        cmd.args(&exec_args);

        info!("Running kubeadm init (this may take a few minutes)...");
        cmd.output().context("Failed to run kubeadm init")
    }

    /// Initialize Kubernetes cluster with kubeadm (single-node, no join info needed).
    ///
    /// `kernel_path`: when `Some`, the full-eBPF kubeadm profile is used (skip kube-proxy).
    async fn initialize_kubernetes_cluster(
        &self,
        container_name: &str,
        vm_ip: &str,
        kernel_path: Option<&std::path::Path>,
    ) -> Result<()> {
        info!(
            "Initializing Kubernetes cluster in container '{}'",
            container_name
        );

        // Extract cluster name from container name (e.g., "mycluster-control-plane" -> "mycluster")
        let cluster_name = container_name
            .strip_suffix("-control-plane")
            .unwrap_or(container_name);

        let kubeadm_config = self.generate_kubeadm_init_config(container_name, vm_ip, cluster_name);
        let output =
            self.run_kubeadm_init(container_name, &kubeadm_config, kernel_path.is_some())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "kubeadm init failed:\nStdout: {}\nStderr: {}",
                stdout,
                stderr
            ));
        }

        info!("Kubernetes cluster initialized successfully");
        Ok(())
    }

    /// Initialize Kubernetes cluster and extract join info for multi-node setup.
    ///
    /// `kernel_path`: when `Some`, the full-eBPF kubeadm profile is used (skip kube-proxy).
    async fn initialize_kubernetes_cluster_with_join_info(
        &self,
        container_name: &str,
        vm_ip: &str,
        cluster_name: &str,
        kernel_path: Option<&std::path::Path>,
    ) -> Result<KubeadmJoinInfo> {
        info!(
            "Initializing Kubernetes cluster in container '{}' (multi-node)",
            container_name
        );

        let kubeadm_config = self.generate_kubeadm_init_config(container_name, vm_ip, cluster_name);
        let output =
            self.run_kubeadm_init(container_name, &kubeadm_config, kernel_path.is_some())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "kubeadm init failed:\nStdout: {}\nStderr: {}",
                stdout,
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let join_info = Self::parse_kubeadm_join_info(&stdout, vm_ip)?;

        info!("Kubernetes cluster initialized, join token extracted for workers");
        Ok(join_info)
    }

    /// Parse kubeadm join info from kubeadm init output
    /// Looks for: kubeadm join <endpoint> --token <token> --discovery-token-ca-cert-hash <hash>
    pub fn parse_kubeadm_join_info(output: &str, cp_ip: &str) -> Result<KubeadmJoinInfo> {
        let mut token = None;
        let mut ca_cert_hash = None;

        // kubeadm outputs the join command across potentially multiple lines with backslash continuations
        // First, normalize the output by joining continuation lines
        let normalized = output.replace("\\\n", " ");

        for line in normalized.lines() {
            let trimmed = line.trim();
            if trimmed.contains("kubeadm join") {
                // Parse the join command line for token and hash
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "--token" {
                        if let Some(val) = parts.get(i + 1) {
                            token = Some(val.to_string());
                        }
                    }
                    if *part == "--discovery-token-ca-cert-hash" {
                        if let Some(val) = parts.get(i + 1) {
                            ca_cert_hash = Some(val.to_string());
                        }
                    }
                }
            }
        }

        let token = token.ok_or_else(|| {
            anyhow::anyhow!("Failed to extract join token from kubeadm init output")
        })?;
        let ca_cert_hash = ca_cert_hash.ok_or_else(|| {
            anyhow::anyhow!("Failed to extract CA cert hash from kubeadm init output")
        })?;

        Ok(KubeadmJoinInfo {
            token,
            ca_cert_hash,
            control_plane_endpoint: format!("{}:6443", cp_ip),
        })
    }

    /// Join a worker node to the cluster using kubeadm join
    async fn join_worker_node(
        &self,
        worker_name: &str,
        worker_ip: &str,
        join_info: &KubeadmJoinInfo,
    ) -> Result<()> {
        info!("Joining worker '{}' to cluster", worker_name);

        // Write a JoinConfiguration YAML to the worker (v1beta4, list form for kubeletExtraArgs)
        let join_config = generate_worker_join_config(worker_name, worker_ip, join_info);

        // Write join config to worker container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            worker_name,
            "sh",
            "-c",
            &format!(
                "mkdir -p /kind && cat > /kind/kubeadm-join.conf << 'EOF'\n{}\nEOF",
                join_config
            ),
        ]);

        let output = cmd
            .output()
            .context("Failed to write join config to worker")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to write join config to worker '{}': {}",
                worker_name,
                stderr
            ));
        }

        // Run kubeadm join
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            worker_name,
            "kubeadm",
            "join",
            "--config=/kind/kubeadm-join.conf",
            "--skip-phases=preflight",
            "--v=1",
        ]);

        info!("Running kubeadm join on worker '{}'...", worker_name);
        let output = cmd.output().context("Failed to run kubeadm join")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow::anyhow!(
                "kubeadm join failed on worker '{}':\nStdout: {}\nStderr: {}",
                worker_name,
                stdout,
                stderr
            ));
        }

        info!("Worker '{}' joined cluster successfully", worker_name);
        Ok(())
    }

    /// Remove control-plane taint for single-node scheduling
    async fn remove_control_plane_taint(&self, container_name: &str) -> Result<()> {
        info!("Removing control-plane taint for single-node scheduling");

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "kubectl",
            "--kubeconfig=/etc/kubernetes/admin.conf",
            "taint",
            "nodes",
            container_name,
            "node-role.kubernetes.io/control-plane:NoSchedule-",
        ]);

        let output = cmd
            .output()
            .context("Failed to remove control-plane taint")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Failed to remove control-plane taint (may already be removed): {}",
                stderr
            );
        } else {
            info!("Control-plane taint removed successfully");
        }

        Ok(())
    }

    /// Install CNI plugin based on the resolved plugin selection.
    ///
    /// `cni` is the result of `select_cni(options.cni_plugin, config.default_cni)` and
    /// reflects the effective choice: CLI flag overrides config default.
    ///
    /// `kernel_path` determines which Cilium profile to use: when `Some`, the full-eBPF
    /// profile (`build_cilium_install_cmd_ebpf`) is selected; when `None`, the stock
    /// workaround profile (`build_cilium_install_cmd`) is used.
    async fn install_cni_plugin(
        &self,
        container_name: &str,
        cni: CniPlugin,
        kernel_path: Option<&std::path::Path>,
    ) -> Result<()> {
        match cni {
            CniPlugin::Ptp => self.install_ptp_cni(container_name).await,
            CniPlugin::Cilium => self.install_cilium_cni(container_name, kernel_path).await,
        }
    }

    /// Install PTP CNI plugin optimized for Apple Container VMs.
    ///
    /// Single-node default uses the full `10.244.0.0/16`. Multi-node clusters
    /// re-run [`install_ptp_cni_with_subnet`] per node with that node's assigned
    /// pod CIDR (see [`configure_ptp_cross_node_routing`]).
    async fn install_ptp_cni(&self, container_name: &str) -> Result<()> {
        self.install_ptp_cni_with_subnet(container_name, "10.244.0.0/16")
            .await
    }

    /// Install the PTP CNI config on a node using a specific pod `subnet`.
    ///
    /// Stale host-local IPAM allocations are cleared first so the node starts
    /// fresh when the subnet changes (e.g. moving from the default `/16` to a
    /// per-node `/24`), then kubelet is restarted to pick up the new config.
    async fn install_ptp_cni_with_subnet(&self, container_name: &str, subnet: &str) -> Result<()> {
        info!(
            "Installing PTP CNI plugin on '{}' with subnet {}",
            container_name, subnet
        );

        // PTP CNI configuration that works with kata-containers kernel limitations.
        let ptp_config = format!(
            r#"{{
  "cniVersion": "0.4.0",
  "name": "ptp-net",
  "plugins": [
    {{
      "type": "ptp",
      "ipMasq": true,
      "ipam": {{
        "type": "host-local",
        "subnet": "{}",
        "routes": [
          {{ "dst": "0.0.0.0/0" }}
        ]
      }}
    }},
    {{
      "type": "portmap",
      "capabilities": {{
        "portMappings": true
      }}
    }}
  ]
}}"#,
            subnet
        );

        // Clear stale IPAM allocations so host-local starts fresh with the new subnet,
        // then write the CNI config.
        let install_cmd = format!(
            r#"mkdir -p /etc/cni/net.d && rm -rf /var/lib/cni/networks/ptp-net && cat > /etc/cni/net.d/10-ptp.conflist << 'EOF'
{}
EOF"#,
            ptp_config
        );

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", &install_cmd]);

        let output = cmd
            .output()
            .context("Failed to install PTP CNI configuration")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to install PTP CNI configuration: {}",
                stderr
            ));
        }

        // Restart kubelet to pick up CNI configuration
        let restart_cmd = "systemctl restart kubelet";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", restart_cmd]);

        let output = cmd.output().context("Failed to restart kubelet")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Kubelet restart returned non-zero: {}", stderr);
        }

        info!(
            "PTP CNI plugin installed on '{}' with subnet {}",
            container_name, subnet
        );
        Ok(())
    }

    /// Configure per-node PTP subnets and cross-node routes for a multi-node PTP cluster.
    ///
    /// PTP CNI is point-to-point: it only wires pods to their local node. Without this,
    /// every node allocates from the same `10.244.0.0/16` (IP conflicts) and pods on
    /// workers cannot reach CoreDNS on the control-plane (DNS failures). This:
    ///   1. Reads each node's assigned `podCIDR` from the Kubernetes API.
    ///   2. Rewrites each node's PTP config to use that node-specific `/24`.
    ///   3. Adds `ip route` entries so traffic to another node's pod subnet is
    ///      forwarded to that node's VM IP.
    async fn configure_ptp_cross_node_routing(
        &self,
        cp_name: &str,
        nodes: &[(String, String)],
    ) -> Result<()> {
        info!(
            "Configuring PTP cross-node routing for {} nodes",
            nodes.len()
        );

        // Query pod CIDRs assigned by kube-controller-manager.
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            cp_name,
            "kubectl",
            "--kubeconfig=/etc/kubernetes/admin.conf",
            "get",
            "nodes",
            "-o",
            "json",
        ]);
        let output = cmd.output().context("Failed to query node pod CIDRs")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("kubectl get nodes failed: {}", stderr));
        }

        let nodes_json: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("Failed to parse node JSON")?;
        let node_cidrs = Self::parse_node_cidrs(&nodes_json);

        if node_cidrs.is_empty() {
            warn!("No pod CIDRs found on nodes — cross-node routing skipped");
            return Ok(());
        }
        info!("Node pod CIDRs: {:?}", node_cidrs);

        for plan in Self::plan_ptp_cross_node(nodes, &node_cidrs) {
            // Rewrite PTP config with the node-specific subnet (clears stale IPAM state).
            self.install_ptp_cni_with_subnet(&plan.node, &plan.subnet)
                .await?;

            // Add a host route for every other node's pod CIDR via that node's VM IP.
            for route in &plan.routes {
                let route_cmd = format!("ip route replace {} via {}", route.cidr, route.via_ip);
                let mut cmd = std::process::Command::new(&self.cli_path);
                cmd.args(["exec", &plan.node, "sh", "-c", &route_cmd]);
                match cmd.output() {
                    Ok(out) if out.status.success() => {
                        info!(
                            "Route added: {} via {} on '{}'",
                            route.cidr, route.via_ip, plan.node
                        );
                    }
                    Ok(out) => {
                        warn!(
                            "Route {} via {} on '{}': {}",
                            route.cidr,
                            route.via_ip,
                            plan.node,
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                    Err(e) => {
                        warn!("Failed to add cross-node route on '{}': {}", plan.node, e);
                    }
                }
            }
        }

        info!("PTP cross-node routing configured");
        Ok(())
    }

    /// Approve any pending kubelet-serving CSRs by exec-ing kubectl inside the
    /// control-plane container.
    ///
    /// kubeadm sets `serverTLSBootstrap: true`, so each kubelet obtains its serving
    /// certificate via a CertificateSigningRequest that stays Pending until approved.
    /// Until then, any API path through the kubelet serving port (:10250) — including
    /// `cilium status` probes that exec into pods via the kubelet API — fails with
    /// "remote error: tls: internal error", and cilium-operator loses leader election
    /// in a 5-minute backoff cascade that outlasts the readiness gate.
    ///
    /// Worker kubelets submit their serving CSRs only AFTER they join, so this must run
    /// again after workers join — not just during the control-plane Cilium install
    /// (issue #43). A 15-second wait gives a freshly-joined kubelet time to submit its
    /// CSR. Non-fatal: if no CSRs are pending the approve step is a no-op.
    fn approve_pending_kubelet_csrs(&self, container_name: &str) {
        info!("Approving pending kubelet-serving CSRs");
        let csr_approve_cmd = "\
            sleep 15 && \
            KUBECONFIG=/etc/kubernetes/admin.conf \
            kubectl get csr -o name 2>/dev/null | \
            grep 'csr.node.eks.amazonaws.com\\|certificatesigningrequest' | \
            xargs -r kubectl certificate approve --kubeconfig=/etc/kubernetes/admin.conf \
            2>/dev/null; \
            KUBECONFIG=/etc/kubernetes/admin.conf \
            kubectl get csr --no-headers 2>/dev/null | \
            awk '/Pending/{print $1}' | \
            xargs -r kubectl certificate approve --kubeconfig=/etc/kubernetes/admin.conf \
            2>/dev/null; \
            true";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", csr_approve_cmd]);
        match cmd.output() {
            Ok(out) if out.status.success() => {
                info!("Kubelet CSR approval step completed");
            }
            Ok(out) => {
                warn!(
                    "Kubelet CSR approval step returned non-zero (may be no CSRs yet): {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => {
                warn!("Failed to run kubelet CSR approval step (non-fatal): {}", e);
            }
        }
    }

    /// Approve pending kubelet-serving CSRs for a named cluster by exec-ing into its
    /// control-plane container (`<cluster>-control-plane`).
    ///
    /// This is the post-create / manual catch-all. It runs inside the container because
    /// on Apple Container the host cannot route to the in-VM API server — host-side
    /// `kubectl` against the saved kubeconfig fails with "no route to host", so the
    /// previous host-kubeconfig approval never approved anything. Covers PTP and
    /// single-node clusters, where no in-create approval runs.
    pub fn approve_cluster_kubelet_csrs(&self, cluster_name: &str) {
        let cp_name = format!("{}-control-plane", cluster_name);
        self.approve_pending_kubelet_csrs(&cp_name);
    }

    /// Install Cilium CNI plugin using the pinned cilium-cli and topology-correct helm values.
    ///
    /// Uses [`build_cilium_cli_install_script`] and either [`build_cilium_install_cmd`] (stock
    /// kernel workaround profile) or [`build_cilium_install_cmd_ebpf`] (full-eBPF profile for
    /// nodes booted on the custom kina kernel) so the exact commands are unit-testable without
    /// spawning containers.
    ///
    /// Profile selection: when `kernel_path` is `Some`, the full-eBPF profile is used (custom
    /// kernel has all required BPF options). When `None`, the stock workaround profile is used.
    ///
    /// After `cilium install`, runs a readiness gate (`cilium status --wait --wait-duration 5m`)
    /// with `KUBECONFIG=/etc/kubernetes/admin.conf`. On failure, captures diagnostics from
    /// `cilium status` (bare), `kubectl -n kube-system get pods -o wide`, and
    /// `kubectl -n kube-system logs ds/cilium --tail=100` and includes them in the error.
    async fn install_cilium_cni(
        &self,
        container_name: &str,
        kernel_path: Option<&std::path::Path>,
    ) -> Result<()> {
        info!("Installing Cilium CNI plugin (pinned versions, topology-correct values)");

        // Step 1: Install the pinned cilium-cli binary inside the container.
        // build_cilium_cli_install_script uses the pinned CILIUM_CLI_VERSION const —
        // no runtime version-discovery curl.
        let install_cli_cmd = build_cilium_cli_install_script(CILIUM_CLI_VERSION);

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", &install_cli_cmd]);

        let output = cmd.output().context("Failed to install Cilium CLI")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to install pinned cilium-cli {}: {}",
                CILIUM_CLI_VERSION,
                stderr
            ));
        }

        info!("cilium-cli {} installed successfully", CILIUM_CLI_VERSION);

        // Step 2: We need the control-plane VM IP to set k8sServiceHost.
        // The container_name is the control-plane node; fetch its IP.
        let cp_ip = self.get_container_ip(container_name).await?;

        // Step 3: Install Cilium with topology-correct --set values.
        // Profile selection: full-eBPF when custom kernel is set; stock workaround otherwise.
        // build_cilium_install_cmd_ebpf (custom kernel) retires all workarounds.
        // build_cilium_install_cmd (stock kernel) retains workarounds for kata-kernel gaps.
        let cilium_install_cmd = if kernel_path.is_some() {
            build_cilium_install_cmd_ebpf(CILIUM_VERSION, &cp_ip)
        } else {
            build_cilium_install_cmd(CILIUM_VERSION, &cp_ip)
        };

        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", &cilium_install_cmd]);

        let output = cmd.output().context("Failed to run cilium install")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let diag = self.collect_cilium_diagnostics(container_name);
            return Err(anyhow::anyhow!(
                "Failed to install Cilium {}: {}\nDiagnostics:\n{}",
                CILIUM_VERSION,
                stderr,
                diag
            ));
        }

        info!(
            "Cilium {} install command completed; running readiness gate",
            CILIUM_VERSION
        );

        // Step 4a: Approve pending kubelet-serving CSRs before the readiness gate so
        // `cilium status` exec probes don't hit "remote error: tls: internal error".
        // At this point only the control-plane kubelet has a CSR; worker CSRs are
        // approved again after workers join (issue #43). See approve_pending_kubelet_csrs.
        self.approve_pending_kubelet_csrs(container_name);

        // Step 4b: Readiness gate — wait until Cilium reports healthy.
        // Bounded to 5 minutes; fail fast with diagnostics if exceeded.
        // CSRs are now approved so cilium-operator kubelet API calls can complete TLS.
        let readiness_cmd =
            "KUBECONFIG=/etc/kubernetes/admin.conf cilium status --wait --wait-duration 5m";
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "sh", "-c", readiness_cmd]);

        let output = cmd
            .output()
            .context("Failed to run Cilium readiness gate")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let diag = self.collect_cilium_diagnostics(container_name);
            return Err(anyhow::anyhow!(
                "Cilium readiness gate failed (cilium status --wait --wait-duration 5m): {}\nDiagnostics:\n{}",
                stderr,
                diag
            ));
        }

        info!(
            "Cilium CNI plugin installed and ready (version {})",
            CILIUM_VERSION
        );
        Ok(())
    }

    /// Setup kubeconfig for external access
    async fn setup_kubeconfig(
        &self,
        cluster_name: &str,
        container_name: &str,
        vm_ip: &str,
    ) -> Result<()> {
        info!("Setting up kubeconfig for external access");

        // Get kubeconfig from container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args(["exec", container_name, "cat", "/etc/kubernetes/admin.conf"]);

        let output = cmd
            .output()
            .context("Failed to get kubeconfig from container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to get kubeconfig: {}", stderr));
        }

        let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();

        // Rewrite server URL to the live VM IP using the pure helper (idempotent,
        // handles any existing host: localhost, 127.0.0.1, old VM IP, cluster IP).
        kubeconfig = crate::core::verify::rewrite_kubeconfig_server(&kubeconfig, vm_ip);

        // Replace context and user names to be cluster-specific
        // kubeadm generates names like "kubernetes-admin@{clusterName}" and "kubernetes-admin"
        let kubeadm_context = format!("kubernetes-admin@{}", cluster_name);
        let cluster_admin = format!("{}-admin", cluster_name);
        kubeconfig = kubeconfig
            .replace(&kubeadm_context, cluster_name)
            .replace(
                "name: kubernetes-admin",
                &format!("name: {}", cluster_admin),
            )
            .replace(
                "user: kubernetes-admin",
                &format!("user: {}", cluster_admin),
            );

        // Save kubeconfig
        self.save_kubeconfig(cluster_name, &kubeconfig).await?;

        // Verify host→<vm_ip>:6443 TCP reachability (bounded retry, non-fatal).
        // Apple Container uses VM-per-container networking; the host may not always have a
        // direct route to the VM IP. We warn rather than hard-fail so the cluster remains
        // usable for in-container kubectl even when host access is blocked.
        let reachable = check_tcp_reachable(vm_ip, 6443, 5).await;
        if !reachable {
            let (bridge, gateway) = inspect_network_bridge(cluster_name, &self.cli_path);
            let diag = crate::core::verify::build_unreachable_diagnostic(
                cluster_name,
                vm_ip,
                6443,
                bridge.as_deref(),
                gateway.as_deref(),
            );
            warn!("{}", diag);
        }

        info!("Kubeconfig saved successfully");
        Ok(())
    }

    /// Re-resolve the control-plane VM IP, rewrite the saved kubeconfig to use it,
    /// and verify host→control-plane TCP reachability.
    ///
    /// Non-fatal: returns `Ok(())` even when the host cannot reach the API server;
    /// a warning is printed in that case so the caller can surface it to the user.
    /// Handles nodes that restarted with a new IP by fetching the live IP via
    /// `get_container_ip` before rewriting.
    pub async fn repair_kubeconfig(&self, cluster_name: &str) -> Result<()> {
        info!("Repairing kubeconfig for cluster '{}'", cluster_name);

        // Control-plane container is always named "<cluster>-control-plane".
        let cp_name = format!("{}-control-plane", cluster_name);

        // Re-resolve the live VM IP in case the node restarted with a new address.
        let vm_ip = self
            .get_container_ip(&cp_name)
            .await
            .context("Failed to get control-plane VM IP; is the cluster running?")?;
        info!("Control-plane current IP: {}", vm_ip);

        // Read the saved kubeconfig; fall back to fetching from the container if absent.
        let home_dir = std::env::var("HOME").context("HOME environment variable not set")?;
        let kubeconfig_path = std::path::Path::new(&home_dir)
            .join(".kube")
            .join(cluster_name);

        let current = if kubeconfig_path.exists() {
            fs::read_to_string(&kubeconfig_path).context("Failed to read saved kubeconfig")?
        } else {
            info!("No saved kubeconfig found; fetching from container");
            let mut cmd = std::process::Command::new(&self.cli_path);
            cmd.args(["exec", &cp_name, "cat", "/etc/kubernetes/admin.conf"]);
            let out = cmd
                .output()
                .context("Failed to run container exec to read kubeconfig")?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(anyhow::anyhow!(
                    "Failed to read kubeconfig from container: {}",
                    stderr
                ));
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        };

        // Rewrite the server URL to the current VM IP (idempotent pure fn).
        let rewritten = crate::core::verify::rewrite_kubeconfig_server(&current, &vm_ip);

        // Persist the updated kubeconfig.
        self.save_kubeconfig(cluster_name, &rewritten).await?;
        info!("Kubeconfig rewritten for cluster '{}'", cluster_name);

        // Re-check reachability after the rewrite.
        println!("Checking host reachability to {}:6443 ...", vm_ip);
        let reachable = check_tcp_reachable(&vm_ip, 6443, 5).await;
        if reachable {
            println!("  OK  {}:6443 is reachable from the host", vm_ip);
        } else {
            let (bridge, gateway) = inspect_network_bridge(cluster_name, &self.cli_path);
            let diag = crate::core::verify::build_unreachable_diagnostic(
                cluster_name,
                &vm_ip,
                6443,
                bridge.as_deref(),
                gateway.as_deref(),
            );
            warn!("{}", diag);
        }

        Ok(())
    }
}

// ===========================================================================
// kina-39 — TCP reachability probe helpers (module-level, not pub)
// ===========================================================================

/// Attempt to open a TCP connection to `addr:port` up to `attempts` times
/// with a 1-second back-off between retries.  Returns `true` on the first
/// successful connection, `false` if all attempts fail.
async fn check_tcp_reachable(addr: &str, port: u16, attempts: u32) -> bool {
    let socket_addr = format!("{}:{}", addr, port);
    for i in 0..attempts {
        if i > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpStream::connect(&socket_addr),
        )
        .await;
        match connect_result {
            Ok(Ok(_)) => return true,
            Ok(Err(_)) | Err(_) => {
                // connect error or per-attempt timeout — try next attempt
            }
        }
    }
    false
}

/// Try to retrieve the bridge interface name and IPv4 gateway for a cluster's
/// Apple Container network by running `container network inspect <cluster>`.
/// Returns `(bridge, gateway)` where either/both may be `None` when the
/// command is unavailable or the fields are absent in the output.
fn inspect_network_bridge(cluster_name: &str, cli_path: &str) -> (Option<String>, Option<String>) {
    let output = std::process::Command::new(cli_path)
        .args(["network", "inspect", cluster_name, "--format", "json"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let bridge = extract_json_string_field(&text, "bridge")
                .or_else(|| extract_json_string_field(&text, "interfaceName"));
            let gateway = extract_json_string_field(&text, "ipv4Gateway")
                .or_else(|| extract_json_string_field(&text, "gateway"));
            (bridge, gateway)
        }
        _ => (None, None),
    }
}

/// Extract a simple JSON string field value from raw JSON text without
/// pulling in an extra dependency.  Returns `None` when the field is absent
/// or its value is not a JSON string.
fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":", field);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim_start();
    if let Some(inner) = rest.strip_prefix('"') {
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        None
    }
}

/// A single host route to another node's pod subnet, via that node's VM IP.
#[derive(Debug, Clone, PartialEq)]
struct PtpRoute {
    cidr: String,
    via_ip: String,
}

/// The PTP networking plan for one node: the pod subnet it should use for its
/// local PTP IPAM, plus host routes to every other node's pod subnet.
#[derive(Debug, Clone, PartialEq)]
struct PtpNodePlan {
    node: String,
    subnet: String,
    routes: Vec<PtpRoute>,
}

impl AppleContainerClient {
    /// Parse `kubectl get nodes -o json` into a `node-name -> podCIDR` map.
    ///
    /// Nodes whose `spec.podCIDR` is absent (not yet assigned by
    /// kube-controller-manager) are skipped.
    fn parse_node_cidrs(nodes_json: &serde_json::Value) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(items) = nodes_json["items"].as_array() {
            for node in items {
                let name = node["metadata"]["name"].as_str().unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                if let Some(cidr) = node["spec"]["podCIDR"].as_str() {
                    map.insert(name.to_string(), cidr.to_string());
                }
            }
        }
        map
    }

    /// Compute the per-node PTP routing plan for a multi-node cluster.
    ///
    /// PTP CNI is point-to-point and only wires pods to their local node, so
    /// without this every node would (a) allocate from the same shared
    /// `10.244.0.0/16` (IP conflicts) and (b) have no route to pods on other
    /// nodes (breaking CoreDNS for any pod off the control-plane). The plan
    /// gives each node its own assigned pod CIDR as the PTP subnet and a route
    /// to every other node's pod CIDR via that node's VM IP.
    ///
    /// Nodes whose pod CIDR is not yet known are omitted, and routes are only
    /// emitted toward peers whose CIDR is known.
    fn plan_ptp_cross_node(
        nodes: &[(String, String)],
        node_cidrs: &HashMap<String, String>,
    ) -> Vec<PtpNodePlan> {
        let mut plans = Vec::new();
        for (node, _vm_ip) in nodes {
            let subnet = match node_cidrs.get(node) {
                Some(cidr) => cidr.clone(),
                None => continue,
            };
            let routes = nodes
                .iter()
                .filter(|(other, _)| other != node)
                .filter_map(|(other, other_ip)| {
                    node_cidrs.get(other).map(|cidr| PtpRoute {
                        cidr: cidr.clone(),
                        via_ip: other_ip.clone(),
                    })
                })
                .collect();
            plans.push(PtpNodePlan {
                node: node.clone(),
                subnet,
                routes,
            });
        }
        plans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kubeadm_join_info_standard() {
        let output = r#"
Your Kubernetes control-plane has initialized successfully!

Then you can join any number of worker nodes by running the following on each as root:

kubeadm join 192.168.64.5:6443 --token abcdef.0123456789abcdef \
    --discovery-token-ca-cert-hash sha256:abc123def456
"#;
        let result = AppleContainerClient::parse_kubeadm_join_info(output, "192.168.64.5").unwrap();
        assert_eq!(result.token, "abcdef.0123456789abcdef");
        assert_eq!(result.ca_cert_hash, "sha256:abc123def456");
        assert_eq!(result.control_plane_endpoint, "192.168.64.5:6443");
    }

    #[test]
    fn test_parse_kubeadm_join_info_single_line() {
        let output = "kubeadm join 10.0.0.1:6443 --token mytoken.1234567890abcdef --discovery-token-ca-cert-hash sha256:deadbeef";
        let result = AppleContainerClient::parse_kubeadm_join_info(output, "10.0.0.1").unwrap();
        assert_eq!(result.token, "mytoken.1234567890abcdef");
        assert_eq!(result.ca_cert_hash, "sha256:deadbeef");
        assert_eq!(result.control_plane_endpoint, "10.0.0.1:6443");
    }

    #[test]
    fn test_parse_kubeadm_join_info_missing_token() {
        let output = "kubeadm join 10.0.0.1:6443 --discovery-token-ca-cert-hash sha256:deadbeef";
        let result = AppleContainerClient::parse_kubeadm_join_info(output, "10.0.0.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("token"));
    }

    #[test]
    fn test_parse_kubeadm_join_info_missing_hash() {
        let output = "kubeadm join 10.0.0.1:6443 --token mytoken.1234567890abcdef";
        let result = AppleContainerClient::parse_kubeadm_join_info(output, "10.0.0.1");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CA cert hash"));
    }

    #[test]
    fn test_parse_kubeadm_join_info_no_join_command() {
        let output = "Some other output without join info";
        let result = AppleContainerClient::parse_kubeadm_join_info(output, "10.0.0.1");
        assert!(result.is_err());
    }

    // --- Multi-node PTP cross-node routing ---

    #[test]
    fn test_parse_node_cidrs_normal() {
        let json = serde_json::json!({
            "items": [
                {"metadata": {"name": "node-cp"}, "spec": {"podCIDR": "10.244.0.0/24"}},
                {"metadata": {"name": "node-worker"}, "spec": {"podCIDR": "10.244.1.0/24"}}
            ]
        });
        let cidrs = AppleContainerClient::parse_node_cidrs(&json);
        assert_eq!(cidrs.len(), 2);
        assert_eq!(cidrs["node-cp"], "10.244.0.0/24");
        assert_eq!(cidrs["node-worker"], "10.244.1.0/24");
    }

    #[test]
    fn test_parse_node_cidrs_missing_pod_cidr_skipped() {
        let json = serde_json::json!({
            "items": [
                {"metadata": {"name": "node-cp"}, "spec": {"podCIDR": "10.244.0.0/24"}},
                {"metadata": {"name": "node-worker"}, "spec": {}}
            ]
        });
        let cidrs = AppleContainerClient::parse_node_cidrs(&json);
        assert_eq!(cidrs.len(), 1);
        assert!(cidrs.contains_key("node-cp"));
        assert!(!cidrs.contains_key("node-worker"));
    }

    #[test]
    fn test_parse_node_cidrs_empty_items() {
        let json = serde_json::json!({"items": []});
        let cidrs = AppleContainerClient::parse_node_cidrs(&json);
        assert!(cidrs.is_empty());
    }

    #[test]
    fn plan_gives_each_node_its_own_cidr_and_routes_to_every_peer() {
        let nodes = vec![
            ("cp".to_string(), "192.168.64.2".to_string()),
            ("w1".to_string(), "192.168.64.3".to_string()),
            ("w2".to_string(), "192.168.64.4".to_string()),
        ];
        let cidrs = HashMap::from([
            ("cp".to_string(), "10.244.0.0/24".to_string()),
            ("w1".to_string(), "10.244.1.0/24".to_string()),
            ("w2".to_string(), "10.244.2.0/24".to_string()),
        ]);

        let plans = AppleContainerClient::plan_ptp_cross_node(&nodes, &cidrs);
        let w1 = plans.iter().find(|p| p.node == "w1").unwrap();

        // Each node must use its OWN /24 as the PTP subnet (not the shared 10.244.0.0/16).
        assert_eq!(w1.subnet, "10.244.1.0/24");

        // Each node must get a route to every OTHER node's pod CIDR via that node's VM IP.
        assert_eq!(w1.routes.len(), 2);
        assert!(w1
            .routes
            .iter()
            .any(|r| r.cidr == "10.244.0.0/24" && r.via_ip == "192.168.64.2"));
        assert!(w1
            .routes
            .iter()
            .any(|r| r.cidr == "10.244.2.0/24" && r.via_ip == "192.168.64.4"));
        // No self-route.
        assert!(!w1.routes.iter().any(|r| r.cidr == "10.244.1.0/24"));
    }

    #[test]
    fn plan_skips_nodes_without_a_known_cidr() {
        let nodes = vec![
            ("cp".to_string(), "192.168.64.2".to_string()),
            ("w1".to_string(), "192.168.64.3".to_string()),
        ];
        // w1 has no assigned podCIDR yet — it cannot be planned.
        let cidrs = HashMap::from([("cp".to_string(), "10.244.0.0/24".to_string())]);

        let plans = AppleContainerClient::plan_ptp_cross_node(&nodes, &cidrs);

        // Only cp is plannable; cp has no peer with a known CIDR, so it gets no routes.
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].node, "cp");
        assert_eq!(plans[0].subnet, "10.244.0.0/24");
        assert!(plans[0].routes.is_empty());
    }

    #[test]
    fn format_created_reformats_iso8601() {
        assert_eq!(
            format_created("2026-06-14T21:52:43Z"),
            "2026-06-14 21:52 UTC"
        );
    }

    #[test]
    fn format_created_falls_back_on_unparseable() {
        assert_eq!(format_created("not-a-date"), "not-a-date");
        assert_eq!(format_created(""), "unknown");
    }

    #[test]
    fn parse_container_list_extracts_creation_date() {
        let json = r#"[{"id":"n1","configuration":{"creationDate":"2026-06-14T21:52:43Z","labels":{"io.kina.cluster":"c"}},"status":{"state":"running"}}]"#;
        let parsed = parse_container_list(json).unwrap();
        assert_eq!(parsed[0].created.as_deref(), Some("2026-06-14T21:52:43Z"));
    }

    // --- normalize_image_ref: Docker reference normalization for CRI lookup ---
    //
    // kubelet requests images from containerd's CRI plugin by exact,
    // fully-qualified key. `kina load` must tag the imported image with the
    // same name kubelet will ask for, or the pull is invisible to the CRI
    // (ImagePullBackOff even though the image is present in containerd).

    #[test]
    fn normalize_image_ref_short_name_gets_docker_io_prefix() {
        assert_eq!(
            normalize_image_ref("myorg/tool:latest"),
            "docker.io/myorg/tool:latest"
        );
    }

    #[test]
    fn normalize_image_ref_single_name_gets_docker_io_library() {
        assert_eq!(
            normalize_image_ref("alpine:latest"),
            "docker.io/library/alpine:latest"
        );
    }

    #[test]
    fn normalize_image_ref_single_name_no_tag_gets_latest_appended() {
        assert_eq!(
            normalize_image_ref("alpine"),
            "docker.io/library/alpine:latest"
        );
    }

    #[test]
    fn normalize_image_ref_dotted_registry_host_unchanged() {
        assert_eq!(
            normalize_image_ref("gcr.io/foo/bar:1.2"),
            "gcr.io/foo/bar:1.2"
        );
    }

    #[test]
    fn normalize_image_ref_localhost_registry_unchanged() {
        assert_eq!(
            normalize_image_ref("localhost:5000/x:latest"),
            "localhost:5000/x:latest"
        );
    }

    #[test]
    fn normalize_image_ref_dotted_host_with_port_unchanged() {
        assert_eq!(
            normalize_image_ref("registry.example.com:5000/a/b:tag"),
            "registry.example.com:5000/a/b:tag"
        );
    }

    #[test]
    fn normalize_image_ref_already_fully_qualified_unchanged() {
        assert_eq!(
            normalize_image_ref("docker.io/myorg/tool:latest"),
            "docker.io/myorg/tool:latest"
        );
    }
}
