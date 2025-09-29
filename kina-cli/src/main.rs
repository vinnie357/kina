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
    // Initialize tracing subscriber for logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Parse command line arguments
    let cli = Cli::parse();

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
