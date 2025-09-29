use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::Config;

mod cluster;
mod config_cmd;

pub use cluster::*;
pub use config_cmd::*;

/// kina - Kubernetes in Apple Container
///
/// A CLI tool for managing local Kubernetes clusters using Apple Container technology,
/// providing similar functionality to kind (Kubernetes in Docker) but optimized for macOS.
#[derive(Parser)]
#[command(name = "kina")]
#[command(about = "Kubernetes in Apple Container - Local Kubernetes cluster management")]
#[command(
    long_about = "kina is a tool for running local Kubernetes clusters using Apple Container.\n\
It is designed as a replacement for kind (Kubernetes in Docker) on macOS systems,\n\
leveraging native Apple Container technology for improved performance and integration."
)]
#[command(version)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Configuration file path
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Kubernetes cluster
    Create(CreateArgs),

    /// Delete a Kubernetes cluster
    Delete(DeleteArgs),

    /// List existing clusters
    #[command(alias = "ls")]
    List(ListArgs),

    /// Show detailed status of a cluster
    Status(StatusArgs),

    /// Get information about clusters or resources
    Get(GetArgs),

    /// Load container images into clusters
    Load(LoadArgs),

    /// Install addons (ingress controllers, CNI, etc.)
    Install(InstallArgs),

    /// Export cluster configuration
    Export(ExportArgs),

    /// Approve pending kubelet Certificate Signing Requests
    #[command(name = "approve-csr")]
    ApproveCSR(ApproveCSRArgs),

    /// Manage kina configuration
    Config(ConfigArgs),
}

impl Cli {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        // Set up logging based on verbosity
        if self.verbose {
            // Increase logging level for verbose mode
            // This would typically adjust the tracing subscriber
        } else if self.quiet {
            // Suppress most output for quiet mode
        }

        // Execute the subcommand
        match &self.command {
            Some(Commands::Create(args)) => args.execute(config).await,
            Some(Commands::Delete(args)) => args.execute(config).await,
            Some(Commands::List(args)) => args.execute(config).await,
            Some(Commands::Status(args)) => args.execute(config).await,
            Some(Commands::Get(args)) => args.execute(config).await,
            Some(Commands::Load(args)) => args.execute(config).await,
            Some(Commands::Install(args)) => args.execute(config).await,
            Some(Commands::Export(args)) => args.execute(config).await,
            Some(Commands::ApproveCSR(args)) => args.execute(config).await,
            Some(Commands::Config(args)) => args.execute(config).await,
            None => {
                // No subcommand provided, show help
                println!("Use --help to see available commands");
                Ok(())
            }
        }
    }
}
