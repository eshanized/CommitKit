// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! CLI argument definitions using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CK - Intelligent Git Commit Assistant
///
/// A production-grade CLI tool for creating high-quality Git commits.
#[derive(Parser, Debug)]
#[command(name = "ck")]
#[command(author = "Eshan Roy")]
#[command(version)]
#[command(about = "Intelligent Git commit assistant", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The command to run (defaults to commit if not specified)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Stage modified and deleted files before committing
    #[arg(short, long, global = true)]
    pub all: bool,

    /// Enable strict CI mode (no prompts, fail on any issue)
    #[arg(long, global = true)]
    pub ci: bool,

    /// Show what would be done without actually doing it
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Disable all interactive prompts
    #[arg(long, global = true)]
    pub non_interactive: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Output format for machine-readable output
    #[arg(long, global = true, value_enum)]
    pub format: Option<OutputFormat>,

    /// Path to configuration file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

/// Output format for CI and scripting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Plain text output (default)
    Text,
    /// JSON output for machine parsing
    Json,
}

/// Available commands.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Interactive commit creation (default command)
    Commit(CommitArgs),

    /// Generate commit message from diff analysis
    Smart(SmartArgs),

    /// Validate commit messages
    Check(CheckArgs),

    /// Fix past commits interactively
    Fix(FixArgs),

    /// Manage git hooks
    Hooks(HooksArgs),

    /// Install ck as git-cz
    Install(InstallArgs),

    /// Print version information
    Version,

    /// Initialize ck configuration
    Init(InitArgs),
}

/// Arguments for the commit command.
#[derive(Parser, Debug, Default, Clone)]
pub struct CommitArgs {
    /// Pre-fill the commit type
    #[arg(short = 't', long)]
    pub r#type: Option<String>,

    /// Pre-fill the scope
    #[arg(short, long)]
    pub scope: Option<String>,

    /// Pre-fill the subject
    #[arg(short = 'm', long)]
    pub message: Option<String>,

    /// Pre-fill the body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Mark as breaking change
    #[arg(long)]
    pub breaking: bool,

    /// Add issue reference
    #[arg(short, long)]
    pub issue: Option<String>,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Sign the commit with GPG
    #[arg(long)]
    pub sign: bool,

    /// Amend the previous commit
    #[arg(long)]
    pub amend: bool,
}

/// Arguments for the smart command.
#[derive(Parser, Debug, Default, Clone)]
pub struct SmartArgs {
    /// Allow editing the generated message
    #[arg(short, long)]
    pub edit: bool,

    /// Maximum number of bullet points in body
    #[arg(long, default_value = "5")]
    pub max_bullets: usize,

    /// Include file names in the body
    #[arg(long)]
    pub include_files: bool,
}

/// Arguments for the check command.
#[derive(Parser, Debug, Clone)]
pub struct CheckArgs {
    /// Commit or range to check (default: staged message or HEAD)
    #[arg(default_value = "HEAD")]
    pub target: String,

    /// Check all commits in a range
    #[arg(long)]
    pub range: bool,

    /// Strict mode: treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

/// Arguments for the fix command.
#[derive(Parser, Debug, Clone)]
pub struct FixArgs {
    /// Commit or range to fix
    #[arg(default_value = "HEAD")]
    pub target: String,

    /// Number of commits to fix
    #[arg(short = 'n', long)]
    pub count: Option<usize>,

    /// Auto-fix without prompts (where possible)
    #[arg(long)]
    pub auto: bool,
}

/// Arguments for the hooks command.
#[derive(Parser, Debug, Clone)]
pub struct HooksArgs {
    /// Hook action to perform
    #[command(subcommand)]
    pub action: HooksAction,
}

/// Hook actions.
#[derive(Subcommand, Debug, Clone)]
pub enum HooksAction {
    /// Install git hooks
    Install {
        /// Specific hook to install
        #[arg(value_name = "HOOK")]
        hook: Option<String>,

        /// Force overwrite existing hooks
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall git hooks
    Uninstall {
        /// Specific hook to uninstall
        #[arg(value_name = "HOOK")]
        hook: Option<String>,
    },

    /// Show hook status
    Status,

    /// Run a hook manually (for testing)
    Run {
        /// Hook to run
        hook: String,

        /// Arguments to pass to the hook
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

/// Arguments for the install command.
#[derive(Parser, Debug, Clone)]
pub struct InstallArgs {
    /// Install as git-cz alias
    #[arg(long)]
    pub as_git_cz: bool,

    /// Installation directory
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

/// Arguments for the init command.
#[derive(Parser, Debug, Clone)]
pub struct InitArgs {
    /// Overwrite existing configuration
    #[arg(short, long)]
    pub force: bool,

    /// Configuration preset
    #[arg(long)]
    pub preset: Option<ConfigPreset>,
}

/// Configuration presets for init.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ConfigPreset {
    /// Minimal configuration
    Minimal,
    /// Standard configuration (recommended)
    Standard,
    /// Strict configuration for CI
    Strict,
    /// Full configuration with all options
    Full,
}

impl Cli {
    /// Check if running in CI mode (either explicit --ci or non-interactive).
    pub fn is_ci_mode(&self) -> bool {
        self.ci || self.non_interactive
    }

    /// Check if any output should be produced.
    pub fn should_output(&self) -> bool {
        !self.dry_run || self.debug
    }

    /// Get the effective command, defaulting to Commit if none specified.
    pub fn effective_command(&self) -> Commands {
        self.command
            .clone()
            .unwrap_or(Commands::Commit(CommitArgs::default()))
    }
}

impl Default for CheckArgs {
    fn default() -> Self {
        Self {
            target: "HEAD".to_string(),
            range: false,
            strict: false,
        }
    }
}

impl Default for FixArgs {
    fn default() -> Self {
        Self {
            target: "HEAD".to_string(),
            count: None,
            auto: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_debug() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_commit() {
        let args = Cli::parse_from(["ck", "commit", "-t", "feat", "-s", "core"]);
        assert!(matches!(args.command, Some(Commands::Commit(_))));
    }

    #[test]
    fn test_parse_smart() {
        let args = Cli::parse_from(["ck", "smart", "--edit"]);
        assert!(matches!(args.command, Some(Commands::Smart(_))));
    }

    #[test]
    fn test_parse_check() {
        let args = Cli::parse_from(["ck", "check", "HEAD~5..HEAD", "--strict"]);
        if let Some(Commands::Check(check_args)) = args.command {
            assert_eq!(check_args.target, "HEAD~5..HEAD");
            assert!(check_args.strict);
        } else {
            panic!("Expected Check command");
        }
    }

    #[test]
    fn test_parse_hooks() {
        let args = Cli::parse_from(["ck", "hooks", "install", "--force"]);
        assert!(matches!(args.command, Some(Commands::Hooks(_))));
    }

    #[test]
    fn test_global_flags() {
        let args = Cli::parse_from(["ck", "--ci", "--dry-run", "commit"]);
        assert!(args.ci);
        assert!(args.dry_run);
        assert!(args.is_ci_mode());
    }

    #[test]
    fn test_default_command() {
        let args = Cli::parse_from(["ck"]);
        assert!(args.command.is_none());
        assert!(matches!(args.effective_command(), Commands::Commit(_)));
    }
}
