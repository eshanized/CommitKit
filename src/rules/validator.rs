// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Validation result types.

use crate::cli::args::OutputFormat;
use console::{style, Style};

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Error code for programmatic handling.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional suggestion for fixing.
    pub suggestion: Option<String>,
    /// Whether this is an error (true) or warning (false).
    pub is_error: bool,
    /// Line number where the issue was found.
    pub line: Option<usize>,
}

impl ValidationIssue {
    /// Format the issue for terminal output.
    pub fn format(&self) -> String {
        let prefix = if self.is_error {
            style("✗").red().bold()
        } else {
            style("⚠").yellow().bold()
        };

        let code_style = if self.is_error {
            Style::new().red()
        } else {
            Style::new().yellow()
        };

        let mut output = format!(
            "{} {} {}",
            prefix,
            code_style.apply_to(&self.code),
            self.message
        );

        if let Some(ref suggestion) = self.suggestion {
            output.push_str(&format!(
                "\n  {} {}",
                style("→").dim(),
                style(suggestion).dim()
            ));
        }

        output
    }
}

/// Result of validating a commit message.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The original message.
    pub message: String,
    /// Commit SHA if validating an existing commit.
    pub commit_sha: Option<String>,
    /// Validation errors.
    pub errors: Vec<ValidationIssue>,
    /// Validation warnings.
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new validation result.
    pub fn new(message: String) -> Self {
        Self {
            message,
            commit_sha: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if the validation passed (no errors).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the total number of issues.
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Print the result to stdout.
    pub fn print(&self, format: Option<OutputFormat>) {
        match format {
            Some(OutputFormat::Json) => self.print_json(),
            _ => self.print_text(),
        }
    }

    /// Print in text format.
    fn print_text(&self) {
        // Print commit header if available
        if let Some(ref sha) = self.commit_sha {
            let short_sha = &sha[..7.min(sha.len())];
            let first_line = self.message.lines().next().unwrap_or("");
            let status = if self.is_valid() {
                style("✓").green().bold()
            } else {
                style("✗").red().bold()
            };
            println!("{} {} {}", status, style(short_sha).cyan(), first_line);
        }

        // Print errors
        for error in &self.errors {
            println!("  {}", error.format());
        }

        // Print warnings
        for warning in &self.warnings {
            println!("  {}", warning.format());
        }
    }

    /// Print in JSON format.
    fn print_json(&self) {
        let json = serde_json::json!({
            "valid": self.is_valid(),
            "commit": self.commit_sha,
            "message": self.message,
            "errors": self.errors.iter().map(|e| {
                serde_json::json!({
                    "code": e.code,
                    "message": e.message,
                    "suggestion": e.suggestion,
                    "line": e.line,
                })
            }).collect::<Vec<_>>(),
            "warnings": self.warnings.iter().map(|w| {
                serde_json::json!({
                    "code": w.code,
                    "message": w.message,
                    "suggestion": w.suggestion,
                    "line": w.line,
                })
            }).collect::<Vec<_>>(),
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    }

    /// Get a summary string.
    pub fn summary(&self) -> String {
        if self.is_valid() {
            if self.warnings.is_empty() {
                "Valid".to_string()
            } else {
                format!("Valid ({} warnings)", self.warnings.len())
            }
        } else {
            format!(
                "Invalid ({} errors, {} warnings)",
                self.errors.len(),
                self.warnings.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::new("feat: test".to_string());
        assert!(result.is_valid());
        assert_eq!(result.issue_count(), 0);
    }

    #[test]
    fn test_validation_result_with_errors() {
        let mut result = ValidationResult::new("test".to_string());
        result.errors.push(ValidationIssue {
            code: "test-error".to_string(),
            message: "Test error".to_string(),
            suggestion: None,
            is_error: true,
            line: Some(1),
        });

        assert!(!result.is_valid());
        assert_eq!(result.issue_count(), 1);
    }

    #[test]
    fn test_validation_issue_format() {
        let issue = ValidationIssue {
            code: "test".to_string(),
            message: "Test message".to_string(),
            suggestion: Some("Fix it".to_string()),
            is_error: true,
            line: Some(1),
        };

        let formatted = issue.format();
        assert!(formatted.contains("test"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_summary() {
        let mut result = ValidationResult::new("test".to_string());
        assert!(result.summary().contains("Valid"));

        result.warnings.push(ValidationIssue {
            code: "warn".to_string(),
            message: "Warning".to_string(),
            suggestion: None,
            is_error: false,
            line: None,
        });
        assert!(result.summary().contains("1 warning"));

        result.errors.push(ValidationIssue {
            code: "err".to_string(),
            message: "Error".to_string(),
            suggestion: None,
            is_error: true,
            line: None,
        });
        assert!(result.summary().contains("Invalid"));
    }
}
