// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit type and scope inference.

use crate::config::{CkConfig, CommitType};
use crate::git::DiffInfo;
use std::path::PathBuf;

use super::context::Package;
use super::diff::{ChangeCategory, DiffAnalysis};

/// Score for a commit type inference.
#[derive(Debug, Clone)]
pub struct CommitTypeScore {
    /// The commit type.
    pub commit_type: CommitType,
    /// Confidence score (0.0 - 1.0).
    pub score: f64,
    /// Reason for the inference.
    pub reason: String,
}

/// Infer the most likely commit type from the diff.
pub fn infer_type(diff: &DiffInfo, files: &[PathBuf]) -> Option<CommitType> {
    let scores = score_commit_types(diff, files);

    // Return the highest scoring type if confidence is above threshold
    scores
        .into_iter()
        .filter(|s| s.score >= 0.5)
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|s| s.commit_type)
}

/// Score all possible commit types.
pub fn score_commit_types(diff: &DiffInfo, files: &[PathBuf]) -> Vec<CommitTypeScore> {
    let analysis = DiffAnalysis::from_diff(diff);
    let mut scores = Vec::new();

    // Documentation-only changes
    if analysis.is_docs_change {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Docs,
            score: 0.95,
            reason: "Only documentation files changed".to_string(),
        });
    }

    // Test-only changes
    if analysis.is_test_change {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Test,
            score: 0.95,
            reason: "Only test files changed".to_string(),
        });
    }

    // Configuration changes
    if analysis.is_config_change {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Chore,
            score: 0.8,
            reason: "Configuration files changed".to_string(),
        });
    }

    // Build/CI changes
    if analysis.categories.contains_key(&ChangeCategory::Build) && analysis.categories.len() == 1 {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Ci,
            score: 0.9,
            reason: "CI/build files changed".to_string(),
        });
    }

    // Refactoring
    if analysis.is_refactoring {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Refactor,
            score: 0.75,
            reason: "Balanced additions/deletions suggest refactoring".to_string(),
        });
    }

    // Fix detection
    if analysis.is_fix {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Fix,
            score: 0.7,
            reason: "Changes look like a bug fix".to_string(),
        });
    }

    // New functionality
    if analysis.adds_functionality {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Feat,
            score: 0.6,
            reason: "New files or significant additions".to_string(),
        });
    }

    // Check file patterns
    for file in files {
        let path_str = file.to_string_lossy().to_lowercase();

        // Performance-related files
        if path_str.contains("perf") || path_str.contains("bench") || path_str.contains("optim") {
            scores.push(CommitTypeScore {
                commit_type: CommitType::Perf,
                score: 0.7,
                reason: format!("Performance-related file: {}", file.display()),
            });
        }

        // Style-related files
        if path_str.contains("style")
            || path_str.contains("lint")
            || path_str.contains("format")
            || path_str.ends_with(".eslintrc")
            || path_str.ends_with(".prettierrc")
            || path_str.ends_with("rustfmt.toml")
        {
            scores.push(CommitTypeScore {
                commit_type: CommitType::Style,
                score: 0.8,
                reason: "Style/formatting files changed".to_string(),
            });
        }
    }

    // Default to feat if nothing else matches
    if scores.is_empty() {
        scores.push(CommitTypeScore {
            commit_type: CommitType::Feat,
            score: 0.3,
            reason: "Default based on source changes".to_string(),
        });
    }

    scores.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}

/// Infer the scope from file paths and packages.
pub fn infer_scope(files: &[PathBuf], packages: &[Package], config: &CkConfig) -> Option<String> {
    // If only one package has changes, use its scope
    let changed_packages: Vec<_> = packages.iter().filter(|p| p.has_changes).collect();
    if changed_packages.len() == 1 {
        return Some(changed_packages[0].name.clone());
    }

    // Try to find a common directory
    let common_dir = find_common_directory(files);
    if let Some(dir) = common_dir {
        // Check if this directory matches a known scope
        let dir_name = dir.to_string_lossy().to_lowercase();

        // Check allowed scopes
        for scope in &config.rules.scope.allowed {
            if dir_name.contains(scope) {
                return Some(scope.clone());
            }
        }

        // Use the directory name as scope
        if let Some(name) = dir.file_name() {
            let scope = name.to_string_lossy().to_string();
            if !scope.is_empty() && scope != "src" {
                return Some(scope);
            }
        }
    }

    // Check path-based rules
    for (pattern, rule) in &config.rules.paths {
        if let Some(ref scope) = rule.scope {
            let glob_pattern = glob::Pattern::new(pattern).ok()?;
            for file in files {
                if glob_pattern.matches_path(file) {
                    return Some(scope.clone());
                }
            }
        }
    }

    None
}

/// Find the common directory for a set of files.
fn find_common_directory(files: &[PathBuf]) -> Option<PathBuf> {
    if files.is_empty() {
        return None;
    }

    let first = files[0].parent()?.to_path_buf();
    let mut common = first.clone();

    for file in files.iter().skip(1) {
        if let Some(parent) = file.parent() {
            // Find common prefix
            let mut new_common = PathBuf::new();
            for (a, b) in common.components().zip(parent.components()) {
                if a == b {
                    new_common.push(a.as_os_str());
                } else {
                    break;
                }
            }
            common = new_common;
        }
    }

    if common.as_os_str().is_empty() {
        None
    } else {
        Some(common)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{ChangeType, DiffStats, FileChange};
    use std::collections::HashMap;

    fn make_diff_with_files(paths: &[&str]) -> DiffInfo {
        DiffInfo {
            files: paths
                .iter()
                .map(|p| FileChange {
                    path: PathBuf::from(p),
                    change_type: ChangeType::Modified,
                    lines_added: 10,
                    lines_removed: 5,
                    is_binary: false,
                    old_path: None,
                })
                .collect(),
            stats: DiffStats {
                files_changed: paths.len(),
                lines_added: 10,
                lines_removed: 5,
                binary_files: 0,
            },
            patches: HashMap::new(),
        }
    }

    #[test]
    fn test_infer_type_docs() {
        let diff = make_diff_with_files(&["README.md", "docs/guide.md"]);
        let files: Vec<PathBuf> = diff.files.iter().map(|f| f.path.clone()).collect();
        let inferred = infer_type(&diff, &files);
        assert_eq!(inferred, Some(CommitType::Docs));
    }

    #[test]
    fn test_infer_type_test() {
        let diff = make_diff_with_files(&["tests/test_main.rs"]);
        let files: Vec<PathBuf> = diff.files.iter().map(|f| f.path.clone()).collect();
        let inferred = infer_type(&diff, &files);
        assert_eq!(inferred, Some(CommitType::Test));
    }

    #[test]
    fn test_infer_scope_single_package() {
        let files = vec![PathBuf::from("crates/core/src/lib.rs")];
        let packages = vec![Package {
            path: PathBuf::from("crates/core"),
            name: "core".to_string(),
            has_changes: true,
        }];

        let scope = infer_scope(&files, &packages, &CkConfig::default());
        assert_eq!(scope, Some("core".to_string()));
    }

    #[test]
    fn test_find_common_directory() {
        let files = vec![
            PathBuf::from("src/cli/args.rs"),
            PathBuf::from("src/cli/dispatch.rs"),
        ];

        let common = find_common_directory(&files);
        assert_eq!(common, Some(PathBuf::from("src/cli")));
    }
}
