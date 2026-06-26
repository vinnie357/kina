/// Build-provenance tests for `kina --version` output — adversarial TDD (P2 test-author stage).
///
/// Tests are INTENTIONALLY RED: they reference `kina_cli::version::{BuildInfo, human_version,
/// version_json}` which do NOT yet exist. The compile errors are the spec; P3 (the separate
/// implementer agent) creates that module and makes these tests green WITHOUT modifying this file.
///
/// All tests are pure: no subprocess, no filesystem, no network, no env mutation.
///
/// Import surface (binding for P3):
///   use kina_cli::version::{BuildInfo, human_version, version_json};
///
/// BuildInfo fields (all must be pub):
///   pub pkg_version: &'static str
///   pub git_describe: Option<&'static str>   // git describe --tags --dirty value, or None
///   pub git_sha: Option<&'static str>         // short sha, or None
///   pub dirty: Option<bool>
///   pub build_timestamp: &'static str
///   pub rustc: &'static str
///   pub profile: &'static str                 // "debug" | "release"
///   pub target: &'static str
///
/// human_version format (exact):
///   format!("kina {} ({}, {}, built {})", pkg_version, describe_or_unknown, profile, build_timestamp)
///   where describe_or_unknown = git_describe.unwrap_or("unknown")
///
/// version_json format:
///   Valid JSON string with keys: version, git_sha, git_describe, dirty, build_timestamp,
///   rustc, target, profile. None values serialize as JSON null.
use kina_cli::version::{human_version, version_json, BuildInfo};

// ===========================================================================
// Shared fixture helpers
// ===========================================================================

/// A BuildInfo with git_describe=Some, git_sha=Some, dirty=Some(false) for clean-build tests.
fn clean_build() -> BuildInfo {
    BuildInfo {
        pkg_version: "0.1.0",
        git_describe: Some("0dd70c4"),
        git_sha: Some("0dd70c4"),
        dirty: Some(false),
        build_timestamp: "2026-06-26T18:40:00Z",
        rustc: "rustc 1.87.0",
        profile: "debug",
        target: "aarch64-apple-darwin",
    }
}

/// A BuildInfo simulating a dirty working tree; describe includes the -dirty suffix.
fn dirty_build() -> BuildInfo {
    BuildInfo {
        pkg_version: "0.1.0",
        git_describe: Some("0dd70c4-dirty"),
        git_sha: Some("0dd70c4"),
        dirty: Some(true),
        build_timestamp: "2026-06-26T18:40:00Z",
        rustc: "rustc 1.87.0",
        profile: "debug",
        target: "aarch64-apple-darwin",
    }
}

/// A BuildInfo where git information is entirely absent (built outside a .git tree).
fn no_git_build() -> BuildInfo {
    BuildInfo {
        pkg_version: "0.1.0",
        git_describe: None,
        git_sha: None,
        dirty: None,
        build_timestamp: "2026-06-26T18:40:00Z",
        rustc: "rustc 1.87.0",
        profile: "debug",
        target: "aarch64-apple-darwin",
    }
}

// ===========================================================================
// Group A — human_version
// ===========================================================================

/// T1 — Clean build: exact full-string check.
///
/// human_version must return exactly:
///   "kina 0.1.0 (0dd70c4, debug, built 2026-06-26T18:40:00Z)"
#[test]
fn human_version_clean_exact_format() {
    let info = clean_build();
    let got = human_version(&info);
    let expected = "kina 0.1.0 (0dd70c4, debug, built 2026-06-26T18:40:00Z)";
    assert_eq!(
        got, expected,
        "human_version exact format mismatch.\nExpected: {expected}\nGot:      {got}"
    );
}

/// T2 — Dirty describe: the -dirty suffix surfaced via git_describe appears in the output.
///
/// When git_describe = Some("0dd70c4-dirty"), the human string must contain "0dd70c4-dirty".
/// The dirty flag is ONLY surfaced through the describe string — no extra "(dirty)" annotation.
#[test]
fn human_version_dirty_describe_surfaced() {
    let info = dirty_build();
    let got = human_version(&info);
    assert!(
        got.contains("0dd70c4-dirty"),
        "human_version must embed the full describe string (including -dirty suffix).\nGot: {got}"
    );
    // Must still follow the general shape: starts with "kina "
    assert!(
        got.starts_with("kina "),
        "human_version must start with \"kina \".\nGot: {got}"
    );
}

/// T3 — No git info: git_describe=None must produce "unknown", NOT panic.
///
/// Format must be: "kina 0.1.0 (unknown, debug, built 2026-06-26T18:40:00Z)"
#[test]
fn human_version_no_git_describe_gives_unknown() {
    let info = no_git_build();
    let got = human_version(&info);
    let expected = "kina 0.1.0 (unknown, debug, built 2026-06-26T18:40:00Z)";
    assert_eq!(
        got, expected,
        "human_version with git_describe=None must use \"unknown\".\nExpected: {expected}\nGot:      {got}"
    );
}

/// T4 — Profile "release" is reflected in the human string.
///
/// Changing profile to "release" on an otherwise-clean build must produce a string
/// that contains "release" and does NOT contain "debug".
#[test]
fn human_version_release_profile_reflected() {
    let info = BuildInfo {
        profile: "release",
        ..clean_build()
    };
    let got = human_version(&info);
    assert!(
        got.contains("release"),
        "human_version must include the profile string \"release\".\nGot: {got}"
    );
    assert!(
        !got.contains("debug"),
        "human_version with profile=\"release\" must NOT contain \"debug\".\nGot: {got}"
    );
}

/// T5 — Build timestamp appears verbatim in the output.
///
/// Whatever is in build_timestamp must appear literally inside the returned string.
#[test]
fn human_version_build_timestamp_present() {
    let info = clean_build();
    let got = human_version(&info);
    assert!(
        got.contains(info.build_timestamp),
        "human_version must contain the build_timestamp verbatim.\nbuild_timestamp: {}\nGot: {got}",
        info.build_timestamp
    );
}

// ===========================================================================
// Group B — version_json
// ===========================================================================

/// T6 — All 8 mandatory keys are present in the JSON object.
///
/// Keys: version, git_sha, git_describe, dirty, build_timestamp, rustc, target, profile.
#[test]
fn version_json_all_8_keys_present() {
    let info = clean_build();
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must produce valid JSON");
    let obj = val
        .as_object()
        .expect("version_json root must be a JSON object");

    for key in &[
        "version",
        "git_sha",
        "git_describe",
        "dirty",
        "build_timestamp",
        "rustc",
        "target",
        "profile",
    ] {
        assert!(
            obj.contains_key(*key),
            "version_json must contain key \"{key}\". Keys present: {keys:?}",
            key = key,
            keys = obj.keys().collect::<Vec<_>>()
        );
    }
}

/// T7 — version_json round-trips through serde_json with no loss.
///
/// Parse the returned string back into serde_json::Value — must not error.
/// Spot-check that "version" equals pkg_version.
#[test]
fn version_json_is_valid_json_and_version_matches() {
    let info = clean_build();
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be parseable JSON");
    assert_eq!(
        val["version"]
            .as_str()
            .expect("version must be a JSON string"),
        info.pkg_version,
        "version_json[\"version\"] must equal pkg_version"
    );
}

/// T8 — dirty=true round-trips as JSON boolean true.
///
/// When dirty=Some(true), the "dirty" key in the JSON object must be JSON `true`.
#[test]
fn version_json_dirty_true_roundtrips() {
    let info = dirty_build(); // dirty = Some(true)
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be valid JSON");
    assert_eq!(
        val["dirty"],
        serde_json::Value::Bool(true),
        "version_json[\"dirty\"] must be JSON true when dirty=Some(true).\nGot: {}",
        val["dirty"]
    );
}

/// T9 — git_sha=None serializes as JSON null.
///
/// When git_sha=None the "git_sha" key must be JSON null, not absent, not "null".
#[test]
fn version_json_git_sha_null_when_none() {
    let info = no_git_build(); // git_sha = None
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be valid JSON");
    assert_eq!(
        val["git_sha"],
        serde_json::Value::Null,
        "version_json[\"git_sha\"] must be JSON null when git_sha=None.\nGot: {}",
        val["git_sha"]
    );
}

/// T10 — git_describe=None serializes as JSON null (not the string "unknown").
///
/// The "unknown" fallback is ONLY for human_version; in JSON, None → null.
#[test]
fn version_json_git_describe_null_when_none() {
    let info = no_git_build(); // git_describe = None
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be valid JSON");
    assert_eq!(
        val["git_describe"],
        serde_json::Value::Null,
        "version_json[\"git_describe\"] must be JSON null when git_describe=None.\nGot: {}",
        val["git_describe"]
    );
    // Double-check: must NOT contain the string "unknown" as a JSON value
    assert!(
        val["git_describe"] != serde_json::Value::String("unknown".to_string()),
        "version_json must NOT serialize None git_describe as the string \"unknown\" — use null"
    );
}

/// T11 — dirty=None serializes as JSON null (not absent, not false).
///
/// When dirty=None (no git context at all), "dirty" must be JSON null.
#[test]
fn version_json_dirty_null_when_none() {
    let info = no_git_build(); // dirty = None
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be valid JSON");
    assert_eq!(
        val["dirty"],
        serde_json::Value::Null,
        "version_json[\"dirty\"] must be JSON null when dirty=None.\nGot: {}",
        val["dirty"]
    );
}

/// T12 — profile and target appear as non-null strings in the JSON.
///
/// These are always available from build-time constants and must never be null.
#[test]
fn version_json_profile_and_target_are_strings() {
    let info = clean_build();
    let json_str = version_json(&info);
    let val: serde_json::Value =
        serde_json::from_str(&json_str).expect("version_json must be valid JSON");

    assert_eq!(
        val["profile"]
            .as_str()
            .expect("profile must be a JSON string"),
        info.profile,
        "version_json[\"profile\"] must match BuildInfo.profile"
    );
    assert_eq!(
        val["target"]
            .as_str()
            .expect("target must be a JSON string"),
        info.target,
        "version_json[\"target\"] must match BuildInfo.target"
    );
}
