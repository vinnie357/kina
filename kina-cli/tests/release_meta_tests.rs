/// Release metadata tests — adversarial TDD (P2 test-author stage, kina-28)
///
/// Tests are INTENTIONALLY RED for T4-T10: release.yml does not exist yet.
/// P3 (the separate implementer agent) delivers .github/workflows/release.yml
/// WITHOUT modifying this file.
///
/// T1-T3 guard the current version and CLI wiring (regression guards — green now,
/// must stay green forever).
/// T4-T10 are structural source-grep guards on release.yml (red until P3 delivers
/// the file; green once P3 implements correctly).
///
/// All tests are pure: NO network, NO filesystem writes, NO env mutation.
/// Source-grep guards open files via env!("CARGO_MANIFEST_DIR").
///
/// Follows the existing convention from version_modernization_tests.rs:
///   env!("CARGO_MANIFEST_DIR") is kina-cli/ so ../ is the workspace root.
use assert_cmd::Command;
use std::path::Path;

// ---------------------------------------------------------------------------
// T1 — cargo_metadata_workspace_version_is_0_2_0
//
// Parse the root Cargo.toml (workspace root = env!("CARGO_MANIFEST_DIR")/../Cargo.toml).
// Assert [workspace.package].version == "0.2.0".
// Already true (root Cargo.toml:8) — this guards against accidental regression.
// ---------------------------------------------------------------------------

#[test]
fn cargo_metadata_workspace_version_is_0_2_0() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml_path = Path::new(manifest_dir)
        .parent()
        .expect("kina-cli has a parent workspace directory")
        .join("Cargo.toml");

    let cargo_toml_src = match std::fs::read_to_string(&cargo_toml_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read root Cargo.toml at {}: {}",
            cargo_toml_path.display(),
            e
        ),
    };

    // Parse with the `toml` crate (already a workspace dep) to get the structured value.
    // Use `toml::from_str` (deserializes a whole document) rather than the `FromStr`
    // impl: in toml 1.0 `str::parse::<toml::Value>` parses only a single value and
    // rejects a multi-table document.
    let doc: toml::Value =
        toml::from_str(&cargo_toml_src).expect("root Cargo.toml must be valid TOML");

    let version = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[workspace.package].version must be present in root Cargo.toml");

    assert_eq!(
        version, "0.2.0",
        "[workspace.package].version in root Cargo.toml must be '0.2.0' (regression guard); \
         found '{}'",
        version
    );
}

// ---------------------------------------------------------------------------
// T2 — cli_version_flag_reports_0_2_0
//
// `kina --version` succeeds and stdout contains "0.2.0".
// Proves clap #[command(version)] (mod.rs:25) surfaces the workspace version.
// ---------------------------------------------------------------------------

#[test]
fn cli_version_flag_reports_0_2_0() {
    let output = Command::cargo_bin("kina")
        .expect("kina binary must be buildable")
        .arg("--version")
        .output()
        .expect("kina --version must run without error");

    assert!(
        output.status.success(),
        "kina --version must exit 0; got status: {}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0.2.0"),
        "kina --version stdout must contain '0.2.0'; got: {:?}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// T3 — cli_version_matches_cargo_pkg_version
//
// stdout of `kina --version` contains env!("CARGO_PKG_VERSION").
// Proves build-time const and runtime CLI string are consistent.
// ---------------------------------------------------------------------------

#[test]
fn cli_version_matches_cargo_pkg_version() {
    const EXPECTED_VERSION: &str = env!("CARGO_PKG_VERSION");

    let output = Command::cargo_bin("kina")
        .expect("kina binary must be buildable")
        .arg("--version")
        .output()
        .expect("kina --version must run without error");

    assert!(
        output.status.success(),
        "kina --version must exit 0; got status: {}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(EXPECTED_VERSION),
        "kina --version stdout must contain CARGO_PKG_VERSION '{}'; got: {:?}",
        EXPECTED_VERSION,
        stdout
    );
}

// ---------------------------------------------------------------------------
// Helpers — shared path to release.yml
// ---------------------------------------------------------------------------

fn release_yml_path() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .expect("kina-cli has a parent workspace directory")
        .join(".github/workflows/release.yml")
}

fn read_release_yml() -> String {
    let path = release_yml_path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read .github/workflows/release.yml at {}: {} \
             (P3 implementer must deliver this file)",
            path.display(),
            e
        )
    })
}

// ---------------------------------------------------------------------------
// T4 — release_workflow_file_exists
//
// Path manifest_dir/../.github/workflows/release.yml is readable.
// Guards that P3 delivered the file.
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_file_exists() {
    let path = release_yml_path();
    let result = std::fs::read_to_string(&path);
    assert!(
        result.is_ok(),
        "release.yml must exist and be readable at {}; \
         P3 implementer must deliver .github/workflows/release.yml \
         (error: {:?})",
        path.display(),
        result.err()
    );
}

// ---------------------------------------------------------------------------
// T5 — release_workflow_triggers_on_semver_tag_only
//
// release.yml source contains the semver tag glob pattern AND does NOT contain
// a branch/PR push trigger. Tag-only cost guard: ubuntu ci.yml stays canonical
// for branch/PR builds.
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_triggers_on_semver_tag_only() {
    let src = read_release_yml();

    assert!(
        src.contains("v[0-9]+.[0-9]+.[0-9]+"),
        "release.yml must contain the semver tag glob 'v[0-9]+.[0-9]+.[0-9]+' \
         in its trigger; got (excerpt):\n{}",
        &src[..src.len().min(1000)]
    );

    // Must NOT have a branch: or pull_request: trigger — tag-only workflow.
    // Check for common branch trigger forms: "branches:" key.
    assert!(
        !src.contains("branches:"),
        "release.yml must NOT contain a 'branches:' trigger \
         (tag-only workflow; ubuntu ci.yml remains canonical for branch/PR CI); \
         got (excerpt):\n{}",
        &src[..src.len().min(1000)]
    );
}

// ---------------------------------------------------------------------------
// T6 — release_workflow_does_not_collide_with_kernel_tags
//
// release.yml does NOT contain "kernel-v" — proves mutual exclusivity with
// kernel-build.yml's 'kernel-v*' trigger. Also verifies kernel-build.yml
// still contains "kernel-v*" (untouched).
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_does_not_collide_with_kernel_tags() {
    let release_src = read_release_yml();

    assert!(
        !release_src.contains("kernel-v"),
        "release.yml must NOT contain 'kernel-v' (must not overlap with \
         kernel-build.yml's trigger pattern 'kernel-v*'); got (excerpt):\n{}",
        &release_src[..release_src.len().min(1000)]
    );

    // Also verify kernel-build.yml still contains "kernel-v*" (untouched by P3).
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let kernel_build_path = Path::new(manifest_dir)
        .parent()
        .expect("kina-cli has a parent workspace directory")
        .join(".github/workflows/kernel-build.yml");

    let kernel_src = match std::fs::read_to_string(&kernel_build_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read .github/workflows/kernel-build.yml: {} \
             (this file must remain untouched by P3)",
            e
        ),
    };

    assert!(
        kernel_src.contains("kernel-v*"),
        "kernel-build.yml must still contain 'kernel-v*' (P3 must NOT modify kernel-build.yml); \
         got (excerpt):\n{}",
        &kernel_src[..kernel_src.len().min(500)]
    );
}

// ---------------------------------------------------------------------------
// T7 — release_workflow_uses_macos_runner
//
// release.yml contains "runs-on: macos" — release build must produce an
// aarch64-apple-darwin Mach-O binary (Apple SDK is macOS-only; linux cross-compile
// is infeasible due to licensing and native-tls framework linking constraints).
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_uses_macos_runner() {
    let src = read_release_yml();

    assert!(
        src.contains("runs-on: macos"),
        "release.yml must contain 'runs-on: macos' \
         (release build requires macOS runner for aarch64-apple-darwin Mach-O); \
         got (excerpt):\n{}",
        &src[..src.len().min(1000)]
    );
}

// ---------------------------------------------------------------------------
// T8 — release_workflow_actions_are_sha_pinned
//
// Every `uses:` line in release.yml references a 40-hex commit SHA.
// No @vN tags, no @main — VERSION POLICY: SHA-pinned actions only.
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_actions_are_sha_pinned() {
    let src = read_release_yml();

    // Collect all `uses:` lines (trim whitespace; skip local composite action refs starting with ./).
    let uses_lines: Vec<&str> = src
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("uses:") || trimmed.contains("uses: ")
        })
        .filter(|line| {
            // Skip local composite actions (e.g. uses: ./.github/actions/setup-mise)
            let after_uses = line.trim().trim_start_matches("uses:").trim();
            !after_uses.starts_with("./")
        })
        .collect();

    // Every external `uses:` must end with @<40 lowercase hex chars>.
    // Pure string checks — no regex crate needed.
    for line in &uses_lines {
        let trimmed = line.trim();
        // Extract the part after `uses:`
        let uses_value = trimmed.trim_start_matches("uses:").trim();
        // Strip inline comment (everything from # onward after whitespace)
        let uses_value = uses_value.split_whitespace().next().unwrap_or(uses_value);

        // Must contain @ separator
        assert!(
            uses_value.contains('@'),
            "release.yml uses: line must specify a ref with @; found: {:?}",
            line.trim()
        );

        let ref_part = uses_value.split('@').last().unwrap_or("");
        // SHA-pinned: exactly 40 lowercase hex characters
        assert!(
            ref_part.len() == 40 && ref_part.chars().all(|c| c.is_ascii_hexdigit()),
            "release.yml uses: line must be SHA-pinned to a 40-hex commit SHA \
             (no @vN tags, no @main); line: {:?}; ref found: {:?}",
            line.trim(),
            ref_part
        );
    }
}

// ---------------------------------------------------------------------------
// T9 — release_asset_naming_is_github_backend_resolvable
//
// release.yml source contains "kina-v", "aarch64-apple-darwin", and ".sha256".
// The github-backend-resolvable asset name (kina-vX.Y.Z-aarch64-apple-darwin.tar.gz)
// plus the checksum sidecar (.sha256).
//
// Naming rationale (from P1 plan): mise asset_matcher.rs regexes:
//   macOS: (?i)(?:\b|_)(?:darwin|mac(?:osx?)?|osx)(?:\b|_)  -> 'darwin' matches
//   Arm64: (?i)(?:\b|_)(?:aarch_?64|arm_?64)(?:\b|_)        -> 'aarch64' matches
// 'kina-v0.1.0-aarch64-apple-darwin.tar.gz' resolves on macOS arm64 with NO tool options.
// ---------------------------------------------------------------------------

#[test]
fn release_asset_naming_is_github_backend_resolvable() {
    let src = read_release_yml();

    assert!(
        src.contains("kina-v"),
        "release.yml must contain 'kina-v' in the asset name template \
         (e.g. kina-v${{VERSION}}-aarch64-apple-darwin.tar.gz); got (excerpt):\n{}",
        &src[..src.len().min(1500)]
    );

    assert!(
        src.contains("aarch64-apple-darwin"),
        "release.yml must contain 'aarch64-apple-darwin' in the asset name \
         (required for mise github backend to resolve macOS arm64 binary unambiguously; \
         matches both 'aarch64' Arm64 regex and 'darwin' macOS regex); got (excerpt):\n{}",
        &src[..src.len().min(1500)]
    );

    assert!(
        src.contains(".sha256"),
        "release.yml must contain '.sha256' — the checksum sidecar asset \
         (e.g. kina-v${{VERSION}}-aarch64-apple-darwin.tar.gz.sha256); got (excerpt):\n{}",
        &src[..src.len().min(1500)]
    );
}

// ---------------------------------------------------------------------------
// T10 — release_workflow_sets_github_token_and_contents_write
//
// release.yml contains "GITHUB_TOKEN" and "contents: write".
// Auth for mise/gh and permission to publish the Release + assets.
// ---------------------------------------------------------------------------

#[test]
fn release_workflow_sets_github_token_and_contents_write() {
    let src = read_release_yml();

    assert!(
        src.contains("GITHUB_TOKEN"),
        "release.yml must contain 'GITHUB_TOKEN' (auth for mise/gh and release upload); \
         got (excerpt):\n{}",
        &src[..src.len().min(1000)]
    );

    assert!(
        src.contains("contents: write"),
        "release.yml must contain 'contents: write' \
         (GitHub permission required to publish Release and upload assets); \
         got (excerpt):\n{}",
        &src[..src.len().min(1000)]
    );
}
