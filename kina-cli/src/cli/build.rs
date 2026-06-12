//! `kina build` subcommand — node image building.
//!
//! Defines `BuildArgs` (the `build` subcommand) and `BuildNodeImageArgs`
//! (the `build node-image` sub-subcommand) as public types exported from
//! the `cli` module.

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

// ---------------------------------------------------------------------------
// Arch enum
// ---------------------------------------------------------------------------

/// Target CPU architecture for the node image build.
///
/// Passed through to `container build --arch <arch>`.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Arch {
    /// ARM64 / Apple Silicon (default)
    Arm64,
    /// AMD64 / x86-64
    Amd64,
}

// ---------------------------------------------------------------------------
// BuildNodeImageArgs
// ---------------------------------------------------------------------------

/// Build a Kubernetes node image for Apple Container
///
/// Constructs a `container build` command against kina-cli/images/Dockerfile
/// using ARG-parameterized version pins.  The resulting image is cached by
/// content hash so unchanged inputs are skipped on subsequent runs.
#[derive(Parser, Debug)]
#[command(name = "node-image")]
pub struct BuildNodeImageArgs {
    /// Kubernetes version to embed (e.g. 1.36.1).
    /// Defaults to the pinned constant matching the Dockerfile ARG.
    #[arg(long)]
    pub kubernetes_version: Option<String>,

    /// Tag to apply to the built image (e.g. kina/node:v1.36.1)
    #[arg(long, required = true)]
    pub tag: String,

    /// Target CPU architecture (passed through to `container build --arch`)
    #[arg(long, value_enum, default_value = "arm64")]
    pub arch: Arch,

    /// Additional `--build-arg KEY=VALUE` pairs (repeatable).
    /// Matches or overrides Dockerfile ARG defaults.
    #[arg(long = "build-arg", value_name = "KEY=VALUE")]
    pub build_arg: Vec<String>,

    /// Skip the content-hash cache and force a rebuild.
    /// The cache entry is refreshed after a successful rebuild.
    #[arg(long)]
    pub no_cache: bool,
}

impl BuildNodeImageArgs {
    /// Parse from an iterator of string-like arguments.
    ///
    /// Delegates to `clap::Parser::parse_from`.  Provided so tests can call
    /// `BuildNodeImageArgs::parse_from_iter(["node-image", "--tag", "..."])`
    /// with the argv[0] convention that clap expects.
    ///
    /// Used by integration tests in `kina-cli/tests/node_image_build_tests.rs`;
    /// not called from the binary itself.
    #[allow(dead_code)]
    pub fn parse_from_iter<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        use clap::Parser as _;
        Self::parse_from(itr)
    }
}

// ---------------------------------------------------------------------------
// BuildSubcommands
// ---------------------------------------------------------------------------

/// Build subcommands
#[derive(Subcommand, Debug)]
pub enum BuildSubcommands {
    /// Build a Kubernetes node image
    #[command(name = "node-image")]
    NodeImage(BuildNodeImageArgs),
}

// ---------------------------------------------------------------------------
// BuildArgs
// ---------------------------------------------------------------------------

/// Build kina artefacts (node images, etc.)
#[derive(Args, Debug)]
pub struct BuildArgs {
    #[command(subcommand)]
    pub subcommand: BuildSubcommands,
}

impl BuildArgs {
    pub async fn execute(&self, config: &crate::config::Config) -> Result<()> {
        match &self.subcommand {
            BuildSubcommands::NodeImage(args) => args.execute(config).await,
        }
    }
}

impl BuildNodeImageArgs {
    pub async fn execute(&self, config: &crate::config::Config) -> Result<()> {
        use crate::core::node_image_builder::{
            build_inputs_hash, resolve_build_args, run_build, RunConfig,
        };
        use std::process::Command;

        // Locate the images directory relative to CARGO_MANIFEST_DIR at
        // runtime we derive it from the binary path or use a known relative path.
        // For a released binary we look next to the binary; for dev we use the
        // source tree.  The images dir is at kina-cli/images/ relative to the
        // workspace root.
        //
        // At runtime we resolve from the binary location.  In tests the binary
        // is in target/debug/kina so images is at ../../kina-cli/images relative
        // to the binary.  We walk up from the binary to find the workspace root.
        let images_dir = {
            let exe = std::env::current_exe().unwrap_or_default();
            // Walk up looking for kina-cli/images
            let candidate = exe
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|workspace| workspace.join("kina-cli").join("images"));
            if candidate.as_ref().map(|p| p.exists()).unwrap_or(false) {
                candidate.unwrap()
            } else {
                // Fallback: assume CWD is the workspace root
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("kina-cli")
                    .join("images")
            }
        };

        let dockerfile_path = images_dir.join("Dockerfile");
        let dockerfile_bytes = std::fs::read(&dockerfile_path).map_err(|e| {
            anyhow::anyhow!(
                "cannot read Dockerfile at {}: {}",
                dockerfile_path.display(),
                e
            )
        })?;

        let k8s_version = self.kubernetes_version.as_deref();
        let extra: Vec<&str> = self.build_arg.iter().map(|s| s.as_str()).collect();
        let resolved_args = resolve_build_args(k8s_version, &extra);

        let arch_str = match self.arch {
            Arch::Arm64 => "arm64",
            Arch::Amd64 => "amd64",
        };

        let data_dir = config.cluster.data_dir.clone();

        let _hash = build_inputs_hash(&dockerfile_bytes, &resolved_args, &self.tag);

        let config_struct = RunConfig {
            tag: self.tag.clone(),
            arch: arch_str.to_string(),
            no_cache: self.no_cache,
            dockerfile_bytes,
            resolved_args,
            data_dir,
            images_dir,
        };

        let runner_fn = |args: &[String]| -> std::result::Result<(), String> {
            let status = Command::new("container")
                .args(args)
                .status()
                .map_err(|e| format!("failed to run container: {}", e))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "container build exited with status: {}",
                    status.code().unwrap_or(-1)
                ))
            }
        };

        let image_lister = |_args: &[String]| -> std::result::Result<String, String> {
            let output = Command::new("container")
                .args(["image", "list", "--format", "json"])
                .output()
                .map_err(|e| format!("failed to run container image list: {}", e))?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        };

        match run_build(config_struct, runner_fn, image_lister) {
            Ok(msg) => {
                println!("{}", msg);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }
}
