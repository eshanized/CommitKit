// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Interactive commit builder.

use crate::analysis::RepositoryContext;
use crate::config::{CkConfig, CommitType};
use crate::error::{CkError, CommitError, Result};
use crate::git;
use crate::rules::RuleEngine;

use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Editor, Input, Select};

use super::message::CommitMessage;
use super::preview::CommitPreview;

/// Interactive commit builder.
pub struct CommitBuilder {
    config: CkConfig,
    commit_type: Option<CommitType>,
    scope: Option<String>,
    subject: Option<String>,
    body: Option<String>,
    is_breaking: bool,
    context: Option<RepositoryContext>,
}

impl CommitBuilder {
    /// Create a new commit builder.
    pub fn new(config: CkConfig) -> Self {
        Self {
            config,
            commit_type: None,
            scope: None,
            subject: None,
            body: None,
            is_breaking: false,
            context: None,
        }
    }

    /// Set the commit type from a string.
    pub fn with_type_str(mut self, type_str: &str) -> Result<Self> {
        self.commit_type = type_str.parse().ok();
        if self.commit_type.is_none() {
            return Err(CkError::Commit(CommitError::ParseFailed {
                message: format!("Invalid commit type: {}", type_str),
            }));
        }
        Ok(self)
    }

    /// Set the commit type.
    pub fn with_type(mut self, commit_type: CommitType) -> Self {
        self.commit_type = Some(commit_type);
        self
    }

    /// Set the scope.
    pub fn with_scope(mut self, scope: &str) -> Self {
        if !scope.is_empty() {
            self.scope = Some(scope.to_string());
        }
        self
    }

    /// Set the subject.
    pub fn with_subject(mut self, subject: &str) -> Self {
        if !subject.is_empty() {
            self.subject = Some(subject.to_string());
        }
        self
    }

    /// Set the body.
    pub fn with_body(mut self, body: &str) -> Self {
        if !body.is_empty() {
            self.body = Some(body.to_string());
        }
        self
    }

    /// Set breaking change flag.
    pub fn with_breaking(mut self, breaking: bool) -> Self {
        self.is_breaking = breaking;
        self
    }

    /// Run the interactive commit flow.
    pub fn run_interactive(
        mut self,
        dry_run: bool,
        skip_confirm: bool,
        sign: bool,
        amend: bool,
    ) -> Result<()> {
        let term = Term::stderr();
        let theme = ColorfulTheme::default();

        // Load repository context
        self.context = Some(RepositoryContext::from_current_repo_with_config(
            &self.config,
        )?);
        let context = self.context.as_ref().unwrap();

        // Check for staged changes
        if !context.has_staged_changes() {
            return Err(CkError::Git(crate::error::GitError::NoStagedChanges));
        }

        // Show context summary
        term.write_line(&format!(
            "\n{} {}\n",
            style("Analyzing changes...").dim(),
            style(&context.summary()).cyan()
        ))?;

        // Show warnings if any
        for warning in context.warnings.iter() {
            let icon = match warning.level {
                crate::analysis::WarningLevel::Error => style("✗").red(),
                crate::analysis::WarningLevel::Warning => style("⚠").yellow(),
                crate::analysis::WarningLevel::Info => style("ℹ").blue(),
            };
            term.write_line(&format!("  {} {}", icon, warning.message))?;
        }

        // Prompt for commit type
        if self.commit_type.is_none() {
            self.commit_type = Some(self.prompt_type(&theme, context)?);
        }

        // Prompt for scope
        if self.scope.is_none() {
            self.scope = self.prompt_scope(&theme, context)?;
        }

        // Prompt for subject
        if self.subject.is_none() {
            self.subject = Some(self.prompt_subject(&theme)?);
        }

        // Prompt for body
        if self.body.is_none() && self.config.rules.require_body {
            self.body = self.prompt_body(&theme)?;
        } else if self.body.is_none() {
            // Optional body
            let wants_body = Confirm::with_theme(&theme)
                .with_prompt("Add a body?")
                .default(false)
                .interact()?;

            if wants_body {
                self.body = self.prompt_body(&theme)?;
            }
        }

        // Prompt for breaking change
        if !self.is_breaking {
            self.is_breaking = Confirm::with_theme(&theme)
                .with_prompt("Is this a breaking change?")
                .default(false)
                .interact()?;
        }

        // Build the message
        let message = self.build_message()?;

        // Validate
        let engine = RuleEngine::new(self.config.clone());
        let validation = engine.validate(&message);

        // Show preview
        let preview = CommitPreview::new(&message);
        term.write_line("\n")?;
        preview.print();

        // Show validation results
        if !validation.is_valid() {
            term.write_line(&format!("\n{}", style("Validation errors:").red().bold()))?;
            for error in &validation.errors {
                term.write_line(&format!("  {}", error.format()))?;
            }
            return Err(CkError::Validation(
                crate::error::ValidationError::MultipleErrors {
                    count: validation.errors.len(),
                },
            ));
        }

        for warning in &validation.warnings {
            term.write_line(&format!("  {}", warning.format()))?;
        }

        // Confirm
        if !skip_confirm {
            let confirmed = Confirm::with_theme(&theme)
                .with_prompt("Commit?")
                .default(true)
                .interact()?;

            if !confirmed {
                return Err(CkError::Cancelled);
            }
        }

        // Commit
        if dry_run {
            term.write_line(&format!(
                "\n{} Would create commit:\n{}",
                style("[dry-run]").yellow(),
                message.format()
            ))?;
        } else {
            let sha = if amend {
                git::commands::amend_commit(&message.format(), sign)?
            } else {
                git::create_commit(&message.format(), sign)?
            };

            let short_sha = &sha[..7.min(sha.len())];
            term.write_line(&format!(
                "\n{} {} {}",
                style("✓").green().bold(),
                style(format!("[{}]", short_sha)).cyan(),
                message.header()
            ))?;
        }

        Ok(())
    }

    /// Commit without interactive prompts.
    pub fn commit_non_interactive(self, dry_run: bool, sign: bool) -> Result<()> {
        let message = self.build_message()?;

        // Validate
        let engine = RuleEngine::new(self.config);
        let validation = engine.validate(&message);

        if !validation.is_valid() {
            for error in &validation.errors {
                eprintln!("{}", error.format());
            }
            return Err(CkError::Validation(
                crate::error::ValidationError::MultipleErrors {
                    count: validation.errors.len(),
                },
            ));
        }

        if dry_run {
            println!("{}", message.format());
        } else {
            let sha = git::create_commit(&message.format(), sign)?;
            let short_sha = &sha[..7.min(sha.len())];
            println!("[{}] {}", short_sha, message.header());
        }

        Ok(())
    }

    /// Build the commit message from collected data.
    fn build_message(&self) -> Result<CommitMessage> {
        let commit_type = self.commit_type.ok_or_else(|| {
            CkError::Commit(CommitError::ParseFailed {
                message: "Commit type is required".to_string(),
            })
        })?;

        let subject = self.subject.clone().ok_or_else(|| {
            CkError::Commit(CommitError::ParseFailed {
                message: "Subject is required".to_string(),
            })
        })?;

        let mut message = CommitMessage::new(commit_type, subject);

        if let Some(ref scope) = self.scope {
            message = message.with_scope(scope);
        }

        if let Some(ref body) = self.body {
            message = message.with_body(body);
        }

        message = message.with_breaking(self.is_breaking);

        Ok(message)
    }

    /// Prompt for commit type.
    fn prompt_type(
        &self,
        theme: &ColorfulTheme,
        context: &RepositoryContext,
    ) -> Result<CommitType> {
        let types: Vec<CommitType> = self
            .config
            .rules
            .allowed_types
            .iter()
            .filter_map(|t| t.parse().ok())
            .collect();

        let items: Vec<String> = types
            .iter()
            .map(|t| format!("{:10} {}", t.as_str(), style(t.description()).dim()))
            .collect();

        // Find default index based on suggestion
        let default_idx = context
            .suggested_type
            .as_ref()
            .and_then(|st| types.iter().position(|t| t == st))
            .unwrap_or(0);

        let selection = Select::with_theme(theme)
            .with_prompt("Select commit type")
            .items(&items)
            .default(default_idx)
            .interact()?;

        Ok(types[selection])
    }

    /// Prompt for scope.
    fn prompt_scope(
        &self,
        theme: &ColorfulTheme,
        context: &RepositoryContext,
    ) -> Result<Option<String>> {
        let default = context.suggested_scope.clone().unwrap_or_default();

        let allowed = &self.config.rules.scope.allowed;

        if !allowed.is_empty() {
            // Use select for predefined scopes
            let mut items: Vec<String> = allowed.clone();
            items.insert(0, "(none)".to_string());

            let default_idx = if default.is_empty() {
                0
            } else {
                items.iter().position(|s| s == &default).unwrap_or(0)
            };

            let selection = Select::with_theme(theme)
                .with_prompt("Select scope")
                .items(&items)
                .default(default_idx)
                .interact()?;

            if selection == 0 {
                Ok(None)
            } else {
                Ok(Some(items[selection].clone()))
            }
        } else {
            // Free-form input
            let prompt = if self.config.rules.require_scope {
                "Scope (required)"
            } else {
                "Scope (optional)"
            };

            let scope: String = Input::with_theme(theme)
                .with_prompt(prompt)
                .default(default)
                .allow_empty(!self.config.rules.require_scope)
                .interact_text()?;

            if scope.is_empty() {
                Ok(None)
            } else {
                Ok(Some(scope))
            }
        }
    }

    /// Prompt for subject.
    fn prompt_subject(&self, theme: &ColorfulTheme) -> Result<String> {
        let max_len = self.config.rules.max_subject_length;

        let subject: String = Input::with_theme(theme)
            .with_prompt(format!("Subject (max {} chars)", max_len))
            .validate_with(|input: &String| {
                if input.is_empty() {
                    Err("Subject is required")
                } else if input.len() > max_len {
                    Err("Subject is too long")
                } else if input.len() < self.config.rules.min_subject_length {
                    Err("Subject is too short")
                } else {
                    Ok(())
                }
            })
            .interact_text()?;

        Ok(subject)
    }

    /// Prompt for body.
    fn prompt_body(&self, _theme: &ColorfulTheme) -> Result<Option<String>> {
        let body = Editor::new()
            .edit("Enter commit body (save and close to continue)")
            .map_err(|e| CkError::Ui(e.to_string()))?;

        Ok(body.filter(|s| !s.trim().is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_builder_new() {
        let config = CkConfig::default();
        let builder = CommitBuilder::new(config);
        assert!(builder.commit_type.is_none());
    }

    #[test]
    fn test_commit_builder_with_type() {
        let config = CkConfig::default();
        let builder = CommitBuilder::new(config).with_type(CommitType::Feat);
        assert_eq!(builder.commit_type, Some(CommitType::Feat));
    }

    #[test]
    fn test_commit_builder_build() {
        let config = CkConfig::default();
        let builder = CommitBuilder::new(config)
            .with_type(CommitType::Feat)
            .with_scope("core")
            .with_subject("add feature");

        let message = builder.build_message().unwrap();
        assert_eq!(message.commit_type, CommitType::Feat);
        assert_eq!(message.scope, Some("core".to_string()));
        assert_eq!(message.subject, "add feature");
    }
}
