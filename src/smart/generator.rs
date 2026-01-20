// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Smart commit message generator.

use crate::analysis::RepositoryContext;
use crate::commit::CommitMessage;
use crate::config::{CkConfig, CommitType};
use crate::error::Result;
use crate::git;

use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Editor};

use super::semantic::SemanticAnalyzer;

/// Generated commit message.
#[derive(Debug, Clone)]
pub struct GeneratedMessage {
    /// The commit type.
    pub commit_type: CommitType,
    /// The scope.
    pub scope: Option<String>,
    /// The subject line.
    pub subject: String,
    /// Body with bullet points.
    pub body: Option<String>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

impl GeneratedMessage {
    /// Format as a complete commit message.
    pub fn format(&self) -> String {
        let message = CommitMessage {
            commit_type: self.commit_type,
            scope: self.scope.clone(),
            subject: self.subject.clone(),
            body: self.body.clone(),
            footer: None,
            is_breaking: false,
        };
        message.format()
    }

    /// Get the header line.
    pub fn header(&self) -> String {
        let mut result = String::new();
        result.push_str(self.commit_type.as_str());

        if let Some(ref scope) = self.scope {
            result.push('(');
            result.push_str(scope);
            result.push(')');
        }

        result.push_str(": ");
        result.push_str(&self.subject);

        result
    }
}

/// Smart commit generator.
pub struct SmartCommit {
    config: CkConfig,
}

impl SmartCommit {
    /// Create a new smart commit generator.
    pub fn new(config: CkConfig) -> Self {
        Self { config }
    }

    /// Generate a commit message from the staged changes.
    pub fn generate(&self, max_bullets: usize, include_files: bool) -> Result<GeneratedMessage> {
        // Get repository context
        let context = RepositoryContext::from_current_repo_with_config(&self.config)?;

        if !context.has_staged_changes() {
            return Err(crate::error::CkError::Git(
                crate::error::GitError::NoStagedChanges,
            ));
        }

        // Perform semantic analysis
        let analyzer = SemanticAnalyzer::from_context(&context);

        // Get suggested type and scope
        let commit_type = context.suggested_type.unwrap_or(analyzer.suggested_type());
        let scope = context.suggested_scope.clone();

        // Generate subject line
        let subject = self.generate_subject(&analyzer, &context);

        // Generate body
        let body = self.generate_body(&analyzer, max_bullets, include_files);

        // Calculate confidence
        let confidence = self.calculate_confidence(&analyzer, &context);

        Ok(GeneratedMessage {
            commit_type,
            scope,
            subject,
            body,
            confidence,
        })
    }

    /// Generate the subject line.
    fn generate_subject(
        &self,
        analyzer: &SemanticAnalyzer,
        _context: &RepositoryContext,
    ) -> String {
        let intent = analyzer.primary_intent();
        let actions = analyzer.extract_actions();

        // Try to create a meaningful subject
        if !actions.is_empty() {
            // Use the first action as the base
            let first_action = &actions[0];

            if actions.len() == 1 {
                // Single action: use it directly
                format!("{} {}", first_action.verb.imperative(), first_action.target)
            } else {
                // Multiple actions: summarize
                let verb = first_action.verb.imperative();
                let target = if actions.len() == 2 {
                    format!("{} and {}", first_action.target, actions[1].target)
                } else {
                    format!("{} and {} more", first_action.target, actions.len() - 1)
                };
                format!("{} {}", verb, target)
            }
        } else {
            // Fallback based on intent
            let areas = analyzer.affected_areas();
            let area = areas.first().map(|s| s.as_str()).unwrap_or("files");

            match intent {
                super::semantic::ChangeIntent::Feature => format!("add new {}", area),
                super::semantic::ChangeIntent::BugFix => "fix issue".to_string(),
                super::semantic::ChangeIntent::Documentation => "update documentation".to_string(),
                super::semantic::ChangeIntent::Testing => "update tests".to_string(),
                super::semantic::ChangeIntent::Refactoring => "refactor code".to_string(),
                super::semantic::ChangeIntent::Configuration => "update configuration".to_string(),
                super::semantic::ChangeIntent::Update => "update files".to_string(),
            }
        }
    }

    /// Generate the body with bullet points.
    fn generate_body(
        &self,
        analyzer: &SemanticAnalyzer,
        max_bullets: usize,
        include_files: bool,
    ) -> Option<String> {
        let actions = analyzer.extract_actions();

        if actions.is_empty() && !include_files {
            return None;
        }

        let mut lines = Vec::new();

        // Add action bullet points
        for action in actions.iter().take(max_bullets) {
            lines.push(action.as_bullet());
        }

        // Add file list if requested
        if include_files {
            let areas = analyzer.affected_areas();
            if !areas.is_empty() {
                if !lines.is_empty() {
                    lines.push(String::new());
                }
                lines.push(format!("Affects: {}", areas.join(", ")));
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Calculate confidence score.
    fn calculate_confidence(
        &self,
        analyzer: &SemanticAnalyzer,
        context: &RepositoryContext,
    ) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Higher confidence for clear intent
        match analyzer.primary_intent() {
            super::semantic::ChangeIntent::Documentation
            | super::semantic::ChangeIntent::Testing => {
                score += 0.3; // Very clear intent
            }
            super::semantic::ChangeIntent::Configuration => {
                score += 0.2;
            }
            super::semantic::ChangeIntent::Feature | super::semantic::ChangeIntent::BugFix => {
                score += 0.1;
            }
            _ => {}
        }

        // Higher confidence if scope is detected
        if context.suggested_scope.is_some() {
            score += 0.1;
        }

        // Lower confidence for large diffs
        if context.diff_stats.total_lines_changed() > 100 {
            score -= 0.2;
        }

        // Clamp to [0.0, 1.0]
        f64::max(f64::min(score, 1.0), 0.0)
    }

    /// Run interactive smart commit flow.
    pub fn run_interactive(
        &self,
        message: GeneratedMessage,
        dry_run: bool,
        allow_edit: bool,
    ) -> Result<()> {
        let term = Term::stderr();
        let theme = ColorfulTheme::default();

        // Show generated message
        term.write_line(&format!("\n{}", style("Generated commit message:").bold()))?;
        term.write_line("")?;

        // Show header with styling
        term.write_line(&format!("  {}", style(&message.header()).green()))?;

        if let Some(ref body) = message.body {
            term.write_line("")?;
            for line in body.lines() {
                term.write_line(&format!("  {}", style(line).dim()))?;
            }
        }

        term.write_line("")?;
        term.write_line(&format!(
            "  {} Confidence: {:.0}%",
            style("ℹ").blue(),
            message.confidence * 100.0
        ))?;

        // Allow editing
        let final_message = if allow_edit {
            let wants_edit = Confirm::with_theme(&theme)
                .with_prompt("Edit message?")
                .default(false)
                .interact()?;

            if wants_edit {
                let edited = Editor::new()
                    .edit(&message.format())
                    .map_err(|e| crate::error::CkError::Ui(e.to_string()))?;

                edited.unwrap_or_else(|| message.format())
            } else {
                message.format()
            }
        } else {
            message.format()
        };

        // Confirm
        let confirmed = Confirm::with_theme(&theme)
            .with_prompt("Commit?")
            .default(true)
            .interact()?;

        if !confirmed {
            return Err(crate::error::CkError::Cancelled);
        }

        // Commit
        if dry_run {
            term.write_line(&format!(
                "\n{} Would create commit:\n{}",
                style("[dry-run]").yellow(),
                final_message
            ))?;
        } else {
            let sha = git::create_commit(&final_message, false)?;
            let short_sha = &sha[..7.min(sha.len())];
            let first_line = final_message.lines().next().unwrap_or("");
            term.write_line(&format!(
                "\n{} {} {}",
                style("✓").green().bold(),
                style(format!("[{}]", short_sha)).cyan(),
                first_line
            ))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_message_format() {
        let msg = GeneratedMessage {
            commit_type: CommitType::Feat,
            scope: Some("core".to_string()),
            subject: "add new feature".to_string(),
            body: Some("- add feature\n- update tests".to_string()),
            confidence: 0.8,
        };

        let formatted = msg.format();
        assert!(formatted.starts_with("feat(core): add new feature"));
        assert!(formatted.contains("- add feature"));
    }

    #[test]
    fn test_generated_message_header() {
        let msg = GeneratedMessage {
            commit_type: CommitType::Fix,
            scope: None,
            subject: "fix bug".to_string(),
            body: None,
            confidence: 0.5,
        };

        assert_eq!(msg.header(), "fix: fix bug");
    }
}
