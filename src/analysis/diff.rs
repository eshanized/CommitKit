// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Diff analysis for semantic understanding.

use crate::git::DiffInfo;
use std::collections::HashMap;
use std::path::PathBuf;

/// Semantic analysis of diff content.
#[derive(Debug, Clone)]
pub struct DiffAnalysis {
    /// Categorized changes by type.
    pub categories: HashMap<ChangeCategory, Vec<PathBuf>>,
    /// Key changes extracted from the diff.
    pub key_changes: Vec<String>,
    /// Whether this looks like a refactoring.
    pub is_refactoring: bool,
    /// Whether this adds new functionality.
    pub adds_functionality: bool,
    /// Whether this fixes something.
    pub is_fix: bool,
    /// Whether this changes configuration.
    pub is_config_change: bool,
    /// Whether this changes documentation.
    pub is_docs_change: bool,
    /// Whether this changes tests.
    pub is_test_change: bool,
}

/// Categories of changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeCategory {
    /// New files added.
    NewFiles,
    /// Files deleted.
    DeletedFiles,
    /// Tests modified.
    Tests,
    /// Documentation modified.
    Documentation,
    /// Configuration files modified.
    Configuration,
    /// Source code modified.
    Source,
    /// Build/CI files modified.
    Build,
    /// Assets (images, etc.) modified.
    Assets,
}

impl DiffAnalysis {
    /// Analyze a diff and categorize the changes.
    pub fn from_diff(diff: &DiffInfo) -> Self {
        let mut categories: HashMap<ChangeCategory, Vec<PathBuf>> = HashMap::new();
        let mut key_changes = Vec::new();

        for file in &diff.files {
            let category = categorize_file(&file.path);
            categories
                .entry(category)
                .or_default()
                .push(file.path.clone());

            // Extract key changes from the path
            if let Some(change) = extract_key_change(&file.path, file.change_type) {
                key_changes.push(change);
            }
        }

        // Analyze patterns
        let is_refactoring = detect_refactoring(diff);
        let adds_functionality = categories.contains_key(&ChangeCategory::NewFiles)
            || diff.stats.lines_added > diff.stats.lines_removed * 2;
        let is_fix = detect_fix_pattern(diff);
        let is_config_change =
            categories.contains_key(&ChangeCategory::Configuration) && categories.len() <= 2;
        let is_docs_change =
            categories.contains_key(&ChangeCategory::Documentation) && categories.len() == 1;
        let is_test_change =
            categories.contains_key(&ChangeCategory::Tests) && categories.len() == 1;

        Self {
            categories,
            key_changes,
            is_refactoring,
            adds_functionality,
            is_fix,
            is_config_change,
            is_docs_change,
            is_test_change,
        }
    }

    /// Get a summary of changes.
    pub fn summary(&self) -> String {
        if self.key_changes.is_empty() {
            return "Various changes".to_string();
        }

        // Take up to 3 key changes
        let changes: Vec<_> = self.key_changes.iter().take(3).cloned().collect();
        if changes.len() < self.key_changes.len() {
            format!(
                "{} and {} more",
                changes.join(", "),
                self.key_changes.len() - changes.len()
            )
        } else {
            changes.join(", ")
        }
    }
}

/// Categorize a file based on its path.
fn categorize_file(path: &PathBuf) -> ChangeCategory {
    let path_str = path.to_string_lossy().to_lowercase();
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Documentation
    if path_str.contains("doc")
        || path_str.contains("readme")
        || path_str.contains("changelog")
        || extension == "md"
        || extension == "rst"
        || extension == "txt"
    {
        return ChangeCategory::Documentation;
    }

    // Tests
    if path_str.contains("test")
        || path_str.contains("spec")
        || path_str.ends_with("_test.go")
        || path_str.ends_with("_test.rs")
        || path_str.ends_with(".test.js")
        || path_str.ends_with(".test.ts")
        || path_str.ends_with(".spec.js")
        || path_str.ends_with(".spec.ts")
    {
        return ChangeCategory::Tests;
    }

    // Build/CI
    if path_str.contains(".github")
        || path_str.contains("gitlab-ci")
        || path_str.contains("jenkinsfile")
        || path_str.contains("makefile")
        || path_str.contains("dockerfile")
        || path_str.contains("docker-compose")
        || path_str.ends_with(".cmake")
    {
        return ChangeCategory::Build;
    }

    // Configuration
    if path_str.contains("config")
        || path_str.ends_with(".toml")
        || path_str.ends_with(".yaml")
        || path_str.ends_with(".yml")
        || path_str.ends_with(".json")
        || path_str.ends_with(".ini")
        || path_str.ends_with(".env")
        || path_str.contains("cargo.toml")
        || path_str.contains("package.json")
        || path_str.contains("go.mod")
    {
        return ChangeCategory::Configuration;
    }

    // Assets
    if matches!(
        extension,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" | "mp4" | "mp3" | "wav" | "pdf"
    ) {
        return ChangeCategory::Assets;
    }

    // Default to source
    ChangeCategory::Source
}

/// Extract a key change description from a file.
fn extract_key_change(path: &PathBuf, change_type: crate::git::ChangeType) -> Option<String> {
    let file_name = path.file_stem()?.to_string_lossy().to_string();

    let action = match change_type {
        crate::git::ChangeType::Added => "add",
        crate::git::ChangeType::Deleted => "remove",
        crate::git::ChangeType::Modified => "update",
        crate::git::ChangeType::Renamed => "rename",
        _ => "modify",
    };

    // Try to make it readable
    let readable_name = file_name.replace('_', " ").replace('-', " ");

    Some(format!("{} {}", action, readable_name))
}

/// Detect if changes look like a refactoring.
fn detect_refactoring(diff: &DiffInfo) -> bool {
    // Refactoring typically has balanced additions and deletions
    // and doesn't add new files
    let balance = if diff.stats.lines_added > 0 && diff.stats.lines_removed > 0 {
        let ratio = diff.stats.lines_added as f64 / diff.stats.lines_removed as f64;
        ratio > 0.5 && ratio < 2.0
    } else {
        false
    };

    let no_new_files = !diff
        .files
        .iter()
        .any(|f| matches!(f.change_type, crate::git::ChangeType::Added));

    balance && no_new_files && diff.stats.total_lines_changed() > 10
}

/// Detect if changes look like a bug fix.
fn detect_fix_pattern(diff: &DiffInfo) -> bool {
    // Check for fix-related patterns in the patches
    for patch in diff.patches.values() {
        let lower = patch.to_lowercase();
        if lower.contains("fix")
            || lower.contains("bug")
            || lower.contains("error")
            || lower.contains("issue")
            || lower.contains("crash")
            || lower.contains("null")
            || lower.contains("undefined")
        {
            return true;
        }
    }

    // Small changes to source files might be fixes
    diff.stats.files_changed <= 3
        && diff.stats.total_lines_changed() <= 50
        && diff
            .files
            .iter()
            .all(|f| matches!(f.change_type, crate::git::ChangeType::Modified))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_file() {
        assert_eq!(
            categorize_file(&PathBuf::from("README.md")),
            ChangeCategory::Documentation
        );
        assert_eq!(
            categorize_file(&PathBuf::from("src/main.rs")),
            ChangeCategory::Source
        );
        assert_eq!(
            categorize_file(&PathBuf::from("tests/test_main.rs")),
            ChangeCategory::Tests
        );
        assert_eq!(
            categorize_file(&PathBuf::from("Cargo.toml")),
            ChangeCategory::Configuration
        );
        assert_eq!(
            categorize_file(&PathBuf::from(".github/workflows/ci.yml")),
            ChangeCategory::Build
        );
    }

    #[test]
    fn test_diff_analysis_empty() {
        let diff = DiffInfo::empty();
        let analysis = DiffAnalysis::from_diff(&diff);
        assert!(analysis.categories.is_empty());
        assert!(!analysis.is_refactoring);
    }
}
