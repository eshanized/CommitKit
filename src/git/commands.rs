// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Git command wrappers for staging and committing.

use crate::error::{CkError, GitError, Result};
use std::path::Path;
use std::process::Command;

use super::repo::Repository;

/// Stage all modified and deleted files.
pub fn stage_all() -> Result<()> {
    let repo = Repository::open_current()?;
    let mut index = repo.inner().index().map_err(|e| {
        CkError::Git(GitError::CommandFailed {
            command: "index".to_string(),
            message: e.message().to_string(),
        })
    })?;

    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| {
            CkError::Git(GitError::CommandFailed {
                command: "add all".to_string(),
                message: e.message().to_string(),
            })
        })?;

    index.write().map_err(|e| {
        CkError::Git(GitError::CommandFailed {
            command: "write index".to_string(),
            message: e.message().to_string(),
        })
    })?;

    Ok(())
}

/// Stage specific files.
pub fn stage_files(paths: &[&Path]) -> Result<()> {
    let repo = Repository::open_current()?;
    stage_files_in_repo(&repo, paths)
}

/// Stage specific files in a given repository.
pub fn stage_files_in_repo(repo: &Repository, paths: &[&Path]) -> Result<()> {
    let mut index = repo.inner().index().map_err(|e| {
        CkError::Git(GitError::CommandFailed {
            command: "index".to_string(),
            message: e.message().to_string(),
        })
    })?;

    for path in paths {
        // Make path relative to workdir
        let relative_path = if path.is_absolute() {
            path.strip_prefix(repo.workdir()).unwrap_or(path)
        } else {
            path
        };

        index.add_path(relative_path).map_err(|e| {
            CkError::Git(GitError::CommandFailed {
                command: format!("add {}", path.display()),
                message: e.message().to_string(),
            })
        })?;
    }

    index.write().map_err(|e| {
        CkError::Git(GitError::CommandFailed {
            command: "write index".to_string(),
            message: e.message().to_string(),
        })
    })?;

    Ok(())
}

/// Create a commit with the given message.
pub fn create_commit(message: &str, sign: bool) -> Result<String> {
    let repo = Repository::open_current()?;

    // Check for staged changes
    if !repo.has_staged_changes()? {
        return Err(CkError::Git(GitError::NoStagedChanges));
    }

    // Get signature
    let sig = repo.inner().signature().map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to get signature: {}", e.message()),
        })
    })?;

    // Get the tree from the index
    let mut index = repo.inner().index().map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to get index: {}", e.message()),
        })
    })?;
    let tree_id = index.write_tree().map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to write tree: {}", e.message()),
        })
    })?;
    let tree = repo.inner().find_tree(tree_id).map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to find tree: {}", e.message()),
        })
    })?;

    // Get parent commits
    let parents: Vec<git2::Commit<'_>> = if let Ok(head) = repo.head_commit() {
        vec![head]
    } else {
        vec![] // Initial commit, no parents
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

    // Create the commit
    if sign {
        // Use git command for signed commits as git2 signing is complex
        create_commit_with_git(message, sign)?;
        let new_head = repo.head_commit()?;
        Ok(new_head.id().to_string())
    } else {
        let commit_oid = repo
            .inner()
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .map_err(|e| {
                CkError::Git(GitError::CommitFailed {
                    message: e.message().to_string(),
                })
            })?;

        Ok(commit_oid.to_string())
    }
}

/// Create a commit using the git command (for signing support).
fn create_commit_with_git(message: &str, sign: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("commit");
    cmd.arg("-m").arg(message);

    if sign {
        cmd.arg("-S");
    }

    let output = cmd.output().map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to run git commit: {}", e),
        })
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CkError::Git(GitError::CommitFailed {
            message: stderr.to_string(),
        }));
    }

    Ok(())
}

/// Amend the last commit with a new message.
pub fn amend_commit(message: &str, sign: bool) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("commit");
    cmd.arg("--amend");
    cmd.arg("-m").arg(message);

    if sign {
        cmd.arg("-S");
    }

    let output = cmd.output().map_err(|e| {
        CkError::Git(GitError::CommitFailed {
            message: format!("Failed to run git commit --amend: {}", e),
        })
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CkError::Git(GitError::CommitFailed {
            message: stderr.to_string(),
        }));
    }

    // Return the new commit SHA
    let repo = Repository::open_current()?;
    let new_head = repo.head_commit()?;
    Ok(new_head.id().to_string())
}

/// Check if a commit is signed.
pub fn is_commit_signed(reference: &str) -> Result<bool> {
    let repo = Repository::open_current()?;
    let commit = repo.get_commit(reference)?;

    // git2 returns the signature if present
    let signature = repo.inner().extract_signature(&commit.id(), None);
    Ok(signature.is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo_with_file() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();

        // Initialize with git command to ensure proper setup
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Create a file
        fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        (dir, repo)
    }

    #[test]
    fn test_stage_files() {
        let (dir, repo) = create_test_repo_with_file();
        let test_file = dir.path().join("test.txt");

        stage_files_in_repo(&repo, &[test_file.as_path()]).unwrap();

        assert!(repo.has_staged_changes().unwrap());
    }
}
