use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod config;
mod core;
mod errors;
mod utils;

use cli::Cli;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first so we can set log level
    let cli = Cli::parse();

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
