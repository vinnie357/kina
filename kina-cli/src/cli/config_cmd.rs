use anyhow::Result;
use clap::{Args, Subcommand};
use tracing::info;

use crate::config::Config;

/// Manage kina configuration
#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set(SetArgs),

    /// Get a configuration value
    Get(GetConfigArgs),

    /// Reset configuration to defaults
    Reset,

    /// Show configuration file path
    Path,
}

#[derive(Args)]
pub struct SetArgs {
    /// Configuration key to set
    pub key: String,

    /// Configuration value to set
    pub value: String,
}

#[derive(Args)]
pub struct GetConfigArgs {
    /// Configuration key to get
    pub key: String,
}

impl ConfigArgs {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        match &self.command {
            ConfigCommands::Show => {
                println!("Current kina configuration:");
                println!("{:#?}", config);
            }
            ConfigCommands::Set(args) => {
                info!("Setting configuration: {} = {}", args.key, args.value);
                println!(
                    "Setting configuration key '{}' to '{}'",
                    args.key, args.value
                );
                println!("Note: Configuration setting will be implemented in a future version.");
                println!("For now, please edit the configuration file directly at:");
                if let Some(config_path) = &config.config_file_path {
                    println!("  {}", config_path.display());
                } else {
                    let default_path = Config::get_config_path();
                    println!("  {}", default_path.display());
                }
            }
            ConfigCommands::Get(args) => {
                info!("Getting configuration: {}", args.key);
                println!("Getting configuration key: '{}'", args.key);
                println!("Note: Configuration getting will be implemented in a future version.");
                println!("For now, please view the configuration file directly using:");
                println!("  kina config show");
            }
            ConfigCommands::Reset => {
                info!("Resetting configuration to defaults");
                println!("Resetting configuration to defaults...");
                let mut default_config = Config::default();

                if let Some(config_path) = &config.config_file_path {
                    default_config.config_file_path = Some(config_path.clone());
                    match default_config.save() {
                        Ok(()) => println!(
                            "Configuration reset to defaults and saved to: {}",
                            config_path.display()
                        ),
                        Err(e) => println!("Error saving reset configuration: {}", e),
                    }
                } else {
                    let default_path = Config::get_config_path();
                    default_config.config_file_path = Some(default_path.clone());
                    match default_config.save() {
                        Ok(()) => println!(
                            "Configuration reset to defaults and saved to: {}",
                            default_path.display()
                        ),
                        Err(e) => println!("Error saving reset configuration: {}", e),
                    }
                }
            }
            ConfigCommands::Path => {
                if let Some(config_path) = &config.config_file_path {
                    println!("{}", config_path.display());
                } else {
                    // Show the default config path even if file doesn't exist
                    let default_path = Config::get_config_path();
                    println!("{}", default_path.display());
                }
            }
        }

        Ok(())
    }
}
