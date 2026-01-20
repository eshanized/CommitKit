// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Smart commit generation module.

mod generator;
mod semantic;

pub use generator::{GeneratedMessage, SmartCommit};
pub use semantic::SemanticAnalyzer;
