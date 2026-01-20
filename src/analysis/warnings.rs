// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Warning generation for commit quality issues.

use crate::config::CkConfig;
use crate::git::DiffInfo;
use std::fmt;
use std::path::PathBuf;

use super::context::Package;

/// Warning severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningLevel {
    /// Informational, not a problem.
    Info,
    /// Something to be aware of.
    Warning,
    /// Serious issue that should be addressed.
    Error,
}

impl fmt::Display for WarningLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarningLevel::Info => write!(f, "info"),
            WarningLevel::Warning => write!(f, "warning"),
            WarningLevel::Error => write!(f, "error"),
        }
    }
}

/// A single warning about the commit.
#[derive(Debug, Clone)]
pub struct Warning {
    /// Warning severity level.
    pub level: WarningLevel,
    /// Warning code for programmatic handling.
    pub code: WarningCode,
    /// Human-readable message.
    pub message: String,
    /// Optional suggestion for fixing.
    pub suggestion: Option<String>,
}

/// Warning codes for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    /// Commit is too large.
    OversizedCommit,
    /// Changes span multiple packages.
    MultiplePackages,
    /// Mixed concerns in the commit.
    MixedConcerns,
    /// No scope detected but might be needed.
    MissingScope,
    /// Risky file changes.
    RiskyChanges,
    /// Binary files included.
    BinaryFiles,
    /// Unstaged changes exist.
    UnstagedChanges,
    /// Large single file change.
    LargeFile,
}

impl fmt::Display for WarningCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarningCode::OversizedCommit => write!(f, "oversized-commit"),
            WarningCode::MultiplePackages => write!(f, "multiple-packages"),
            WarningCode::MixedConcerns => write!(f, "mixed-concerns"),
            WarningCode::MissingScope => write!(f, "missing-scope"),
            WarningCode::RiskyChanges => write!(f, "risky-changes"),
            WarningCode::BinaryFiles => write!(f, "binary-files"),
            WarningCode::UnstagedChanges => write!(f, "unstaged-changes"),
            WarningCode::LargeFile => write!(f, "large-file"),
        }
    }
}

/// Collection of warnings.
#[derive(Debug, Clone, Default)]
pub struct Warnings {
    warnings: Vec<Warning>,
}

impl Warnings {
    /// Create an empty warnings collection.
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Add a warning.
    pub fn add(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// Check if there are any warnings.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Get the number of warnings.
    pub fn len(&self) -> usize {
        self.warnings.len()
    }

    /// Get all warnings.
    pub fn all(&self) -> &[Warning] {
        &self.warnings
    }

    /// Get warnings of a specific level or higher.
    pub fn at_level(&self, min_level: WarningLevel) -> Vec<&Warning> {
        self.warnings
            .iter()
            .filter(|w| w.level >= min_level)
            .collect()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.warnings.iter().any(|w| w.level == WarningLevel::Error)
    }

    /// Get the highest severity level.
    pub fn max_level(&self) -> Option<WarningLevel> {
        self.warnings.iter().map(|w| w.level).max()
    }

    /// Iterate over warnings.
    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.warnings.iter()
    }
}

impl IntoIterator for Warnings {
    type Item = Warning;
    type IntoIter = std::vec::IntoIter<Warning>;

    fn into_iter(self) -> Self::IntoIter {
        self.warnings.into_iter()
    }
}

/// Generate warnings based on diff and context.
pub fn generate_warnings(
    diff: &DiffInfo,
    files: &[PathBuf],
    packages: &[Package],
    config: &CkConfig,
) -> Warnings {
    let mut warnings = Warnings::new();

    // Check for oversized commit
    const DEFAULT_SIZE_THRESHOLD: usize = 500;
    if diff.stats.total_lines_changed() > DEFAULT_SIZE_THRESHOLD {
        warnings.add(Warning {
            level: WarningLevel::Warning,
            code: WarningCode::OversizedCommit,
            message: format!(
                "Commit is very large: {} lines changed",
                diff.stats.total_lines_changed()
            ),
            suggestion: Some("Consider splitting into smaller, focused commits".to_string()),
        });
    }

    // Check for multiple packages
    let changed_packages: Vec<_> = packages.iter().filter(|p| p.has_changes).collect();
    if changed_packages.len() > 1 {
        let names: Vec<_> = changed_packages.iter().map(|p| p.name.as_str()).collect();
        warnings.add(Warning {
            level: WarningLevel::Warning,
            code: WarningCode::MultiplePackages,
            message: format!(
                "Changes span {} packages: {}",
                changed_packages.len(),
                names.join(", ")
            ),
            suggestion: Some("Consider separate commits per package".to_string()),
        });
    }

    // Check for mixed concerns (source + tests + docs in same commit)
    let has_source = files.iter().any(|f| {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(ext, "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp")
            && !f.to_string_lossy().contains("test")
    });
    let has_tests = files
        .iter()
        .any(|f| f.to_string_lossy().to_lowercase().contains("test"));
    let has_docs = files.iter().any(|f| {
        f.extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "md" || e == "rst")
            .unwrap_or(false)
    });

    let concerns_count = [has_source, has_tests, has_docs]
        .iter()
        .filter(|&&x| x)
        .count();
    if concerns_count > 2 {
        warnings.add(Warning {
            level: WarningLevel::Info,
            code: WarningCode::MixedConcerns,
            message: "Commit includes source, tests, and documentation".to_string(),
            suggestion: Some(
                "This might be intentional for a feature, but consider if they should be separate"
                    .to_string(),
            ),
        });
    }

    // Check for missing scope when required
    if config.rules.require_scope && !files.is_empty() {
        // This is just a pre-warning; actual validation happens in rules
        let has_obvious_scope = packages.iter().any(|p| p.has_changes);
        if !has_obvious_scope {
            warnings.add(Warning {
                level: WarningLevel::Info,
                code: WarningCode::MissingScope,
                message: "No obvious scope detected".to_string(),
                suggestion: Some(
                    "Consider which component or area these changes affect".to_string(),
                ),
            });
        }
    }

    // Check for risky file changes
    let risky_patterns = [
        "secret",
        "password",
        "key",
        "credential",
        ".env",
        "id_rsa",
        "id_ed25519",
    ];
    for file in files {
        let path_str = file.to_string_lossy().to_lowercase();
        for pattern in risky_patterns {
            if path_str.contains(pattern) {
                warnings.add(Warning {
                    level: WarningLevel::Error,
                    code: WarningCode::RiskyChanges,
                    message: format!("Potentially sensitive file in commit: {}", file.display()),
                    suggestion: Some("Make sure this file doesn't contain secrets".to_string()),
                });
                break;
            }
        }
    }

    // Check for binary files
    if diff.stats.binary_files > 0 {
        warnings.add(Warning {
            level: WarningLevel::Info,
            code: WarningCode::BinaryFiles,
            message: format!(
                "{} binary file{} in commit",
                diff.stats.binary_files,
                if diff.stats.binary_files == 1 {
                    ""
                } else {
                    "s"
                }
            ),
            suggestion: Some("Consider using Git LFS for large binary files".to_string()),
        });
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::DiffStats;
    use std::collections::HashMap;

    #[test]
    fn test_warnings_empty() {
        let warnings = Warnings::new();
        assert!(warnings.is_empty());
        assert!(!warnings.has_errors());
    }

    #[test]
    fn test_warnings_add() {
        let mut warnings = Warnings::new();
        warnings.add(Warning {
            level: WarningLevel::Warning,
            code: WarningCode::OversizedCommit,
            message: "Test".to_string(),
            suggestion: None,
        });
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_warnings_at_level() {
        let mut warnings = Warnings::new();
        warnings.add(Warning {
            level: WarningLevel::Info,
            code: WarningCode::BinaryFiles,
            message: "Info".to_string(),
            suggestion: None,
        });
        warnings.add(Warning {
            level: WarningLevel::Error,
            code: WarningCode::RiskyChanges,
            message: "Error".to_string(),
            suggestion: None,
        });

        let errors_only = warnings.at_level(WarningLevel::Error);
        assert_eq!(errors_only.len(), 1);
    }

    #[test]
    fn test_generate_warnings_oversized() {
        let diff = DiffInfo {
            files: vec![],
            stats: DiffStats {
                files_changed: 50,
                lines_added: 400,
                lines_removed: 200,
                binary_files: 0,
            },
            patches: HashMap::new(),
        };

        let warnings = generate_warnings(&diff, &[], &[], &CkConfig::default());
        assert!(!warnings.is_empty());
        assert!(warnings
            .iter()
            .any(|w| w.code == WarningCode::OversizedCommit));
    }

    #[test]
    fn test_generate_warnings_risky() {
        let diff = DiffInfo::empty();
        let files = vec![PathBuf::from(".env.production")];

        let warnings = generate_warnings(&diff, &files, &[], &CkConfig::default());
        assert!(warnings.has_errors());
        assert!(warnings.iter().any(|w| w.code == WarningCode::RiskyChanges));
    }
}
