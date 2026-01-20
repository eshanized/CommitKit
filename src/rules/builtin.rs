// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Built-in validation rules.

use crate::commit::CommitMessage;
use crate::config::CkConfig;

use super::validator::ValidationIssue;

/// Trait for custom rules.
pub trait Rule: std::fmt::Debug + Send + Sync {
    /// Check the commit message and return an issue if validation fails.
    fn check(&self, message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue>;

    /// Get the rule name.
    fn name(&self) -> &str;
}

impl Clone for Box<dyn Rule> {
    fn clone(&self) -> Self {
        // For now, we can't clone trait objects, so just panic
        // In a real implementation, we'd use dyn-clone or similar
        panic!("Cannot clone custom rules")
    }
}

/// Apply all built-in rules to a commit message.
pub fn apply_builtin_rules(message: &CommitMessage, config: &CkConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Subject length rules
    if let Some(issue) = check_max_subject_length(message, config) {
        issues.push(issue);
    }
    if let Some(issue) = check_min_subject_length(message, config) {
        issues.push(issue);
    }

    // Type rules
    if let Some(issue) = check_allowed_types(message, config) {
        issues.push(issue);
    }
    if let Some(issue) = check_forbidden_types(message, config) {
        issues.push(issue);
    }

    // Scope rules
    if let Some(issue) = check_require_scope(message, config) {
        issues.push(issue);
    }
    if let Some(issue) = check_allowed_scopes(message, config) {
        issues.push(issue);
    }

    // Body rules
    if let Some(issue) = check_require_body(message, config) {
        issues.push(issue);
    }

    // Format rules
    if let Some(issue) = check_imperative_mood(message) {
        issues.push(issue);
    }
    if let Some(issue) = check_subject_case(message) {
        issues.push(issue);
    }
    if let Some(issue) = check_subject_trailing_period(message) {
        issues.push(issue);
    }

    issues
}

/// Check maximum subject length.
fn check_max_subject_length(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    let max = config.rules.max_subject_length;
    let len = message.subject.len();

    if len > max {
        Some(ValidationIssue {
            code: "subject-max-length".to_string(),
            message: format!("Subject is too long: {} characters (max: {})", len, max),
            suggestion: Some(format!("Shorten the subject to {} characters or less", max)),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check minimum subject length.
fn check_min_subject_length(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    let min = config.rules.min_subject_length;
    let len = message.subject.len();

    if len < min {
        Some(ValidationIssue {
            code: "subject-min-length".to_string(),
            message: format!("Subject is too short: {} characters (min: {})", len, min),
            suggestion: Some("Add more detail to the subject".to_string()),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if the commit type is allowed.
fn check_allowed_types(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    let type_str = message.commit_type.as_str();

    if !config.rules.allowed_types.is_empty()
        && !config.rules.allowed_types.iter().any(|t| t == type_str)
    {
        Some(ValidationIssue {
            code: "type-not-allowed".to_string(),
            message: format!("Commit type '{}' is not allowed", type_str),
            suggestion: Some(format!(
                "Use one of: {}",
                config.rules.allowed_types.join(", ")
            )),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if the commit type is forbidden.
fn check_forbidden_types(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    let type_str = message.commit_type.as_str();

    if config.rules.forbidden_types.iter().any(|t| t == type_str) {
        Some(ValidationIssue {
            code: "type-forbidden".to_string(),
            message: format!("Commit type '{}' is forbidden", type_str),
            suggestion: Some("Use a different commit type".to_string()),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if scope is required.
fn check_require_scope(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    if config.rules.require_scope && message.scope.is_none() {
        Some(ValidationIssue {
            code: "scope-required".to_string(),
            message: "Scope is required but not provided".to_string(),
            suggestion: Some("Add a scope in parentheses: type(scope): subject".to_string()),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if scope is in the allowed list.
fn check_allowed_scopes(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    if let Some(ref scope) = message.scope {
        if !config.rules.scope.allowed.is_empty()
            && !config.rules.scope.allowed.iter().any(|s| s == scope)
        {
            return Some(ValidationIssue {
                code: "scope-not-allowed".to_string(),
                message: format!("Scope '{}' is not allowed", scope),
                suggestion: Some(format!(
                    "Use one of: {}",
                    config.rules.scope.allowed.join(", ")
                )),
                is_error: true,
                line: Some(1),
            });
        }
    }
    None
}

/// Check if body is required.
fn check_require_body(message: &CommitMessage, config: &CkConfig) -> Option<ValidationIssue> {
    if config.rules.require_body && message.body.is_none() {
        Some(ValidationIssue {
            code: "body-required".to_string(),
            message: "Body is required but not provided".to_string(),
            suggestion: Some("Add a body with more details about the change".to_string()),
            is_error: true,
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if subject starts with imperative mood.
fn check_imperative_mood(message: &CommitMessage) -> Option<ValidationIssue> {
    let first_word = message.subject.split_whitespace().next()?;
    let lower = first_word.to_lowercase();

    // Common non-imperative patterns
    let non_imperative = [
        "added",
        "adding",
        "adds",
        "fixed",
        "fixing",
        "fixes",
        "updated",
        "updating",
        "updates",
        "removed",
        "removing",
        "removes",
        "changed",
        "changing",
        "changes",
        "implemented",
        "implementing",
        "implements",
        "created",
        "creating",
        "creates",
    ];

    if non_imperative.contains(&lower.as_str()) {
        Some(ValidationIssue {
            code: "subject-imperative".to_string(),
            message: format!(
                "Subject should use imperative mood (found '{}')",
                first_word
            ),
            suggestion: Some(format!(
                "Use imperative form like 'add' instead of '{}'",
                first_word
            )),
            is_error: false, // Warning, not error
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if subject starts with lowercase.
fn check_subject_case(message: &CommitMessage) -> Option<ValidationIssue> {
    let first_char = message.subject.chars().next()?;

    if first_char.is_uppercase() {
        Some(ValidationIssue {
            code: "subject-case".to_string(),
            message: "Subject should start with lowercase".to_string(),
            suggestion: Some("Start the subject with a lowercase letter".to_string()),
            is_error: false, // Warning
            line: Some(1),
        })
    } else {
        None
    }
}

/// Check if subject ends with a period.
fn check_subject_trailing_period(message: &CommitMessage) -> Option<ValidationIssue> {
    if message.subject.ends_with('.') {
        Some(ValidationIssue {
            code: "subject-trailing-period".to_string(),
            message: "Subject should not end with a period".to_string(),
            suggestion: Some("Remove the trailing period".to_string()),
            is_error: false, // Warning
            line: Some(1),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CommitType;

    fn make_message(subject: &str) -> CommitMessage {
        CommitMessage {
            commit_type: CommitType::Feat,
            scope: None,
            subject: subject.to_string(),
            body: None,
            footer: None,
            is_breaking: false,
        }
    }

    #[test]
    fn test_max_subject_length() {
        let config = CkConfig::default();
        let message = make_message(&"a".repeat(100));
        let issue = check_max_subject_length(&message, &config);
        assert!(issue.is_some());
        assert!(issue.unwrap().is_error);
    }

    #[test]
    fn test_min_subject_length() {
        let config = CkConfig::default();
        let message = make_message("ab");
        let issue = check_min_subject_length(&message, &config);
        assert!(issue.is_some());
    }

    #[test]
    fn test_imperative_mood() {
        let message = make_message("added new feature");
        let issue = check_imperative_mood(&message);
        assert!(issue.is_some());
        assert!(!issue.unwrap().is_error); // Should be warning

        let message = make_message("add new feature");
        let issue = check_imperative_mood(&message);
        assert!(issue.is_none());
    }

    #[test]
    fn test_subject_case() {
        let message = make_message("Add new feature");
        let issue = check_subject_case(&message);
        assert!(issue.is_some());

        let message = make_message("add new feature");
        let issue = check_subject_case(&message);
        assert!(issue.is_none());
    }

    #[test]
    fn test_trailing_period() {
        let message = make_message("add new feature.");
        let issue = check_subject_trailing_period(&message);
        assert!(issue.is_some());
    }
}
