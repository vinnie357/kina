# Custom Kernel for Node Containers

kina ships a custom Linux kernel for node containers so that Cilium can run in full-eBPF mode — with `kubeProxyReplacement`, BPF host routing, L7 proxy, transparent DNS proxy, and Hubble all enabled. The kernel is injected per node container via `container run --kernel`; the host system kernel is never modified. The pinned artifact is Linux `6.18.5-kina.1`, measured at 33,503,744 bytes (~32 MB), sha256 `f1a40c2c00e8a7f2e2c0165355c13ff6dcdd2742d294babe31dd5c5b14aec3fe`.

## Why (stock kernel cannot reach full eBPF)

The operator directive for kina's Cilium integration is full-eBPF feature parity: `kubeProxyReplacement=true`, BPF host routing (`bpf.hostLegacyRouting=false`), L7 proxy and transparent DNS proxy enabled, BPF masquerade, and Hubble. No workarounds.

The Apple Container 1.0.0 default kernel is `vmlinux-6.18.15-186`, sourced from the kata-containers 3.28.0 arm64 distribution. It is a hardened-minimal config. Live boot probing (kina-6 PREBUILT SCOUT) confirmed 15 required options are absent or not set, including the two critical blockers:

- `CONFIG_NETFILTER_XT_MATCH_SOCKET` (absent) — cilium-agent installs xt_socket-based proxy rules regardless of `l7Proxy` setting; missing xt_socket causes `Extension socket revision 0 not supported` crash at startup.
- `CONFIG_BPF_JIT` (not set) — Cilium hard-requires BPF JIT; without it, the eBPF datapath cannot function.

The kata kernel also has `CONFIG_MODULES=n`: modules cannot be loaded at runtime. There is no `modprobe` path to add the missing options. Rebuilding the kernel is the only path.

The prebuilt-scout verdict (kina-6): no downloadable arm64 kernel in a format compatible with Apple Virtualization.framework satisfies the full required option set. The apple/containerization build base already has most of the options kina needs; only a small fragment must be added.

**Workarounds retired by the custom kernel:**

| Workaround | Root cause it masked | Retired by |
|---|---|---|
| `enableLocalNodeRoute=false` | `ip rule` EAFNOSUPPORT — IPv4 policy routing unavailable | `IP_MULTIPLE_TABLES`, `FIB_RULES`, `IPV6_MULTIPLE_TABLES` (already in base config) |
| `l7Proxy=false` | xt_socket absent; L7 proxy rule insertion crashes cilium-agent | `NETFILTER_XT_MATCH_SOCKET` (fragment) |
| `dnsproxy-enable-transparent-mode=false` | `xt_socket --transparent` unsupported | `NETFILTER_XT_MATCH_SOCKET` + `NETFILTER_XT_TARGET_TPROXY` (fragment + base) |
| `kubeProxyReplacement=false` (kube-proxy retained) | Insufficient BPF + cgroup support for full kube-proxy replacement | `BPF_JIT`, `CGROUP_BPF`, `INET_DIAG_DESTROY`, full socket-LB set (fragment + base) |

## What (the pinned artifact)

The kernel is built from the apple/containerization `kernel/` build tooling, pinned at tag `0.33.3` of `github.com/apple/containerization`. That tag builds upstream Linux `6.18.5` from the KSOURCE tarball at:

```
https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.18.5.tar.xz
```

A kina-specific config fragment (`cilium.fragment`) containing 9 MUST-ADD options is merged on top of the `config-arm64` baseline before the final `make olddefconfig`. The `localversion` is set to `-kina.1`, producing the `uname -r` string `6.18.5-kina.1`. The pinned release tag is `kernel-v6.18.5+kina.1`.

**Measured artifact** (kina-6 KERNEL BUILD + VERIFIER check #8, re-confirmed by `shasum -a 256` on disk):

| Field | Value |
|---|---|
| `uname -r` | `6.18.5-kina.1` |
| Size | 33,503,744 bytes (~32 MB) |
| sha256 | `f1a40c2c00e8a7f2e2c0165355c13ff6dcdd2742d294babe31dd5c5b14aec3fe` |

**The 9 fragment MUST-ADD options** (`cilium.fragment`, all `=y` on the built kernel — confirmed by `zcat /proc/config.gz` in kina-6 configProbe):

| Option | Purpose |
|---|---|
| `CONFIG_BPF_JIT` | Cilium hard requirement; absent from base config |
| `CONFIG_BPF_JIT_ALWAYS_ON` | Harden JIT; absent from base config |
| `CONFIG_BPF_EVENTS` | Perf/tracing attach for datapath + Hubble |
| `CONFIG_FTRACE` | Dependency for `BPF_EVENTS` |
| `CONFIG_KPROBES` | Dependency for `BPF_EVENTS` |
| `CONFIG_BPF_STREAM_PARSER` | sockops/sockmap |
| `CONFIG_CRYPTO_USER_API_HASH` | Cilium base requirement |
| `CONFIG_SCHEDSTATS` | Cilium base requirement |
| `CONFIG_INET_DIAG_DESTROY` | Socket-LB backend deletion (kubeProxyReplacement) |

These 9 options plus the options already present in `config-arm64@0.33.3` yield the full set of 18 required options, all confirmed `=y` in the running 3-node cluster (kina-6 GOAL EVIDENCE STEP 3 + VERIFIER).

Linux 6.18.5 clears all Cilium 1.18 per-feature kernel minimums: base/kube-proxy-replacement/socket-LB (5.10), netkit (6.8), IPv4 BIG TCP (6.3). (kina-5 §2)

## How it's built

The apple/containerization project provides the build tooling under `kernel/`. Building requires cloning it at the pinned tag, adding the kina fragment, patching the build to merge the fragment before `make olddefconfig`, and running `make`.

**Exact command sequence** (from kina-5 §3 Phase A and confirmed in kina-6 KERNEL BUILD):

```bash
# 1. Clone containerization at the pinned tag
git clone --depth 1 --branch 0.33.3 https://github.com/apple/containerization /tmp/cz

# 2. Add the kina cilium fragment alongside the vendored kernel config
cp cilium.fragment /tmp/cz/kernel/cilium.fragment

# 3. Patch the build to merge the fragment before make olddefconfig:
#    In the build script, insert before `make olddefconfig`:
#      scripts/kconfig/merge_config.sh -m .config cilium.fragment
#    Then:
cd /tmp/cz/kernel && make

# 4. Record the sha256 of the output vmlinux
shasum -a 256 vmlinux
```

The `make` target runs the `kernel-build-image` step (container image with the build toolchain) then `kernel-build`. Output is `vmlinux` — a Linux kernel ARM64 boot executable Image, little-endian, 4K pages, compatible with Apple Virtualization.framework.

**Fragment file location:** the current spike path is `.kernel-spike/cz/kernel/cilium.fragment` in the kina repo. The Phase-B deliverable (kina-5 §3) will vendor the fragment under `kina-cli/kernel/cilium.fragment` alongside the build tooling. The fragment content (the 9 MUST-ADD options above) is stable regardless of path.

**Build cost** (measured, kina-6): ~4 minutes wall time. The Apple Makefile provisions 8 vCPUs / 16 GB for the compile. (kina-5 §3 Phase A)

**Config verification** after boot (requires `CONFIG_IKCONFIG_PROC=y`, present in the built kernel):

```bash
container run --kernel /path/to/vmlinux --rm alpine:latest sh -c \
  'zcat /proc/config.gz | grep -E "CONFIG_(BPF_JIT|BPF_JIT_ALWAYS_ON|BPF_EVENTS|FTRACE|KPROBES|BPF_STREAM_PARSER|CRYPTO_USER_API_HASH|SCHEDSTATS|INET_DIAG_DESTROY|NETFILTER_XT_MATCH_SOCKET)="'
```

All 10 options must appear as `=y`.

## How to use it

kina passes `container run --kernel <abs-path>` on every node container it creates. The host system kernel is never modified: `container system kernel set` is not called, and `config.toml [kernel]` is not written. Per-container kernel injection is a zero-mutation operation.

**Create a cluster with the custom kernel:**

```bash
kina create <name> --workers N --cni cilium --kernel-path /path/to/vmlinux
```

Example from kina-6 attempt-2 (the passing run):

```bash
target/release/kina create kina-test --workers 2 --cni cilium \
  --kernel-path /Users/vinnie/github/kina/.kernel-spike/cz/kernel/vmlinux \
  --wait 240
```

**Config file alternative:** set `node_kernel_path` in `~/.config/kina/config.toml`:

```toml
[cluster]
node_kernel_path = "/path/to/vmlinux"
```

Precedence: CLI `--kernel-path` flag > `config.toml` `node_kernel_path` > `None` (stock kernel). The `node_kernel_path` config field is defined in `kina-cli/src/config/mod.rs` (lines 57, 168); precedence logic is implemented in `select_kernel_path` at `kina-cli/src/core/apple_container.rs:156`, called from `kina-cli/src/cli/cluster.rs:208`.

**Profile coupling:** when `node_kernel_path` resolves to `Some`, kina selects the full-eBPF Cilium install profile (`kubeProxyReplacement=true`, `bpf.masquerade=true`, `bpf.hostLegacyRouting=false`, `hubble.enabled=true`) and passes `--skip-phases=addon/kube-proxy` to kubeadm. When `node_kernel_path` is `None` (stock kernel), the stock workaround profile is used and kube-proxy is retained.

**Rollback:** omit `--kernel-path` and leave `node_kernel_path` unset in config. kina uses the stock kernel and the stock Cilium profile. No cluster mutation needed.

**Verified end-state** (kina-6 attempt-2 PASS + VERIFIER, 3-node cluster):

```
NAME                      STATUS   ROLES           KERNEL-VERSION
kina-test-control-plane   Ready    control-plane   6.18.5-kina.1
kina-test-worker          Ready    <none>          6.18.5-kina.1
kina-test-worker-2        Ready    <none>          6.18.5-kina.1

KubeProxyReplacement: True
Routing: Host: BPF
Masquerading: BPF
Hubble: Ok
kube-proxy DaemonSet: NotFound

HTTP 200 + "Kina Demo" body on 192.168.65.23 and 192.168.65.24
```

Host default symlink `default.kernel-arm64 -> vmlinux-6.12.28-153` — unchanged. Zero system mutation confirmed.

## How it's distributed

The `vmlinux` image is published as a **GitHub Release asset** on the kina repository, tagged `kernel-v6.18.5+kina.1`. A sha256 checksum is published alongside the asset. The kernel is NOT stored in git or git-LFS. (kina-8 directive; kina-7 content list)

**Planned zero-step UX (not yet shipped — mark as planned):**

The following distribution UX is planned for a future release (kina-8 directives #1–4, kina-5 §3 Phase B). None of it is implemented today. The current `config.toml` field `node_kernel_path` is `Option<PathBuf>` with no auto-download mode, version field, sha256 field, or fetch logic (see `kina-cli/src/config/mod.rs`).

> **Planned:** `kina create --cni cilium` will fetch the pinned kernel automatically with no additional flags required. On first download, kina will print a one-line notice (`downloading kina kernel kernel-v6.18.5+kina.1 (~32 MB, one time)...`) with progress. The kernel will be cached to `~/.kina/kernels/<tag>/vmlinux`. Download uses an atomic temp-download → sha256-verify → rename sequence; a sha256 mismatch is a hard failure with exact remediation text. An offline error names the GitHub Release asset URL and documents the `--kernel-path` escape hatch. `kina doctor` will report kernel cache status, sha256 match, and stock-kernel fallback status. PTP clusters will never download the kernel. `--kernel-path` remains as the power-user override at all times.

**Today:** `--kernel-path <path-to-vmlinux>` (or `config.toml` `node_kernel_path`) is the only supported path. Download and verify the kernel manually:

```bash
# Verify after download
shasum -a 256 vmlinux
# Expected: f1a40c2c00e8a7f2e2c0165355c13ff6dcdd2742d294babe31dd5c5b14aec3fe
```

## Host-kernel gotchas

These notes apply to users who also manage the Apple Container system kernel directly. They are moot for kina's per-container path (kina never touches the system kernel), but users operating both kina clusters and other containers should be aware.

**Apple Container upgrades never bump an already-installed kernel.** If the system service upgrades to a new version of Apple Container, any kernel already installed via `config.toml [kernel]` or `container system kernel set` remains at the previously installed version. The upgrade does not replace it. (kina-5 §6)

**`container system property list` shows the configured value, not the active symlink.** The property list reports the `[kernel]` table value from `config.toml` — that is, the kernel configured for first-install or `--recommended` use, not necessarily the kernel that was actually installed or is currently active. The ground truth for the active default kernel is the symlink target in `~/Library/Application Support/com.apple.container/kernels/`. During kina-6 verification, `property list` reported `6.18.15-186` while `default.kernel-arm64 -> vmlinux-6.12.28-153` (the symlink pointed to an older kernel). (kina-5 risk #8; kina-6 VERIFIER check #2)

**`config.toml [kernel]` only governs first-install and `--recommended`.** It does not affect containers that pass an explicit `--kernel` flag, which always wins. kina's per-container injection uses `--kernel` exclusively.

## Verification checklist (future kernel bumps)

Use this checklist when bumping the pinned kernel version. It reproduces the acceptance criteria from kina-5 §4 as a maintenance runbook.

### Config probes (in-guest, requires `/proc/config.gz`)

```bash
# All options must appear as =y
zcat /proc/config.gz | grep -E 'CONFIG_(BPF|BPF_JIT|BPF_JIT_ALWAYS_ON|BPF_EVENTS|NET_CLS_BPF|NET_CLS_ACT|NET_SCH_INGRESS|CGROUP_BPF|PERF_EVENTS|SCHEDSTATS|CRYPTO_USER_API_HASH|VXLAN|GENEVE|FIB_RULES|IP_MULTIPLE_TABLES|IPV6_MULTIPLE_TABLES|NETFILTER_XT_TARGET_TPROXY|NETFILTER_XT_TARGET_MARK|NETFILTER_XT_TARGET_CT|NETFILTER_XT_MATCH_MARK|NETFILTER_XT_MATCH_SOCKET|INET_DIAG|INET_UDP_DIAG|INET_DIAG_DESTROY|NET_SCH_FQ|KPROBES|FTRACE|BPF_STREAM_PARSER)='
```

Verify `CONFIG_BPF_JIT=y` specifically on the built kernel (not just the fragment) — this was a hypothesis in kina-5 that phase-A confirmed; it must be re-confirmed on every new build.

### Datapath probes (non-destructive, run in a scratch container)

```bash
# Policy routing
ip rule add pref 32700 lookup 100 && ip rule del pref 32700
ip -6 rule list

# xt_socket transparent
iptables -t mangle -N KINA_PROBE
iptables -t mangle -A KINA_PROBE -m socket --transparent -j RETURN
iptables -t mangle -X KINA_PROBE
# Must succeed with no "unknown option --transparent"

# BPF feature probe
bpftool feature probe kernel

# cgroup2 and BPF mounts
stat -fc %T /sys/fs/cgroup   # must be cgroup2fs
mount | grep /sys/fs/bpf
```

### End-state acceptance

```bash
# No workaround keys in cilium-config ConfigMap
# enable-l7-proxy must be "true"; dnsproxy-enable-transparent-mode must be "true" (no false override)
# NO enable-local-node-route=false, NO xtSocketFallback=false

# Zero errors in cilium-agent log
# (no "socket --transparent" errors, no "address family not supported" for ip rule)

# Cilium healthy
cilium status --wait

# Full-eBPF confirmed
kubectl -n kube-system exec ds/cilium -- cilium-dbg status --verbose
# KubeProxyReplacement: True
# Host Routing: BPF

# Connectivity
cilium connectivity test

# hostPort served by BPF (not kube-proxy)
# kube-proxy DaemonSet must be NotFound
kubectl -n kube-system get ds kube-proxy
```

### Maintenance triggers (kina-2 VERSION POLICY)

Rebuild and cut a new tag when any of the following occur:

- A kernel.org point release is available for the pinned 6.18.x line
- A CVE is issued against the pinned 6.18.x kernel in netfilter, BPF, or virtio subsystems
- The apple/containerization `config-arm64` baseline changes in a way that affects the required option set
- Apple Container ships a major version bump that changes vminitd expectations

Each trigger requires: a version-bump PR, a bees comment on the relevant issue with the reason, a new sha256 for the built artifact, and a new release tag (e.g., `kernel-v6.18.5+kina.2` for the first patch of the same upstream). Tag format follows `kernel-v<upstream>+kina.<n>`. Tags are immutable; new content always gets a new tag. (kina-2 VERSION POLICY; kina-5 §3 Phase C)
