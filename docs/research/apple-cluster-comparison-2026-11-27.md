# Kina versus Apple Container clusters

Research date: **2026-11-27**. Kina baseline: **`a9b36123eccb0a12138c8bd8e3c7aa4ae9411f46`**, inspected in the issue #73 worktree.

**Recommendation — investigate whether Kina should retire, narrow, integrate, or retain an independent lifecycle.** Apple now covers the original basic cluster proposition. Confirm an active consumer need before testing differentiation or implementing an adapter.

This is a source comparison, not a runtime qualification. No host runtime changes, upgrades, builds, or live cluster tests were performed.

## What changed upstream

Apple’s latest published release observed today is **Container 1.7.1, released November 6, 2026**. Kubernetes initially shipped as `container k8s` in 1.2.1 on August 7. Multi-worker support shipped in 1.6.0 on October 20. Version 1.6.1 renamed it `container cluster` on October 23. Version 1.7.1 adds grouped worker resources and optional access to the host image store, removing `load-image`. It also changes cluster listing and prevents ordinary container lifecycle commands from managing cluster nodes. The feature remains experimental. [Apple release history](https://github.com/apple/container/releases), [tagged cluster command reference](https://github.com/apple/container/blob/1.7.1/docs/command-reference.md#container-cluster-management)

The earlier June proposal used k3s. That discussion does **not** describe the released architecture: current clusters use upstream Kubernetes, `kindest/node`, and `kubeadm`. [Original proposal](https://github.com/apple/container/discussions/1673), [current cluster creation source](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Commands/ClusterCreate.swift)

## Capability comparison

| Capability | Apple Container 1.7.1 | Kina at inspected baseline | Assessment |
|---|---|---|---|
| Native Apple Container Kubernetes | Implemented | Implemented | Strong overlap |
| Single control plane plus workers | `--workers`; repeated groups support different CPU/memory allocations | Single node or control plane plus workers | Multiple nodes no longer differentiate Kina |
| Kubernetes/node image selection | Custom `--node-image`; source default pins kindest 1.35.5 by digest | Default kindest 1.36.1; node image builder also exists | Version choice alone is weak differentiation |
| CNI selection | Bundled kindnet, custom manifest, or `--cni NONE` | PTP or integrated Cilium installation | Kina packages a specific operational workflow |
| Per-node custom kernel | Core `container run --kernel` exists; cluster create exposes no kernel flag | Kernel passed to control plane and workers; Cilium kernel cache/download/verification | Concrete integration gap, subject to latest stock-kernel tests |
| Full Cilium eBPF profile | No dedicated Cilium profile found in inspected cluster CLI | Kube-proxy replacement, BPF masquerading/routing, Hubble enabled | Strongest technical difference |
| Image development loop | Optional shared local image store; otherwise registry pulls | Explicit export, verified transfer, import into each node | Both avoid registry pushes; different isolation semantics |
| Addons/Gateway API | Inspected cluster CLI has create/delete/list/write-config; addons installed with Kubernetes tooling | Integrated nginx-ingress, Traefik, NGINX Gateway Fabric, metrics-server, demo | Convenience and validation difference |
| End-to-end verification | Standard Kubernetes tools documented | `kina verify` combines readiness/Cilium/HTTP evidence | Useful higher-level workflow |
| Agent/Relay integration | Kubernetes API and machine-readable cluster listings available | Existing Rust library exports, CLI, JSON surfaces | No Relay-specific implementation found in this repository |

Apple evidence: [tagged command reference](https://github.com/apple/container/blob/1.7.1/docs/command-reference.md#container-cluster-management), [cluster guide](https://github.com/apple/container/blob/1.7.1/docs/cluster.md), [create flags](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Commands/ClusterCreate.swift), [digest-pinned node default](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/ClusterHelper.swift#L38).

Kina evidence:

- Cluster topology: [apple_container.rs:924](../../kina-cli/src/core/apple_container.rs#L924).
- Cilium profile: [apple_container.rs:103](../../kina-cli/src/core/apple_container.rs#L103).
- Kernel injection: [apple_container.rs:1235](../../kina-cli/src/core/apple_container.rs#L1235).
- Kernel resolution/download: [cluster.rs:247](../../kina-cli/src/cli/cluster.rs#L247).
- Image loading: [apple_container.rs:2010](../../kina-cli/src/core/apple_container.rs#L2010), transfer verification/import at [apple_container.rs:2136](../../kina-cli/src/core/apple_container.rs#L2136).
- Addons: [cluster.rs:214](../../kina-cli/src/cli/cluster.rs#L214).
- Verification entry point: [cluster.rs:1444](../../kina-cli/src/cli/cluster.rs#L1444).
- Node build versions: [node_image_builder.rs:34](../../kina-cli/src/core/node_image_builder.rs#L34).
- Existing library exports: [lib.rs:1](../../kina-cli/src/lib.rs#L1).

## Differences that matter

**Cilium and kernel integration remain the strongest reason to retain Kina.** Kina selects a custom kernel and configures kubeadm to omit kube-proxy when using its full eBPF profile. Apple’s custom-CNI option is an integration hook, not evidence that the equivalent profile works. Apple’s cluster creation flags do not expose custom kernel or kubeadm phase configuration. [Apple create source](https://github.com/apple/container/blob/1.7.1/Sources/ContainerCluster/Commands/ClusterCreate.swift), [Kina kubeadm argument selection](../../kina-cli/src/core/apple_container.rs#L275)

**The underlying kernel gap must be revalidated.** Kina documents probing the Apple **1.0.0** default kernel and finding missing options. Apple 1.7.1 now recommends Kata **4.3.1 / Linux 6.18.56-208-debug**. Historical missing options do not prove that today’s stock kernel still fails. Kina pins **6.18.5-kina.1**, so owning that kernel also entails maintenance. [Kina historical probe](../development/custom-kernel.md#L9), [Apple kernel update](https://github.com/apple/container/pull/2447), [Kina pinned kernel](../../kina-cli/src/core/kernel_fetch.rs#L30)

**Image loading is no longer a unique capability.** Apple’s shared-store mode lets pods use images built or pulled locally. Sharing is optional, reads the complete host image store, and prefers local images over remote images. Kina’s explicit per-node import offers a narrower transfer boundary. That is a potentially useful product choice, not proof of overall superiority. [Apple shared-store guide](https://github.com/apple/container/blob/1.7.1/docs/cluster.md#use-container-images-in-your-cluster), [implementation change](https://github.com/apple/container/pull/2437)

**Addons and verification can survive a backend change.** Their value comes from installation order, readiness checks, configuration, and usable failure evidence. They need not inherently own node creation. However, current Kina discovery filters `io.kina.cluster` labels from ordinary `container list`, while Apple hides its cluster nodes there. Integration therefore requires a deliberate adapter; current discovery cannot simply adopt Apple clusters. [Kina discovery](../../kina-cli/src/core/apple_container.rs#L1630), [Apple lifecycle and discovery behavior](https://github.com/apple/container/blob/1.7.1/docs/cluster.md#cluster-nodes-and-container-commands)

**Relay is a consumer hypothesis, not established differentiation.** A repository search found no agent Relay integration; the sole relay mention concerned Cilium Hubble configuration. Assuming Relay needs isolated Kubernetes environments, either backend can potentially satisfy that need. The deciding contract should specify environment lifecycle, artifact handling, endpoints, diagnostics, cancellation, and ownership. Extracting a library before settling that contract risks preserving duplicated infrastructure.

## Options

| Option | Simplest scope | When justified |
|---|---|---|
| Stop | Keep existing releases; document Apple migration | Consumers only need standard local clusters and image development |
| Narrow | Retain Cilium/kernel profiles, addon installation, verification, and diagnostics | These workflows save demonstrated work |
| Integrate — recommended investigation | Apple owns ordinary clusters; Kina adds profiles and validation | Live tests show required customization can be supported cleanly |
| Continue independent lifecycle | Maintain current backend for a defined unsupported requirement | Latest Apple cannot satisfy an important, tested requirement |

Do not make a library extraction the strategic decision. Establish the consumer contract and backend feasibility first, then extract the smallest stable interface. Apple itself publishes a Swift `ContainerCluster` library; that is evidence of a library boundary upstream, not a ready-made Rust integration or stability promise. [Apple package manifest](https://github.com/apple/container/blob/1.7.1/Package.swift#L53)

## Smallest next decision and experiment

First ask the operator which concrete outcome requires Kina beyond ordinary local Kubernetes.
Keep retirement on equal footing with narrowing, integration and continued independent lifecycle.
If no such outcome exists, validate the standard Apple journey before choosing retirement.

When full Cilium eBPF is required:

1. Inventory the installed runtime and existing clusters without changing them. Do not upgrade the host implicitly.
2. Inspect the actual stock kernel configuration or BPF features against Kina's required options.
3. If kernel capabilities permit it, create one disposable Apple cluster with one control plane and one worker, using `--cni NONE`.
4. Through supported interfaces, test the required Cilium profile and cross-node traffic. Record the first failing step.
5. Repeat a failed required check on existing Kina before claiming that Kina provides a working alternative.

Use two workers only when the consumer needs worker-to-worker behavior.
Add Gateway, image isolation or Relay checks only when they decide a stated consumer requirement.
Record versions, commands, ownership and teardown evidence in GitHub. No live probe has been performed.
Missing kernel options identify a feasibility blocker, not automatic justification for maintaining Kina.

Broader addon matrices, upgrades, library extraction and adapter implementation remain behind the operator's decision.
Existing Cilium tests are pure and do not qualify these live journeys.

## Research limits and documentation discrepancy

The README still describes automatic Cilium kernel download as planned, while the inspected CLI implements it. Use implementation evidence when assessing present capabilities. [README:425](../../README.md#L425), [implemented resolution](../../kina-cli/src/cli/cluster.rs#L247)

Official issue pages `vinnie357/kina#72` and `#73` returned fetch cache misses. Their detailed contracts were not independently verified. Issue #72’s proposed library extraction and Relay context were supplied in the research brief.

Skills loaded: core and research. README smoke check returned `# kina - Kubernetes in Apple Container`. No files were edited during research; this report was subsequently captured at the lead’s request.
