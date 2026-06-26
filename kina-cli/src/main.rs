use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod config;
mod core;
mod errors;
mod version;

use cli::Cli;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Build a human-readable version string that includes git sha + build timestamp,
    // then override clap's default CARGO_PKG_VERSION string before parsing.
    //
    // human_version returns "kina X.Y.Z (...)" but clap prefixes the binary name
    // automatically, so we strip the leading "kina " to avoid "kina kina X.Y.Z".
    let version_str = version::human_version(&version::BUILD);
    let clap_version = version_str
        .strip_prefix("kina ")
        .map(str::to_owned)
        .unwrap_or_else(|| version_str.clone());
    let matches = Cli::command().version(clap_version).get_matches();
    // Parse command line arguments first so we can set log level
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // Initialize tracing subscriber — write to stderr so stdout stays clean for JSON/machine output
    let log_level = if cli.quiet {
        Level::ERROR
    } else if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Load configuration - respect the --config flag if provided
    let config = if let Some(config_path) = &cli.config {
        Config::load_from_file(config_path)?
    } else {
        Config::load()?
    };

    info!("Starting kina CLI application");

    // Execute the command
    cli.execute(&config).await
}
