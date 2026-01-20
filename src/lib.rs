// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! CK - Intelligent Git Commit Assistant
//!
//! A production-grade CLI tool for creating high-quality Git commits.
//!
//! # Features
//!
//! - **Interactive Commit Builder**: Guided commit creation with live preview
//! - **Smart Commit Generation**: Automatic commit message generation from diffs
//! - **Rule Engine**: Configurable validation rules for commit messages
//! - **Monorepo Support**: Package-aware scoping and validation
//! - **Security Scanning**: Detect secrets before they're committed
//! - **Git Hooks**: Native hook management without shell scripts
//! - **Plugin System**: Extend functionality via WASM plugins
//!
//! # Example
//!
//! ```no_run
//! use ck::config::CkConfig;
//! use ck::analysis::RepositoryContext;
//!
//! // Load configuration
//! let config = CkConfig::load().unwrap();
//!
//! // Analyze repository state
//! let context = RepositoryContext::from_current_repo().unwrap();
//!
//! // Get suggestions
//! if let Some(suggested_type) = context.suggested_type {
//!     println!("Suggested type: {:?}", suggested_type);
//! }
//! ```

// Module declarations
pub mod analysis;
pub mod cli;
pub mod commit;
pub mod config;
pub mod error;
pub mod git;
pub mod hooks;
pub mod monorepo;
pub mod plugins;
pub mod rules;
pub mod security;
pub mod smart;

// Re-exports for convenience
pub use config::CkConfig;
pub use error::{CkError, Result};

/// Version information embedded at compile time.
pub mod version {
    /// The current version of ck.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// The git SHA at compile time (if available).
    pub const GIT_SHA: Option<&str> = option_env!("VERGEN_GIT_SHA");

    /// The git commit date at compile time (if available).
    pub const GIT_COMMIT_DATE: Option<&str> = option_env!("VERGEN_GIT_COMMIT_DATE");

    /// Get a formatted version string.
    pub fn version_string() -> String {
        match (GIT_SHA, GIT_COMMIT_DATE) {
            (Some(sha), Some(date)) => {
                format!("{} ({} {})", VERSION, &sha[..7.min(sha.len())], date)
            }
            (Some(sha), None) => {
                format!("{} ({})", VERSION, &sha[..7.min(sha.len())])
            }
            _ => VERSION.to_string(),
        }
    }
}
