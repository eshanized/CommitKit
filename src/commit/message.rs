// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit message structure and parsing.

use crate::config::CommitType;
use crate::error::{CkError, CommitError, Result};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for parsing conventional commit messages.
    static ref CONVENTIONAL_REGEX: Regex = Regex::new(
        r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<subject>.+?)(?:\n\n(?P<body>[\s\S]*?))?(?:\n\n(?P<footer>[\s\S]*))?$"
    ).unwrap();
}

/// A structured commit message.
#[derive(Debug, Clone)]
pub struct CommitMessage {
    /// Commit type (feat, fix, etc.).
    pub commit_type: CommitType,
    /// Optional scope.
    pub scope: Option<String>,
    /// Subject line.
    pub subject: String,
    /// Optional body.
    pub body: Option<String>,
    /// Optional footer (references, breaking changes, etc.).
    pub footer: Option<String>,
    /// Whether this is a breaking change.
    pub is_breaking: bool,
}

impl CommitMessage {
    /// Create a new commit message.
    pub fn new(commit_type: CommitType, subject: impl Into<String>) -> Self {
        Self {
            commit_type,
            scope: None,
            subject: subject.into(),
            body: None,
            footer: None,
            is_breaking: false,
        }
    }

    /// Set the scope.
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Set the body.
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        let body_str = body.into();
        if !body_str.is_empty() {
            self.body = Some(body_str);
        }
        self
    }

    /// Set the footer.
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        let footer_str = footer.into();
        if !footer_str.is_empty() {
            self.footer = Some(footer_str);
        }
        self
    }

    /// Set the breaking flag.
    pub fn with_breaking(mut self, breaking: bool) -> Self {
        self.is_breaking = breaking;
        self
    }

    /// Parse a commit message from a string.
    pub fn parse(message: &str) -> Result<Self> {
        let message = message.trim();

        if message.is_empty() {
            return Err(CkError::Commit(CommitError::EmptyMessage));
        }

        // Try to parse as conventional commit
        if let Some(captures) = CONVENTIONAL_REGEX.captures(message) {
            let type_str = captures.name("type").map(|m| m.as_str()).unwrap_or("");
            let commit_type = type_str.parse::<CommitType>().ok().ok_or_else(|| {
                CkError::Commit(CommitError::ParseFailed {
                    message: format!("Unknown commit type: {}", type_str),
                })
            })?;

            let scope = captures.name("scope").map(|m| m.as_str().to_string());
            let subject = captures
                .name("subject")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let body = captures
                .name("body")
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            let footer = captures
                .name("footer")
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            let is_breaking = captures.name("breaking").is_some()
                || footer
                    .as_ref()
                    .map(|f| f.contains("BREAKING CHANGE"))
                    .unwrap_or(false);

            Ok(Self {
                commit_type,
                scope,
                subject,
                body,
                footer,
                is_breaking,
            })
        } else {
            // Try to parse as simple message (no type)
            let first_line = message.lines().next().unwrap_or("");

            // Check if it looks like it has a type
            if first_line.contains(':') {
                let parts: Vec<&str> = first_line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    // Extract type from first part (might have scope)
                    let type_part = parts[0].trim();
                    let subject = parts[1].trim().to_string();

                    // Check for scope in parentheses
                    let (type_str, scope) = if type_part.contains('(') && type_part.contains(')') {
                        let type_end = type_part.find('(').unwrap();
                        let scope_start = type_end + 1;
                        let scope_end = type_part.find(')').unwrap();
                        (
                            &type_part[..type_end],
                            Some(type_part[scope_start..scope_end].to_string()),
                        )
                    } else {
                        (type_part, None)
                    };

                    // Check for breaking indicator
                    let (type_str, is_breaking) = if let Some(stripped) = type_str.strip_suffix('!')
                    {
                        (stripped, true)
                    } else {
                        (type_str, false)
                    };

                    if let Ok(commit_type) = type_str.parse::<CommitType>() {
                        // Extract body if present
                        let body = message
                            .lines()
                            .skip(2) // Skip first line and blank line
                            .collect::<Vec<_>>()
                            .join("\n");
                        let body = if body.is_empty() { None } else { Some(body) };

                        return Ok(Self {
                            commit_type,
                            scope,
                            subject,
                            body,
                            footer: None,
                            is_breaking,
                        });
                    }
                }
            }

            Err(CkError::Commit(CommitError::InvalidConventionalFormat))
        }
    }

    /// Format the commit message as a string.
    pub fn format(&self) -> String {
        let mut result = String::new();

        // Type
        result.push_str(self.commit_type.as_str());

        // Scope
        if let Some(ref scope) = self.scope {
            result.push('(');
            result.push_str(scope);
            result.push(')');
        }

        // Breaking change indicator
        if self.is_breaking {
            result.push('!');
        }

        // Subject
        result.push_str(": ");
        result.push_str(&self.subject);

        // Body
        if let Some(ref body) = self.body {
            result.push_str("\n\n");
            result.push_str(body);
        }

        // Footer
        if let Some(ref footer) = self.footer {
            result.push_str("\n\n");
            result.push_str(footer);
        }

        result
    }

    /// Get the first line (header) of the commit message.
    pub fn header(&self) -> String {
        let mut result = String::new();
        result.push_str(self.commit_type.as_str());

        if let Some(ref scope) = self.scope {
            result.push('(');
            result.push_str(scope);
            result.push(')');
        }

        if self.is_breaking {
            result.push('!');
        }

        result.push_str(": ");
        result.push_str(&self.subject);

        result
    }

    /// Get the header length.
    pub fn header_len(&self) -> usize {
        self.header().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_message_new() {
        let msg = CommitMessage::new(CommitType::Feat, "add new feature");
        assert_eq!(msg.commit_type, CommitType::Feat);
        assert_eq!(msg.subject, "add new feature");
        assert!(msg.scope.is_none());
        assert!(!msg.is_breaking);
    }

    #[test]
    fn test_commit_message_builder() {
        let msg = CommitMessage::new(CommitType::Fix, "fix bug")
            .with_scope("core")
            .with_body("This fixes the bug")
            .with_breaking(true);

        assert_eq!(msg.scope, Some("core".to_string()));
        assert!(msg.body.is_some());
        assert!(msg.is_breaking);
    }

    #[test]
    fn test_commit_message_format() {
        let msg = CommitMessage::new(CommitType::Feat, "add feature")
            .with_scope("api")
            .with_body("Detailed description");

        let formatted = msg.format();
        assert_eq!(formatted, "feat(api): add feature\n\nDetailed description");
    }

    #[test]
    fn test_commit_message_format_breaking() {
        let msg = CommitMessage::new(CommitType::Feat, "breaking change").with_breaking(true);

        let formatted = msg.format();
        assert!(formatted.starts_with("feat!:"));
    }

    #[test]
    fn test_commit_message_parse() {
        let msg = CommitMessage::parse("feat(core): add new feature").unwrap();
        assert_eq!(msg.commit_type, CommitType::Feat);
        assert_eq!(msg.scope, Some("core".to_string()));
        assert_eq!(msg.subject, "add new feature");
    }

    #[test]
    fn test_commit_message_parse_with_body() {
        let msg = CommitMessage::parse("fix: fix bug\n\nThis is the body").unwrap();
        assert_eq!(msg.commit_type, CommitType::Fix);
        assert_eq!(msg.body, Some("This is the body".to_string()));
    }

    #[test]
    fn test_commit_message_parse_breaking() {
        let msg = CommitMessage::parse("feat!: breaking change").unwrap();
        assert!(msg.is_breaking);
    }

    #[test]
    fn test_commit_message_parse_invalid() {
        let result = CommitMessage::parse("not a conventional commit");
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_message_header() {
        let msg = CommitMessage::new(CommitType::Feat, "add feature").with_scope("cli");

        assert_eq!(msg.header(), "feat(cli): add feature");
        assert_eq!(msg.header_len(), 22);
    }
}
