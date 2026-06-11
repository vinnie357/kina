/// load_image_into_container — adversarial TDD (P2 test-author stage)
///
/// Tests are INTENTIONALLY RED: they reference pub fns that do not yet exist in
/// kina_cli::core::apple_container. The compile errors are the spec; P3 (the
/// separate implementer agent) makes them green WITHOUT modifying this file.
///
/// All tests are pure: NO live `container` CLI invocations, NO process spawns,
/// NO network, NO filesystem writes.
/// Source-grep guard test (all_cp_call_sites_migrated_or_verified) opens the
/// source file via env!("CARGO_MANIFEST_DIR") — same pattern as version_modernization_tests.rs.
///
/// Import surface (BINDING for P3 implementer):
///   use kina_cli::core::apple_container::{
///       build_inject_tar_args,
///       build_remote_size_args,
///       build_remote_sha256_args,
///       sha256_hex,
///       parse_remote_size_output,
///       parse_remote_sha256_output,
///       verify_injection,
///   };
///
/// Required pub free functions (P3 must expose in kina_cli::core::apple_container):
///
///   pub fn build_inject_tar_args(container_id: &str, dest_path: &str) -> Vec<String>
///   pub fn build_remote_size_args(container_id: &str, dest_path: &str) -> Vec<String>
///   pub fn build_remote_sha256_args(container_id: &str, dest_path: &str) -> Vec<String>
///   pub fn sha256_hex(bytes: &[u8]) -> String
///   pub fn parse_remote_size_output(raw: &str) -> anyhow::Result<u64>
///   pub fn parse_remote_sha256_output(raw: &str) -> anyhow::Result<String>
///   pub fn verify_injection(
///       local_len: u64, local_sha: &str, remote_size_raw: &str, remote_sha_raw: &str,
///   ) -> anyhow::Result<()>
use kina_cli::core::apple_container::{
    build_inject_tar_args, build_remote_sha256_args, build_remote_size_args,
    parse_remote_sha256_output, parse_remote_size_output, sha256_hex, verify_injection,
};

// ===========================================================================
// AC1 — exec-stdin injection command construction
// ===========================================================================

/// build_inject_tar_args_streams_via_exec_stdin
///
/// AC1: the first element of the returned arg vector is "exec", indicating
/// the exec-stdin pattern (not "cp"). The container CLI is invoked as
/// `container exec -i <id> sh -c 'cat > /path'`, whose first arg after the
/// binary name is "exec".
#[test]
fn build_inject_tar_args_streams_via_exec_stdin() {
    let args = build_inject_tar_args("mycontainer", "/tmp/image.tar");
    assert_eq!(
        args.first().map(String::as_str),
        Some("exec"),
        "build_inject_tar_args must start with \"exec\" (exec-stdin pattern); got {:?}",
        args
    );
}

/// build_inject_tar_args_targets_correct_container_and_path
///
/// AC1: the returned vector contains the container_id and a sh -c payload
/// that references the dest_path with a "cat >" redirection.
#[test]
fn build_inject_tar_args_targets_correct_container_and_path() {
    let container_id = "kina-control-plane";
    let dest_path = "/tmp/image.tar";
    let args = build_inject_tar_args(container_id, dest_path);

    // container_id must appear in the vector
    assert!(
        args.iter().any(|a| a == container_id),
        "build_inject_tar_args must contain the container_id \"{}\"; got {:?}",
        container_id,
        args
    );

    // At least one arg must contain both the dest_path and "cat >"
    let has_payload = args
        .iter()
        .any(|a| a.contains(dest_path) && a.contains("cat >"));
    assert!(
        has_payload,
        "build_inject_tar_args must contain a sh -c payload with \"cat >\" and the dest_path \"{}\"; got {:?}",
        dest_path,
        args
    );

    // Must include "-i" for stdin piping
    assert!(
        args.iter().any(|a| a == "-i"),
        "build_inject_tar_args must contain \"-i\" (stdin piping flag); got {:?}",
        args
    );
}

/// build_inject_tar_args_contains_no_cp_subcommand
///
/// AC1/AC3: the exec-stdin vector must never contain an element equal to "cp".
/// This is the hard guard that confirms the cp subcommand was fully replaced.
#[test]
fn build_inject_tar_args_contains_no_cp_subcommand() {
    let args = build_inject_tar_args("any-container", "/tmp/image.tar");
    assert!(
        !args.iter().any(|a| a == "cp"),
        "build_inject_tar_args must NOT contain \"cp\"; the cp subcommand has been replaced by exec-stdin; got {:?}",
        args
    );
}

// ===========================================================================
// AC2 — remote-size and remote-sha command construction
// ===========================================================================

/// build_remote_size_args_constructs_stat_for_dest_path
///
/// AC2: the remote-size vector starts with "exec" and its sh -c payload
/// references the dest_path, producing a byte-count command (wc -c or stat).
#[test]
fn build_remote_size_args_constructs_stat_for_dest_path() {
    let container_id = "kina-worker";
    let dest_path = "/tmp/image.tar";
    let args = build_remote_size_args(container_id, dest_path);

    assert_eq!(
        args.first().map(String::as_str),
        Some("exec"),
        "build_remote_size_args must start with \"exec\"; got {:?}",
        args
    );

    assert!(
        args.iter().any(|a| a == container_id),
        "build_remote_size_args must contain the container_id \"{}\"; got {:?}",
        container_id,
        args
    );

    let has_path_in_payload = args.iter().any(|a| a.contains(dest_path));
    assert!(
        has_path_in_payload,
        "build_remote_size_args payload must reference the dest_path \"{}\"; got {:?}",
        dest_path, args
    );
}

/// build_remote_sha256_args_constructs_sha_for_dest_path
///
/// AC2: the remote-sha256 vector starts with "exec" and its sh -c payload
/// contains "sha256sum" and references the dest_path.
#[test]
fn build_remote_sha256_args_constructs_sha_for_dest_path() {
    let container_id = "kina-worker";
    let dest_path = "/tmp/image.tar";
    let args = build_remote_sha256_args(container_id, dest_path);

    assert_eq!(
        args.first().map(String::as_str),
        Some("exec"),
        "build_remote_sha256_args must start with \"exec\"; got {:?}",
        args
    );

    assert!(
        args.iter().any(|a| a == container_id),
        "build_remote_sha256_args must contain the container_id \"{}\"; got {:?}",
        container_id,
        args
    );

    let has_sha256sum = args.iter().any(|a| a.contains("sha256sum"));
    assert!(
        has_sha256sum,
        "build_remote_sha256_args payload must contain \"sha256sum\"; got {:?}",
        args
    );

    let has_path_in_payload = args.iter().any(|a| a.contains(dest_path));
    assert!(
        has_path_in_payload,
        "build_remote_sha256_args payload must reference the dest_path \"{}\"; got {:?}",
        dest_path, args
    );
}

// ===========================================================================
// AC2 — local digest helper
// ===========================================================================

/// local_tar_sha256_matches_known_digest
///
/// AC2: sha256_hex on a known byte slice produces the expected hex digest.
/// The sha256 of an empty byte slice is the standard empty-string sha256.
#[test]
fn local_tar_sha256_matches_known_digest() {
    // sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let digest = sha256_hex(b"");
    assert_eq!(
        digest, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "sha256_hex(b\"\") must return the standard empty-string sha256 digest"
    );
}

/// sha256_hex produces correct digest for known non-empty input.
/// sha256("abc") = ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7
#[test]
fn local_tar_sha256_abc_matches_known_digest() {
    let digest = sha256_hex(b"abc");
    assert_eq!(
        digest, "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7",
        "sha256_hex(b\"abc\") must return the known sha256 digest"
    );
}

// ===========================================================================
// AC2 — remote output parsing
// ===========================================================================

/// parse_remote_size_output_extracts_byte_count
///
/// AC2: a line like "  12345\n" (typical wc -c output) is parsed into 12345u64.
#[test]
fn parse_remote_size_output_extracts_byte_count() {
    let raw = "  12345\n";
    let result = parse_remote_size_output(raw);
    assert!(
        result.is_ok(),
        "parse_remote_size_output(\"  12345\\n\") must be Ok; got {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        12345u64,
        "parse_remote_size_output must extract the numeric byte count"
    );
}

/// parse_remote_size_output_also_handles_stat_style
///
/// AC2: output with trailing whitespace or a leading number with spaces is parsed.
#[test]
fn parse_remote_size_output_also_handles_leading_whitespace() {
    let raw = "       0";
    let result = parse_remote_size_output(raw);
    assert!(
        result.is_ok(),
        "parse_remote_size_output(\"       0\") must be Ok; got {:?}",
        result
    );
    assert_eq!(result.unwrap(), 0u64);
}

/// parse_remote_size_output_errors_on_unparseable
///
/// AC2/AC5: garbage or empty remote size output is an error (treated as failed
/// verification), not silently zero. This prevents a missing file from being
/// treated as a 0-byte match.
#[test]
fn parse_remote_size_output_errors_on_unparseable() {
    for garbage in &["", "not-a-number", "abc 123", "\n\n", "   "] {
        let result = parse_remote_size_output(garbage);
        assert!(
            result.is_err(),
            "parse_remote_size_output({:?}) must be Err for unparseable input; got {:?}",
            garbage,
            result
        );
    }
}

/// parse_remote_sha256_output_extracts_hex_digest
///
/// AC2: sha256sum prints "<hex>  <path>"; the leading hex token is extracted.
#[test]
fn parse_remote_sha256_output_extracts_hex_digest() {
    let raw = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7  /tmp/image.tar\n";
    let result = parse_remote_sha256_output(raw);
    assert!(
        result.is_ok(),
        "parse_remote_sha256_output must be Ok for standard sha256sum output; got {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7",
        "parse_remote_sha256_output must return only the hex token"
    );
}

/// parse_remote_sha256_output_errors_on_garbage
///
/// AC2: empty or garbage sha256sum output is an error.
#[test]
fn parse_remote_sha256_output_errors_on_garbage() {
    for garbage in &["", "not-a-hash", "\n", "   "] {
        let result = parse_remote_sha256_output(garbage);
        assert!(
            result.is_err(),
            "parse_remote_sha256_output({:?}) must be Err for garbage input; got {:?}",
            garbage,
            result
        );
    }
}

// ===========================================================================
// AC2/AC5 — verify_injection (pure hard-error gate)
// ===========================================================================

/// verify_injection_ok_when_local_and_remote_match
///
/// AC2: when local_len matches remote size AND local_sha matches remote sha,
/// verify_injection returns Ok(()) so ctr import proceeds.
#[test]
fn verify_injection_ok_when_local_and_remote_match() {
    let local_len: u64 = 12345;
    let local_sha = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7";
    let remote_size_raw = "12345";
    let remote_sha_raw =
        "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7  /tmp/image.tar";

    let result = verify_injection(local_len, local_sha, remote_size_raw, remote_sha_raw);
    assert!(
        result.is_ok(),
        "verify_injection must return Ok when sizes and digests match; got {:?}",
        result
    );
}

/// verify_injection_errors_on_size_mismatch
///
/// AC2: mismatched remote size (nonzero local vs smaller remote) is a hard
/// error, not a warning. The error message must cite both expected and actual.
#[test]
fn verify_injection_errors_on_size_mismatch() {
    let local_len: u64 = 99999;
    let local_sha = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7";
    let remote_size_raw = "50000"; // mismatch
    let remote_sha_raw =
        "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7  /tmp/image.tar";

    let result = verify_injection(local_len, local_sha, remote_size_raw, remote_sha_raw);
    assert!(
        result.is_err(),
        "verify_injection must return Err when remote size (50000) != local size (99999)"
    );

    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("99999") || msg.contains("50000"),
        "error message must cite expected ({}) and/or actual ({}) sizes; got: {}",
        local_len,
        50000,
        msg
    );
}

/// verify_injection_errors_on_silent_noop_zero_bytes
///
/// AC2/AC5: simulated silent cp no-op — remote size reports 0 (file absent or
/// cp silently did nothing) against a nonzero local_len — is a hard error
/// before import runs.
#[test]
fn verify_injection_errors_on_silent_noop_zero_bytes() {
    let local_len: u64 = 4096;
    let local_sha = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7";
    let remote_size_raw = "0"; // silent no-op / file absent
    let remote_sha_raw =
        "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7  /tmp/image.tar";

    let result = verify_injection(local_len, local_sha, remote_size_raw, remote_sha_raw);
    assert!(
        result.is_err(),
        "verify_injection must return Err when remote size is 0 (silent no-op) but local_len is {}",
        local_len
    );
}

/// verify_injection_errors_on_sha_mismatch
///
/// AC2: equal sizes but mismatched sha256 is a hard error.
#[test]
fn verify_injection_errors_on_sha_mismatch() {
    let local_len: u64 = 12345;
    let local_sha = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7";
    let remote_size_raw = "12345"; // sizes match
    let remote_sha_raw =
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef  /tmp/image.tar";

    let result = verify_injection(local_len, local_sha, remote_size_raw, remote_sha_raw);
    assert!(
        result.is_err(),
        "verify_injection must return Err when sha256 does not match (corruption guard)"
    );
}

/// verify_injection_error_message_names_expected_and_actual
///
/// AC2/AC5: the verification error propagates a message citing expected vs
/// actual values so callers can see why the import was blocked.
#[test]
fn verify_injection_error_message_names_expected_and_actual() {
    let local_len: u64 = 8192;
    let local_sha = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7";
    let remote_size_raw = "1024"; // mismatch
    let remote_sha_raw =
        "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132bffc726eac3fa7  /tmp/image.tar";

    let err = verify_injection(local_len, local_sha, remote_size_raw, remote_sha_raw)
        .expect_err("verify_injection must return Err on size mismatch");

    let msg = format!("{:#}", err);
    // The error must reference the local expected size and the remote actual size
    // so operators can diagnose a silent no-op without inspecting raw bytes.
    let cites_expected = msg.contains("8192");
    let cites_actual = msg.contains("1024");
    assert!(
        cites_expected || cites_actual,
        "verify_injection error must cite expected ({}) and/or actual ({}) in message; got: {}",
        local_len,
        1024,
        msg
    );
}

// ===========================================================================
// AC3 — source-grep guard: no remaining "cp" subcommand call sites
// ===========================================================================

/// all_cp_call_sites_migrated_or_verified
///
/// AC3: reads apple_container.rs via CARGO_MANIFEST_DIR and asserts it
/// contains no `"cp",` array-element literal — i.e. the single cp call site
/// at the old load_image_into_container block has been replaced by the
/// exec-stdin path. This mirrors the source-introspection guard style of
/// version_modernization_tests.rs.
#[test]
fn all_cp_call_sites_migrated_or_verified() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = std::path::Path::new(manifest_dir).join("src/core/apple_container.rs");
    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => panic!(
            "cannot read src/core/apple_container.rs for guard test: {}",
            e
        ),
    };

    // The implementer must have removed the "cp", array element.
    // If this assertion fails, the cp subcommand was not replaced.
    assert!(
        !src.contains("\"cp\","),
        "apple_container.rs must contain NO `\"cp\",` array-element literal after migration; \
         the load_image_into_container cp call site must be replaced by the exec-stdin path. \
         Found `\"cp\",` in the source."
    );
}
