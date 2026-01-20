// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Repository operations.

use crate::error::{CkError, GitError, Result};
use git2::{Oid, Repository as Git2Repo};
use std::path::{Path, PathBuf};

/// Wrapper around git2::Repository with additional functionality.
pub struct Repository {
    inner: Git2Repo,
    workdir: PathBuf,
}

impl Repository {
    /// Open a repository from the current directory.
    pub fn open_current() -> Result<Self> {
        let current_dir = std::env::current_dir().map_err(|e| {
            CkError::Git(GitError::OpenFailed {
                message: format!("Failed to get current directory: {}", e),
            })
        })?;
        Self::open(&current_dir)
    }

    /// Open a repository from a path.
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Git2Repo::discover(path).map_err(|e| {
            if e.code() == git2::ErrorCode::NotFound {
                CkError::Git(GitError::NotARepository)
            } else {
                CkError::Git(GitError::OpenFailed {
                    message: e.message().to_string(),
                })
            }
        })?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| {
                CkError::Git(GitError::OpenFailed {
                    message: "Repository has no working directory (bare repository)".to_string(),
                })
            })?
            .to_path_buf();

        Ok(Self {
            inner: repo,
            workdir,
        })
    }

    /// Get a reference to the inner git2 repository.
    pub fn inner(&self) -> &Git2Repo {
        &self.inner
    }

    /// Get the working directory path.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Get the current branch name.
    pub fn branch_name(&self) -> Result<String> {
        let head = self.inner.head().map_err(|e| {
            if e.code() == git2::ErrorCode::UnbornBranch {
                CkError::Git(GitError::DetachedHead)
            } else {
                CkError::Git(GitError::BranchFailed {
                    message: e.message().to_string(),
                })
            }
        })?;

        if head.is_branch() {
            let name = head.shorthand().ok_or_else(|| {
                CkError::Git(GitError::BranchFailed {
                    message: "Invalid branch name encoding".to_string(),
                })
            })?;
            Ok(name.to_string())
        } else {
            Err(CkError::Git(GitError::DetachedHead))
        }
    }

    /// Get the HEAD commit.
    pub fn head_commit(&self) -> Result<git2::Commit<'_>> {
        let head = self.inner.head().map_err(|e| {
            CkError::Git(GitError::BranchFailed {
                message: e.message().to_string(),
            })
        })?;

        let commit = head.peel_to_commit().map_err(|e| {
            CkError::Git(GitError::InvalidReference {
                reference: format!("HEAD: {}", e.message()),
            })
        })?;

        Ok(commit)
    }

    /// Get a commit by reference (SHA, branch name, etc.).
    pub fn get_commit(&self, reference: &str) -> Result<git2::Commit<'_>> {
        let obj = self.inner.revparse_single(reference).map_err(|e| {
            CkError::Git(GitError::InvalidReference {
                reference: format!("{}: {}", reference, e.message()),
            })
        })?;

        let commit = obj.peel_to_commit().map_err(|e| {
            CkError::Git(GitError::InvalidReference {
                reference: format!("{}: {}", reference, e.message()),
            })
        })?;

        Ok(commit)
    }

    /// Get the commit message for a reference.
    pub fn get_commit_message(&self, reference: &str) -> Result<String> {
        let commit = self.get_commit(reference)?;
        let message = commit.message().ok_or_else(|| {
            CkError::Git(GitError::InvalidReference {
                reference: format!("{}: Invalid message encoding", reference),
            })
        })?;
        Ok(message.to_string())
    }

    /// Get commits in a range.
    pub fn get_commits_in_range(&self, range: &str) -> Result<Vec<(Oid, String)>> {
        let mut revwalk = self.inner.revwalk().map_err(|e| {
            CkError::Git(GitError::CommandFailed {
                command: "revwalk".to_string(),
                message: e.message().to_string(),
            })
        })?;

        // Parse range specification
        if range.contains("..") {
            let parts: Vec<&str> = range.split("..").collect();
            if parts.len() == 2 {
                let from = self.get_commit(parts[0])?;
                let to = self.get_commit(parts[1])?;

                revwalk.push(to.id()).map_err(|e| {
                    CkError::Git(GitError::CommandFailed {
                        command: "revwalk.push".to_string(),
                        message: e.message().to_string(),
                    })
                })?;
                revwalk.hide(from.id()).map_err(|e| {
                    CkError::Git(GitError::CommandFailed {
                        command: "revwalk.hide".to_string(),
                        message: e.message().to_string(),
                    })
                })?;
            }
        } else {
            // Single reference, get that commit only
            let commit = self.get_commit(range)?;
            return Ok(vec![(
                commit.id(),
                commit.message().unwrap_or("").to_string(),
            )]);
        }

        let mut commits = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result.map_err(|e| {
                CkError::Git(GitError::CommandFailed {
                    command: "revwalk".to_string(),
                    message: e.message().to_string(),
                })
            })?;
            let commit = self.inner.find_commit(oid).map_err(|e| {
                CkError::Git(GitError::InvalidReference {
                    reference: format!("{}: {}", oid, e.message()),
                })
            })?;
            let message = commit.message().unwrap_or("").to_string();
            commits.push((oid, message));
        }

        Ok(commits)
    }

    /// Check if there are staged changes.
    pub fn has_staged_changes(&self) -> Result<bool> {
        let head = self.inner.head().ok();
        let head_tree = head.as_ref().and_then(|h| h.peel_to_tree().ok());

        let diff = self
            .inner
            .diff_tree_to_index(head_tree.as_ref(), None, None)
            .map_err(|e| {
                CkError::Git(GitError::DiffFailed {
                    message: e.message().to_string(),
                })
            })?;

        Ok(diff.stats().map(|s| s.files_changed() > 0).unwrap_or(false))
    }

    /// Get the git directory path (.git).
    pub fn git_dir(&self) -> &Path {
        self.inner.path()
    }
}

/// Open the repository from the current directory.
pub fn open_repo() -> Result<Repository> {
    Repository::open_current()
}

/// Check if the current directory is within a git repository.
pub fn is_git_repo() -> bool {
    Repository::open_current().is_ok()
}

/// Get the current branch name.
pub fn get_branch_name() -> Result<String> {
    let repo = Repository::open_current()?;
    repo.branch_name()
}

/// Get the HEAD commit OID.
pub fn get_head_commit() -> Result<String> {
    let repo = Repository::open_current()?;
    let commit = repo.head_commit()?;
    Ok(commit.id().to_string())
}

/// Get the commit message for a reference.
pub fn get_commit_message(reference: &str) -> Result<String> {
    let repo = Repository::open_current()?;
    repo.get_commit_message(reference)
}

/// Get commits in a range.
pub fn get_commit_range(range: &str) -> Result<Vec<(String, String)>> {
    let repo = Repository::open_current()?;
    let commits = repo.get_commits_in_range(range)?;
    Ok(commits
        .into_iter()
        .map(|(oid, msg)| (oid.to_string(), msg))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Git2Repo::init(dir.path()).unwrap();

        // Create initial commit
        {
            let sig = repo.signature().unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        let wrapper = Repository::open(dir.path()).unwrap();
        (dir, wrapper)
    }

    #[test]
    fn test_open_repo() {
        let (dir, _repo) = create_test_repo();
        assert!(Repository::open(dir.path()).is_ok());
    }

    #[test]
    fn test_not_a_repo() {
        let dir = TempDir::new().unwrap();
        let result = Repository::open(dir.path());
        assert!(matches!(
            result,
            Err(CkError::Git(GitError::NotARepository))
        ));
    }

    #[test]
    fn test_branch_name() {
        let (_dir, repo) = create_test_repo();
        // Default branch might be master or main depending on git config
        let branch = repo.branch_name().unwrap();
        assert!(!branch.is_empty());
    }
}
