// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Diff operations for analyzing changes.

use crate::error::{CkError, GitError, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use super::repo::Repository;

/// Statistics about a diff.
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    /// Number of files changed.
    pub files_changed: usize,
    /// Number of lines added.
    pub lines_added: usize,
    /// Number of lines removed.
    pub lines_removed: usize,
    /// Number of binary files changed.
    pub binary_files: usize,
}

impl DiffStats {
    /// Calculate the total number of lines changed.
    pub fn total_lines_changed(&self) -> usize {
        self.lines_added + self.lines_removed
    }

    /// Check if this is an oversized diff.
    pub fn is_oversized(&self, threshold: usize) -> bool {
        self.total_lines_changed() > threshold
    }
}

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChange,
}

/// Information about a changed file.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path to the file.
    pub path: PathBuf,
    /// Type of change.
    pub change_type: ChangeType,
    /// Lines added in this file.
    pub lines_added: usize,
    /// Lines removed in this file.
    pub lines_removed: usize,
    /// Whether this is a binary file.
    pub is_binary: bool,
    /// Old path (for renames).
    pub old_path: Option<PathBuf>,
}

/// Complete diff information.
#[derive(Debug, Clone)]
pub struct DiffInfo {
    /// Per-file change information.
    pub files: Vec<FileChange>,
    /// Aggregate statistics.
    pub stats: DiffStats,
    /// Diff content for semantic analysis.
    pub patches: HashMap<PathBuf, String>,
}

impl DiffInfo {
    /// Create an empty diff info.
    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
            stats: DiffStats::default(),
            patches: HashMap::new(),
        }
    }

    /// Get files matching a glob pattern.
    pub fn files_matching(&self, pattern: &str) -> Vec<&FileChange> {
        let glob_pattern = glob::Pattern::new(pattern).ok();
        self.files
            .iter()
            .filter(|f| {
                if let Some(ref pat) = glob_pattern {
                    pat.matches_path(&f.path)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get all unique directories in the diff.
    pub fn affected_directories(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .files
            .iter()
            .filter_map(|f| f.path.parent().map(|p| p.to_path_buf()))
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }

    /// Check if changes span multiple top-level directories.
    pub fn is_multi_package(&self) -> bool {
        let top_dirs: Vec<_> = self
            .files
            .iter()
            .filter_map(|f| {
                f.path
                    .iter()
                    .next()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .collect();

        let mut unique_top: Vec<_> = top_dirs.clone();
        unique_top.sort();
        unique_top.dedup();
        unique_top.len() > 1
    }
}

/// Get the diff for staged changes.
pub fn get_staged_diff() -> Result<DiffInfo> {
    let repo = Repository::open_current()?;
    get_staged_diff_for_repo(&repo)
}

/// Get staged diff for a specific repository.
fn get_staged_diff_for_repo(repo: &Repository) -> Result<DiffInfo> {
    let head = repo.inner().head().ok();
    let head_tree = head.as_ref().and_then(|h| h.peel_to_tree().ok());

    let diff = repo
        .inner()
        .diff_tree_to_index(head_tree.as_ref(), None, None)
        .map_err(|e| {
            CkError::Git(GitError::DiffFailed {
                message: e.message().to_string(),
            })
        })?;

    parse_diff(&diff)
}

/// Get the diff for a specific commit.
pub fn get_diff(reference: &str) -> Result<DiffInfo> {
    let repo = Repository::open_current()?;
    let commit = repo.get_commit(reference)?;

    // Get the parent commit (if any)
    let parent = commit.parents().next();
    let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());
    let commit_tree = commit.tree().map_err(|e| {
        CkError::Git(GitError::DiffFailed {
            message: e.message().to_string(),
        })
    })?;

    let diff = repo
        .inner()
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)
        .map_err(|e| {
            CkError::Git(GitError::DiffFailed {
                message: e.message().to_string(),
            })
        })?;

    parse_diff(&diff)
}

/// Parse a git2 diff into our DiffInfo structure.
fn parse_diff(diff: &git2::Diff<'_>) -> Result<DiffInfo> {
    let mut files = Vec::new();
    let mut patches = HashMap::new();
    let mut stats = DiffStats::default();

    diff.foreach(
        &mut |delta, _| {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_path_buf())
                .unwrap_or_default();

            let old_path = if delta.status() == git2::Delta::Renamed {
                delta.old_file().path().map(|p| p.to_path_buf())
            } else {
                None
            };

            let change_type = match delta.status() {
                git2::Delta::Added => ChangeType::Added,
                git2::Delta::Deleted => ChangeType::Deleted,
                git2::Delta::Modified => ChangeType::Modified,
                git2::Delta::Renamed => ChangeType::Renamed,
                git2::Delta::Copied => ChangeType::Copied,
                git2::Delta::Typechange => ChangeType::TypeChange,
                _ => ChangeType::Modified,
            };

            let is_binary = delta.new_file().is_binary() || delta.old_file().is_binary();

            if is_binary {
                stats.binary_files += 1;
            }

            files.push(FileChange {
                path,
                change_type,
                lines_added: 0,
                lines_removed: 0,
                is_binary,
                old_path,
            });

            true
        },
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
            match line.origin() {
                '+' => stats.lines_added += 1,
                '-' => stats.lines_removed += 1,
                _ => {}
            }
            true
        }),
    )
    .map_err(|e| {
        CkError::Git(GitError::DiffFailed {
            message: e.message().to_string(),
        })
    })?;

    stats.files_changed = files.len();

    // Get patch content for semantic analysis
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        let content = patches.entry(path).or_insert_with(String::new);
        if let Ok(s) = std::str::from_utf8(line.content()) {
            content.push(line.origin());
            content.push_str(s);
        }
        true
    })
    .ok(); // Ignore errors in patch generation

    Ok(DiffInfo {
        files,
        stats,
        patches,
    })
}

/// Get a summary string for the diff.
pub fn diff_summary(info: &DiffInfo) -> String {
    let mut parts = Vec::new();

    if info.stats.files_changed > 0 {
        parts.push(format!(
            "{} file{} changed",
            info.stats.files_changed,
            if info.stats.files_changed == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    if info.stats.lines_added > 0 {
        parts.push(format!(
            "{} insertion{}",
            info.stats.lines_added,
            if info.stats.lines_added == 1 { "" } else { "s" }
        ));
    }

    if info.stats.lines_removed > 0 {
        parts.push(format!(
            "{} deletion{}",
            info.stats.lines_removed,
            if info.stats.lines_removed == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    if parts.is_empty() {
        "No changes".to_string()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_stats_total() {
        let stats = DiffStats {
            files_changed: 5,
            lines_added: 100,
            lines_removed: 50,
            binary_files: 0,
        };
        assert_eq!(stats.total_lines_changed(), 150);
    }

    #[test]
    fn test_diff_stats_oversized() {
        let stats = DiffStats {
            files_changed: 10,
            lines_added: 300,
            lines_removed: 200,
            binary_files: 0,
        };
        assert!(stats.is_oversized(400));
        assert!(!stats.is_oversized(600));
    }

    #[test]
    fn test_diff_info_empty() {
        let info = DiffInfo::empty();
        assert!(info.files.is_empty());
        assert_eq!(info.stats.files_changed, 0);
    }

    #[test]
    fn test_diff_summary() {
        let info = DiffInfo {
            files: vec![FileChange {
                path: PathBuf::from("test.rs"),
                change_type: ChangeType::Modified,
                lines_added: 10,
                lines_removed: 5,
                is_binary: false,
                old_path: None,
            }],
            stats: DiffStats {
                files_changed: 1,
                lines_added: 10,
                lines_removed: 5,
                binary_files: 0,
            },
            patches: HashMap::new(),
        };

        let summary = diff_summary(&info);
        assert!(summary.contains("1 file changed"));
        assert!(summary.contains("10 insertion"));
        assert!(summary.contains("5 deletion"));
    }
}
