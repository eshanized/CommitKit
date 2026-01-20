// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Repository context analysis module.
//!
//! This module analyzes repository state to provide intelligent suggestions.

mod context;
pub mod diff;
mod inference;
mod warnings;

pub use context::RepositoryContext;
pub use diff::DiffAnalysis;
pub use inference::{infer_scope, infer_type, CommitTypeScore};
pub use warnings::{Warning, WarningLevel, Warnings};
