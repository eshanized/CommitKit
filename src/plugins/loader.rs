// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Plugin loader for WASM plugins.

use crate::error::{CkError, PluginError, Result};
use std::path::{Path, PathBuf};

use super::abi::PluginManifest;

/// Plugin loader.
pub struct PluginLoader {
    plugins_dir: PathBuf,
}

impl PluginLoader {
    /// Create a new plugin loader.
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugins_dir: plugins_dir.into(),
        }
    }

    /// Discover available plugins.
    pub fn discover(&self) -> Result<Vec<PluginInfo>> {
        let mut plugins = Vec::new();

        if !self.plugins_dir.exists() {
            return Ok(plugins);
        }

        for entry in std::fs::read_dir(&self.plugins_dir).map_err(|e| {
            CkError::Plugin(PluginError::LoadFailed {
                name: "discovery".to_string(),
                message: format!("Failed to read plugins directory: {}", e),
            })
        })? {
            let entry = entry.map_err(|e| {
                CkError::Plugin(PluginError::LoadFailed {
                    name: "discovery".to_string(),
                    message: format!("Failed to read directory entry: {}", e),
                })
            })?;

            let path = entry.path();
            if path.is_dir() {
                if let Some(info) = self.load_plugin_info(&path)? {
                    plugins.push(info);
                }
            }
        }

        Ok(plugins)
    }

    /// Load plugin info from a directory.
    fn load_plugin_info(&self, plugin_dir: &Path) -> Result<Option<PluginInfo>> {
        let manifest_path = plugin_dir.join("plugin.toml");
        let wasm_path = plugin_dir.join("plugin.wasm");

        if !manifest_path.exists() {
            return Ok(None);
        }

        let manifest_content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            CkError::Plugin(PluginError::InvalidManifest {
                message: format!("Failed to read manifest: {}", e),
            })
        })?;

        let manifest = PluginManifest::from_toml(&manifest_content).map_err(|e| {
            CkError::Plugin(PluginError::InvalidManifest {
                message: format!("Failed to parse manifest: {}", e),
            })
        })?;

        Ok(Some(PluginInfo {
            name: manifest.name.clone(),
            path: plugin_dir.to_path_buf(),
            manifest,
            has_wasm: wasm_path.exists(),
        }))
    }

    /// Load a plugin by name.
    pub fn load(&self, name: &str) -> Result<LoadedPlugin> {
        let plugin_dir = self.plugins_dir.join(name);
        let info = self.load_plugin_info(&plugin_dir)?.ok_or_else(|| {
            CkError::Plugin(PluginError::NotFound {
                name: name.to_string(),
            })
        })?;

        if !info.has_wasm {
            return Err(CkError::Plugin(PluginError::LoadFailed {
                name: name.to_string(),
                message: "Plugin WASM file not found".to_string(),
            }));
        }

        // Load WASM module
        let wasm_path = plugin_dir.join("plugin.wasm");
        let wasm_bytes = std::fs::read(&wasm_path).map_err(|e| {
            CkError::Plugin(PluginError::LoadFailed {
                name: name.to_string(),
                message: format!("Failed to read WASM: {}", e),
            })
        })?;

        Ok(LoadedPlugin { info, wasm_bytes })
    }
}

/// Information about a discovered plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name.
    pub name: String,
    /// Path to the plugin directory.
    pub path: PathBuf,
    /// Plugin manifest.
    pub manifest: PluginManifest,
    /// Whether the WASM file exists.
    pub has_wasm: bool,
}

/// A loaded plugin ready for execution.
#[derive(Debug)]
pub struct LoadedPlugin {
    /// Plugin info.
    pub info: PluginInfo,
    /// Raw WASM bytes.
    pub wasm_bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_empty() {
        let dir = TempDir::new().unwrap();
        let loader = PluginLoader::new(dir.path());
        let plugins = loader.discover().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_with_plugin() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
name = "test-plugin"
version = "1.0.0"
ck_version = ">=0.1.0"
"#,
        )
        .unwrap();

        let loader = PluginLoader::new(dir.path());
        let plugins = loader.discover().unwrap();

        assert!(!plugins.is_empty());
        assert_eq!(plugins[0].name, "test-plugin");
    }
}
