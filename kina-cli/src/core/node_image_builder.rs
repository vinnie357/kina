//! Node image builder — constructs and runs `container build` for Kubernetes node images.
//!
//! All pure functions are free of I/O.  The subprocess boundary is abstracted by
//! function-pointer seams (`RunnerFn`, `ImageListerFn`) so tests can inject fakes
//! without spawning real `container` processes, following the same pattern as
//! `kernel_fetch.rs` / `KernelFetcher`.
//!
//! Public API (binding for P3 — all items must remain exported):
//!
//! Constants:
//!   DEFAULT_KUBERNETES_VERSION, DEFAULT_CONTAINERD_VERSION,
//!   DEFAULT_RUNC_VERSION, DEFAULT_CNI_PLUGINS_VERSION
//!
//! Types:
//!   BuildDecision, RunConfig, ImageEntry
//!
//! Functions:
//!   resolve_build_args, build_command_args, build_inputs_hash,
//!   cache_decision, write_cache_entry, build_decision_with_no_cache,
//!   run_build, parse_image_list, image_present
//!
//! VERSION POLICY (kina-2): every default version is pinned to an explicit
//! release.  Floating tags or non-specific versions are FORBIDDEN in default constants.

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Pinned default version constants (must match kina-cli/images/Dockerfile ARGs)
// ---------------------------------------------------------------------------

/// Pinned default Kubernetes version (matches Dockerfile `ARG KUBERNETES_VERSION`).
pub const DEFAULT_KUBERNETES_VERSION: &str = "1.35.5";

/// Pinned default containerd version (matches Dockerfile `ARG CONTAINERD_VERSION`).
pub const DEFAULT_CONTAINERD_VERSION: &str = "2.3.1";

/// Pinned default runc version (matches Dockerfile `ARG RUNC_VERSION`).
pub const DEFAULT_RUNC_VERSION: &str = "1.4.2";

/// Pinned default CNI plugins version (matches Dockerfile `ARG CNI_PLUGINS_VERSION`).
pub const DEFAULT_CNI_PLUGINS_VERSION: &str = "1.9.1";

// ---------------------------------------------------------------------------
// Cache file name (stored under data_dir)
// ---------------------------------------------------------------------------

const CACHE_FILE_NAME: &str = "node-image-cache.json";

// ---------------------------------------------------------------------------
// BuildDecision
// ---------------------------------------------------------------------------

/// Decision returned by cache lookup functions.
#[derive(Debug, PartialEq, Eq)]
pub enum BuildDecision {
    /// A valid cache entry exists for the current inputs — skip the build.
    Hit,

    /// No cache entry exists (or inputs changed) — perform a build.
    Miss,

    /// `--no-cache` was specified — force a build regardless of stored entry.
    Rebuild,
}

// ---------------------------------------------------------------------------
// ImageEntry
// ---------------------------------------------------------------------------

/// A single entry from `container image list --format json`.
#[derive(Debug, serde::Deserialize)]
pub struct ImageEntry {
    /// Image name / tag (e.g. `kina/node:v1.35.5`).
    pub name: String,

    /// Image ID (e.g. `sha256:abc123…`).  Optional — field may be absent.
    #[serde(default)]
    pub id: String,
}

// ---------------------------------------------------------------------------
// RunConfig
// ---------------------------------------------------------------------------

/// All inputs for a single `run_build` invocation.
pub struct RunConfig {
    /// The image tag to build (passed as `-t <tag>`).
    pub tag: String,

    /// Target CPU architecture string (e.g. `"arm64"`, `"amd64"`).
    pub arch: String,

    /// When `true`, bypass the content-hash cache and force a rebuild.
    pub no_cache: bool,

    /// Raw bytes of the Dockerfile — used for the content hash.
    pub dockerfile_bytes: Vec<u8>,

    /// Resolved `(KEY, VALUE)` build-arg pairs — used for the content hash
    /// and emitted as `--build-arg KEY=VALUE` flags.
    pub resolved_args: Vec<(String, String)>,

    /// Directory under which the cache file is stored (`config.cluster.data_dir`).
    pub data_dir: PathBuf,

    /// Directory containing `Dockerfile` and the build context
    /// (`kina-cli/images/`).
    pub images_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// resolve_build_args
// ---------------------------------------------------------------------------

/// Build the full set of `(KEY, VALUE)` build-arg pairs for a node image build.
///
/// Starts from the four pinned defaults:
///   - `KUBERNETES_VERSION` → `DEFAULT_KUBERNETES_VERSION` (overridden by `kubernetes_version` when `Some`)
///   - `CONTAINERD_VERSION` → `DEFAULT_CONTAINERD_VERSION`
///   - `RUNC_VERSION`       → `DEFAULT_RUNC_VERSION`
///   - `CNI_PLUGINS_VERSION`→ `DEFAULT_CNI_PLUGINS_VERSION`
///
/// Each entry in `extra_build_args` has the form `"KEY=VALUE"`.  When a key
/// matches one of the four defaults, the default is replaced.  Novel keys are
/// appended in order.
pub fn resolve_build_args(
    kubernetes_version: Option<&str>,
    extra_build_args: &[&str],
) -> Vec<(String, String)> {
    // Start with the four pinned defaults.
    let mut map: Vec<(String, String)> = vec![
        (
            "KUBERNETES_VERSION".to_string(),
            kubernetes_version
                .unwrap_or(DEFAULT_KUBERNETES_VERSION)
                .to_string(),
        ),
        (
            "CONTAINERD_VERSION".to_string(),
            DEFAULT_CONTAINERD_VERSION.to_string(),
        ),
        ("RUNC_VERSION".to_string(), DEFAULT_RUNC_VERSION.to_string()),
        (
            "CNI_PLUGINS_VERSION".to_string(),
            DEFAULT_CNI_PLUGINS_VERSION.to_string(),
        ),
    ];

    // Apply extra_build_args: override existing keys, append novel ones.
    for entry in extra_build_args {
        let (key, value) = split_build_arg(entry);
        if let Some(pos) = map.iter().position(|(k, _)| k == key) {
            map[pos].1 = value.to_string();
        } else {
            map.push((key.to_string(), value.to_string()));
        }
    }

    map
}

/// Split a `"KEY=VALUE"` string into `(key, value)`.
///
/// If the string contains no `=`, the whole string is treated as the key and
/// the value is empty.
fn split_build_arg(s: &str) -> (&str, &str) {
    match s.find('=') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s, ""),
    }
}

// ---------------------------------------------------------------------------
// build_command_args
// ---------------------------------------------------------------------------

/// Construct the argument list for a `container build` invocation.
///
/// Returns a `Vec<String>` of arguments (not including the `container` binary
/// itself) suitable for passing to `std::process::Command::args`.
///
/// Shape:
/// ```text
/// build
///   -f <images_dir>/Dockerfile
///   -t <tag>
///   --arch <arch>
///   [--build-arg KEY=VALUE ...]
///   [--no-cache]
///   <images_dir>          ← the build context
/// ```
pub fn build_command_args(
    tag: &str,
    arch: &str,
    resolved_args: &[(String, String)],
    no_cache: bool,
    dockerfile_dir: &Path,
) -> Vec<String> {
    let dockerfile_path = dockerfile_dir.join("Dockerfile");

    let mut args: Vec<String> = vec![
        "build".to_string(),
        "-f".to_string(),
        dockerfile_path.to_string_lossy().to_string(),
        "-t".to_string(),
        tag.to_string(),
        "--arch".to_string(),
        arch.to_string(),
    ];

    // Emit one --build-arg KEY=VALUE per resolved pair.
    for (key, value) in resolved_args {
        args.push("--build-arg".to_string());
        args.push(format!("{}={}", key, value));
    }

    if no_cache {
        args.push("--no-cache".to_string());
    }

    // Build context: the images directory itself.
    args.push(dockerfile_dir.to_string_lossy().to_string());

    args
}

// ---------------------------------------------------------------------------
// build_inputs_hash
// ---------------------------------------------------------------------------

/// Compute a SHA-256 content hash of all build inputs.
///
/// Inputs: Dockerfile bytes + canonical `KEY=VALUE` string for each resolved
/// build arg (sorted for stability) + the image tag.
///
/// Returns a lowercase hex-encoded SHA-256 digest string.
pub fn build_inputs_hash(
    dockerfile_bytes: &[u8],
    resolved_args: &[(String, String)],
    tag: &str,
) -> String {
    let mut hasher = Sha256::new();

    // Hash the Dockerfile bytes.
    hasher.update(dockerfile_bytes);

    // Hash each build-arg pair in stable order (sorted by key).
    let mut sorted_args: Vec<(&String, &String)> =
        resolved_args.iter().map(|(k, v)| (k, v)).collect();
    sorted_args.sort_by_key(|(k, _)| k.as_str());
    for (key, value) in &sorted_args {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }

    // Hash the tag.
    hasher.update(tag.as_bytes());

    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Cache entry persistence
// ---------------------------------------------------------------------------

/// The JSON structure persisted in `data_dir/node-image-cache.json`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheFile {
    /// SHA-256 hex digest of the last successful build's inputs.
    hash: String,
}

fn cache_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CACHE_FILE_NAME)
}

/// Write (or overwrite) the cache entry for the given hash under `data_dir`.
///
/// Creates `data_dir` if it does not exist.
pub fn write_cache_entry(hash: &str, data_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("cannot create data_dir {}: {}", data_dir.display(), e))?;

    let entry = CacheFile {
        hash: hash.to_string(),
    };
    let json = serde_json::to_string(&entry)
        .map_err(|e| format!("cannot serialise cache entry: {}", e))?;

    std::fs::write(cache_file_path(data_dir), json)
        .map_err(|e| format!("cannot write cache file: {}", e))?;

    Ok(())
}

/// Read the stored hash from `data_dir/node-image-cache.json`.
///
/// Returns `None` if the file does not exist or cannot be parsed.
fn read_cache_hash(data_dir: &Path) -> Option<String> {
    let path = cache_file_path(data_dir);
    let json = std::fs::read_to_string(&path).ok()?;
    let entry: CacheFile = serde_json::from_str(&json).ok()?;
    Some(entry.hash)
}

// ---------------------------------------------------------------------------
// cache_decision
// ---------------------------------------------------------------------------

/// Check whether `hash` matches the stored cache entry under `data_dir`.
///
/// - Returns `BuildDecision::Hit`  when the stored hash equals `hash`.
/// - Returns `BuildDecision::Miss` when no entry exists or the hashes differ.
pub fn cache_decision(hash: &str, data_dir: &Path) -> BuildDecision {
    match read_cache_hash(data_dir) {
        Some(stored) if stored == hash => BuildDecision::Hit,
        _ => BuildDecision::Miss,
    }
}

// ---------------------------------------------------------------------------
// build_decision_with_no_cache
// ---------------------------------------------------------------------------

/// Like `cache_decision`, but also considers the `no_cache` flag.
///
/// When `no_cache` is `true`, returns `BuildDecision::Rebuild` regardless of
/// what the stored cache entry says.  When `false`, delegates to
/// `cache_decision`.
pub fn build_decision_with_no_cache(hash: &str, data_dir: &Path, no_cache: bool) -> BuildDecision {
    if no_cache {
        BuildDecision::Rebuild
    } else {
        cache_decision(hash, data_dir)
    }
}

// ---------------------------------------------------------------------------
// parse_image_list / image_present
// ---------------------------------------------------------------------------

/// Parse the JSON output of `container image list --format json`.
///
/// - Returns `Ok(vec![])` for whitespace-only or empty input (mirrors
///   `parse_container_list` contract).
/// - Returns `Err` for malformed JSON or a non-array top-level value.
pub fn parse_image_list(json: &str) -> Result<Vec<ImageEntry>, String> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("malformed JSON: {}", e))?;

    let array = value
        .as_array()
        .ok_or_else(|| "expected a JSON array at top level".to_string())?;

    let mut entries = Vec::with_capacity(array.len());
    for elem in array {
        let name = elem
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let id = elem
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        entries.push(ImageEntry { name, id });
    }

    Ok(entries)
}

/// Return `true` if `tag` appears in `list` (exact match on `entry.name`).
pub fn image_present(list: &[ImageEntry], tag: &str) -> bool {
    list.iter().any(|e| e.name == tag)
}

// ---------------------------------------------------------------------------
// run_build
// ---------------------------------------------------------------------------

/// Run a full node image build with cache check and post-build verification.
///
/// # Flow
///
/// 1. Compute content hash from `config.dockerfile_bytes`, `config.resolved_args`, and `config.tag`.
/// 2. Consult the cache (respecting `config.no_cache`):
///    - `Hit`     → return early with a cache-hit message; runner is NOT called.
///    - `Miss` or `Rebuild` → invoke `runner(build_args)`.
/// 3. After a successful runner invocation:
///    a. Call `image_lister(["image", "list", "--format", "json"])` to verify the image exists.
///    b. If the image is NOT present → return `Err` (exit 0 is not trusted as success — AC4).
///    c. If present → refresh the cache entry and return `Ok(success_message)`.
/// 4. Runner failure → return `Err`.
///
/// # Parameters
///
/// - `runner`       : `Fn(&[String]) -> Result<(), String>` — executes `container build …`
/// - `image_lister` : `Fn(&[String]) -> Result<String, String>` — returns JSON from
///   `container image list --format json`
pub fn run_build<R, L>(
    config: RunConfig,
    mut runner: R,
    mut image_lister: L,
) -> Result<String, String>
where
    R: FnMut(&[String]) -> Result<(), String>,
    L: FnMut(&[String]) -> Result<String, String>,
{
    let hash = build_inputs_hash(&config.dockerfile_bytes, &config.resolved_args, &config.tag);

    let decision = build_decision_with_no_cache(&hash, &config.data_dir, config.no_cache);

    match decision {
        BuildDecision::Hit => {
            // Cache hit — skip the build and return an informational message.
            Ok(format!(
                "cache hit for {} (inputs unchanged) — skipping build",
                config.tag
            ))
        }
        BuildDecision::Miss | BuildDecision::Rebuild => {
            // Perform the build.
            let build_args = build_command_args(
                &config.tag,
                &config.arch,
                &config.resolved_args,
                config.no_cache,
                &config.images_dir,
            );

            runner(&build_args).map_err(|e| format!("container build failed: {}", e))?;

            // Post-build verification: confirm the image is actually present.
            let list_args = vec![
                "image".to_string(),
                "list".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ];
            let list_json =
                image_lister(&list_args).map_err(|e| format!("image list failed: {}", e))?;

            let entries = parse_image_list(&list_json)
                .map_err(|e| format!("image list parse error: {}", e))?;

            if !image_present(&entries, &config.tag) {
                Err(format!(
                    "build reported success but image '{}' not found in `container image list` \
                     output (exit 0 is not trusted — AC4 post-build verification failed)",
                    config.tag
                ))
            } else {
                // Refresh the cache entry.
                write_cache_entry(&hash, &config.data_dir)?;
                Ok(format!("built and verified image '{}'", config.tag))
            }
        }
    }
}
