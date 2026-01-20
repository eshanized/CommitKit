// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Rule engine for commit validation.

use crate::commit::CommitMessage;
use crate::config::CkConfig;
use crate::error::Result;
use crate::git;

use super::builtin::{apply_builtin_rules, Rule};
use super::validator::ValidationResult;

/// Rule engine for validating commit messages.
#[derive(Debug, Clone)]
pub struct RuleEngine {
    config: CkConfig,
    custom_rules: Vec<Box<dyn Rule>>,
}

impl RuleEngine {
    /// Create a new rule engine with the given configuration.
    pub fn new(config: CkConfig) -> Self {
        Self {
            config,
            custom_rules: Vec::new(),
        }
    }

    /// Add a custom rule to the engine.
    pub fn add_rule(&mut self, rule: Box<dyn Rule>) {
        self.custom_rules.push(rule);
    }

    /// Validate a commit message.
    pub fn validate(&self, message: &CommitMessage) -> ValidationResult {
        let mut result = ValidationResult::new(message.format());

        // Apply built-in rules
        let builtin_issues = apply_builtin_rules(message, &self.config);
        for issue in builtin_issues {
            if issue.is_error {
                result.errors.push(issue);
            } else {
                result.warnings.push(issue);
            }
        }

        // Apply custom rules
        for rule in &self.custom_rules {
            if let Some(issue) = rule.check(message, &self.config) {
                if issue.is_error {
                    result.errors.push(issue);
                } else {
                    result.warnings.push(issue);
                }
            }
        }

        result
    }

    /// Validate a commit message string.
    pub fn validate_string(&self, message: &str) -> Result<ValidationResult> {
        let parsed = CommitMessage::parse(message)?;
        Ok(self.validate(&parsed))
    }

    /// Check a specific commit by reference.
    pub fn check_commit(&self, reference: &str) -> Result<ValidationResult> {
        let message = git::get_commit_message(reference)?;
        self.validate_string(&message)
    }

    /// Check a range of commits.
    pub fn check_range(&self, range: &str) -> Result<Vec<ValidationResult>> {
        let commits = git::get_commit_range(range)?;
        let mut results = Vec::new();

        for (oid, message) in commits {
            let mut result = self.validate_string(&message)?;
            result.commit_sha = Some(oid);
            results.push(result);
        }

        Ok(results)
    }

    /// Get the current branch rules.
    pub fn get_branch_rules(&self) -> Option<&crate::config::BranchRuleConfig> {
        let branch = git::get_branch_name().ok()?;

        // Check for exact match first
        if let Some(rules) = self.config.rules.branch.get(&branch) {
            return Some(rules);
        }

        // Check for pattern match (e.g., "feature/*")
        for (pattern, rules) in &self.config.rules.branch {
            if pattern.contains('*') {
                let glob = glob::Pattern::new(pattern).ok()?;
                if glob.matches(&branch) {
                    return Some(rules);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CommitType;

    #[test]
    fn test_rule_engine_validate() {
        let config = CkConfig::default();
        let engine = RuleEngine::new(config);

        let message = CommitMessage {
            commit_type: CommitType::Feat,
            scope: Some("core".to_string()),
            subject: "add new feature".to_string(),
            body: None,
            footer: None,
            is_breaking: false,
        };

        let result = engine.validate(&message);
        assert!(result.is_valid());
    }

    #[test]
    fn test_rule_engine_subject_too_long() {
        let config = CkConfig::default();
        let engine = RuleEngine::new(config);

        let message = CommitMessage {
            commit_type: CommitType::Feat,
            scope: None,
            subject: "a".repeat(100), // Way too long
            body: None,
            footer: None,
            is_breaking: false,
        };

        let result = engine.validate(&message);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "subject-max-length"));
    }

    #[test]
    fn test_rule_engine_forbidden_type() {
        let mut config = CkConfig::default();
        config.rules.forbidden_types = vec!["wip".to_string()];
        let engine = RuleEngine::new(config);

        // We can't construct a CommitMessage with a custom type easily,
        // so we test via parse
        let result = engine.validate_string("wip: work in progress").unwrap();
        assert!(!result.is_valid());
    }
}
