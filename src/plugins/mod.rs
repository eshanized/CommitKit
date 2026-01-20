// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! WASM plugin system.

mod abi;
mod loader;
mod runtime;

pub use abi::{PluginCapability, PluginManifest};
pub use loader::PluginLoader;
pub use runtime::PluginRuntime;
