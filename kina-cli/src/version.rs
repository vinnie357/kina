//! Build provenance — git sha, describe, rustc version, build timestamp.
//!
//! [`BuildInfo`] is a plain struct with all `&'static str` / `Option` fields
//! so it can be a `const` value initialised entirely from `env!` / `option_env!`
//! at compile time.
//!
//! [`human_version`] and [`version_json`] are pure functions that accept
//! `&BuildInfo` — no side effects, easily tested with hand-crafted fixtures.
//!
//! [`BUILD`] is the real singleton populated from vergen-gitcl build vars.

use serde::Serialize;

// ---------------------------------------------------------------------------
// BuildInfo
// ---------------------------------------------------------------------------

/// Compile-time build provenance.
///
/// All fields are static strings (or options thereof) so the constant [`BUILD`]
/// can be initialised in a `const` context without allocating.
pub struct BuildInfo {
    /// `CARGO_PKG_VERSION` — e.g. `"0.1.0"`.
    pub pkg_version: &'static str,
    /// `git describe --tags --dirty` output, or `None` when not in a git tree.
    pub git_describe: Option<&'static str>,
    /// Short git SHA (7 hex chars), or `None` when not in a git tree.
    pub git_sha: Option<&'static str>,
    /// `true` if the working tree was dirty at build time; `None` when not in a
    /// git tree.
    pub dirty: Option<bool>,
    /// ISO-8601 build timestamp, e.g. `"2026-06-26T18:40:00Z"`.
    pub build_timestamp: &'static str,
    /// Rustc version string, e.g. `"rustc 1.87.0"`.
    pub rustc: &'static str,
    /// `"debug"` or `"release"`.
    pub profile: &'static str,
    /// Cargo target triple, e.g. `"aarch64-apple-darwin"`.
    pub target: &'static str,
}

// ---------------------------------------------------------------------------
// human_version
// ---------------------------------------------------------------------------

/// Returns a human-readable version string.
///
/// Format: `kina {pkg_version} ({describe_or_unknown}, {profile}, built {build_timestamp})`
///
/// When `git_describe` is `None`, the placeholder `"unknown"` is used.
pub fn human_version(info: &BuildInfo) -> String {
    let describe = info.git_describe.unwrap_or("unknown");
    format!(
        "kina {} ({}, {}, built {})",
        info.pkg_version, describe, info.profile, info.build_timestamp
    )
}

// ---------------------------------------------------------------------------
// version_json
// ---------------------------------------------------------------------------

/// Internal serialisation shape — all 8 mandatory keys, `None` → JSON `null`.
#[derive(Serialize)]
struct VersionJsonData<'a> {
    version: &'a str,
    git_sha: Option<&'a str>,
    git_describe: Option<&'a str>,
    dirty: Option<bool>,
    build_timestamp: &'a str,
    rustc: &'a str,
    target: &'a str,
    profile: &'a str,
}

/// Returns a JSON string with all 8 build-provenance keys.
///
/// `None` fields serialise as JSON `null` (not the string `"null"` or absent).
pub fn version_json(info: &BuildInfo) -> String {
    let data = VersionJsonData {
        version: info.pkg_version,
        git_sha: info.git_sha,
        git_describe: info.git_describe,
        dirty: info.dirty,
        build_timestamp: info.build_timestamp,
        rustc: info.rustc,
        target: info.target,
        profile: info.profile,
    };
    serde_json::to_string(&data)
        .unwrap_or_else(|_| r#"{"error":"version_json serialization failed"}"#.to_owned())
}

// ---------------------------------------------------------------------------
// parse helpers (const-safe)
// ---------------------------------------------------------------------------

/// Parse the VERGEN_GIT_DIRTY env var string into `Option<bool>` at compile time.
const fn parse_dirty(s: Option<&'static str>) -> Option<bool> {
    match s {
        None => None,
        Some(v) => {
            let b = v.as_bytes();
            // "true" → Some(true), "false" → Some(false), anything else → None
            if b.len() == 4 && b[0] == b't' && b[1] == b'r' && b[2] == b'u' && b[3] == b'e' {
                Some(true)
            } else if b.len() == 5
                && b[0] == b'f'
                && b[1] == b'a'
                && b[2] == b'l'
                && b[3] == b's'
                && b[4] == b'e'
            {
                Some(false)
            } else {
                None
            }
        }
    }
}

/// Unwrap `Option<&'static str>` at compile time, falling back to a literal.
const fn unwrap_or_static(opt: Option<&'static str>, fallback: &'static str) -> &'static str {
    match opt {
        Some(v) => v,
        None => fallback,
    }
}

// ---------------------------------------------------------------------------
// BUILD — real compile-time constant
// ---------------------------------------------------------------------------

/// Real build provenance, populated at compile time from `vergen-gitcl` env
/// vars emitted by `build.rs`.
///
/// Git fields are `None` when the build ran outside a `.git` tree.
pub const BUILD: BuildInfo = BuildInfo {
    pkg_version: env!("CARGO_PKG_VERSION"),
    git_describe: option_env!("VERGEN_GIT_DESCRIBE"),
    git_sha: option_env!("VERGEN_GIT_SHA"),
    dirty: parse_dirty(option_env!("VERGEN_GIT_DIRTY")),
    build_timestamp: unwrap_or_static(option_env!("VERGEN_BUILD_TIMESTAMP"), "unknown"),
    rustc: unwrap_or_static(option_env!("VERGEN_RUSTC_SEMVER"), "unknown"),
    profile: if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    },
    target: unwrap_or_static(option_env!("VERGEN_CARGO_TARGET_TRIPLE"), "unknown"),
};
