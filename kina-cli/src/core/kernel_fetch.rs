//! Kernel distribution: auto-download, sha256 verification, atomic cache, and doctor reporting.
//!
//! This module implements phase B of the kina kernel distribution plan:
//! - Builds GitHub release asset URLs (pure, interpolated)
//! - Computes cache directory layout under `~/.kina/kernels/<tag>/`
//! - Verifies sha256 (case-insensitive hex comparison)
//! - Renders first-run notice with real size and one-time language
//! - Decides download/cache-hit/corrupt states
//! - Installs kernels atomically (temp → verify → rename; fail-closed on mismatch)
//! - Resolves kernel precedence for --cni cilium (explicit flag > cached pinned > fetch > error)
//! - Reports doctor/preflight state
//!
//! All public functions are pure (no I/O) except `install_kernel`, which uses a
//! `KernelFetcher` trait seam so tests can inject a fake without network calls.
//!
//! Many items in this module are used only from the integration test binary
//! (`kina-cli/tests/kernel_fetch_tests.rs`), so the bin crate may report dead_code
//! warnings.  Suppress them at the module level — these are intentional public API
//! surfaces consumed by tests and future callers.
#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Pinned constants (authoritative from kina-8 task header)
// ---------------------------------------------------------------------------

/// Pinned kernel release tag. Format: `kernel-v<linux>-kina.<n>` (hyphen, not plus).
pub const KERNEL_TAG: &str = "kernel-v6.18.5-kina.1";

/// sha256 of the pinned vmlinux kernel artifact (lowercase hex).
pub const KERNEL_SHA256: &str = "f1a40c2c00e8a7f2e2c0165355c13ff6dcdd2742d294babe31dd5c5b14aec3fe";

/// Size in bytes of the pinned vmlinux artifact (used for the first-run notice).
pub const KERNEL_SIZE_BYTES: u64 = 33503744;

/// Name of the kernel asset in the GitHub release.
const KERNEL_ASSET: &str = "vmlinux";

/// GitHub repository owning the kina releases.
const KERNEL_REPO: &str = "vinnie357/kina";

// ---------------------------------------------------------------------------
// Group A — release asset URL construction
// ---------------------------------------------------------------------------

/// Build a GitHub release asset URL from repo, tag, and asset name.
///
/// Returns: `https://github.com/<repo>/releases/download/<tag>/<asset>`
pub fn release_asset_url(repo: &str, tag: &str, asset: &str) -> String {
    format!(
        "https://github.com/{}/releases/download/{}/{}",
        repo, tag, asset
    )
}

/// Return the pinned (zero-flag default) kernel asset URL.
///
/// Equivalent to `release_asset_url(KERNEL_REPO, KERNEL_TAG, "vmlinux")`.
pub fn pinned_asset_url() -> String {
    release_asset_url(KERNEL_REPO, KERNEL_TAG, KERNEL_ASSET)
}

// ---------------------------------------------------------------------------
// Group B — cache directory layout
// ---------------------------------------------------------------------------

/// Return the cache directory for a given home dir and kernel tag.
///
/// Layout: `<home>/.kina/kernels/<tag>/`
pub fn kernel_cache_dir(home: &Path, tag: &str) -> PathBuf {
    home.join(".kina").join("kernels").join(tag)
}

/// Return the path of the cached vmlinux artifact for a given home dir and tag.
///
/// Layout: `<home>/.kina/kernels/<tag>/vmlinux`
pub fn kernel_cache_file(home: &Path, tag: &str) -> PathBuf {
    kernel_cache_dir(home, tag).join(KERNEL_ASSET)
}

// ---------------------------------------------------------------------------
// Group C — sha256 verification and remediation text
// ---------------------------------------------------------------------------

/// Verify that two hex-encoded sha256 strings refer to the same digest.
///
/// Comparison is case-insensitive so that uppercase, lowercase, or mixed-case
/// hex strings from different sources compare as equal when they represent the
/// same bytes.
pub fn verify_sha256(observed: &str, expected: &str) -> bool {
    observed.to_lowercase() == expected.to_lowercase()
}

/// Build an actionable error message for a sha256 mismatch.
///
/// The message includes:
/// - the expected and observed sha256 values (for self-diagnosis)
/// - the asset URL (so the user knows what was downloaded)
/// - the config key `kernel.sha256` where the pin lives
/// - a `rm -rf ~/.kina/kernels/<tag>` re-download command
pub fn sha256_mismatch_remediation(tag: &str, expected: &str, observed: &str, url: &str) -> String {
    format!(
        "sha256 mismatch for kernel {tag}:\n  expected: {expected}\n  observed: {observed}\n  source:   {url}\n\nThe pinned sha256 is stored in the config key 'kernel.sha256' (KERNEL_SHA256 constant).\nTo re-download, remove the corrupt cache and retry:\n  rm -rf ~/.kina/kernels/{tag}\n  kina create --cni cilium",
        tag = tag,
        expected = expected,
        observed = observed,
        url = url,
    )
}

// ---------------------------------------------------------------------------
// Group D — download-needed decision
// ---------------------------------------------------------------------------

/// Decision returned by `fetch_decision`.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchDecision {
    /// The cache file is present and its sha256 matches the expected value.
    /// No download is needed; the cached file is ready to use.
    CacheHit,

    /// The cache file is absent. A fresh download is required.
    Download,

    /// The cache file is present but its sha256 does not match the expected value.
    /// The file must be re-downloaded to a temp path; the corrupt file must not be used.
    Corrupt,
}

/// Determine whether a download is needed.
///
/// - `cache_present`: `true` if the kernel file already exists in the cache dir.
/// - `cached_sha_matches`: `true` if the sha256 of the existing cached file equals
///   the expected sha256 (only meaningful when `cache_present` is `true`).
pub fn fetch_decision(cache_present: bool, cached_sha_matches: bool) -> FetchDecision {
    match (cache_present, cached_sha_matches) {
        (false, _) => FetchDecision::Download,
        (true, true) => FetchDecision::CacheHit,
        (true, false) => FetchDecision::Corrupt,
    }
}

// ---------------------------------------------------------------------------
// Group E — first-run notice
// ---------------------------------------------------------------------------

/// Return `true` if the first-run download notice should be shown.
///
/// The notice is shown for `Download` (first time) and `Corrupt` (re-download
/// needed) decisions, but not for `CacheHit` (silent, instant).
pub fn should_show_first_run_notice(decision: &FetchDecision) -> bool {
    matches!(decision, FetchDecision::Download | FetchDecision::Corrupt)
}

/// Build the first-run notice line shown when downloading the pinned kernel.
///
/// The notice includes the tag, a human-readable size (rounded to the nearest MiB),
/// and "one time" language so the user knows subsequent creates are instant.
///
/// Size is computed as `round(size_bytes / 1048576)` — nearest MiB, so 33503744
/// bytes (31.97 MiB) rounds to 32 MB.
pub fn first_run_notice(tag: &str, size_bytes: u64) -> String {
    // Round to nearest MiB rather than truncating, so 33503744 bytes → 32 MB.
    let mib = 1024u64 * 1024;
    let size_mib = (size_bytes + mib / 2) / mib;
    format!(
        "downloading kina kernel {} (~{} MB, one time)...",
        tag, size_mib
    )
}

// ---------------------------------------------------------------------------
// Group F — atomic install via KernelFetcher seam
// ---------------------------------------------------------------------------

/// Trait for the network download side-effect.
///
/// In production this wraps `reqwest`; in tests a fake writes known bytes.
pub trait KernelFetcher {
    /// Download the asset at `url` and write it to `dest_tmp`.
    ///
    /// Returns the number of bytes written on success, or a human-readable
    /// error string on failure.
    fn fetch(&self, url: &str, dest_tmp: &Path) -> Result<u64, String>;
}

/// Compute the sha256 hex digest of a file at `path`.
///
/// Reads the file in chunks to avoid loading the entire kernel into memory.
fn sha256_file(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("cannot open {} for sha256: {}", path.display(), e))?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("read error during sha256: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Download, verify sha256, and atomically install a kernel artifact.
///
/// Steps:
/// 1. Create the cache directory if absent.
/// 2. Invoke `fetcher.fetch(url, tmp_path)` — writes to a `.tmp`-suffixed temp file.
/// 3. Compute sha256 of the temp file.
/// 4. If sha matches: rename temp → final (`vmlinux`); return the final path.
/// 5. If sha mismatches: delete the temp file, return Err with remediation text.
/// 6. If fetch fails: ensure no temp file remains, return actionable Err.
///
/// The final artifact is never written unless sha verification passes (fail-closed).
pub fn install_kernel<F: KernelFetcher>(
    fetcher: &F,
    url: &str,
    expected_sha256: &str,
    cache_dir: &Path,
) -> Result<PathBuf, String> {
    // 1. Ensure cache directory exists.
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("cannot create cache dir {}: {}", cache_dir.display(), e))?;

    let final_path = cache_dir.join(KERNEL_ASSET);
    let tmp_path = cache_dir.join(format!("{}.tmp", KERNEL_ASSET));

    // 2. Download to temp file.
    match fetcher.fetch(url, &tmp_path) {
        Ok(_bytes_written) => {
            // 3. Compute sha256 of the downloaded temp file.
            let computed = match sha256_file(&tmp_path) {
                Ok(h) => h,
                Err(e) => {
                    // Clean up temp on sha computation failure.
                    let _ = std::fs::remove_file(&tmp_path);
                    return Err(format!(
                        "sha256 computation failed: {}\nUse --kernel-path to specify a local kernel: {}",
                        e, url
                    ));
                }
            };

            // 4. Verify sha.
            if verify_sha256(&computed, expected_sha256) {
                // Atomic rename: temp → final.
                std::fs::rename(&tmp_path, &final_path).map_err(|e| {
                    let _ = std::fs::remove_file(&tmp_path);
                    format!("atomic rename failed: {}", e)
                })?;
                Ok(final_path)
            } else {
                // 5. Sha mismatch — delete temp, hard fail with remediation.
                let _ = std::fs::remove_file(&tmp_path);
                let tag = url
                    .split("/releases/download/")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or(KERNEL_TAG);
                Err(sha256_mismatch_remediation(
                    tag,
                    expected_sha256,
                    &computed,
                    url,
                ))
            }
        }
        Err(fetch_err) => {
            // 6. Fetch failed — ensure no temp file remains.
            let _ = std::fs::remove_file(&tmp_path);
            Err(format!(
                "failed to download kernel from {}: {}\n\
                 Use --kernel-path to specify a local kernel file instead.",
                url, fetch_err
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Group H — kernel resolution precedence for --cni cilium
// ---------------------------------------------------------------------------

/// The resolved kernel choice for a cluster create operation.
#[derive(Debug, PartialEq, Eq)]
pub enum KernelChoice {
    /// The user explicitly passed `--kernel-path <path>`.
    ExplicitPath(PathBuf),

    /// The pinned kernel is already in the cache and its sha is verified.
    CachedPinned(PathBuf),

    /// The pinned kernel is not cached; the caller must invoke `install_kernel`.
    FetchPinned,
}

/// Resolve which kernel to use for a `--cni cilium` cluster create.
///
/// Precedence (highest to lowest):
/// 1. `cli_flag` (`--kernel-path`) — always wins when set.
/// 2. `cached_pinned` — use the already-verified cache file.
/// 3. `fetch_ok = true` — trigger a download (`FetchPinned`).
/// 4. `fetch_ok = false` (offline / unreachable) — hard error with escape hatch.
pub fn resolve_kernel_for_cilium(
    cli_flag: Option<PathBuf>,
    cached_pinned: Option<PathBuf>,
    fetch_ok: bool,
) -> Result<KernelChoice, String> {
    // Precedence 1: explicit flag wins unconditionally.
    if let Some(path) = cli_flag {
        return Ok(KernelChoice::ExplicitPath(path));
    }

    // Precedence 2: valid cached pinned kernel.
    if let Some(path) = cached_pinned {
        return Ok(KernelChoice::CachedPinned(path));
    }

    // Precedence 3: attempt download.
    if fetch_ok {
        return Ok(KernelChoice::FetchPinned);
    }

    // Precedence 4: offline error with escape hatch.
    let url = pinned_asset_url();
    Err(format!(
        "cannot fetch kernel {tag}: network unreachable.\n\
         Asset URL: {url}\n\
         Use --kernel-path to specify a local copy of the kernel:\n  \
         kina create --cni cilium --kernel-path /path/to/vmlinux",
        tag = KERNEL_TAG,
        url = url,
    ))
}

/// Return `true` if the given CNI plugin requires a custom kernel.
///
/// Only the Cilium full-eBPF profile requires the pinned custom kernel.
/// PTP clusters use the stock kernel and must never trigger a download.
pub fn requires_kernel(cni: &crate::config::CniPlugin) -> bool {
    matches!(cni, crate::config::CniPlugin::Cilium)
}

// ---------------------------------------------------------------------------
// Group I — doctor / create-preflight report
// ---------------------------------------------------------------------------

/// Input state for the doctor/preflight report.
pub struct KernelDoctorState {
    /// Whether the pinned kernel is present in the local cache.
    pub kernel_cached: bool,

    /// Whether the cached kernel's sha256 has been verified against the pinned constant.
    pub sha_verified: bool,

    /// Whether the stock (system default) kernel will be used as a fallback.
    pub stock_fallback: bool,
}

/// Preflight/doctor report for the kernel subsystem.
#[derive(Debug)]
pub struct DoctorReport {
    /// `true` if the pinned kernel is present in `~/.kina/kernels/<tag>/vmlinux`.
    pub kernel_cached: bool,

    /// `true` if the cached kernel's sha256 matches the pinned constant.
    pub sha_verified: bool,

    /// `true` if the stock kernel will be used (PTP CNI or no pinned kernel found).
    pub stock_fallback: bool,
}

/// Build a doctor/preflight report from the given state.
///
/// Pure function: no filesystem or network access.
/// Directive #4: reports kernel cached yes/no, sha verified, stock-kernel fallback status.
pub fn doctor_report(state: &KernelDoctorState) -> DoctorReport {
    DoctorReport {
        kernel_cached: state.kernel_cached,
        sha_verified: state.sha_verified,
        stock_fallback: state.stock_fallback,
    }
}
