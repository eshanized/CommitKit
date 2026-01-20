// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Error types for the ck application.
//!
//! This module defines all error types used throughout the application,
//! with proper error categorization and context propagation.

use std::path::PathBuf;
use thiserror::Error;

/// The main error type for ck operations.
#[derive(Error, Debug)]
pub enum CkError {
    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    // Git errors
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    // Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    // Plugin errors
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),

    // Security errors
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    // Commit errors
    #[error("Commit error: {0}")]
    Commit(#[from] CommitError),

    // Hook errors
    #[error("Hook error: {0}")]
    Hook(#[from] HookError),

    // IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // UI/Interactive errors
    #[error("UI error: {0}")]
    Ui(String),

    // User cancelled operation
    #[error("Operation cancelled by user")]
    Cancelled,

    // Generic error with context
    #[error("{context}: {message}")]
    WithContext { context: String, message: String },
}

impl From<dialoguer::Error> for CkError {
    fn from(err: dialoguer::Error) -> Self {
        CkError::Ui(err.to_string())
    }
}

/// Configuration-related errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Failed to parse configuration: {message}")]
    ParseError { message: String },

    #[error("Invalid configuration value for '{key}': {message}")]
    InvalidValue { key: String, message: String },

    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    #[error("Configuration merge error: {message}")]
    MergeError { message: String },
}

/// Git-related errors.
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository")]
    NotARepository,

    #[error("Failed to open repository: {message}")]
    OpenFailed { message: String },

    #[error("No staged changes found")]
    NoStagedChanges,

    #[error("Failed to get diff: {message}")]
    DiffFailed { message: String },

    #[error("Failed to create commit: {message}")]
    CommitFailed { message: String },

    #[error("Failed to get branch: {message}")]
    BranchFailed { message: String },

    #[error("Invalid commit reference: {reference}")]
    InvalidReference { reference: String },

    #[error("Git command failed: {command} - {message}")]
    CommandFailed { command: String, message: String },

    #[error("Detached HEAD state")]
    DetachedHead,
}

impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::OpenFailed {
            message: err.message().to_string(),
        }
    }
}

/// Validation-related errors.
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Subject line too long: {length} characters (max: {max})")]
    SubjectTooLong { length: usize, max: usize },

    #[error("Subject line too short: {length} characters (min: {min})")]
    SubjectTooShort { length: usize, min: usize },

    #[error("Invalid commit type: '{commit_type}'")]
    InvalidType { commit_type: String },

    #[error("Forbidden commit type on this branch: '{commit_type}'")]
    ForbiddenType { commit_type: String },

    #[error("Scope is required but not provided")]
    MissingScope,

    #[error("Invalid scope: '{scope}'")]
    InvalidScope { scope: String },

    #[error("Body is required but not provided")]
    MissingBody,

    #[error("Commit message format is invalid: {message}")]
    InvalidFormat { message: String },

    #[error("Rule violation: {rule} - {message}")]
    RuleViolation { rule: String, message: String },

    #[error("Multiple validation errors: {count} issues found")]
    MultipleErrors { count: usize },
}

/// Plugin-related errors.
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {name}")]
    NotFound { name: String },

    #[error("Failed to load plugin: {name} - {message}")]
    LoadFailed { name: String, message: String },

    #[error("Plugin version mismatch: {name} requires ck {required}, have {current}")]
    VersionMismatch {
        name: String,
        required: String,
        current: String,
    },

    #[error("Plugin permission denied: {name} requires '{permission}'")]
    PermissionDenied { name: String, permission: String },

    #[error("Plugin execution failed: {name} - {message}")]
    ExecutionFailed { name: String, message: String },

    #[error("Invalid plugin manifest: {message}")]
    InvalidManifest { message: String },
}

/// Security-related errors.
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Secret detected in diff: {pattern_name}")]
    SecretDetected { pattern_name: String },

    #[error("Multiple secrets detected: {count} patterns matched")]
    MultipleSecrets { count: usize },

    #[error("Commit signing required but not configured")]
    SigningRequired,

    #[error("Invalid signature on commit: {commit}")]
    InvalidSignature { commit: String },

    #[error("Security check failed: {message}")]
    CheckFailed { message: String },
}

/// Commit-related errors.
#[derive(Error, Debug)]
pub enum CommitError {
    #[error("Failed to parse commit message: {message}")]
    ParseFailed { message: String },

    #[error("Empty commit message")]
    EmptyMessage,

    #[error("Invalid conventional commit format")]
    InvalidConventionalFormat,

    #[error("Commit was aborted")]
    Aborted,
}

/// Hook-related errors.
#[derive(Error, Debug)]
pub enum HookError {
    #[error("Failed to install hook '{hook}': {message}")]
    InstallFailed { hook: String, message: String },

    #[error("Hook already exists: {hook}")]
    AlreadyExists { hook: String },

    #[error("Hook not found: {hook}")]
    NotFound { hook: String },

    #[error("Failed to remove hook '{hook}': {message}")]
    RemoveFailed { hook: String, message: String },

    #[error("Hook execution failed: {hook} - {message}")]
    ExecutionFailed { hook: String, message: String },
}

/// Result type alias for ck operations.
pub type Result<T> = std::result::Result<T, CkError>;

/// Extension trait for adding context to errors.
pub trait ResultExt<T> {
    /// Add context to an error.
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T, E: std::error::Error + 'static> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| CkError::WithContext {
            context: context.into(),
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::NotFound {
            path: PathBuf::from("/path/to/config"),
        };
        assert!(err.to_string().contains("/path/to/config"));
    }

    #[test]
    fn test_git_error_from_git2() {
        // Test that git2 errors convert properly
        let err = GitError::OpenFailed {
            message: "test error".to_string(),
        };
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::SubjectTooLong {
            length: 100,
            max: 72,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("72"));
    }

    #[test]
    fn test_ck_error_from_config_error() {
        let config_err = ConfigError::MissingRequired {
            key: "scope".to_string(),
        };
        let ck_err: CkError = config_err.into();
        assert!(ck_err.to_string().contains("scope"));
    }
}
