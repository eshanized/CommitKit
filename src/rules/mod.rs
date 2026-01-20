// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Rule engine module for commit validation.
//!
//! This module provides a configurable rule engine for validating
//! commit messages against a set of rules.

mod builtin;
mod engine;
mod validator;

pub use builtin::*;
pub use engine::RuleEngine;
pub use validator::{ValidationIssue, ValidationResult};
