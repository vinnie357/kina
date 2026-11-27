# Cilium and Hubble live probe — issue #101

Date: 2026-11-27. Tracker: [kina#101](https://github.com/vinnie357/kina/issues/101).

**The scoped full Cilium eBPF and node-local Hubble journey passed using the current recommended Kata kernel through a small fork kernel-selection hook.**
Both agents used TCX, full kube-proxy replacement, BPF host routing, and BPF masquerading.
Cross-node/service traffic and HTTP/DNS allow/deny policies passed, with matching Hubble flow verdicts.

The unchanged installed Apple CLI failed earlier with this host's older configured default kernel.
The fork exposes one cluster-wide `--kernel` option applied to every node; it does not change the global kernel default or replace cluster bootstrap.
This does not prove that the unchanged stock CLI works on this host or that Kina is obsolete.
Product decision [#76](https://github.com/vinnie357/kina/issues/76) remains human-owned.

## Final acceptance result

| Check | Phase 1: release binary, default kernel | Phase 2: standalone guest, recommended kernel | Phase 3: fork debug binary, recommended kernel |
|---|---|---|---|
| Two-node creation | Failed before kubeadm | Not a cluster test | Passed, 17.5 seconds |
| Effective full profile on both agents | Not reached | Not a cluster test | Replacement=True; Host=BPF; Masquerading=BPF; Attach=TCX |
| Actual TCX attachment evidence | Not reached | Not a cluster test | 11 control-plane and 9 worker TCX kernel links |
| Cross-node pod HTTP | Not reached | Not a cluster test | Passed |
| ClusterIP service and DNS | Not reached | Not a cluster test | Passed |
| External egress and BPF NAT | Not reached | Not a cluster test | Passed; map contains actual client translation |
| L7 HTTP allow/deny | Not reached | Not a cluster test | Allowed body; denied HTTP403 |
| DNS policy allow/deny | Not reached | Not a cluster test | Allowed address; denied REFUSED |
| Node-local Hubble, client-side correlation | Not reached | Not a cluster test | Matching FORWARDED and DROPPED HTTP/DNS flows |
| Cleanup and original workloads | Passed | Passed | Passed; original eleven ID/status values preserved |

Phase 2 passed nftables rules, BPF map creation, and two program loads; it attached no BPF programs.
Phase 3 changed both the kernel and binary; stock CLI with the tested kernel as its default remains untested.
Hubble Relay/UI, upgrades, persistence, stress, broader protocols, and a live Kina comparison were not tested.

## Scope and baseline

All runtime checks used `/Users/vinnie/github/kina/.worktrees/101-cilium-probe` at revision `87483edbbd8b5796064c0138d8d38cf262a26e76`.
Phase 1 used a clean tree. Phases 2 and 3 used the lead-approved documentation-only untracked report exception.
Branch: `docs/101-cilium-probe`.
The initial report followed phase 1; later sections retain each phase and consolidate the final result.
The operator authorized disposable runtime testing and investigation in their fork, with no upstream writes.

Loaded instructions:

- `/Users/vinnie/.agents/skills/core/SKILL.md`
- `/Users/vinnie/.agents/skills/container/SKILL.md`
- `/Users/vinnie/.agents/skills/security/SKILL.md`
- `/Users/vinnie/.agents/skills/agent-loop/references/evidence.md`
- `docs/agents/worktrees.md`
- `docs/research/apple-cluster-comparison-2026-11-27.md`

The README smoke returned `# kina - Kubernetes in Apple Container`.

Host inventory found 64 GiB memory, 16 logical CPUs, and no Apple clusters.
Eleven unrelated ordinary containers were already present.
The CLI reported `container CLI version 1.7.1 (build: release, commit: a15b460)`.
Host clients reported kubectl `v1.37.1` and Helm `v4.3.1+gc0d08d0`.
Phase 1 did not reach a Kubernetes server. Phase 3 used node-bundled kubectl `v1.35.5` with server `v1.35.5`.

**CLI version and configured guest kernel are separate facts.**
The default diagnostic guest actually booted:

```text
Linux kina101-diag-20261127 6.12.28 #1 SMP Tue May 20 15:19:05 UTC 2025 aarch64 GNU/Linux
```

No host upgrade, global kernel change, or global container service restart occurred.

## Supported creation attempt

Tagged Apple source confirms that a single `KUBECONFIG` path controls creation's merge destination.
`--cni NONE` skips CNI installation and waits for worker registration and API availability.
This avoids waiting for Node Ready before installing Cilium.
Sources: [kubeconfig merge](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Support/ClusterHelper%2BKubeconfig.swift), [bootstrap](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Bootstrapper/ClusterBootstrapper.swift).

The evidence directory was ignored and mode `0700`.
The shell used `umask 077` and this single isolated kubeconfig path:
`$PWD/.altana/runs/101/kubeconfig`.
Creation failed before writing that file.

```sh
container cluster create --name kina101-stock-20261127 \
  --cpus 2 --memory 4G --workers count=1,cpus=2,memory=4G --cni NONE
```

Both the first attempt and one transient retry failed with exit `1`:

```text
Error: node prep failed on kina101-stock-20261127: net.ipv4.ip_forward = 1
registry.k8s.io/pause:3.10.1
```

The error omits the failing command's stderr and guest exit code.
The preparation script tolerates sysctl and pause-tag failures, then runs two fatal TCPMSS commands.
Source: [LinuxNodeProvisioner preparation](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Provisioners/LinuxNodeProvisioner.swift).
The direct diagnostic below identifies their behavior without modifying that implementation.

## Discriminating diagnostic

An owned standalone guest used the exact default cluster node image:
`docker.io/kindest/node:v1.35.5@sha256:ce977ae6d65918d0b58a5f8b5e940429c2ce42fa3a5619ec2bbc60b949c0ac95`.

```sh
container run --detach --name kina101-diag-20261127 --cpus 2 --memory 4G \
  --cap-add ALL --masked-path NONE --read-only-path NONE --entrypoint /bin/sh \
  docker.io/kindest/node:v1.35.5@sha256:ce977ae6d65918d0b58a5f8b5e940429c2ce42fa3a5619ec2bbc60b949c0ac95 \
  -c 'mount --make-rshared /; exec /sbin/init'
```

This matches cluster node capabilities and clears masked/read-only paths, as tagged provisioning does.
It uses no host mounts, shared image store, published ports, or forwarded credentials.
It omits cluster-specific image hook changes and IP reconciliation because this guest does not bootstrap Kubernetes.
Containerd was active, verified by `systemctl is-active containerd`, exit `0`.

Run each following command through `container exec kina101-diag-20261127`:

| Guest command | Exit | Result |
|---|---:|---|
| `/usr/sbin/iptables -t mangle -A OUTPUT -p tcp --tcp-flags SYN,RST SYN -j TCPMSS --set-mss 1220` | 4 | nftables backend failure |
| `/usr/sbin/iptables -t mangle -A FORWARD -p tcp --tcp-flags SYN,RST SYN -j TCPMSS --set-mss 1220` | 4 | Same failure |
| `/usr/sbin/iptables-legacy -t mangle -A OUTPUT -p tcp --tcp-flags SYN,RST SYN -j TCPMSS --set-mss 1220` | 0 | Rule installed |
| `/usr/sbin/iptables-legacy -t mangle -A FORWARD -p tcp --tcp-flags SYN,RST SYN -j TCPMSS --set-mss 1220` | 0 | Rule installed |

Both default-backend failures printed:

```text
Warning: Extension tcp revision 0 not supported, missing kernel module?
Warning: Extension TCPMSS revision 0 not supported, missing kernel module?
iptables v1.8.11 (nf_tables): Could not fetch rule set generation id: Invalid argument
```

`iptables --version` identified `nf_tables`; `/proc/config.gz` explicitly disables `CONFIG_NF_TABLES`.
Legacy succeeds with the same guest, kernel, capability set, rule, and target.
This reproduces a backend mismatch in the diagnostic guest, consistent with the stock preparation failure.
The cluster command itself omitted the failing command and guest exit code.
It does not prove that selecting legacy alone completes cluster creation.

## Kernel capability evidence

`zcat /proc/config.gz` returned exit `0`; the complete decoded configuration remains in private evidence.
Selected exact entries:

```text
CONFIG_BPF=y
CONFIG_BPF_SYSCALL=y
CONFIG_CGROUPS=y
CONFIG_CGROUP_BPF=y
CONFIG_PERF_EVENTS=y
# CONFIG_BPF_JIT is not set
# CONFIG_NET_CLS_BPF is not set
# CONFIG_NET_CLS_ACT is not set
# CONFIG_KPROBES is not set
# CONFIG_VXLAN is not set
# CONFIG_GENEVE is not set
# CONFIG_NF_TABLES is not set
CONFIG_NETFILTER_XT_TARGET_TCPMSS=y
# CONFIG_NETFILTER_XT_MATCH_SOCKET is not set
```

The documented BPF JIT requirement and VXLAN prerequisite imply incompatibility with the intended profile on this configured kernel.
This is a documentation inference: phase 1 never started Cilium.
Missing legacy classifier options alone are not decisive because modern Cilium can use TCX.
The saved configuration also records `NET_INGRESS=y`, `NET_EGRESS=y`, and `NET_XGRESS=y`.
It contains no `DEBUG_INFO_BTF` entry; BTF availability and actual TCX behavior were not established in phase 1.
The missing socket match alone is not decisive: Cilium documents an L7 compatibility fallback.
Sources: [base and tunnel requirements](https://docs.cilium.io/en/stable/operations/system_requirements/), [kube-proxy replacement](https://docs.cilium.io/en/stable/network/kubernetes/kubeproxy-free/).

## Phase 1 acceptance coverage and limits

| Check | Result |
|---|---|
| Installed runtime and resource inventory | Recorded |
| Supported two-node Apple creation with CNI NONE | Failed before kubeadm |
| Bootstrap backend comparison | Default fails; legacy succeeds for both individual rules |
| Actual default kernel configuration | Required base features absent |
| Full Cilium effective status | Not performed; no cluster |
| kube-proxy installation or documented removal | Not performed; kubeadm was not reached |
| Cross-node pod and service traffic | Not performed |
| BPF masquerading and host routing | Not performed |
| L7 HTTP and DNS policy behavior | Not performed |
| Observable Hubble flow | Not performed |
| Equivalent failed check on Kina | Not performed; no working alternative claimed |
| Latest recommended Apple kernel | Not tested; actual configured kernel differs |

The intended profile remains `kubeProxyReplacement=true`, `bpf.masquerade=true`, `bpf.hostLegacyRouting=false`, and `hubble.enabled=true`.
It also requires `ipam.mode=kubernetes`, one operator replica, and the reachable control-plane IP on port `6443`.
These settings were not applied or counted as successful validation.
Cilium's official migration permits removing kube-proxy from an existing disposable cluster before installing its replacement.
No such removal occurred here.

## Evidence and ownership

Raw output and per-command JSON records remain under the ignored `.altana/runs/101/` directory in this worktree.
JSON records contain argv, cwd, revision, tree state, UTC timestamps, integer exit, and raw log path.
Phase 1 records verified the same clean source revision before and after execution.
Later records retain the approved documentation-only exception and fork source checks.
The initial inventory/source reads predate the structured runner and retain tool output rather than equivalent JSON records.

| Record stem | UTC start | Exit |
|---|---|---:|
| `create` | 2026-11-27T14:11:13 | 1 |
| `create-retry` | 2026-11-27T14:12:06 | 1 |
| `kernel` | 2026-11-27T14:12:44 | 0 |
| `tcpmss-output` | 2026-11-27T14:12:44 | 4 |
| `legacy-output` | 2026-11-27T14:13:01 | 0 |
| `kernel-config-full` | 2026-11-27T14:13:21 | 0 |
| `tcpmss-forward` | 2026-11-27T14:13:21 | 4 |
| `legacy-forward` | 2026-11-27T14:13:21 | 0 |
| `diag-cleanup` | 2026-11-27T14:13:22 | 0 |
| `clusters-final` | 2026-11-27T14:13:22 | 0 |
| `containers-final` | 2026-11-27T14:13:22 | 0 |

Apple automatically removed failed cluster nodes and the failed cluster record.
`container rm -f kina101-diag-20261127` removed the sole owned diagnostic container, exit `0`.
Final `container cluster list --nodes --format json` returned `[]`, exit `0`.
Final ordinary container inventory retained the same eleven IDs and status values as the original inventory.
No owned runtime resource remains running.
The runtime may retain downloaded image content; no shared image pruning occurred.

Phase 1 motivated the isolated recommended-kernel test below.
A bootstrap fallback alone cannot supply the missing features in the older configured kernel.

## Phase 2 — isolated current recommended kernel

The lead authorized one additional owned guest with an explicitly selected kernel.
The global default remained unchanged.
The lead accepted the untracked initial Markdown report as a documentation-only exception to clean-tree execution evidence.
Each phase 2 JSON record retains that exception and both tree snapshots.

Apple Container 1.7.1 explicitly selects Kata 4.3.1 and `vmlinux-6.18.56-208-debug` as its defaults.
Source: [tagged kernel defaults](https://github.com/apple/container/blob/1.7.1/Sources/ContainerPersistence/ContainerSystemConfig.swift#L202-L204).
The tested artifact is that debug variant; no non-debug kernel equivalence is claimed.
The lead downloaded and verified the Kata 4.3.1 ARM64 archive before extraction:

- Archive size: `676287269` bytes.
- Archive SHA256: `2c5bf7cde334ad5307d482f45cceda496f91c9b6b96f5278c47f4f4eac1bbae6`.
- Extracted member: `opt/kata/share/kata-containers/vmlinux-6.18.56-208-debug`.
- Kernel size: `30564864` bytes.
- Kernel SHA256: `98a1005f8cc395e99b85775b9dce2b239190520b102bf549f4225104dcd7f511`.
- Lead verification metadata: `/tmp/kina101/kernel/verified.json`.

Creation used the phase 1 standalone command with name `kina101-kata-20261127` and this additional supported option:

```sh
--kernel /tmp/kina101/kernel/opt/kata/share/kata-containers/vmlinux-6.18.56-208-debug
```

The guest reported:

```text
Linux kina101-kata-20261127 6.18.56 #1 SMP Fri Oct 16 18:41:09 UTC 2026 aarch64 GNU/Linux
```

Both original nftables TCPMSS rules succeeded, exit `0`.
This eliminates the phase 1 backend mismatch for those rules under this selected kernel.
It does not establish complete cluster bootstrap.

Selected configuration evidence:

```text
CONFIG_BPF_SYSCALL=y
CONFIG_BPF_JIT=y
CONFIG_BPF_EVENTS=y
CONFIG_CGROUP_BPF=y
CONFIG_KPROBES=y
CONFIG_IP_MULTIPLE_TABLES=y
CONFIG_NF_TABLES=y
CONFIG_NETFILTER_XT_MATCH_SOCKET=y
CONFIG_VXLAN=y
CONFIG_DEBUG_INFO_BTF=y
CONFIG_NET_INGRESS=y
CONFIG_NET_EGRESS=y
CONFIG_NET_XGRESS=y
# CONFIG_NET_CLS_BPF is not set
# CONFIG_NET_CLS_ACT is not set
# CONFIG_GENEVE is not set
# CONFIG_NETKIT is not set
```

`CONFIG_NET_SCH_INGRESS` was absent from the complete decoded configuration.
`tc qdisc add dev lo clsact` failed with exit `2`:

```text
Error: Cannot find ingress queue for specified device.
```

A follow-up assumed the primary device was `eth0` without inventorying its name.
That command failed with `Cannot find device "eth0"`, exit `1`, and provides no TC capability evidence.

### Actual BPF operations

A standard-library-only Go probe was built for Linux/ARM64 in the ignored evidence directory.
The binary ran only inside the owned guest.
It created one hash map and loaded two minimal programs, closing successful file descriptors without attaching programs.

```text
BPF_MAP_CREATE hash: fd=3 error=<nil>
BPF_PROG_LOAD SCHED_CLS: fd=3 error=<nil>
BPF_PROG_LOAD CGROUP_INET4_CONNECT: fd=3 error=<nil>
```

The probe exited `0`.
This proves those minimal map/program loads, not Cilium dataplane behavior or attachment support.

Reproduction artifacts:

- Source: `.altana/runs/101/bpf-probe.go`.
- Source SHA256: `19d01b334576153e8b094403c235f0e5b9493be014cbfcb7b0932be45761b412`.
- Binary SHA256: `e68418658c084d79449b5b88e2af59aba92dacdc413cda516cc220db804aa09a`.
- Build: `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 GOCACHE="$PWD/.altana/runs/101/go-cache" go build -o .altana/runs/101/bpf-probe .altana/runs/101/bpf-probe.go`.
- Copy: `container cp .altana/runs/101/bpf-probe kina101-kata-20261127:/usr/local/bin/bpf-probe`.
- Mode: `container exec kina101-kata-20261127 chmod 755 /usr/local/bin/bpf-probe`.
- Execute: `container exec kina101-kata-20261127 /usr/local/bin/bpf-probe`.

The initial copy to `/tmp` returned success but was invisible inside the running guest.
Copying to `/usr/local/bin` made the file visible; executable mode then required the explicit chmod above.
The failed execution attempts remain in private logs and do not count as BPF test results.

### Phase 2 interpretation and cleanup

The current recommended kernel supplies several capabilities absent from the configured default.
It still lacks the documented legacy TC classifier/action prerequisites, with a matching clsact failure.
However, current Cilium enables TCX on supported kernels and the kernel exposes NET_XGRESS.
Cilium 1.20.5 startup accepts a successful TCX probe when the legacy TC probe fails and TCX is enabled.
Its TCX probe loads a SchedCLS program and attaches a TCX ingress link in a temporary network namespace.
A missing legacy TC feature therefore does not establish a startup blocker for the intended profile.
At the end of phase 2, TCX attachment and Cilium startup remained untested. Phase 3 tested both.
Sources: [startup requirements](https://github.com/cilium/cilium/blob/v1.20.5/pkg/datapath/linux/requirements.go#L48-L60), [TCX probe](https://github.com/cilium/cilium/blob/v1.20.5/pkg/datapath/linux/probes/probes.go#L252-L285), [TCX chart option](https://github.com/cilium/cilium/blob/v1.20.5/install/kubernetes/cilium/values.yaml).

No fork code change or cluster bootstrap occurred during this phase.
Those full-profile checks remained unqualified at the end of phase 2. Their phase 3 results follow.

| Record stem | UTC start | Exit |
|---|---|---:|
| `phase2-create` | 2026-11-27T14:18:13 | 0 |
| `phase2-kernel` | 2026-11-27T14:18:25 | 0 |
| `phase2-tcpmss-output` | 2026-11-27T14:18:25 | 0 |
| `phase2-tcpmss-forward` | 2026-11-27T14:18:36 | 0 |
| `phase2-ingress` | 2026-11-27T14:18:56 | 2 |
| `phase2-bpf-result` | 2026-11-27T14:19:06 | 0 |
| `phase2-ingress-eth0` | 2026-11-27T14:19:27 | 1 |
| `phase2-cleanup` | 2026-11-27T14:19:27 | 0 |
| `phase2-clusters-final` | 2026-11-27T14:19:28 | 0 |
| `phase2-containers-final` | 2026-11-27T14:19:28 | 0 |

`container rm -f kina101-kata-20261127` succeeded.
The final Apple cluster inventory remained empty.
The original eleven ordinary container IDs and status values remained unchanged.
No worker-owned runtime resource remains running.
Downloaded kernel and private evidence artifacts remain for the lead's investigation.

## Phase 3 — full profile through the owner fork

The reviewed owner fork added optional cluster kernel selection at Apple Container 1.7.1's baseline.
`git rev-parse 1.7.1` returned `a15b46097aa4c2325da13c1fb5192b9274aab083`, matching the installed CLI's reported `a15b460`.
The runtime used a debug build of the fork, not the installed release executable.
It forwards the same explicit kernel to the control plane and worker.
The source tree was clean at commit `2cd07909e74fe3a3974a9f3e65cbbe5d765e4574`.
The lead reported passing fork CI: build and 95 tests.
The worker verified the clean commit and binary/kernel hashes before further dependent checks.
Source: [measured owner fork commit](https://github.com/vinnie357/container/commit/2cd07909e74fe3a3974a9f3e65cbbe5d765e4574).
No upstream issue, pull request, or other upstream write occurred.

The standalone fork binary was:
`/Users/vinnie/github/kina/.worktrees/apple-container/.worktrees/101-kernel-selection/.build/out/Products/Debug/cluster`.
Its SHA256 was `b75c286338a515d2ef83d20162b6aa25aeacdc006caa038109873b43af0c99f5`.
The kernel retained phase 2's verified SHA256.

### Creation, migration, and exact Cilium profile

The shell used `umask 077` and isolated `KUBECONFIG=$PWD/.altana/runs/101/kubeconfig-fork`.
The resulting credential file was mode `0600`; its parent remained mode `0700`.

```sh
# Invoke the verified standalone fork binary, not the installed plugin.
cluster create --name kina101-fork-20261127 --cpus 2 --memory 4G \
  --workers count=1,cpus=2,memory=4G --cni NONE \
  --kernel /tmp/kina101/kernel/opt/kata/share/kata-containers/vmlinux-6.18.56-208-debug
```

Creation passed in 17.5 seconds.
Nodes reported Kubernetes `v1.35.5`, kernel `6.18.56`, and containerd `2.3.1`.
The control-plane address was `192.168.64.148`; the worker address was `192.168.64.149`.
Their actual interface name was `enp0s1`.

The compatible kubectl command below ran inside only the owned control plane:

```sh
container exec kina101-fork-20261127 /bin/kubectl \
  --kubeconfig=/etc/kubernetes/admin.conf --request-timeout=20s ...
```

The following examples abbreviate that full command as `K`.
Kube-proxy was present and Ready `2/2` before migration.
These documented migration commands all passed:

```sh
K -n kube-system delete ds kube-proxy
K -n kube-system delete cm kube-proxy
# Run separately on each owned node only:
container exec kina101-fork-20261127 /bin/sh -c 'iptables-save | grep -v KUBE | iptables-restore'
container exec kina101-fork-20261127-worker-1 /bin/sh -c 'iptables-save | grep -v KUBE | iptables-restore'
```

The isolated host kubeconfig then selected only this cluster for Helm:

```sh
helm upgrade --install cilium cilium --repo https://helm.cilium.io/ \
  --version 1.20.5 --namespace kube-system \
  --set ipam.mode=kubernetes --set kubeProxyReplacement=true \
  --set bpf.masquerade=true --set bpf.hostLegacyRouting=false \
  --set bpf.enableTCX=true --set hubble.enabled=true \
  --set operator.replicas=1 \
  --set k8sServiceHost=192.168.64.148 --set k8sServicePort=6443
```

The install passed. A bounded 180-second Cilium rollout wait passed.
Both nodes became Ready.
Both agents, both Envoy instances, the operator, CoreDNS, and fixtures were Ready with zero restarts before teardown.
A final kube-proxy label query returned no pods.

### Effective modes and actual traffic

`cilium-dbg status --verbose` on each agent reported:

| Effective status | Control plane | Worker |
|---|---|---|
| KubeProxyReplacement | True | True |
| Routing | Tunnel `[vxlan]`, Host `BPF` | Tunnel `[vxlan]`, Host `BPF` |
| Attach Mode | TCX | TCX |
| Masquerading | BPF on `enp0s1`, IPv4 enabled | BPF on `enp0s1`, IPv4 enabled |
| Hubble | Ok | Ok |

A read-only syscall helper independently enumerated existing BPF links.
It created no maps, programs, or attachments.
The control plane had 11 links of type `BPF_LINK_TYPE_TCX`; the worker had 9.
For example, control-plane link 19 referenced program 320, ifindex 2, TCX egress attach type 47.
Worker link 19 referenced program 285, ifindex 2, attach type 47.
Source: [Linux BPF link ABI](https://github.com/torvalds/linux/blob/v6.18/include/uapi/linux/bpf.h).
Helper source SHA256: `93430bc7512d8ab0e39cedfef8d6c4639ede7efc316a2de7791ee55b30c19a5c`.
Helper binary SHA256: `ccf5c37a1be924cf00ca00b42e57cc92fc2e98e8863814b6822e338df914a6a6`.
Its source and build output remain in private evidence as `bpf-links.go` and `bpf-links`.

The client pod ran on the control plane at `10.244.0.75`.
The server ran on the worker at `10.244.1.122`.
The echo service address was `10.97.201.54`.
Before policy installation, these client commands passed:

```sh
curl --fail --max-time 10 http://10.244.1.122:8080/allowed
curl --fail --max-time 10 http://echo.kina101.svc.cluster.local:8080/allowed
curl --fail --max-time 15 https://example.com/
```

The first two returned `allowed`; the third returned the external page successfully.
Cilium's actual BPF NAT map contained the client egress translation and reverse mapping:

```text
TCP OUT 10.244.0.75:55298 -> 104.20.23.154:443 XLATE_SRC 192.168.64.148:55298 Created=84sec ago NeedsCT=0
TCP IN 104.20.23.154:443 -> 192.168.64.148:55298 XLATE_DST 10.244.0.75:55298 Created=84sec ago NeedsCT=0
```

This combines effective BPF mode, actual cross-node/service traffic, and actual translated external traffic.
It does not qualify every routing or masquerading edge case.

### Policy results and Hubble correlation

The server served both `/allowed` and `/denied` as ordinary files.
The client-only policy allowed HTTP GET `/allowed` and DNS for the echo service name.
It withheld those permissions for `/denied` and the Kubernetes service DNS name.
Both agents imported `client-policy` and passed bounded realized-policy waits before testing.

| Client check | Concrete result | Test exit |
|---|---|---:|
| HTTP `/allowed` with `curl --fail` | Body `allowed` | 0 |
| HTTP `/denied` | `HTTP=403` | 0, assertion passed |
| `nslookup echo.kina101.svc.cluster.local.` | Address `10.97.201.54` | 0 |
| `nslookup kubernetes.default.svc.cluster.local.` | `REFUSED`, `NSLOOKUP_EXIT=1` | 0, expected failure assertion passed |

Node-local Hubble queries used the CLI inside each Cilium agent:

```sh
K -n kube-system exec CILIUM_POD -c cilium-agent -- \
  hubble observe --last 100 --from-pod kina101/client -o json
```

The image defaults Hubble to its local Unix socket.
The control-plane flow records correlated the same fixture source with these actual tests:

| UTC timestamp, 2026-11-27 | L7 request | Verdict |
|---|---|---|
| 14:35:38.471352466 | GET `http://10.244.1.122:8080/allowed` | FORWARDED |
| 14:35:38.639226967 | GET `http://10.244.1.122:8080/denied` | DROPPED |
| 14:35:38.798328675 | DNS `echo.kina101.svc.cluster.local.`, A | FORWARDED |
| 14:35:38.961060050 | DNS `kubernetes.default.svc.cluster.local.`, A | DROPPED |

The HTTP flows identified `kina101/client` and `kina101/server`.
The DNS flows identified `kina101/client` and CoreDNS, with proxy observation source.
This validates Hubble flow observability; Relay/UI were not deployed or tested.

### Reproduction fixtures and image identities

Actual runtime imageIDs, including both agents and both Envoy instances, were retained before cleanup:

| Component | Actual imageID |
|---|---|
| Cilium agents | `quay.io/cilium/cilium@sha256:aa72499f075ad729947ea91b026218aceba24f1a8e4534bf36b8368c7c561526` |
| Envoy | `quay.io/cilium/cilium-envoy@sha256:bdef13043ebaa7b9dcd8e3a86be41080bb5369e017cd759a12daf89e2b8fafe7` |
| Operator | `quay.io/cilium/operator-generic@sha256:c6e7de6665aa9701db86c4fe46828a75a0d167afe084a9ffb5bce34b2c4fa8c4` |
| Client curl | `docker.io/curlimages/curl@sha256:94e9e444bcba979c2ea12e27ae39bee4cd10bc7041a472c4727a558e213744e6` |
| Server and DNS client | `docker.io/library/busybox@sha256:a7dbc319b1438115a207e11930dc4bb3e643dcb3ec0bdd694bb36cd8fad289ec` |

The exact rendered fixture and policy files remain in private evidence.
The sanitized manifests follow; creation used `container cp` to `/root` and node-bundled kubectl `apply -f`.
The first stdin-based apply passed no objects because `container exec` lacked `-i`; the file-copy path corrected the adapter.
That preparation failure provides no product dataplane evidence.

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: kina101
---
apiVersion: v1
kind: Pod
metadata:
  name: server
  namespace: kina101
  labels: {app: server}
spec:
  nodeName: kina101-fork-20261127-worker-1
  containers:
  - name: server
    image: busybox:1.37.0
    command: [sh, -c, 'mkdir -p /www; echo allowed > /www/allowed; echo denied > /www/denied; exec httpd -f -p 8080 -h /www']
    ports: [{containerPort: 8080}]
    resources:
      requests: {cpu: 10m, memory: 16Mi}
      limits: {memory: 64Mi}
---
apiVersion: v1
kind: Service
metadata:
  name: echo
  namespace: kina101
spec:
  selector: {app: server}
  ports: [{port: 8080, targetPort: 8080}]
---
apiVersion: v1
kind: Pod
metadata:
  name: client
  namespace: kina101
  labels: {app: client}
spec:
  nodeName: kina101-fork-20261127
  containers:
  - name: curl
    image: curlimages/curl:8.12.1
    command: [sh, -c, 'sleep 3600']
    resources:
      requests: {cpu: 10m, memory: 16Mi}
      limits: {memory: 64Mi}
  - name: dns
    image: busybox:1.37.0
    command: [sh, -c, 'sleep 3600']
    resources:
      requests: {cpu: 10m, memory: 16Mi}
      limits: {memory: 64Mi}
```

```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: client-policy
  namespace: kina101
spec:
  endpointSelector:
    matchLabels: {app: client}
  egress:
  - toEndpoints:
    - matchLabels: {k8s:io.kubernetes.pod.namespace: kube-system, k8s:k8s-app: kube-dns}
    toPorts:
    - ports: [{port: '53', protocol: ANY}]
      rules:
        dns:
        - matchName: echo.kina101.svc.cluster.local
  - toEndpoints:
    - matchLabels: {app: server}
    toPorts:
    - ports: [{port: '8080', protocol: TCP}]
      rules:
        http:
        - method: GET
          path: /allowed
```

### Phase 3 evidence and final cleanup

Every structured phase 3 check retained argv, cwd, source state, timestamps, integer exit, and private raw log path.
The fork commit was pinned, clean, and unchanged across checks that recorded it.
The creation record predates the runner's fork-state fields; separate source/hash inspection verified the running binary's provenance.
The documentation-only dirty-tree exception was explicitly accepted by the lead for this phase.

| Evidence stem | UTC start, 2026-11-27 | Exit |
|---|---|---:|
| `phase3-create` | 14:31:40 | 0 |
| `phase3-flows-cilium-9l6jr` | 14:35:48 | 0 |
| `phase3-helm-install` | 14:32:20 | 0 |
| `phase3-link-query-cp` | 14:34:55 | 0 |
| `phase3-link-query-worker` | 14:35:08 | 0 |
| `phase3-nat-map` | 14:36:07 | 0 |
| `phase3-policy-realized-cilium-9l6jr` | 14:35:16 | 0 |
| `phase3-policy-realized-cilium-dfm8w` | 14:35:17 | 0 |
| `phase3-proxy-config-delete` | 14:32:17 | 0 |
| `phase3-proxy-delete` | 14:32:17 | 0 |
| `phase3-proxy-iptables-kina101-fork-20261127-worker-1` | 14:32:19 | 0 |
| `phase3-proxy-iptables-kina101-fork-20261127` | 14:32:18 | 0 |
| `phase3-status-cilium-9l6jr` | 14:33:48 | 0 |
| `phase3-status-cilium-dfm8w` | 14:33:48 | 0 |
| `phase3-cilium-rollout` | 14:32:31 | 0 |
| `phase3-direct-cross-node` | 14:34:42 | 0 |
| `phase3-cluster-service` | 14:34:43 | 0 |
| `phase3-external-egress` | 14:34:43 | 0 |
| `phase3-http-allow` | 14:35:38 | 0 |
| `phase3-http-deny` | 14:35:38 | 0 |
| `phase3-dns-allow` | 14:35:38 | 0 |
| `phase3-dns-deny` | 14:35:38 | 0 |
| `phase3-flows-cilium-dfm8w` | 14:35:48 | 0 |
| `phase3-cleanup` | 14:36:50 | 0 |
| `phase3-clusters-final` | 14:37:02 | 0 |
| `phase3-containers-final` | 14:37:02 | 0 |

Cleanup used only `container cluster delete --name kina101-fork-20261127` with its isolated `KUBECONFIG`.
Deletion passed; the private kubeconfig file was then removed.
Final Apple cluster inventory was `[]`.
Final ordinary inventory retained the exact original eleven IDs and status values.
No worker-owned runtime resource or active runtime command remains.
Downloaded images, kernel artifacts, and private evidence remain; no shared pruning occurred.

In one run, the kernel hook plus documented kube-proxy migration and explicit Helm configuration enabled the scoped journey.
The hook preserved the global default; it was not shown necessary for a host already configured with the tested kernel.
The run does not demonstrate a need for a separate cluster lifecycle or a custom-built kernel.
It leaves the product choice among retirement, narrowing, integration, and independent lifecycle to issue #76.
It does not establish a working live Kina comparison or authorize runtime upgrades, library extraction, or upstream publication.

Final worker checks: `mise run core:secrets` passed with exit `0` after credential cleanup and without cache warnings.
No tracked executable source changed in the Kina report worktree; the lead owns final documentation gates and publication.

## Additional limits from independent review

This was one short-lived, IPv4 cluster run; the creation duration is one observation.
Cross-node traffic ran from the control-plane client to the worker server, not in both directions.
DNS traffic was not separately qualified as cross-node traffic.
NodePort, host-namespace service access, and service-map contents were not tested.
BPF host routing was reported by both agents; no independent host-routing instrumentation was collected.
The denied URL and forbidden DNS name lacked pre-policy control requests.
HTTP403, DNS REFUSED, and correlated Hubble drops support policy attribution despite that missing control.
DNS policy means DNS-proxy name filtering here; `toFQDNs` egress was not tested.
TCX link counts were not fully mapped by interface and direction.
Relay aggregation requires another acceptance decision if cluster-wide Hubble is required.
Issue #101 explicitly excluded Relay APIs; node-local evidence does not qualify them.

## Build runner follow-up

The measured runtime commit remains `2cd0790`.
Follow-up [1735286](https://github.com/vinnie357/container/commit/17352864986eedf9c0512c36efe4bfd00df227e4) changes only the build runner.
Swift persisted inherited credential-shaped values in ignored plugin-state caches during the earlier build.
Those generated files were removed without printing values; outgoing file and commit scans were clean.
The runner now clears inherited variables and passes only required paths plus optional Xcode selection.
Independent review approved it; strict build, 95 unit tests, and the regenerated cache scan passed.
Any real credentials captured in the earlier local caches should be rotated.

## Council findings review

Fable and Astra reviewed the report and fork diff read-only. Carbon reached its configured deadline without an answer.
The council command exited 1 because `--require-all` was unsatisfied; no unanimous approval is claimed.
Carbon's log shows successful reads followed by interrupted generation, not a completed review.

| Harness | Status | Duration | Assessment |
|---|---|---:|---|
| Fable | answered | 118.005 s | Sound scoped evidence; required wording and evidence-index corrections |
| Astra | answered | 40.410 s | Accepts the scoped two-node pass; broader conclusions remain blocked |
| Carbon | failed, timeout | 605.089 s | No answer; excluded from synthesis |

The report incorporates the completed reviewers' corrections.
It identifies one cluster-wide kernel option, exact Apple defaults, binary differences, and diagnostic inference limits.
It indexes both agents' status and Hubble queries, TCX links, NAT, migration, installation, and policy realization.
The observed Hubble verdicts remain explicitly client-side; worker query success does not establish Relay aggregation.

Both completed reviewers support considering a narrow integration, without treating it as product approval.
Neither finds demonstrated need for a custom kernel or separate lifecycle for the measured journey.
Both reject claims that the fork is necessary, Kina is obsolete, or Kina is already a proven working alternative.

Their proposed next questions differ.
Fable prioritizes Relay aggregation; Astra prioritizes unchanged stock CLI with the exact tested kernel already configured.
Relay APIs were outside this issue; global kernel changes were also not authorized.
The operator should decide which remaining user journey matters in #76 before another runtime experiment.

Full raw council responses remain in private evidence at `/tmp/kina101/council-findings.json`.
Carbon's failed run log is `/var/folders/60/_jrn5kb943jcp0n061nkwx080000gn/T/altana/d77c4fb5751504b8/carbon.log`.
The operator was asked whether to authorize a new bounded Carbon run with a shorter brief.
