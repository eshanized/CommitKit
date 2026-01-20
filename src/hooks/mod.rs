// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Git hooks management.

mod manager;
mod templates;

pub use manager::HookManager;
pub use templates::HookTemplate;
