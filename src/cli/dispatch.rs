// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Command dispatch and execution.

use crate::config::CkConfig;
use crate::error::Result;

use super::args::{Cli, Commands, HooksAction};

/// Run the CLI with the given arguments.
pub fn run(cli: Cli) -> Result<()> {
    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        CkConfig::load_from(config_path)?
    } else {
        CkConfig::load()?
    };

    // Dispatch to the appropriate command handler
    match cli.effective_command() {
        Commands::Commit(args) => run_commit(&cli, &config, args),
        Commands::Smart(args) => run_smart(&cli, &config, args),
        Commands::Check(args) => run_check(&cli, &config, args),
        Commands::Fix(args) => run_fix(&cli, &config, args),
        Commands::Hooks(args) => run_hooks(&cli, &config, args),
        Commands::Install(args) => run_install(&cli, args),
        Commands::Version => run_version(),
        Commands::Init(args) => run_init(&cli, args),
    }
}

/// Run the commit command.
fn run_commit(cli: &Cli, config: &CkConfig, args: super::args::CommitArgs) -> Result<()> {
    use crate::commit::CommitBuilder;

    tracing::debug!("Running commit command with args: {:?}", args);

    let mut builder = CommitBuilder::new(config.clone());

    // Pre-fill values from arguments
    if let Some(ref t) = args.r#type {
        builder = builder.with_type_str(t)?;
    }
    if let Some(ref scope) = args.scope {
        builder = builder.with_scope(scope);
    }
    if let Some(ref message) = args.message {
        builder = builder.with_subject(message);
    }
    if let Some(ref body) = args.body {
        builder = builder.with_body(body);
    }
    if args.breaking {
        builder = builder.with_breaking(true);
    }

    // Stage all if requested
    if cli.all {
        crate::git::stage_all()?;
    }

    // Run the interactive builder or non-interactive commit
    if cli.is_ci_mode() || cli.non_interactive {
        builder.commit_non_interactive(cli.dry_run, args.sign)
    } else {
        builder.run_interactive(cli.dry_run, args.yes, args.sign, args.amend)
    }
}

/// Run the smart command.
fn run_smart(cli: &Cli, config: &CkConfig, args: super::args::SmartArgs) -> Result<()> {
    use crate::smart::SmartCommit;

    tracing::debug!("Running smart command with args: {:?}", args);

    // Stage all if requested
    if cli.all {
        crate::git::stage_all()?;
    }

    let smart = SmartCommit::new(config.clone());
    let message = smart.generate(args.max_bullets, args.include_files)?;

    if cli.is_ci_mode() || cli.non_interactive {
        if cli.dry_run {
            println!("{}", message.format());
            Ok(())
        } else {
            crate::git::create_commit(&message.format(), false)?;
            Ok(())
        }
    } else {
        smart.run_interactive(message, cli.dry_run, args.edit)
    }
}

/// Run the check command.
fn run_check(cli: &Cli, config: &CkConfig, args: super::args::CheckArgs) -> Result<()> {
    use crate::rules::RuleEngine;

    tracing::debug!("Running check command with args: {:?}", args);

    let engine = RuleEngine::new(config.clone());
    let strict = args.strict || (cli.ci && config.rules.ci.strict);

    let results = if args.range || args.target.contains("..") {
        engine.check_range(&args.target)?
    } else {
        vec![engine.check_commit(&args.target)?]
    };

    // Output results
    let mut has_errors = false;
    let mut has_warnings = false;

    for result in &results {
        if !result.errors.is_empty() {
            has_errors = true;
        }
        if !result.warnings.is_empty() {
            has_warnings = true;
        }
        result.print(cli.format);
    }

    // Determine exit status
    if has_errors || (strict && has_warnings) {
        Err(crate::error::CkError::Validation(
            crate::error::ValidationError::MultipleErrors {
                count: results.iter().map(|r| r.errors.len()).sum(),
            },
        ))
    } else {
        Ok(())
    }
}

/// Run the fix command.
fn run_fix(cli: &Cli, _config: &CkConfig, args: super::args::FixArgs) -> Result<()> {
    use crate::commit::fix::CommitFixer;

    tracing::debug!("Running fix command with args: {:?}", args);

    if cli.is_ci_mode() && !args.auto {
        return Err(crate::error::CkError::WithContext {
            context: "fix".to_string(),
            message: "Cannot run fix in CI mode without --auto".to_string(),
        });
    }

    let fixer = CommitFixer::new();
    fixer.fix(&args.target, args.count, cli.dry_run, args.auto)
}

/// Run the hooks command.
fn run_hooks(_cli: &Cli, _config: &CkConfig, args: super::args::HooksArgs) -> Result<()> {
    use crate::hooks::HookManager;

    tracing::debug!("Running hooks command");

    let manager = HookManager::new()?;

    match args.action {
        HooksAction::Install { hook, force } => {
            if let Some(hook_name) = hook {
                manager.install_hook(&hook_name, force)?;
                println!("✓ Installed {} hook", hook_name);
            } else {
                manager.install_all(force)?;
                println!("✓ Installed all hooks");
            }
        }
        HooksAction::Uninstall { hook } => {
            if let Some(hook_name) = hook {
                manager.uninstall_hook(&hook_name)?;
                println!("✓ Uninstalled {} hook", hook_name);
            } else {
                manager.uninstall_all()?;
                println!("✓ Uninstalled all hooks");
            }
        }
        HooksAction::Status => {
            let status = manager.status()?;
            for (hook, installed) in status {
                let icon = if installed { "✓" } else { "✗" };
                println!("{} {}", icon, hook);
            }
        }
        HooksAction::Run { hook, args } => {
            manager.run_hook(&hook, &args)?;
        }
    }

    Ok(())
}

/// Run the install command.
fn run_install(_cli: &Cli, args: super::args::InstallArgs) -> Result<()> {
    tracing::debug!("Running install command with args: {:?}", args);

    if args.as_git_cz {
        // Create a git alias for ck as git-cz
        let output = std::process::Command::new("git")
            .args(["config", "--global", "alias.cz", "!ck"])
            .output()
            .map_err(|e| crate::error::CkError::WithContext {
                context: "install".to_string(),
                message: format!("Failed to set git alias: {}", e),
            })?;

        if output.status.success() {
            println!("✓ Installed ck as git-cz");
            println!("  You can now use: git cz");
        } else {
            return Err(crate::error::CkError::WithContext {
                context: "install".to_string(),
                message: "Failed to set git alias".to_string(),
            });
        }
    }

    if let Some(dir) = args.dir {
        // Install the binary to the specified directory
        let current_exe =
            std::env::current_exe().map_err(|e| crate::error::CkError::WithContext {
                context: "install".to_string(),
                message: format!("Failed to get current executable: {}", e),
            })?;

        let target = dir.join("ck");
        std::fs::copy(&current_exe, &target).map_err(|e| crate::error::CkError::WithContext {
            context: "install".to_string(),
            message: format!("Failed to copy binary: {}", e),
        })?;

        println!("✓ Installed ck to {}", target.display());
    }

    Ok(())
}

/// Run the version command.
fn run_version() -> Result<()> {
    println!("ck {}", crate::version::version_string());

    if let Some(sha) = crate::version::GIT_SHA {
        println!("git commit: {}", sha);
    }
    if let Some(date) = crate::version::GIT_COMMIT_DATE {
        println!("commit date: {}", date);
    }

    Ok(())
}

/// Run the init command.
fn run_init(_cli: &Cli, args: super::args::InitArgs) -> Result<()> {
    use crate::config::default::example_config;

    tracing::debug!("Running init command with args: {:?}", args);

    let config_path = std::path::Path::new("ck.toml");

    if config_path.exists() && !args.force {
        return Err(crate::error::CkError::Config(
            crate::error::ConfigError::MergeError {
                message: "Configuration file already exists. Use --force to overwrite.".to_string(),
            },
        ));
    }

    let config_content = match args.preset {
        Some(super::args::ConfigPreset::Minimal) => {
            r#"# CK Configuration (Minimal)
[rules]
max_subject_length = 72
"#
        }
        Some(super::args::ConfigPreset::Strict) => {
            r#"# CK Configuration (Strict)
[rules]
max_subject_length = 72
min_subject_length = 10
require_scope = true
require_body = true
forbidden_types = ["wip", "fixup", "squash"]

[rules.ci]
strict = true
fail_on_warning = true

[security]
enabled = true
block_on_secret = true
"#
        }
        Some(super::args::ConfigPreset::Full) | None => example_config(),
        Some(super::args::ConfigPreset::Standard) => {
            r#"# CK Configuration (Standard)
[rules]
max_subject_length = 72
min_subject_length = 10
require_scope = false
require_body = false
allowed_types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "revert"]
forbidden_types = ["wip"]

[security]
enabled = true
block_on_secret = true

[hooks]
enabled = true

[hooks.commit_msg]
enabled = true
"#
        }
    };

    std::fs::write(config_path, config_content).map_err(|e| {
        crate::error::CkError::WithContext {
            context: "init".to_string(),
            message: format!("Failed to write configuration: {}", e),
        }
    })?;

    println!("✓ Created ck.toml");

    Ok(())
}
