// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Repository context for intelligent commit assistance.

use crate::config::{CkConfig, CommitType};
use crate::error::Result;
use crate::git::{self, DiffInfo, DiffStats};
use std::path::PathBuf;

use super::inference::{infer_scope, infer_type};
use super::warnings::{generate_warnings, Warnings};

/// Complete context about the repository state for commit assistance.
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    /// Files that are staged for commit.
    pub staged_files: Vec<PathBuf>,
    /// Files with unstaged changes.
    pub unstaged_files: Vec<PathBuf>,
    /// Statistics about the staged diff.
    pub diff_stats: DiffStats,
    /// Current branch name.
    pub branch: String,
    /// Detected packages (for monorepo support).
    pub packages: Vec<Package>,
    /// Suggested commit type based on diff analysis.
    pub suggested_type: Option<CommitType>,
    /// Suggested scope based on file paths.
    pub suggested_scope: Option<String>,
    /// Warnings about the current commit.
    pub warnings: Warnings,
    /// Raw diff information.
    pub diff_info: DiffInfo,
}

/// Package information for monorepo support.
#[derive(Debug, Clone)]
pub struct Package {
    /// Path to the package root.
    pub path: PathBuf,
    /// Package name/scope.
    pub name: String,
    /// Whether this package has changes.
    pub has_changes: bool,
}

impl RepositoryContext {
    /// Build context from the current repository state.
    pub fn from_current_repo() -> Result<Self> {
        Self::from_current_repo_with_config(&CkConfig::default())
    }

    /// Build context with custom configuration.
    pub fn from_current_repo_with_config(config: &CkConfig) -> Result<Self> {
        // Get branch name
        let branch = git::get_branch_name().unwrap_or_else(|_| "HEAD".to_string());

        // Get staged diff
        let diff_info = git::get_staged_diff()?;

        // Extract file lists
        let staged_files: Vec<PathBuf> = diff_info.files.iter().map(|f| f.path.clone()).collect();

        // TODO: Get unstaged files - for now return empty
        let unstaged_files = Vec::new();

        // Detect packages
        let packages = detect_packages(&staged_files, config);

        // Infer type and scope
        let suggested_type = infer_type(&diff_info, &staged_files);
        let suggested_scope = infer_scope(&staged_files, &packages, config);

        // Generate warnings
        let warnings = generate_warnings(&diff_info, &staged_files, &packages, config);

        Ok(Self {
            staged_files,
            unstaged_files,
            diff_stats: diff_info.stats.clone(),
            branch,
            packages,
            suggested_type,
            suggested_scope,
            warnings,
            diff_info,
        })
    }

    /// Check if there are any staged changes.
    pub fn has_staged_changes(&self) -> bool {
        !self.staged_files.is_empty()
    }

    /// Get a human-readable summary of the context.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("Branch: {}", self.branch));
        parts.push(format!("Files: {}", self.staged_files.len()));
        parts.push(format!(
            "Changes: +{} -{}",
            self.diff_stats.lines_added, self.diff_stats.lines_removed
        ));

        if let Some(ref t) = self.suggested_type {
            parts.push(format!("Suggested type: {}", t));
        }

        if let Some(ref s) = self.suggested_scope {
            parts.push(format!("Suggested scope: {}", s));
        }

        if !self.warnings.is_empty() {
            parts.push(format!("Warnings: {}", self.warnings.len()));
        }

        parts.join(" | ")
    }
}

/// Detect packages in the changed files.
fn detect_packages(files: &[PathBuf], config: &CkConfig) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    // First, add explicitly configured packages
    for pkg_config in &config.monorepo.packages {
        let has_changes = files.iter().any(|f| f.starts_with(&pkg_config.path));
        packages.push(Package {
            path: pkg_config.path.clone(),
            name: pkg_config.scope.clone(),
            has_changes,
        });
        seen_paths.insert(pkg_config.path.clone());
    }

    // Then, auto-detect packages from markers
    if config.monorepo.enabled {
        for file in files {
            // Walk up the directory tree looking for package markers
            let mut current = file.parent();
            while let Some(dir) = current {
                if !seen_paths.contains(&dir.to_path_buf()) {
                    for marker in &config.monorepo.package_markers {
                        let marker_path = dir.join(marker);
                        if marker_path.exists() {
                            let name = dir
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| config.monorepo.root_scope.clone());

                            packages.push(Package {
                                path: dir.to_path_buf(),
                                name,
                                has_changes: true,
                            });
                            seen_paths.insert(dir.to_path_buf());
                            break;
                        }
                    }
                }
                current = dir.parent();
            }
        }
    }

    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_detection() {
        let files = vec![
            PathBuf::from("crates/core/src/lib.rs"),
            PathBuf::from("crates/cli/src/main.rs"),
        ];

        let mut config = CkConfig::default();
        config.monorepo.packages.push(crate::config::PackageConfig {
            path: PathBuf::from("crates/core"),
            scope: "core".to_string(),
            name: None,
        });

        let packages = detect_packages(&files, &config);
        assert!(!packages.is_empty());
    }

    #[test]
    fn test_context_summary() {
        let ctx = RepositoryContext {
            staged_files: vec![PathBuf::from("test.rs")],
            unstaged_files: vec![],
            diff_stats: DiffStats {
                files_changed: 1,
                lines_added: 10,
                lines_removed: 5,
                binary_files: 0,
            },
            branch: "main".to_string(),
            packages: vec![],
            suggested_type: Some(CommitType::Feat),
            suggested_scope: Some("core".to_string()),
            warnings: Warnings::new(),
            diff_info: DiffInfo::empty(),
        };

        let summary = ctx.summary();
        assert!(summary.contains("main"));
        assert!(summary.contains("feat"));
        assert!(summary.contains("core"));
    }
}
