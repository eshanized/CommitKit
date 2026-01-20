// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Git integration module.
//!
//! This module provides high-level Git operations for ck.

pub mod commands;
pub mod diff;
mod repo;

pub use commands::{create_commit, stage_all, stage_files};
pub use diff::{get_diff, get_staged_diff, ChangeType, DiffInfo, DiffStats, FileChange};
pub use repo::{
    get_branch_name, get_commit_message, get_commit_range, get_head_commit, is_git_repo, open_repo,
    Repository,
};
