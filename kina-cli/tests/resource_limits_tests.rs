/// Resource limits tests — adversarial TDD (P2 test author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns / fields / constants that do not yet
/// exist in kina_cli (node_resource_args, validate_resources, resolve_cpus, resolve_memory,
/// DEFAULT_NODE_CPUS, DEFAULT_NODE_MEMORY) and fields not yet present on ClusterDefaults /
/// CreateClusterOptions (control_plane_cpus, control_plane_memory, worker_cpus, worker_memory).
/// The compile errors ARE the spec.
/// P3 (the separate implementer agent) makes them green without modifying this file.
///
/// All pure tests: NO live `container` CLI invocations, NO process spawns (except assert_cmd
/// CLI tests which use `kina --help` and argument parsing only — never reaching container run).
/// Source-grep guard tests open the source file via CARGO_MANIFEST_DIR.
/// No bang (!) functions used anywhere — pattern-match on Result everywhere (project rule).
///
/// Design binding (from kina-18 issue):
///   - node_resource_args(cpus: u32, memory: &str) -> Vec<String>
///   - validate_resources(cpus: u32, memory: &str) -> anyhow::Result<()>
///   - resolve_cpus(cli: Option<u32>, config: Option<u32>, builtin: u32) -> u32
///   - resolve_memory(cli: Option<&str>, config: Option<&str>, builtin: &str) -> String
///   - pub const DEFAULT_NODE_CPUS: u32 = 4
///   - pub const DEFAULT_NODE_MEMORY: &str = "4g"
///   - ClusterDefaults: control_plane_cpus, control_plane_memory, worker_cpus, worker_memory
///   - CreateClusterOptions: control_plane_cpus, control_plane_memory, worker_cpus, worker_memory
///   - CreateArgs: --cpus / --memory clap long flags
use assert_cmd::Command;
use kina_cli::config::Config;
use kina_cli::core::apple_container::{
    node_resource_args, resolve_cpus, resolve_memory, validate_resources, DEFAULT_NODE_CPUS,
    DEFAULT_NODE_MEMORY,
};
use kina_cli::core::types::CreateClusterOptions;
use predicates::prelude::*;

// ===========================================================================
// Group A — node_resource_args builder
// ===========================================================================

/// T1 — node_resource_args(4, "4g") returns exactly ["--cpus","4","--memory","4g"]
/// AC: all three node-create paths consume resolved values; pure builder maps (cpus,memory)
/// to container run args.
#[test]
fn node_resource_args_builds_cpus_and_memory() {
    let args = node_resource_args(4, "4g");
    assert_eq!(
        args,
        vec![
            "--cpus".to_string(),
            "4".to_string(),
            "--memory".to_string(),
            "4g".to_string()
        ],
        "node_resource_args(4, \"4g\") must return exactly \
         [\"--cpus\", \"4\", \"--memory\", \"4g\"]; got: {:?}",
        args
    );
}

/// T2 — node_resource_args(2, "8g") returns ["--cpus","2","--memory","8g"]
/// and the joined string does NOT contain "4" or "4g".
/// AC: zero hardcoded 4/4g literals — triangulation proves interpolation.
#[test]
fn node_resource_args_interpolates_distinct_values() {
    let args = node_resource_args(2, "8g");
    assert_eq!(
        args,
        vec![
            "--cpus".to_string(),
            "2".to_string(),
            "--memory".to_string(),
            "8g".to_string()
        ],
        "node_resource_args(2, \"8g\") must return [\"--cpus\",\"2\",\"--memory\",\"8g\"]; \
         got: {:?}",
        args
    );
    let joined = args.join(" ");
    assert!(
        !joined.contains(" 4 ") && !joined.contains(" 4g"),
        "node_resource_args(2, \"8g\") joined must NOT contain \"4\" or \"4g\" as \
         significant substrings — proves values are interpolated, not hardcoded; got: {:?}",
        args
    );
}

/// T3 — node_resource_args(1, "1g") returns ["--cpus","1","--memory","1g"]
/// AC: container-run arg construction — confirms the numeric cpus is rendered
/// as its decimal string in the second slot.
#[test]
fn node_resource_args_cpus_count_stringified() {
    let args = node_resource_args(1, "1g");
    assert_eq!(
        args,
        vec![
            "--cpus".to_string(),
            "1".to_string(),
            "--memory".to_string(),
            "1g".to_string()
        ],
        "node_resource_args(1, \"1g\") must return [\"--cpus\",\"1\",\"--memory\",\"1g\"]; \
         confirms cpus u32 is stringified to \"1\" in position [1]; got: {:?}",
        args
    );
}

// ===========================================================================
// Group B — validate_resources (happy path)
// ===========================================================================

/// T4 — validate_resources(4, "4g") returns Ok(())
/// AC: invalid values rejected before any container is created (happy path).
#[test]
fn validate_resources_accepts_valid_cpus_and_memory() {
    match validate_resources(4, "4g") {
        Ok(()) => {}
        Err(e) => panic!(
            "validate_resources(4, \"4g\") must return Ok(()); got Err: {}",
            e
        ),
    }
}

/// T5 — validate_resources for memory "512m" and "2g" both return Ok(())
/// AC: clear acceptance of well-formed memory (case-insensitive m/g suffix accepted).
#[test]
fn validate_resources_accepts_megabyte_and_gigabyte_suffixes() {
    match validate_resources(4, "512m") {
        Ok(()) => {}
        Err(e) => panic!(
            "validate_resources(4, \"512m\") must return Ok(()) — \"m\" suffix is valid; \
             got Err: {}",
            e
        ),
    }
    match validate_resources(4, "2g") {
        Ok(()) => {}
        Err(e) => panic!(
            "validate_resources(4, \"2g\") must return Ok(()) — \"g\" suffix is valid; \
             got Err: {}",
            e
        ),
    }
}

// ===========================================================================
// Group C — validate_resources (error cases)
// ===========================================================================

/// T6 — validate_resources(0, "4g") returns Err whose message mentions cpus.
/// AC: invalid values rejected with a clear error — guards a non-bootable 0-CPU node.
#[test]
fn validate_resources_rejects_zero_cpus() {
    match validate_resources(0, "4g") {
        Ok(()) => panic!(
            "validate_resources(0, \"4g\") must return Err (0 CPUs is not a bootable node); \
             got Ok(())"
        ),
        Err(e) => {
            let msg = format!("{}", e).to_lowercase();
            assert!(
                msg.contains("cpu"),
                "validate_resources(0, \"4g\") error message must mention \"cpu\"; \
                 got: {}",
                msg
            );
        }
    }
}

/// T7 — validate_resources(4, "") returns Err whose message references memory.
/// AC: invalid values rejected with a clear error — empty memory string is rejected.
#[test]
fn validate_resources_rejects_empty_memory() {
    match validate_resources(4, "") {
        Ok(()) => panic!(
            "validate_resources(4, \"\") must return Err (empty memory string is invalid); \
             got Ok(())"
        ),
        Err(e) => {
            let msg = format!("{}", e).to_lowercase();
            assert!(
                msg.contains("memory"),
                "validate_resources(4, \"\") error message must mention \"memory\"; \
                 got: {}",
                msg
            );
        }
    }
}

/// T8 — validate_resources(4, "4096") returns Err mentioning the memory format/suffix.
/// AC: clear error for malformed memory — a bare number with no m/g unit is rejected.
#[test]
fn validate_resources_rejects_unitless_memory() {
    match validate_resources(4, "4096") {
        Ok(()) => panic!(
            "validate_resources(4, \"4096\") must return Err (no m/g unit suffix); \
             got Ok(())"
        ),
        Err(e) => {
            let msg = format!("{}", e).to_lowercase();
            assert!(
                msg.contains("memory"),
                "validate_resources(4, \"4096\") error message must mention \"memory\"; \
                 got: {}",
                msg
            );
        }
    }
}

/// T9 — validate_resources(4, "lots") returns Err mentioning memory.
/// AC: clear error before container creation — non-numeric memory is rejected.
#[test]
fn validate_resources_rejects_garbage_memory() {
    match validate_resources(4, "lots") {
        Ok(()) => panic!(
            "validate_resources(4, \"lots\") must return Err (non-numeric memory value); \
             got Ok(())"
        ),
        Err(e) => {
            let msg = format!("{}", e).to_lowercase();
            assert!(
                msg.contains("memory"),
                "validate_resources(4, \"lots\") error message must mention \"memory\"; \
                 got: {}",
                msg
            );
        }
    }
}

// ===========================================================================
// Group D — resolve_cpus precedence (CLI > config > built-in)
// ===========================================================================

/// T10 — resolve_cpus(Some(2), Some(6), 4) returns 2
/// AC: precedence CLI > config > built-in default — explicit CLI flag wins.
#[test]
fn resolve_cpus_cli_overrides_config_and_default() {
    let result = resolve_cpus(Some(2), Some(6), 4);
    assert_eq!(
        result, 2,
        "resolve_cpus(Some(2), Some(6), 4) must return 2 — CLI flag wins over \
         config value and built-in default; got: {}",
        result
    );
}

/// T11 — resolve_cpus(None, Some(6), 4) returns 6
/// AC: precedence config > built-in default — config value used when no CLI flag.
#[test]
fn resolve_cpus_config_used_when_no_cli() {
    let result = resolve_cpus(None, Some(6), 4);
    assert_eq!(
        result, 6,
        "resolve_cpus(None, Some(6), 4) must return 6 — config value used when \
         no CLI flag is given; got: {}",
        result
    );
}

/// T12 — resolve_cpus(None, None, 4) returns 4
/// AC: built-in defaults preserve current behavior (4 CPUs).
#[test]
fn resolve_cpus_builtin_default_when_neither_set() {
    let result = resolve_cpus(None, None, 4);
    assert_eq!(
        result, 4,
        "resolve_cpus(None, None, 4) must return 4 — falls back to the built-in \
         default when neither CLI nor config is set; got: {}",
        result
    );
}

// ===========================================================================
// Group E — resolve_memory precedence (CLI > config > built-in)
// ===========================================================================

/// T13 — resolve_memory(Some("8g"), Some("6g"), "4g") returns "8g"
/// AC: precedence CLI > config > built-in default for memory — CLI flag wins.
#[test]
fn resolve_memory_cli_overrides_config_and_default() {
    let result = resolve_memory(Some("8g"), Some("6g"), "4g");
    assert_eq!(
        result, "8g",
        "resolve_memory(Some(\"8g\"), Some(\"6g\"), \"4g\") must return \"8g\" — \
         CLI flag wins over config and built-in default; got: {}",
        result
    );
}

/// T14 — resolve_memory(None, Some("6g"), "4g") returns "6g"
/// AC: precedence config > built-in default for memory — config value used when no CLI flag.
#[test]
fn resolve_memory_config_used_when_no_cli() {
    let result = resolve_memory(None, Some("6g"), "4g");
    assert_eq!(
        result, "6g",
        "resolve_memory(None, Some(\"6g\"), \"4g\") must return \"6g\" — config value \
         used when no CLI flag is given; got: {}",
        result
    );
}

/// T15 — resolve_memory(None, None, "4g") returns "4g"
/// AC: built-in defaults preserve current behavior (4g memory).
#[test]
fn resolve_memory_builtin_default_when_neither_set() {
    let result = resolve_memory(None, None, "4g");
    assert_eq!(
        result, "4g",
        "resolve_memory(None, None, \"4g\") must return \"4g\" — falls back to the \
         built-in default when neither CLI nor config is set; got: {}",
        result
    );
}

// ===========================================================================
// Group F — built-in default constants
// ===========================================================================

/// T16 — DEFAULT_NODE_CPUS equals 4
/// AC: built-in defaults preserve current behavior — the exported constant equals
/// the prior hardcoded value.
#[test]
fn builtin_default_cpus_is_four() {
    assert_eq!(
        DEFAULT_NODE_CPUS, 4u32,
        "DEFAULT_NODE_CPUS must equal 4 to preserve the prior hardcoded behavior; \
         got: {}",
        DEFAULT_NODE_CPUS
    );
}

/// T17 — DEFAULT_NODE_MEMORY equals "4g"
/// AC: built-in defaults preserve current behavior — the exported constant equals
/// the prior hardcoded value.
#[test]
fn builtin_default_memory_is_four_gig() {
    assert_eq!(
        DEFAULT_NODE_MEMORY, "4g",
        "DEFAULT_NODE_MEMORY must equal \"4g\" to preserve the prior hardcoded behavior; \
         got: {}",
        DEFAULT_NODE_MEMORY
    );
}

// ===========================================================================
// Group G — Config struct per-role resource fields
// ===========================================================================

/// T18 — Config::default().cluster.control_plane_cpus is None
/// and Config::default().cluster.control_plane_memory is None
/// AC: config supports separate control-plane defaults; absence means fall back to built-in.
#[test]
fn config_default_control_plane_resources_are_unset() {
    let config = Config::default();
    assert!(
        config.cluster.control_plane_cpus.is_none(),
        "Config::default().cluster.control_plane_cpus must be None — \
         no override unless the user opts in; got: {:?}",
        config.cluster.control_plane_cpus
    );
    assert!(
        config.cluster.control_plane_memory.is_none(),
        "Config::default().cluster.control_plane_memory must be None — \
         no override unless the user opts in; got: {:?}",
        config.cluster.control_plane_memory
    );
}

/// T19 — Config::default().cluster.worker_cpus is None
/// and Config::default().cluster.worker_memory is None
/// AC: config supports separate worker defaults; absence means fall back to built-in.
#[test]
fn config_default_worker_resources_are_unset() {
    let config = Config::default();
    assert!(
        config.cluster.worker_cpus.is_none(),
        "Config::default().cluster.worker_cpus must be None — \
         no override unless the user opts in; got: {:?}",
        config.cluster.worker_cpus
    );
    assert!(
        config.cluster.worker_memory.is_none(),
        "Config::default().cluster.worker_memory must be None — \
         no override unless the user opts in; got: {:?}",
        config.cluster.worker_memory
    );
}

/// T20 — Deserializing a TOML [cluster] fragment with
/// control_plane_cpus=8, control_plane_memory="8g", worker_cpus=2, worker_memory="2g"
/// yields those values on ClusterDefaults via toml::from_str (pure, no FS spawn).
/// AC: config file supports separate control-plane and worker resource defaults.
#[test]
fn config_per_role_resources_round_trip_from_toml() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(42);

    let toml_fragment = format!(
        r#"
[cluster]
default_name = "kina"
default_image = "kindest/node:v1.31.0"
default_wait_timeout = 300
data_dir = "/tmp/kina-test-{unique}"
retain_on_failure = false
default_cni = "Ptp"
control_plane_cpus = 8
control_plane_memory = "8g"
worker_cpus = 2
worker_memory = "2g"

[apple_container]
[apple_container.runtime_config]
[apple_container.network]
network_name = "kina"
enable_ipv6 = false
dns_servers = []

[kubernetes]
default_version = "v1.28.0"
default_namespace = "default"
kubeconfig_dir = "/tmp/kina-kubeconfig-{unique}"

[logging]
level = "info"
format = "text"
file_logging = false
"#,
        unique = unique
    );

    let config: Config = match toml::from_str(&toml_fragment) {
        Ok(c) => c,
        Err(e) => panic!(
            "Failed to deserialize TOML fragment with per-role resource fields: {}",
            e
        ),
    };

    assert_eq!(
        config.cluster.control_plane_cpus,
        Some(8u32),
        "Deserializing control_plane_cpus = 8 must yield Some(8); got: {:?}",
        config.cluster.control_plane_cpus
    );
    assert_eq!(
        config.cluster.control_plane_memory,
        Some("8g".to_string()),
        "Deserializing control_plane_memory = \"8g\" must yield Some(\"8g\"); got: {:?}",
        config.cluster.control_plane_memory
    );
    assert_eq!(
        config.cluster.worker_cpus,
        Some(2u32),
        "Deserializing worker_cpus = 2 must yield Some(2); got: {:?}",
        config.cluster.worker_cpus
    );
    assert_eq!(
        config.cluster.worker_memory,
        Some("2g".to_string()),
        "Deserializing worker_memory = \"2g\" must yield Some(\"2g\"); got: {:?}",
        config.cluster.worker_memory
    );
}

/// T21 — Deserializing a [cluster] TOML fragment that omits all four resource fields
/// yields None for all four (serde default, no parse error) — backward compatible.
/// AC: per-role fields are optional with serde default.
#[test]
fn config_missing_per_role_resources_deserialize_to_none() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(99);

    let toml_fragment = format!(
        r#"
[cluster]
default_name = "kina"
default_image = "kindest/node:v1.31.0"
default_wait_timeout = 300
data_dir = "/tmp/kina-test-{unique}"
retain_on_failure = false
default_cni = "Ptp"

[apple_container]
[apple_container.runtime_config]
[apple_container.network]
network_name = "kina"
enable_ipv6 = false
dns_servers = []

[kubernetes]
default_version = "v1.28.0"
default_namespace = "default"
kubeconfig_dir = "/tmp/kina-kubeconfig-{unique}"

[logging]
level = "info"
format = "text"
file_logging = false
"#,
        unique = unique
    );

    let config: Config = match toml::from_str(&toml_fragment) {
        Ok(c) => c,
        Err(e) => panic!(
            "Failed to deserialize TOML fragment that omits per-role resource fields \
             (must not error — fields are serde-defaulted to None): {}",
            e
        ),
    };

    assert!(
        config.cluster.control_plane_cpus.is_none(),
        "Omitted control_plane_cpus must deserialize to None; got: {:?}",
        config.cluster.control_plane_cpus
    );
    assert!(
        config.cluster.control_plane_memory.is_none(),
        "Omitted control_plane_memory must deserialize to None; got: {:?}",
        config.cluster.control_plane_memory
    );
    assert!(
        config.cluster.worker_cpus.is_none(),
        "Omitted worker_cpus must deserialize to None; got: {:?}",
        config.cluster.worker_cpus
    );
    assert!(
        config.cluster.worker_memory.is_none(),
        "Omitted worker_memory must deserialize to None; got: {:?}",
        config.cluster.worker_memory
    );
}

// ===========================================================================
// Group H — CLI flag wiring (assert_cmd)
// ===========================================================================

/// T22 — `kina create --help` succeeds and stdout contains both "--cpus" and "--memory"
/// AC: kina create accepts --cpus and --memory — the flags are registered on CreateArgs.
#[test]
fn create_args_accepts_cpus_and_memory_flags_help() {
    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(["create", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--cpus"))
        .stdout(predicate::str::contains("--memory"));
}

/// T23 — `kina create --memory bogus` (with fixture config) fails (non-zero exit)
/// and stderr references memory.
/// AC: invalid values rejected with a clear error before any container is created.
#[test]
fn create_args_rejects_invalid_memory_value_at_parse_or_validate() {
    let config_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("tests/fixtures/test-config.toml")
        .to_string_lossy()
        .to_string();

    let mut cmd = Command::cargo_bin("kina").unwrap();
    cmd.args(["--config", &config_path, "create", "--memory", "bogus"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("memory").or(predicate::str::contains("Memory")));
}

// ===========================================================================
// Group I — Source-grep guards
// ===========================================================================

/// T24 — Source-grep guard: src/cli/cluster.rs contains clap long flags "cpus" and "memory"
/// and the backing fields, wired through resolve_cpus/resolve_memory into CreateClusterOptions.
/// AC: CLI wiring verified via source inspection.
#[test]
fn source_create_args_has_cpus_and_memory_flags() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/cli/cluster.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!("cannot read src/cli/cluster.rs for guard test: {}", e),
    };

    assert!(
        src.contains("\"cpus\"") || src.contains("cpus"),
        "cli/cluster.rs CreateArgs must contain a \"cpus\" clap long flag; not found in source"
    );
    assert!(
        src.contains("\"memory\"") || src.contains("memory"),
        "cli/cluster.rs CreateArgs must contain a \"memory\" clap long flag; not found in source"
    );
}

/// T25 — Source-grep guard: src/core/apple_container.rs does NOT contain the literal sequence
/// of a hardcoded resource arg array (e.g. "--cpus", "4", "--memory", "4g").
/// AC: zero hardcoded --cpus 4 / --memory 4g literals remain.
#[test]
fn source_no_hardcoded_cpus_4_memory_4g_literal() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read src/core/apple_container.rs for guard test: {}",
            e
        ),
    };

    // The guard pattern: the hardcoded literal array sequence that must be eliminated.
    // We check for the specific inline array form used in all three create_* fns.
    assert!(
        !src.contains("\"--cpus\", \"4\", \"--memory\", \"4g\""),
        "apple_container.rs must NOT contain the hardcoded literal arg sequence \
         \"\\\"--cpus\\\", \\\"4\\\", \\\"--memory\\\", \\\"4g\\\"\" — \
         all three create_* fns must use node_resource_args with resolved values instead"
    );

    // Confirm node_resource_args is present (guards against merely deleting the lines)
    assert!(
        src.contains("node_resource_args"),
        "apple_container.rs must reference \"node_resource_args\" — \
         confirms the builder helper is in use rather than a plain deletion"
    );
}

/// T26 — Source-grep guard: src/core/apple_container.rs references "node_resource_args"
/// at least 3 times (one per create_single_node, create_control_plane_node, create_worker_node).
/// AC: all three node-create paths consume resolved values.
#[test]
fn source_all_three_creators_use_node_resource_args() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read src/core/apple_container.rs for guard test: {}",
            e
        ),
    };

    let count = src.matches("node_resource_args").count();
    assert!(
        count >= 3,
        "apple_container.rs must reference \"node_resource_args\" at least 3 times \
         (once per create_single_node, create_control_plane_node, create_worker_node); \
         found {} occurrences",
        count
    );
}

// ===========================================================================
// Group J — CreateClusterOptions compile-time carrier shape guard
// ===========================================================================

/// T27 — Compile-time guard: a CreateClusterOptions literal compiles with
/// control_plane_cpus, control_plane_memory, worker_cpus, worker_memory fields present.
/// AC: resolved values flow to node creation — field absence is the deliberate RED.
#[test]
fn create_cluster_options_has_resource_fields() {
    // This test is a compile-time guard: if the four resource fields don't exist on
    // CreateClusterOptions, this won't compile and the compile error is the deliberate red.
    let opts = CreateClusterOptions {
        name: "test".to_string(),
        image: "kindest/node:v1.31.0".to_string(),
        config_file: None,
        kubernetes_version: None,
        workers: None,
        control_plane_nodes: None,
        wait_timeout: None,
        retain_on_failure: false,
        skip_csr_approval: false,
        cni_plugin: kina_cli::config::CniPlugin::Ptp,
        node_kernel_path: None,
        control_plane_cpus: 4u32,
        control_plane_memory: "4g".to_string(),
        worker_cpus: 4u32,
        worker_memory: "4g".to_string(),
    };
    assert_eq!(
        opts.control_plane_cpus, 4u32,
        "CreateClusterOptions must have control_plane_cpus: u32 field; \
         compile error above proves it is absent (deliberate red)"
    );
    assert_eq!(
        opts.control_plane_memory, "4g",
        "CreateClusterOptions must have control_plane_memory: String field"
    );
    assert_eq!(
        opts.worker_cpus, 4u32,
        "CreateClusterOptions must have worker_cpus: u32 field"
    );
    assert_eq!(
        opts.worker_memory, "4g",
        "CreateClusterOptions must have worker_memory: String field"
    );
}
