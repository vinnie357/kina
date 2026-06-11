# `container cp` exits 0 on stopped container but with wrong error (or possible earlier silent-fail)

## Summary

During kina-8 development work (2026-06-11), `container cp` of a large image tarball (~hundreds of MB)
into a running container returned exit 0 but the file was not present inside the container. A workaround
using `cat <tar> | container exec -i <name> sh -c 'cat > <dest>'` succeeded.

A structured reproduction attempt on the same host and CLI version (2026-06-11, same session) did NOT
reproduce the silent-failure for any payload size (1KB, 1MB, 50MB, 300MB) or for the exact original
shape (container image tarball, 3.8MB). All cells passed with correct sha256 matches.

This document records the non-reproduction and a secondary finding: `container cp` silently creates
non-existent intermediate directories rather than returning an error (exit 0, directory created).

---

## Environment

- **macOS**: 26.3.1 (Build 25D771280a, ProductVersionExtra: (a))
- **container CLI**: version 1.0.0 (build: release, commit: ee848e3)
- **Hardware**: Apple silicon (arm64)

---

## Original Bug Report (2026-06-11, kina-8 session)

```
container cp /tmp/kina8-cilium-img.tar kina-test-worker:/tmp/cilium-img.tar
# exit 0, file absent inside container
```

Workaround that succeeded:
```
cat /tmp/kina8-cilium-img.tar | container exec -i kina-test-worker sh -c 'cat > /tmp/cilium-img.tar'
```

The original payload was a Cilium container image tarball, size: hundreds of MB (exact size not recorded).

---

## Reproduction Attempt (2026-06-11, kina-12 session)

### Test container

Image: `alpine:3.21` (local, arm64). Container name: `kina12-repro-alpine`.
Entry command: `sh -c 'while true; do sleep 60; done'` (running, non-interactive).

### Payload construction

All payloads: `dd if=/dev/urandom` → `.bin` → `tar cf` → `.tar`.

| Payload | sha256 (host) |
|---------|--------------|
| 1KB tar (4.5KB on disk) | `f91fb51b99c1f3c0cdf87c90039e0676619bf27ba91fd4cd7dc22f63e5b15415` |
| 1MB tar | `7b136356793e1654a2e0f973ddefbfb72f86f7910763b57ef002d3c9f626886c` |
| 50MB tar | `374f957f0fe2db9d34af6ee53823f89d46fde0e70c8a5cd86f485ea5bad84775` |
| 300MB tar | `53cbf626648bfc9cbbb3c4949a303dc315e5ad7903e0e70a5e9100351165a406` |
| alpine:3.21 image tar (3.8MB) | `9183759cbdc22b83ad46767b9949ba44ce1c3af978c2351286cdf7a64917c29f` |

---

## Characterization Table

Verification inside container: `container exec <name> sh -c 'ls -l <dest>; sha256sum <dest>'`.

| Cell | State | Payload | Destination | cp exit code | File present? | sha match? | Notes |
|------|-------|---------|-------------|:---:|:---:|:---:|-------|
| 1a | Running | 1KB tar | /tmp/ | 0 | YES | YES | |
| 1b | Running | 1MB tar | /tmp/ | 0 | YES | YES | |
| 1c | Running | 50MB tar | /tmp/ | 0 | YES | YES | |
| 1d | Running | 300MB tar | /tmp/ | 0 | YES | YES | |
| 2a | Running | 50MB tar | /root/ | 0 | YES | YES | |
| 2b | Running | 50MB tar | /no/such/dir/ | 0 | YES | YES | **SECONDARY FINDING**: cp silently mkdir -p'd `/no/such/dir/`; no error; file present and intact |
| 3a | Stopped | 1MB tar | /tmp/ | **1** | N/A | N/A | Error: `"invalidState: \"container kina12-repro-alpine is not running\""` |
| 3b | Stopped | 50MB tar | /tmp/ | **1** | N/A | N/A | Same error as 3a |
| 4  | Running | alpine:3.21 image tar (3.8MB) | /tmp/ | 0 | YES | YES | Exact original bug shape |
| 5 (control) | Running | 50MB tar via exec-stdin | /tmp/ | 0 | YES | YES | Workaround confirmed working |

---

## Verdict

**NOT REPRODUCED** in this session. Every `container cp` invocation into a running container exited 0
and the file was present inside the container with matching sha256.

Possible explanations for the original silent failure:
- Transient runtime state (container was in a transitional state not reflected in `container list`)
- Specific payload size or filesystem state in the original kina-test-worker container
- A race between container start and the cp call in the original session
- Bug fixed between the original session and this repro (same CLI version 1.0.0 ee848e3, so unlikely
  to be a version difference, but runtime service state could differ)
- The original container (`kina-test-worker`) may have had a full or unmountable filesystem

---

## Secondary Finding: Silent Directory Creation

`container cp` into a non-existent path (`/no/such/dir/payload.tar`) exits 0 and creates the full
directory tree. This differs from `docker cp` behavior (which returns an error if the parent directory
does not exist). This may be intentional, but is not documented in `container cp --help`.

Expected (docker-compat): exit non-zero with "no such file or directory"
Actual: exit 0, intermediate directories created, file placed

---

## Exec-stdin Workaround

For cases where `container cp` fails silently, this workaround reliably transfers files:

```bash
cat <local-file> | container exec -i <container-name> sh -c 'cat > <dest-path>'
```

Verified at 50MB with matching sha256.

---

## Recommended Next Steps

1. Identify the original kina-test-worker container state (was it newly started? had prior copy failures?).
2. Attempt reproduction with a freshly started kina node container immediately after boot.
3. Check whether `container cp` of very large files (>500MB) exhibits the silent failure.
4. Test under filesystem pressure (nearly-full container rootfs).

---

## Repro Attempt Against a Live kina Cluster (2026-06-11, kina-12 follow-up)

### Cluster

Created via `kina create kina12-repro --workers 1` (main branch, `kindest/node:v1.35.5`):
- `kina12-repro-control-plane` — 192.168.65.61, running
- `kina12-repro-worker` — 192.168.65.62, running

Container CLI: 1.0.0 ee848e3 (same version as original bug report).

### Key discovery: `/tmp` is a `tmpfs` mount inside kina node containers

```
container exec kina12-repro-worker sh -c 'df -h /tmp && stat /tmp'
Filesystem      Size  Used Avail Use% Mounted on
tmpfs           2.0G     0  2.0G   0% /tmp
Device: 0,28	Inode: 1
```

The kina node container mounts `/tmp`, `/run`, and `/run/lock` as `tmpfs` (in-memory). The root filesystem `/` is on `/dev/vdb` (block device). This is confirmed in the container configuration JSON (`"mounts":[{"destination":"/tmp","type":{"tmpfs":{}}},...`).

### Characterization Table (kina node containers, 2026-06-11)

Verification: `container exec <node> sh -c 'ls -l <dest> && sha256sum <dest>'`.

| Payload | Size | Destination | Node | cp exit code | File present? | sha match? | Notes |
|---------|------|-------------|------|:---:|:---:|:---:|-------|
| random data | 1MB | /tmp/ | worker | 0 | NO | N/A | **FAILS-SILENTLY** |
| random data | 50MB | /tmp/ | worker | 0 | NO | N/A | **FAILS-SILENTLY** |
| random data | 300MB | /tmp/ | worker | 0 | NO | N/A | **FAILS-SILENTLY** |
| kina/node:v1.35.5 tar | 246MB | /tmp/ | worker | 0 | NO | N/A | **FAILS-SILENTLY** — original bug shape |
| kina/node:v1.35.5 tar | 246MB | /tmp/ | worker (run 2) | 0 | NO | N/A | **FAILS-SILENTLY** — consistent |
| kina/node:v1.35.5 tar | 246MB | /tmp/ | worker (run 3) | 0 | NO | N/A | **FAILS-SILENTLY** — consistent |
| random data | 1MB | /tmp/ | control-plane | 0 | NO | N/A | **FAILS-SILENTLY** — both node types |
| kina/node:v1.35.5 tar | 246MB | /root/ | worker | 0 | YES | YES | Succeeds on block-device FS (`/dev/vdb`) |
| random data | 1MB | /root/ | worker | 0 | YES | `3454d9365b72...` ✓ | Succeeds on block-device FS |

### Verdict: REPRODUCED

`container cp` silently fails (exit 0, file absent) when the destination directory is on a `tmpfs`
mount inside a kina node container. The failure is:
- **100% consistent** — every cp into `/tmp` failed across all 8 attempts
- **size-independent** — 1MB, 50MB, 246MB, 300MB all fail identically
- **node-type-independent** — both worker and control-plane nodes affected
- **filesystem-dependent** — cp to `/root/` (block device `/dev/vdb`) succeeds with correct sha256

The original alpine-container repro (prior session) did NOT reproduce because the alpine container's
`/tmp` is on the container's overlay rootfs (not a separate tmpfs mount). The kina node containers
explicitly mount `/tmp` as `tmpfs`, which triggers the `container cp` silent failure.

Root cause hypothesis: `container cp` does not handle `tmpfs` mounts inside the target container's
filesystem namespace correctly in container CLI 1.0.0 ee848e3. The copy operation is silently dropped
when the destination resolves to a `tmpfs` mount point.

### Fix Branch Verification (exec-stdin injection path)

Branch: `fix/kina-12-container-cp-verification` at `/tmp/kina-agents/kina-12/kina`

The fix implements `container exec -i <node> sh -c 'cat > <dest>'` (exec-stdin injection) instead
of `container cp`. Run against the live `kina12-repro` cluster:

```
$ kina load kina/node:v1.35.5 --cluster kina12-repro -v
...
DEBUG kina::core::apple_container: Loading image 'kina/node:v1.35.5' into container 'kina12-repro-worker'
DEBUG kina::core::apple_container: Successfully loaded image 'kina/node:v1.35.5' into container 'kina12-repro-worker'
DEBUG kina::core::apple_container: Loading image 'kina/node:v1.35.5' into container 'kina12-repro-control-plane'
DEBUG kina::core::apple_container: Successfully loaded image 'kina/node:v1.35.5' into container 'kina12-repro-control-plane'
INFO kina::core::apple_container: Image 'kina/node:v1.35.5' loaded successfully into cluster 'kina12-repro'
✅ Image 'kina/node:v1.35.5' loaded successfully into cluster 'kina12-repro'
```

Containerd verification on both nodes:
```
$ container exec kina12-repro-worker ctr images ls | grep kina
kina/node:v1.35.5  application/vnd.oci.image.index.v1+json  sha256:d1731879ce31...  246.4 MiB  linux/arm64

$ container exec kina12-repro-control-plane ctr images ls | grep kina
kina/node:v1.35.5  application/vnd.oci.image.index.v1+json  sha256:d1731879ce31...  246.4 MiB  linux/arm64
```

The exec-stdin path successfully transferred the 246MB image into the default containerd namespace
on both nodes. The fix resolves the silent failure by bypassing `container cp` entirely for
node-container image injection.
