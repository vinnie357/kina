use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
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
/// All `--set` values are topology-correct for Apple Container clusters:
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
///   struct field ConfigMap.data of type string". This is a **separate code path** from the L7
///   proxy (Envoy) disabled by `l7Proxy=false`. Even with `l7Proxy=false`, Cilium 1.18.x
///   unconditionally installs a `CILIUM_PRE_mangle -m socket --transparent` iptables rule in
///   the mangle table. That rule requires the `xt_socket` kernel module
///   (`CONFIG_NETFILTER_XT_MATCH_SOCKET`), which is absent from the Apple Container kata-kernel
///   (confirmed: `iptables v1.8.8 legacy: unknown option --transparent`, exit status 2,
///   cilium-agent CrashLoopBackOff).
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

        result.push(ParsedContainer {
            id,
            labels,
            state,
            ipv4,
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
            self.create_single_node(&options.name, &node_name, &options.image, cni)
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
        self.create_control_plane_node(&options.name, &cp_name, &options.image, true)
            .await?;

        // 2. Wait for control-plane container to be ready and get IP
        self.wait_for_container_ready(&cp_name).await?;
        let cp_ip = self.get_container_ip(&cp_name).await?;
        info!("Control-plane '{}' running at IP: {}", cp_name, cp_ip);

        // 3. Initialize Kubernetes on control-plane and get join info
        let join_info = self
            .initialize_kubernetes_cluster_with_join_info(&cp_name, &cp_ip, &options.name)
            .await?;

        // 4. Setup kubeconfig early (user gets kubectl access even if workers fail)
        self.setup_kubeconfig(&options.name, &cp_name, &cp_ip)
            .await?;

        // 5. Install CNI on control-plane (must be before workers join)
        self.install_cni_plugin(&cp_name, cni.clone()).await?;

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

            self.create_worker_node(&options.name, &worker_name, &options.image)
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
        }

        // After all workers have joined, re-run the Cilium readiness gate and then
        // wait for all nodes to be Ready before reporting success.
        if matches!(cni, CniPlugin::Cilium) {
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
    async fn create_single_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        cni: CniPlugin,
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

        // Initialize Kubernetes cluster
        self.initialize_kubernetes_cluster(node_name, &vm_ip)
            .await?;

        // Generate and save kubeconfig immediately after cluster init
        // This ensures user has kubectl access even if CNI installation fails
        self.setup_kubeconfig(cluster_name, node_name, &vm_ip)
            .await?;

        // Remove control-plane taint for single-node scheduling
        self.remove_control_plane_taint(node_name).await?;

        // Install CNI plugin (now user has kubectl access if this fails)
        // Use the resolved CNI plugin (CLI flag overrides config default).
        self.install_cni_plugin(node_name, cni).await?;

        info!(
            "Kubernetes cluster '{}' initialized successfully",
            cluster_name
        );
        Ok(())
    }

    /// Create a control plane node
    async fn create_control_plane_node(
        &self,
        cluster_name: &str,
        node_name: &str,
        image: &str,
        is_primary: bool,
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
                            created: "unknown".to_string(),
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
                let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();

                // Get the VM's IP address to update the server URL
                if let Some(vm_ip) = &control_plane_node.ip_address {
                    info!("Updating kubeconfig server URL to use VM IP: {}", vm_ip);

                    // Replace localhost/127.0.0.1 references with the VM's IP
                    kubeconfig = kubeconfig
                        .replace("https://127.0.0.1:6443", &format!("https://{}:6443", vm_ip))
                        .replace("https://localhost:6443", &format!("https://{}:6443", vm_ip));

                    // Also replace any internal cluster IP with VM IP
                    if kubeconfig.contains("https://10.") || kubeconfig.contains("https://172.") {
                        // Use regex or string manipulation to replace internal IPs
                        // For now, use a simple approach
                        kubeconfig = kubeconfig
                            .lines()
                            .map(|line| {
                                if line.trim().starts_with("server: https://") {
                                    format!("    server: https://{}:6443", vm_ip)
                                } else {
                                    line.to_string()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }

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

        // Copy the tar file into the container
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "cp",
            &image_tar.to_string_lossy(),
            &format!("{}:/tmp/image.tar", container_id),
        ]);

        let output = cmd
            .output()
            .context("Failed to copy image tar to container")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up the temp file
            let _ = fs::remove_file(&image_tar);
            return Err(anyhow::anyhow!(
                "Failed to copy image to container: {}",
                stderr
            ));
        }

        // Load the image in the container using ctr (containerd CLI)
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_id,
            "ctr",
            "images",
            "import",
            "/tmp/image.tar",
        ]);

        let output = cmd.output().context("Failed to load image in container")?;

        // Clean up the temp file
        let _ = fs::remove_file(&image_tar);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to load image in container: {}",
                stderr
            ));
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

    /// Generate kubeadm init configuration YAML
    fn generate_kubeadm_init_config(
        &self,
        container_name: &str,
        vm_ip: &str,
        cluster_name: &str,
    ) -> String {
        format!(
            r#"apiVersion: kubeadm.k8s.io/v1beta3
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: "{vm_ip}"
  bindPort: 6443
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
    node-ip: "{vm_ip}"
    provider-id: "kind://docker/{cluster_name}/{container_name}"
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: ClusterConfiguration
kubernetesVersion: v1.31.0
clusterName: "{cluster_name}"
controlPlaneEndpoint: "{vm_ip}:6443"
apiServer:
  certSANs:
  - "{vm_ip}"
  - "{container_name}"
  - "localhost"
  - "127.0.0.1"
  extraArgs:
    runtime-config: "api/all=true"
networking:
  serviceSubnet: "10.96.0.0/16"
  podSubnet: "10.244.0.0/16"
  dnsDomain: "cluster.local"
controllerManager:
  extraArgs:
    enable-hostpath-provisioner: "true"
scheduler: {{}}
etcd:
  local:
    dataDir: "/var/lib/etcd"
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: JoinConfiguration
nodeRegistration:
  criSocket: unix:///run/containerd/containerd.sock
  kubeletExtraArgs:
    node-ip: "{vm_ip}"
    provider-id: "kind://docker/{cluster_name}/{container_name}"
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

    /// Write kubeadm config and run kubeadm init in a container
    fn run_kubeadm_init(
        &self,
        container_name: &str,
        kubeadm_config: &str,
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

        // Initialize cluster with kubeadm
        let mut cmd = std::process::Command::new(&self.cli_path);
        cmd.args([
            "exec",
            container_name,
            "kubeadm",
            "init",
            "--config=/kind/kubeadm.conf",
            "--skip-phases=preflight",
            "--v=1",
        ]);

        info!("Running kubeadm init (this may take a few minutes)...");
        cmd.output().context("Failed to run kubeadm init")
    }

    /// Initialize Kubernetes cluster with kubeadm (single-node, no join info needed)
    async fn initialize_kubernetes_cluster(&self, container_name: &str, vm_ip: &str) -> Result<()> {
        info!(
            "Initializing Kubernetes cluster in container '{}'",
            container_name
        );

        // Extract cluster name from container name (e.g., "mycluster-control-plane" -> "mycluster")
        let cluster_name = container_name
            .strip_suffix("-control-plane")
            .unwrap_or(container_name);

        let kubeadm_config = self.generate_kubeadm_init_config(container_name, vm_ip, cluster_name);
        let output = self.run_kubeadm_init(container_name, &kubeadm_config)?;

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

    /// Initialize Kubernetes cluster and extract join info for multi-node setup
    async fn initialize_kubernetes_cluster_with_join_info(
        &self,
        container_name: &str,
        vm_ip: &str,
        cluster_name: &str,
    ) -> Result<KubeadmJoinInfo> {
        info!(
            "Initializing Kubernetes cluster in container '{}' (multi-node)",
            container_name
        );

        let kubeadm_config = self.generate_kubeadm_init_config(container_name, vm_ip, cluster_name);
        let output = self.run_kubeadm_init(container_name, &kubeadm_config)?;

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

        // Write a JoinConfiguration YAML to the worker
        let join_config = format!(
            r#"apiVersion: kubeadm.k8s.io/v1beta3
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
    node-ip: "{worker_ip}"
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
        );

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
    async fn install_cni_plugin(&self, container_name: &str, cni: CniPlugin) -> Result<()> {
        match cni {
            CniPlugin::Ptp => self.install_ptp_cni(container_name).await,
            CniPlugin::Cilium => self.install_cilium_cni(container_name).await,
        }
    }

    /// Install PTP CNI plugin optimized for Apple Container VMs
    async fn install_ptp_cni(&self, container_name: &str) -> Result<()> {
        info!("Installing PTP CNI plugin optimized for Apple Container VMs");

        // PTP CNI configuration that works with kata-containers kernel limitations
        let ptp_config = r#"{
  "cniVersion": "0.4.0",
  "name": "ptp-net",
  "plugins": [
    {
      "type": "ptp",
      "ipMasq": true,
      "ipam": {
        "type": "host-local",
        "subnet": "10.244.0.0/16",
        "routes": [
          { "dst": "0.0.0.0/0" }
        ]
      }
    },
    {
      "type": "portmap",
      "capabilities": {
        "portMappings": true
      }
    }
  ]
}"#;

        // Create CNI configuration directory and install config
        let install_cmd = format!(
            r#"mkdir -p /etc/cni/net.d && cat > /etc/cni/net.d/10-ptp.conflist << 'EOF'
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

        info!("PTP CNI plugin installed successfully - compatible with Apple Container VMs");
        Ok(())
    }

    /// Install Cilium CNI plugin using the pinned cilium-cli and topology-correct helm values.
    ///
    /// Uses [`build_cilium_cli_install_script`] and [`build_cilium_install_cmd`] (pure fns)
    /// so the exact commands are unit-testable without spawning containers.
    ///
    /// After `cilium install`, runs a readiness gate (`cilium status --wait --wait-duration 5m`)
    /// with `KUBECONFIG=/etc/kubernetes/admin.conf`. On failure, captures diagnostics from
    /// `cilium status` (bare), `kubectl -n kube-system get pods -o wide`, and
    /// `kubectl -n kube-system logs ds/cilium --tail=100` and includes them in the error.
    async fn install_cilium_cni(&self, container_name: &str) -> Result<()> {
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
        // build_cilium_install_cmd documents every flag rationale in its doc-comment.
        let cilium_install_cmd = build_cilium_install_cmd(CILIUM_VERSION, &cp_ip);

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

        // Step 4: Readiness gate — wait until Cilium reports healthy.
        // Bounded to 5 minutes; fail fast with diagnostics if exceeded.
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

        // Update server URL to use VM IP — replace any server line with the correct VM IP
        kubeconfig = kubeconfig
            .replace("https://127.0.0.1:6443", &format!("https://{}:6443", vm_ip))
            .replace("https://localhost:6443", &format!("https://{}:6443", vm_ip));

        // Also fix any IP that kubeadm may have advertised (the advertiseAddress)
        // by replacing server lines generically
        kubeconfig = kubeconfig
            .lines()
            .map(|line| {
                if line.trim().starts_with("server: https://") && line.contains(":6443") {
                    format!("    server: https://{}:6443", vm_ip)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

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

        info!("Kubeconfig saved successfully");
        Ok(())
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
}
