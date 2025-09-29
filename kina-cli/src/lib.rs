//! Kina CLI Library
//!
//! Core library for the Kina CLI - Kubernetes in Apple Container

pub mod cli;
pub mod config;
pub mod core;
pub mod errors;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use errors::{KinaError, KinaResult};
