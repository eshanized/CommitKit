// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Secret detection in diffs.

use crate::config::CkConfig;
use crate::error::{CkError, Result, SecurityError};
use crate::git::DiffInfo;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Built-in secret patterns.
    static ref BUILTIN_PATTERNS: Vec<(&'static str, Regex)> = vec![
        (
            "AWS Access Key",
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()
        ),
        (
            "AWS Secret Key",
            Regex::new(r#"(?i)aws(.{0,20})?['"][0-9a-zA-Z/+]{40}['"]"#).unwrap()
        ),
        (
            "Generic API Key",
            Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9]{16,}['"]?"#).unwrap()
        ),
        (
            "Generic Secret",
            Regex::new(r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}['"]"#).unwrap()
        ),
        (
            "Private Key",
            Regex::new(r"-----BEGIN (RSA|DSA|EC|OPENSSH|PGP) PRIVATE KEY-----").unwrap()
        ),
        (
            "GitHub Token",
            Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap()
        ),
        (
            "Slack Token",
            Regex::new(r"xox[baprs]-[0-9]{10,}-[0-9A-Za-z]{10,}").unwrap()
        ),
        (
            "JWT Token",
            Regex::new(r"eyJ[A-Za-z0-9-_=]+\.eyJ[A-Za-z0-9-_=]+\.?[A-Za-z0-9-_.+/=]*").unwrap()
        ),
    ];
}

/// A detected secret match.
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// Name of the pattern that matched.
    pub pattern_name: String,
    /// The file where the secret was found.
    pub file: String,
    /// Line number (if available).
    pub line: Option<usize>,
    /// Redacted preview of the match.
    pub preview: String,
}

impl SecretMatch {
    /// Format for display.
    pub fn format(&self) -> String {
        let location = if let Some(line) = self.line {
            format!("{}:{}", self.file, line)
        } else {
            self.file.clone()
        };

        format!("{}: {} ({})", self.pattern_name, location, self.preview)
    }
}

/// Secret scanner for detecting sensitive data.
pub struct SecretScanner {
    patterns: Vec<(String, Regex)>,
    block_on_secret: bool,
}

impl SecretScanner {
    /// Create a new secret scanner with default configuration.
    pub fn new() -> Self {
        Self::with_config(&CkConfig::default())
    }

    /// Create a scanner from configuration.
    pub fn with_config(config: &CkConfig) -> Self {
        let mut patterns: Vec<(String, Regex)> = BUILTIN_PATTERNS
            .iter()
            .map(|(name, re)| (name.to_string(), re.clone()))
            .collect();

        // Add custom patterns from config
        for custom in &config.security.patterns {
            if let Ok(re) = Regex::new(&custom.pattern) {
                patterns.push((custom.name.clone(), re));
            }
        }

        Self {
            patterns,
            block_on_secret: config.security.block_on_secret,
        }
    }

    /// Scan a diff for secrets.
    pub fn scan_diff(&self, diff: &DiffInfo) -> Vec<SecretMatch> {
        let mut matches = Vec::new();

        for (path, content) in &diff.patches {
            let file_str = path.to_string_lossy().to_string();

            for (line_num, line) in content.lines().enumerate() {
                // Only scan added lines
                if !line.starts_with('+') {
                    continue;
                }

                let line_content = &line[1..]; // Skip the '+' prefix

                for (name, pattern) in &self.patterns {
                    if pattern.is_match(line_content) {
                        // Create redacted preview
                        let preview = if line_content.len() > 40 {
                            format!("{}...", &line_content[..40])
                        } else {
                            line_content.to_string()
                        };

                        // Redact the actual secret
                        let preview = pattern.replace_all(&preview, "[REDACTED]").to_string();

                        matches.push(SecretMatch {
                            pattern_name: name.clone(),
                            file: file_str.clone(),
                            line: Some(line_num + 1),
                            preview,
                        });
                    }
                }
            }
        }

        matches
    }

    /// Scan and return an error if secrets are found.
    pub fn scan_and_block(&self, diff: &DiffInfo) -> Result<()> {
        let matches = self.scan_diff(diff);

        if !matches.is_empty() && self.block_on_secret {
            if matches.len() == 1 {
                Err(CkError::Security(SecurityError::SecretDetected {
                    pattern_name: matches[0].pattern_name.clone(),
                }))
            } else {
                Err(CkError::Security(SecurityError::MultipleSecrets {
                    count: matches.len(),
                }))
            }
        } else {
            Ok(())
        }
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan a diff for secrets.
pub fn detect_secrets(diff: &DiffInfo, config: &CkConfig) -> Vec<SecretMatch> {
    if !config.security.enabled {
        return Vec::new();
    }

    let scanner = SecretScanner::with_config(config);
    scanner.scan_diff(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecretPattern;
    use crate::git::DiffStats;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_diff_with_content(file: &str, content: &str) -> DiffInfo {
        let mut patches = HashMap::new();
        patches.insert(PathBuf::from(file), content.to_string());

        DiffInfo {
            files: vec![],
            stats: DiffStats::default(),
            patches,
        }
    }

    #[test]
    fn test_detect_aws_key() {
        let diff = make_diff_with_content("config.py", "+AWS_KEY = 'AKIAIOSFODNN7EXAMPLE'\n");

        let scanner = SecretScanner::new();
        let matches = scanner.scan_diff(&diff);

        assert!(!matches.is_empty());
        assert!(matches[0].pattern_name.contains("AWS"));
    }

    #[test]
    fn test_detect_private_key() {
        let diff = make_diff_with_content("key.pem", "+-----BEGIN RSA PRIVATE KEY-----\n");

        let scanner = SecretScanner::new();
        let matches = scanner.scan_diff(&diff);

        assert!(!matches.is_empty());
        assert!(matches[0].pattern_name.contains("Private Key"));
    }

    #[test]
    fn test_no_false_positive_removed_lines() {
        let diff = make_diff_with_content("config.py", "-AWS_KEY = 'AKIAIOSFODNN7EXAMPLE'\n");

        let scanner = SecretScanner::new();
        let matches = scanner.scan_diff(&diff);

        // Should not match removed lines
        assert!(matches.is_empty());
    }

    #[test]
    fn test_scan_and_block() {
        let diff = make_diff_with_content("config.py", "+API_KEY = 'supersecretkey12345678'\n");

        let scanner = SecretScanner::new();
        let result = scanner.scan_and_block(&diff);

        assert!(result.is_err());
    }

    #[test]
    fn test_custom_pattern() {
        let mut config = CkConfig::default();
        config.security.patterns.push(SecretPattern {
            name: "Custom Token".to_string(),
            pattern: r"MYTOKEN_[A-Z0-9]{20}".to_string(),
            description: None,
        });

        let diff = make_diff_with_content("config.py", "+TOKEN = 'MYTOKEN_ABCD1234567890123456'\n");

        let scanner = SecretScanner::with_config(&config);
        let matches = scanner.scan_diff(&diff);

        assert!(!matches.is_empty());
        assert_eq!(matches[0].pattern_name, "Custom Token");
    }
}
