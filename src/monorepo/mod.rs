// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Monorepo support module.

mod detector;
mod scope;

pub use detector::{detect_packages, PackageInfo};
pub use scope::{resolve_scope, ScopeResolver};
