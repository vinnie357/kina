/// Node image build system tests — adversarial TDD (P2 test-author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub types / fns / constants that do not
/// yet exist in:
///   - kina_cli::cli (Commands::Build, BuildArgs, BuildNodeImageArgs, Arch)
///   - kina_cli::core::node_image_builder (NodeImageBuilder, BuildDecision, build_inputs_hash,
///     cache_decision, CacheEntry, parse_image_list, image_present)
///
/// The compile errors ARE the spec. P3 (the separate implementer agent) makes them green
/// WITHOUT modifying this file.
///
/// BINDING VERSION CONSTANTS (from kina-cli/images/Dockerfile ARG defaults):
///   KUBERNETES_VERSION  = "1.35.5"
///   CONTAINERD_VERSION  = "2.3.1"
///   RUNC_VERSION        = "1.4.2"
///   CNI_PLUGINS_VERSION = "1.9.1"
///
/// All tests are PURE for pure-fn groups; subprocess boundary tests use a
/// RunnerFn trait-seam / fn-pointer pattern (same approach as KernelFetcher).
/// Zero real `container build` / `container image list` invocations.
/// Unique temp dirs with scope-drop cleanup. No bang (!) functions.
///
/// Import surface (binding for P3 implementer — module must export ALL of these from
/// kina_cli::core::node_image_builder):
///   NodeImageBuilder, BuildDecision, build_inputs_hash, cache_decision,
///   resolve_build_args, build_command_args, parse_image_list, image_present,
///   DEFAULT_KUBERNETES_VERSION, DEFAULT_CONTAINERD_VERSION,
///   DEFAULT_RUNC_VERSION, DEFAULT_CNI_PLUGINS_VERSION,
///
/// And from kina_cli::cli (via the clap integration):
///   BuildArgs, BuildNodeImageArgs, Arch, Commands::Build
use assert_cmd::Command;
use kina_cli::core::node_image_builder::{
    build_command_args, build_inputs_hash, cache_decision, image_present, parse_image_list,
    resolve_build_args, BuildDecision, DEFAULT_CNI_PLUGINS_VERSION, DEFAULT_CONTAINERD_VERSION,
    DEFAULT_KUBERNETES_VERSION, DEFAULT_RUNC_VERSION,
};
use std::collections::HashMap;
use std::path::PathBuf;

// ===========================================================================
// AC1 CLI: kina build node-image subcommand parsing
//
// Tests use assert_cmd to invoke the compiled `kina` binary so they exercise
// the real clap arg parser — no mock needed for CLI shape tests.
// ===========================================================================

/// AC1-1: `kina build node-image --help` documents all five flags
#[test]
fn build_node_image_help_lists_all_flags() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(["build", "node-image", "--help"]);
    let output = cmd.output().expect("kina binary must be invocable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let help_text = format!("{}{}", stdout, stderr);

    assert!(
        help_text.contains("--kubernetes-version") || help_text.contains("kubernetes-version"),
        "`kina build node-image --help` must document --kubernetes-version; got:\n{}",
        help_text
    );
    assert!(
        help_text.contains("--tag") || help_text.contains("tag"),
        "`kina build node-image --help` must document --tag; got:\n{}",
        help_text
    );
    assert!(
        help_text.contains("--arch") || help_text.contains("arch"),
        "`kina build node-image --help` must document --arch; got:\n{}",
        help_text
    );
    assert!(
        help_text.contains("--build-arg") || help_text.contains("build-arg"),
        "`kina build node-image --help` must document --build-arg; got:\n{}",
        help_text
    );
    assert!(
        help_text.contains("--no-cache") || help_text.contains("no-cache"),
        "`kina build node-image --help` must document --no-cache; got:\n{}",
        help_text
    );
}

/// AC1-2: `kina build --help` shows the node-image subcommand
#[test]
fn build_subcommand_appears_in_top_level_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(["build", "--help"]);
    let output = cmd.output().expect("kina binary must be invocable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let help_text = format!("{}{}", stdout, stderr);

    assert!(
        help_text.contains("node-image"),
        "`kina build --help` must list `node-image` as a subcommand; got:\n{}",
        help_text
    );
}

/// AC1-3: omitting --arch yields arch=arm64 in parsed BuildNodeImageArgs
#[test]
fn build_node_image_arch_defaults_to_arm64() {
    // We test this via the public parse path: parse BuildNodeImageArgs using
    // clap's parse_from so we don't need a live binary.
    use kina_cli::cli::BuildNodeImageArgs;
    // parse_from expects the command name as argv[0]
    let args = BuildNodeImageArgs::parse_from_iter(["node-image", "--tag", "kina/node:test"]);
    let arch_str = format!("{:?}", args.arch).to_lowercase();
    assert!(
        arch_str.contains("arm64"),
        "omitting --arch must yield arm64; got: {:?}",
        args.arch
    );
}

/// AC1-4: omitting --kubernetes-version yields the pinned default 1.35.5
#[test]
fn build_node_image_kubernetes_version_default_matches_dockerfile_arg() {
    use kina_cli::cli::BuildNodeImageArgs;
    let args = BuildNodeImageArgs::parse_from_iter(["node-image", "--tag", "kina/node:test"]);
    assert_eq!(
        args.kubernetes_version
            .as_deref()
            .unwrap_or(DEFAULT_KUBERNETES_VERSION),
        "1.35.5",
        "omitting --kubernetes-version must yield pinned default 1.35.5 (matches Dockerfile ARG); \
         got: {:?}",
        args.kubernetes_version
    );
}

/// AC1-5: two --build-arg K1=V1 --build-arg K2=V2 parse into Vec with both entries in order
#[test]
fn build_node_image_repeatable_build_arg_collects_multiple() {
    use kina_cli::cli::BuildNodeImageArgs;
    let args = BuildNodeImageArgs::parse_from_iter([
        "node-image",
        "--tag",
        "kina/node:test",
        "--build-arg",
        "K1=V1",
        "--build-arg",
        "K2=V2",
    ]);
    assert_eq!(
        args.build_arg.len(),
        2,
        "--build-arg repeated twice must collect 2 entries; got {} entries: {:?}",
        args.build_arg.len(),
        args.build_arg
    );
    assert_eq!(
        args.build_arg[0], "K1=V1",
        "first --build-arg must be K1=V1; got: {:?}",
        args.build_arg[0]
    );
    assert_eq!(
        args.build_arg[1], "K2=V2",
        "second --build-arg must be K2=V2; got: {:?}",
        args.build_arg[1]
    );
}

/// AC1-6: --no-cache sets no_cache=true; absence yields false
#[test]
fn build_node_image_no_cache_flag_parses_true() {
    use kina_cli::cli::BuildNodeImageArgs;

    let args_with = BuildNodeImageArgs::parse_from_iter([
        "node-image",
        "--tag",
        "kina/node:test",
        "--no-cache",
    ]);
    assert!(
        args_with.no_cache,
        "--no-cache flag must set no_cache=true; got false"
    );

    let args_without =
        BuildNodeImageArgs::parse_from_iter(["node-image", "--tag", "kina/node:test"]);
    assert!(
        !args_without.no_cache,
        "absence of --no-cache must yield no_cache=false; got true"
    );
}

/// AC1-7: an unsupported --arch value (e.g. mips) is rejected with non-zero exit
#[test]
fn build_node_image_invalid_arch_rejected() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args([
        "build",
        "node-image",
        "--tag",
        "kina/node:test",
        "--arch",
        "mips",
    ]);
    let output = cmd.output().expect("kina binary must be invocable");
    // clap ValueEnum with invalid value must fail with non-zero exit
    assert!(
        !output.status.success(),
        "--arch mips (unsupported value) must cause a non-zero exit; kina exited successfully"
    );
}

// ===========================================================================
// AC1+AC2: resolve_build_args — pinned defaults and overrides
//
// resolve_build_args(
//     kubernetes_version: Option<&str>,
//     extra_build_args: &[&str],   // ["KEY=VALUE", ...]
// ) -> Vec<(String, String)>
// ===========================================================================

/// AC1+AC2-8: resolve_build_args with no overrides returns all four pinned ARG pairs
#[test]
fn resolve_build_args_uses_pinned_defaults_when_unset() {
    let pairs = resolve_build_args(None, &[]);

    // Convert to HashMap for lookup
    let map: HashMap<String, String> = pairs.into_iter().collect();

    assert_eq!(
        map.get("KUBERNETES_VERSION").map(|s| s.as_str()),
        Some("1.35.5"),
        "KUBERNETES_VERSION must default to 1.35.5; got: {:?}",
        map.get("KUBERNETES_VERSION")
    );
    assert_eq!(
        map.get("CONTAINERD_VERSION").map(|s| s.as_str()),
        Some("2.3.1"),
        "CONTAINERD_VERSION must default to 2.3.1; got: {:?}",
        map.get("CONTAINERD_VERSION")
    );
    assert_eq!(
        map.get("RUNC_VERSION").map(|s| s.as_str()),
        Some("1.4.2"),
        "RUNC_VERSION must default to 1.4.2; got: {:?}",
        map.get("RUNC_VERSION")
    );
    assert_eq!(
        map.get("CNI_PLUGINS_VERSION").map(|s| s.as_str()),
        Some("1.9.1"),
        "CNI_PLUGINS_VERSION must default to 1.9.1; got: {:?}",
        map.get("CNI_PLUGINS_VERSION")
    );
}

/// AC1+AC2-9: --kubernetes-version override replaces only KUBERNETES_VERSION
#[test]
fn resolve_build_args_kubernetes_version_override_applies() {
    let pairs = resolve_build_args(Some("1.99.0"), &[]);
    let map: HashMap<String, String> = pairs.into_iter().collect();

    assert_eq!(
        map.get("KUBERNETES_VERSION").map(|s| s.as_str()),
        Some("1.99.0"),
        "--kubernetes-version 1.99.0 must override KUBERNETES_VERSION; got: {:?}",
        map.get("KUBERNETES_VERSION")
    );
    // Other three defaults must remain unchanged
    assert_eq!(
        map.get("CONTAINERD_VERSION").map(|s| s.as_str()),
        Some("2.3.1"),
        "CONTAINERD_VERSION must remain pinned at 2.3.1 when only kubernetes-version is overridden; \
         got: {:?}",
        map.get("CONTAINERD_VERSION")
    );
    assert_eq!(
        map.get("RUNC_VERSION").map(|s| s.as_str()),
        Some("1.4.2"),
        "RUNC_VERSION must remain pinned at 1.4.2 when only kubernetes-version is overridden; \
         got: {:?}",
        map.get("RUNC_VERSION")
    );
    assert_eq!(
        map.get("CNI_PLUGINS_VERSION").map(|s| s.as_str()),
        Some("1.9.1"),
        "CNI_PLUGINS_VERSION must remain pinned at 1.9.1 when only kubernetes-version is overridden; \
         got: {:?}",
        map.get("CNI_PLUGINS_VERSION")
    );
}

/// AC2-10: --build-arg with a key matching a default overrides it; novel key appended
#[test]
fn resolve_build_args_extra_build_args_merge_and_override() {
    // RUNC_VERSION=9.9.9 overrides the pinned default
    // MY_EXTRA_KEY=hello is a novel key that must be appended
    let pairs = resolve_build_args(None, &["RUNC_VERSION=9.9.9", "MY_EXTRA_KEY=hello"]);
    let map: HashMap<String, String> = pairs.into_iter().collect();

    assert_eq!(
        map.get("RUNC_VERSION").map(|s| s.as_str()),
        Some("9.9.9"),
        "--build-arg RUNC_VERSION=9.9.9 must override the default 1.4.2; got: {:?}",
        map.get("RUNC_VERSION")
    );
    assert_eq!(
        map.get("MY_EXTRA_KEY").map(|s| s.as_str()),
        Some("hello"),
        "novel --build-arg MY_EXTRA_KEY=hello must be appended; got: {:?}",
        map.get("MY_EXTRA_KEY")
    );
    // Other pinned defaults remain unchanged
    assert_eq!(
        map.get("KUBERNETES_VERSION").map(|s| s.as_str()),
        Some("1.35.5"),
        "KUBERNETES_VERSION must remain 1.35.5 when RUNC_VERSION is overridden; got: {:?}",
        map.get("KUBERNETES_VERSION")
    );
}

// ===========================================================================
// AC2: build_command_args — command construction
//
// build_command_args(
//     tag: &str,
//     arch: &str,
//     resolved_args: &[(String, String)],
//     no_cache: bool,
//     dockerfile_dir: &std::path::Path,
// ) -> Vec<String>
// ===========================================================================

/// AC2-11: build_command_args includes `build`, `-f <dockerfile>`, context dir, and `-t <tag>`
#[test]
fn build_command_args_target_dockerfile_and_context() {
    let resolved = resolve_build_args(None, &[]);
    let images_dir = PathBuf::from("/some/repo/kina-cli/images");
    let args = build_command_args("kina/node:v1.35.5", "arm64", &resolved, false, &images_dir);

    let joined = args.join(" ");

    assert!(
        args.contains(&"build".to_string()),
        "build_command_args must contain `build` as a token; got: {:?}",
        args
    );
    // Must reference the Dockerfile path
    let has_dockerfile =
        args.iter().any(|a| a.contains("Dockerfile")) && args.windows(2).any(|w| w[0] == "-f");
    assert!(
        has_dockerfile,
        "build_command_args must include `-f <path/to/Dockerfile>`; joined: {}",
        joined
    );
    // Must include the images context dir
    assert!(
        args.iter().any(|a| a.contains("images")),
        "build_command_args must include the images context directory path; joined: {}",
        joined
    );
    // Must include -t <tag>
    let has_tag = args
        .windows(2)
        .any(|w| w[0] == "-t" && w[1] == "kina/node:v1.35.5");
    assert!(
        has_tag,
        "build_command_args must include `-t kina/node:v1.35.5`; joined: {}",
        joined
    );
}

/// AC2-12: build_command_args passes the arch through as a platform/arch flag
#[test]
fn build_command_args_include_arch_platform_passthrough() {
    let resolved = resolve_build_args(None, &[]);
    let images_dir = PathBuf::from("/repo/kina-cli/images");
    let args = build_command_args("kina/node:test", "arm64", &resolved, false, &images_dir);
    let joined = args.join(" ");

    // Must contain arm64 somewhere in the args (either --arch arm64 or --platform linux/arm64)
    let has_arch = joined.contains("arm64");
    assert!(
        has_arch,
        "build_command_args must pass arch=arm64 through as an arch/platform flag; joined: {}",
        joined
    );
}

/// AC2-13: each resolved (KEY,VALUE) pair appears as `--build-arg KEY=VALUE` in the build command
#[test]
fn build_command_args_emit_each_build_arg_as_build_arg_flag() {
    let resolved = vec![
        ("KUBERNETES_VERSION".to_string(), "1.35.5".to_string()),
        ("CONTAINERD_VERSION".to_string(), "2.3.1".to_string()),
    ];
    let images_dir = PathBuf::from("/repo/kina-cli/images");
    let args = build_command_args("kina/node:test", "arm64", &resolved, false, &images_dir);
    let joined = args.join(" ");

    assert!(
        joined.contains("--build-arg") && joined.contains("KUBERNETES_VERSION=1.35.5"),
        "build_command_args must include `--build-arg KUBERNETES_VERSION=1.35.5`; joined: {}",
        joined
    );
    assert!(
        joined.contains("--build-arg") && joined.contains("CONTAINERD_VERSION=2.3.1"),
        "build_command_args must include `--build-arg CONTAINERD_VERSION=2.3.1`; joined: {}",
        joined
    );
}

/// AC2+AC3-14: when no_cache=true, `--no-cache` is in the args; when false, it is absent
#[test]
fn build_command_args_no_cache_adds_no_cache_flag() {
    let resolved = resolve_build_args(None, &[]);
    let images_dir = PathBuf::from("/repo/kina-cli/images");

    let args_with = build_command_args("kina/node:test", "arm64", &resolved, true, &images_dir);
    assert!(
        args_with.contains(&"--no-cache".to_string()),
        "build_command_args with no_cache=true must contain `--no-cache`; got: {:?}",
        args_with
    );

    let args_without = build_command_args("kina/node:test", "arm64", &resolved, false, &images_dir);
    assert!(
        !args_without.contains(&"--no-cache".to_string()),
        "build_command_args with no_cache=false must NOT contain `--no-cache`; got: {:?}",
        args_without
    );
}

// ===========================================================================
// AC3: BuildCache — content hash and cache hit/miss/invalidation
//
// build_inputs_hash(dockerfile_bytes: &[u8], resolved_args: &[(String, String)], tag: &str) -> String
// cache_decision(hash: &str, data_dir: &Path) -> BuildDecision
// ===========================================================================

/// AC3-15: build_inputs_hash is deterministic for identical inputs
#[test]
fn content_hash_is_deterministic_for_same_inputs() {
    let dockerfile_bytes = b"FROM debian:13-slim\nARG KUBERNETES_VERSION=1.35.5\n";
    let resolved = vec![("KUBERNETES_VERSION".to_string(), "1.35.5".to_string())];
    let tag = "kina/node:v1.35.5";

    let h1 = build_inputs_hash(dockerfile_bytes, &resolved, tag);
    let h2 = build_inputs_hash(dockerfile_bytes, &resolved, tag);

    assert_eq!(
        h1, h2,
        "build_inputs_hash must return the same hex string for identical inputs; h1={}, h2={}",
        h1, h2
    );
    // Must be a non-empty hex string
    assert!(
        !h1.is_empty() && h1.chars().all(|c| c.is_ascii_hexdigit()),
        "build_inputs_hash must return a non-empty lowercase hex string; got: {}",
        h1
    );
}

/// AC3-16: changing the tag produces a different content hash
#[test]
fn content_hash_changes_when_tag_changes() {
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let resolved = vec![("KUBERNETES_VERSION".to_string(), "1.35.5".to_string())];

    let h1 = build_inputs_hash(dockerfile_bytes, &resolved, "kina/node:v1.35.5");
    let h2 = build_inputs_hash(dockerfile_bytes, &resolved, "kina/node:v1.99.0");

    assert_ne!(
        h1, h2,
        "changing only the tag must produce a different content hash; both gave: {}",
        h1
    );
}

/// AC3-17: changing one resolved build-arg value produces a different content hash
#[test]
fn content_hash_changes_when_build_arg_changes() {
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let tag = "kina/node:v1.35.5";

    let resolved1 = vec![("KUBERNETES_VERSION".to_string(), "1.35.5".to_string())];
    let resolved2 = vec![("KUBERNETES_VERSION".to_string(), "1.99.0".to_string())];

    let h1 = build_inputs_hash(dockerfile_bytes, &resolved1, tag);
    let h2 = build_inputs_hash(dockerfile_bytes, &resolved2, tag);

    assert_ne!(
        h1, h2,
        "changing a build-arg value must produce a different content hash; both gave: {}",
        h1
    );
}

/// AC3-18: different Dockerfile bytes produce a different content hash
#[test]
fn content_hash_changes_when_dockerfile_bytes_change() {
    let tag = "kina/node:v1.35.5";
    let resolved = vec![("KUBERNETES_VERSION".to_string(), "1.35.5".to_string())];

    let h1 = build_inputs_hash(b"FROM debian:13-slim\n", &resolved, tag);
    let h2 = build_inputs_hash(b"FROM debian:bookworm-slim\n", &resolved, tag);

    assert_ne!(
        h1, h2,
        "different Dockerfile bytes must produce a different content hash; both gave: {}",
        h1
    );
}

/// AC3-19: cache_decision against empty data_dir returns BuildDecision::Miss
#[test]
fn cache_lookup_miss_when_no_entry_exists() {
    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let hash = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";

    let decision = cache_decision(hash, tmp.path());

    assert!(
        matches!(decision, BuildDecision::Miss),
        "cache_decision against empty data_dir must return BuildDecision::Miss; got: {:?}",
        decision
    );
}

/// AC3-20: write then lookup returns BuildDecision::Hit for the same hash
#[test]
fn cache_write_then_lookup_is_hit_for_same_hash() {
    use kina_cli::core::node_image_builder::write_cache_entry;

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let hash = "1122334455667788990011223344556677889900112233445566778899001122";

    // Write a cache entry for this hash
    write_cache_entry(hash, tmp.path()).expect("write_cache_entry must not fail");

    // Now look it up
    let decision = cache_decision(hash, tmp.path());
    assert!(
        matches!(decision, BuildDecision::Hit),
        "after write_cache_entry, cache_decision for same hash must return BuildDecision::Hit; \
         got: {:?}",
        decision
    );
}

/// AC3-21: a stored entry for H1 yields Miss when queried with H2
#[test]
fn cache_lookup_miss_when_hash_differs_from_stored() {
    use kina_cli::core::node_image_builder::write_cache_entry;

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let h1 = "aaaabbbbccccddddeeeeffffaaaabbbbccccddddeeeeffffaaaabbbbccccddd0";
    let h2 = "1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff";

    write_cache_entry(h1, tmp.path()).expect("write_cache_entry must not fail");

    let decision = cache_decision(h2, tmp.path());
    assert!(
        matches!(decision, BuildDecision::Miss),
        "cache_decision with hash H2 (different from stored H1) must return BuildDecision::Miss \
         (invalidation on input change); got: {:?}",
        decision
    );
}

/// AC3-22: with a Hit-eligible entry, no_cache=true forces a rebuild decision
#[test]
fn no_cache_forces_miss_even_when_entry_present() {
    use kina_cli::core::node_image_builder::{build_decision_with_no_cache, write_cache_entry};

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let hash = "cafebabe00112233445566778899aabbccddeeff00112233445566778899aabb";

    write_cache_entry(hash, tmp.path()).expect("write_cache_entry must not fail");

    // With no_cache=true, even a Hit-eligible entry must return Rebuild
    let decision = build_decision_with_no_cache(hash, tmp.path(), true);
    assert!(
        matches!(decision, BuildDecision::Rebuild),
        "build_decision_with_no_cache(no_cache=true) must return BuildDecision::Rebuild even \
         when a cache entry exists; got: {:?}",
        decision
    );
}

/// AC3-23: run_build with a pre-seeded cache Hit does NOT invoke runner and surfaces cache-hit message
#[test]
fn cache_hit_skips_build_with_explicit_message() {
    use kina_cli::core::node_image_builder::{run_build, write_cache_entry, RunConfig};

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let resolved = resolve_build_args(None, &[]);
    let tag = "kina/node:cache-hit-test";

    // Pre-seed the cache entry so run_build sees a Hit
    let hash = build_inputs_hash(dockerfile_bytes, &resolved, tag);
    write_cache_entry(&hash, tmp.path()).expect("write_cache_entry must not fail");

    let mut runner_called = false;
    let runner_fn = |_args: &[String]| -> Result<(), String> {
        runner_called = true;
        Ok(())
    };

    // Fake image lister — not called on cache hit, but must be provided
    let image_lister = |_args: &[String]| -> Result<String, String> { Ok("[]".to_string()) };

    let config = RunConfig {
        tag: tag.to_string(),
        arch: "arm64".to_string(),
        no_cache: false,
        dockerfile_bytes: dockerfile_bytes.to_vec(),
        resolved_args: resolved,
        data_dir: tmp.path().to_path_buf(),
        images_dir: PathBuf::from("/fake/images"),
    };

    let result = run_build(config, runner_fn, image_lister);
    assert!(
        result.is_ok(),
        "run_build on cache Hit must return Ok; got Err: {:?}",
        result
    );
    assert!(
        !runner_called,
        "run_build on cache Hit must NOT invoke the runner (skips build); runner was called"
    );
    let message = result.unwrap();
    assert!(
        message.contains(tag) || message.to_lowercase().contains("cache"),
        "run_build cache-hit result must surface a message containing the tag or 'cache'; \
         got: {}",
        message
    );
}

/// AC3-24: run_build with no_cache=true invokes runner and rewrites cache entry
#[test]
fn no_cache_run_refreshes_entry_after_rebuild() {
    use kina_cli::core::node_image_builder::{run_build, write_cache_entry, RunConfig};

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let resolved = resolve_build_args(None, &[]);
    let tag = "kina/node:no-cache-refresh";

    // Pre-seed a cache entry (would normally be a Hit)
    let hash = build_inputs_hash(dockerfile_bytes, &resolved, tag);
    write_cache_entry(&hash, tmp.path()).expect("write_cache_entry must not fail");

    let mut runner_call_count = 0u32;
    let runner_fn = |_args: &[String]| -> Result<(), String> {
        runner_call_count += 1;
        Ok(())
    };

    // Fake image lister that returns the tag as present
    let image_lister_json = format!("[{{\"name\":\"{}\",\"id\":\"sha256:abc\"}}]", tag);
    let image_lister =
        move |_args: &[String]| -> Result<String, String> { Ok(image_lister_json.clone()) };

    let config = RunConfig {
        tag: tag.to_string(),
        arch: "arm64".to_string(),
        no_cache: true,
        dockerfile_bytes: dockerfile_bytes.to_vec(),
        resolved_args: resolved,
        data_dir: tmp.path().to_path_buf(),
        images_dir: PathBuf::from("/fake/images"),
    };

    let result = run_build(config, runner_fn, image_lister);
    assert!(
        result.is_ok(),
        "run_build with no_cache=true must return Ok; got Err: {:?}",
        result
    );
    assert_eq!(
        runner_call_count, 1,
        "run_build with no_cache=true must invoke the runner exactly once; called {} times",
        runner_call_count
    );

    // Cache entry must have been refreshed (still exists after rebuild)
    let decision_after = cache_decision(&hash, tmp.path());
    assert!(
        matches!(decision_after, BuildDecision::Hit),
        "after no_cache run_build success, cache entry must be refreshed (Hit); got: {:?}",
        decision_after
    );
}

// ===========================================================================
// AC4: post-build verification — parse_image_list and image_present
//
// parse_image_list(json: &str) -> Result<Vec<ImageEntry>, String>
// image_present(list: &[ImageEntry], tag: &str) -> bool
// struct ImageEntry { name: String, ... }
// ===========================================================================

/// AC4-25: parse_image_list over JSON containing the built tag returns the entry
#[test]
fn post_build_verification_parses_image_list_json_present() {
    let json = r#"[{"name":"kina/node:v1.35.5","id":"sha256:abc123","size":"500MB"}]"#;
    let list = parse_image_list(json).expect("parse_image_list must succeed for valid JSON");

    assert_eq!(
        list.len(),
        1,
        "parse_image_list for one-element JSON array must return 1 entry; got {} entries",
        list.len()
    );
    assert!(
        image_present(&list, "kina/node:v1.35.5"),
        "image_present must return true when tag 'kina/node:v1.35.5' is in the parsed list"
    );
}

/// AC4-26: image_present returns false when the tag is missing from the parsed list
#[test]
fn post_build_verification_absent_image_is_failure() {
    let json = r#"[{"name":"kina/node:other-tag","id":"sha256:def456"}]"#;
    let list = parse_image_list(json).expect("parse_image_list must succeed for valid JSON");

    assert!(
        !image_present(&list, "kina/node:v1.35.5"),
        "image_present must return false when 'kina/node:v1.35.5' is not in the list"
    );
}

/// AC4-27: fake runner exit 0 + image lister missing the tag causes run_build to return Err
#[test]
fn run_build_treats_runner_exit_zero_without_image_as_error() {
    use kina_cli::core::node_image_builder::{run_build, RunConfig};

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let resolved = resolve_build_args(None, &[]);
    let tag = "kina/node:verify-fail-test";

    // Runner succeeds (exit 0 equivalent)
    let runner_fn = |_args: &[String]| -> Result<(), String> { Ok(()) };

    // Image lister returns a list WITHOUT the tag (build "succeeded" but image not found)
    let image_lister = |_args: &[String]| -> Result<String, String> {
        Ok(r#"[{"name":"kina/node:other","id":"sha256:aaa"}]"#.to_string())
    };

    let config = RunConfig {
        tag: tag.to_string(),
        arch: "arm64".to_string(),
        no_cache: false,
        dockerfile_bytes: dockerfile_bytes.to_vec(),
        resolved_args: resolved,
        data_dir: tmp.path().to_path_buf(),
        images_dir: PathBuf::from("/fake/images"),
    };

    let result = run_build(config, runner_fn, image_lister);
    assert!(
        result.is_err(),
        "run_build must return Err when runner exits 0 but image is NOT present in list \
         (exit 0 is not trusted as success — AC4); got Ok"
    );
}

/// AC4-28: fake runner Ok + fake lister returning the tag causes run_build to return Ok
#[test]
fn run_build_succeeds_when_runner_ok_and_image_present() {
    use kina_cli::core::node_image_builder::{run_build, RunConfig};

    let tmp = tempfile::TempDir::new().expect("tempdir must be creatable");
    let dockerfile_bytes = b"FROM debian:13-slim\n";
    let resolved = resolve_build_args(None, &[]);
    let tag = "kina/node:success-test";

    let runner_fn = |_args: &[String]| -> Result<(), String> { Ok(()) };

    let tag_clone = tag.to_string();
    let image_lister = move |_args: &[String]| -> Result<String, String> {
        Ok(format!(r#"[{{"name":"{}","id":"sha256:bbb"}}]"#, tag_clone))
    };

    let config = RunConfig {
        tag: tag.to_string(),
        arch: "arm64".to_string(),
        no_cache: false,
        dockerfile_bytes: dockerfile_bytes.to_vec(),
        resolved_args: resolved,
        data_dir: tmp.path().to_path_buf(),
        images_dir: PathBuf::from("/fake/images"),
    };

    let result = run_build(config, runner_fn, image_lister);
    assert!(
        result.is_ok(),
        "run_build must return Ok when runner exits 0 AND image is present in lister output; \
         got Err: {:?}",
        result
    );
}

/// AC4-29: parse_image_list returns empty for whitespace; Err for malformed JSON
#[test]
fn parse_image_list_empty_and_malformed_handled() {
    // Whitespace-only input must return empty list (mirrors parse_container_list contract)
    let empty_result = parse_image_list("   \n  ");
    assert!(
        empty_result.is_ok(),
        "parse_image_list for whitespace input must return Ok([]); got Err: {:?}",
        empty_result
    );
    assert!(
        empty_result.unwrap().is_empty(),
        "parse_image_list for whitespace input must return an empty list"
    );

    // Malformed JSON must return Err
    let malformed_result = parse_image_list("{not valid json}}}");
    assert!(
        malformed_result.is_err(),
        "parse_image_list for malformed JSON must return Err; got Ok"
    );

    // Non-array top level must return Err
    let non_array_result = parse_image_list(r#"{"name":"foo"}"#);
    assert!(
        non_array_result.is_err(),
        "parse_image_list for non-array top level must return Err; got Ok"
    );
}

// ===========================================================================
// AC2/AC4 guard: source-grep — node_image_builder.rs version policy check
// ===========================================================================

/// AC2/AC4-30: node_image_builder.rs references images/Dockerfile and contains no `latest` in
/// pinned default constants (version policy)
#[test]
fn source_guard_builder_uses_images_dockerfile_path() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/node_image_builder.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read src/core/node_image_builder.rs for source-grep guard: {}. \
             This is expected to fail RED until P3 creates the file.",
            e
        ),
    };

    // Must reference images/Dockerfile path (the -f argument construction)
    let has_dockerfile_ref = src.contains("images/Dockerfile") || src.contains("Dockerfile");
    assert!(
        has_dockerfile_ref,
        "node_image_builder.rs must reference 'images/Dockerfile' for the build -f argument; \
         not found in source"
    );

    // Version policy: pinned default constant strings must NOT be "latest" or floating tags
    let has_latest_in_default = src.contains("\"latest\"")
        && (src.contains("DEFAULT_KUBERNETES_VERSION")
            || src.contains("DEFAULT_CONTAINERD_VERSION")
            || src.contains("DEFAULT_RUNC_VERSION")
            || src.contains("DEFAULT_CNI_PLUGINS_VERSION"));
    assert!(
        !has_latest_in_default,
        "node_image_builder.rs pinned default constants must NOT use 'latest' — version policy \
         kina-2: every version pinned to an explicit release"
    );
}

/// AC1-31: DEFAULT_* constants from node_image_builder match Dockerfile ARG pinned values
#[test]
fn pinned_defaults_match_dockerfile_args() {
    assert_eq!(
        DEFAULT_KUBERNETES_VERSION, "1.35.5",
        "DEFAULT_KUBERNETES_VERSION must be 1.35.5 (matches Dockerfile ARG); got: {}",
        DEFAULT_KUBERNETES_VERSION
    );
    assert_eq!(
        DEFAULT_CONTAINERD_VERSION, "2.3.1",
        "DEFAULT_CONTAINERD_VERSION must be 2.3.1 (matches Dockerfile ARG); got: {}",
        DEFAULT_CONTAINERD_VERSION
    );
    assert_eq!(
        DEFAULT_RUNC_VERSION, "1.4.2",
        "DEFAULT_RUNC_VERSION must be 1.4.2 (matches Dockerfile ARG); got: {}",
        DEFAULT_RUNC_VERSION
    );
    assert_eq!(
        DEFAULT_CNI_PLUGINS_VERSION, "1.9.1",
        "DEFAULT_CNI_PLUGINS_VERSION must be 1.9.1 (matches Dockerfile ARG); got: {}",
        DEFAULT_CNI_PLUGINS_VERSION
    );
}
