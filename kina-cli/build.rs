/// Build script — emits git provenance and build metadata via vergen-gitcl.
///
/// Non-git vars (build timestamp, rustc, cargo target) are always emitted.
/// Git vars are optional: if the build runs outside a `.git` tree (e.g. a bare
/// source tarball), the script prints a warning and continues.  Source code
/// uses `option_env!("VERGEN_GIT_*")` so missing vars gracefully become `None`.
fn main() {
    // Non-git vars always available.
    emit_non_git_vars();
    // Git vars optional — warn and continue if unavailable.
    if let Err(e) = emit_git_vars() {
        println!("cargo:warning=kina build.rs: git provenance unavailable: {e}");
    }
}

fn emit_non_git_vars() {
    use vergen_gitcl::{Build, Cargo, Emitter, Rustc};

    let build = Build::all_build();
    let cargo = Cargo::all_cargo();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .add_instructions(&build)
        .expect("add build instructions failed")
        .add_instructions(&cargo)
        .expect("add cargo instructions failed")
        .add_instructions(&rustc)
        .expect("add rustc instructions failed")
        .emit()
        .expect("emit non-git vars failed");
}

fn emit_git_vars() -> Result<(), Box<dyn std::error::Error>> {
    use vergen_gitcl::{Emitter, Gitcl};

    // Enable all git vars.
    // - describe(true, true, None): use `git describe --tags --dirty` so we get
    //   a tag-relative string like `v0.1.0-21-g4e2ffe7-dirty`.
    // - sha(true): pass `--short` to `git rev-parse HEAD` → 7-char hex sha.
    let gitcl = Gitcl::all().describe(true, true, None).sha(true).build();

    Emitter::default().add_instructions(&gitcl)?.emit()?;

    Ok(())
}
