// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Configuration module for ck.
//!
//! This module handles loading, parsing, and merging configuration from
//! various sources (files, environment variables, defaults).

pub mod default;
mod loader;
mod schema;

pub use default::default_config;
pub use loader::{find_config_file, load_config, merge_configs};
pub use schema::*;
