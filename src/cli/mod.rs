// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! CLI module for ck.
//!
//! This module handles command-line argument parsing and command dispatch.

pub mod args;
mod dispatch;

pub use args::{Cli, Commands};
pub use dispatch::run;
