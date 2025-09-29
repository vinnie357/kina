use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Utility functions for file and directory operations
#[allow(dead_code)]
pub mod fs {
    use super::*;
    use std::fs;

    /// Ensure a directory exists, creating it if necessary
    pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            fs::create_dir_all(path)?;
            debug!("Created directory: {}", path.display());
        }
        Ok(())
    }

    /// Get the user's home directory
    pub fn home_dir() -> Option<PathBuf> {
        dirs::home_dir()
    }

    /// Get the user's config directory for the application
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("kina"))
    }

    /// Get the user's data directory for the application
    pub fn data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join("kina"))
    }

    /// Check if a file exists and is readable
    pub fn is_readable_file<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.is_file()
            && path
                .metadata()
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false)
    }

    /// Safe file writing with atomic operations
    pub fn write_file_atomic<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
        let path = path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            ensure_dir(parent)?;
        }

        // Use a temporary file for atomic writing
        let temp_path = path.with_extension("tmp");

        std::fs::write(&temp_path, content)?;
        std::fs::rename(&temp_path, path)?;

        debug!("Wrote file atomically: {}", path.display());
        Ok(())
    }
}

/// Utility functions for process and command execution
#[allow(dead_code)]
pub mod process {
    use super::*;
    use std::process::{Command, Stdio};
    use tokio::process::Command as AsyncCommand;

    /// Check if a command exists in PATH
    pub fn command_exists(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Execute a command and capture output
    pub async fn execute_command(command: &str, args: &[&str]) -> Result<(String, String, bool)> {
        let mut cmd = AsyncCommand::new(command);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        Ok((stdout, stderr, success))
    }

    /// Get the version of a command
    pub async fn get_command_version(command: &str) -> Result<String> {
        let (stdout, stderr, success) = execute_command(command, &["--version"]).await?;

        if success {
            Ok(stdout.lines().next().unwrap_or("").trim().to_string())
        } else {
            // Some commands use -v instead of --version
            let (stdout, _, success) = execute_command(command, &["-v"]).await?;
            if success {
                Ok(stdout.lines().next().unwrap_or("").trim().to_string())
            } else {
                Err(anyhow::anyhow!("Could not determine version: {}", stderr))
            }
        }
    }
}

/// Utility functions for text and string manipulation
#[allow(dead_code)]
pub mod text {
    /// Truncate text to a maximum length with ellipsis
    pub fn truncate(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }

    /// Format a duration in human-readable form
    pub fn format_duration(duration: std::time::Duration) -> String {
        let secs = duration.as_secs();
        let mins = secs / 60;
        let hours = mins / 60;
        let days = hours / 24;

        if days > 0 {
            format!("{}d {}h", days, hours % 24)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins % 60)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs % 60)
        } else {
            format!("{}s", secs)
        }
    }

    /// Format bytes in human-readable form
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: u64 = 1024;

        if bytes < THRESHOLD {
            return format!("{} B", bytes);
        }

        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD as f64;
            unit_index += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Validation utilities
#[allow(dead_code)]
pub mod validate {
    use super::*;

    /// Validate cluster name format
    pub fn cluster_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(anyhow::anyhow!("Cluster name cannot be empty"));
        }

        if name.len() > 63 {
            return Err(anyhow::anyhow!("Cluster name cannot exceed 63 characters"));
        }

        // Check for valid DNS label format (simplified)
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(anyhow::anyhow!(
                "Cluster name can only contain alphanumeric characters and hyphens"
            ));
        }

        if name.starts_with('-') || name.ends_with('-') {
            return Err(anyhow::anyhow!(
                "Cluster name cannot start or end with a hyphen"
            ));
        }

        Ok(())
    }

    /// Validate image name format
    pub fn image_name(image: &str) -> Result<()> {
        if image.is_empty() {
            return Err(anyhow::anyhow!("Image name cannot be empty"));
        }

        // Basic validation - could be more comprehensive
        if !image.chars().all(|c| c.is_ascii_graphic() && c != ' ') {
            return Err(anyhow::anyhow!("Image name contains invalid characters"));
        }

        Ok(())
    }

    /// Validate file path exists and is readable
    pub fn file_path<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {}", path.display()));
        }

        if !fs::is_readable_file(path) {
            return Err(anyhow::anyhow!("File is not readable: {}", path.display()));
        }

        Ok(())
    }
}
