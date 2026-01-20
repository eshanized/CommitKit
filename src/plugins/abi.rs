// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Plugin ABI definitions.

use serde::{Deserialize, Serialize};

/// Plugin manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name.
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Minimum ck version required.
    pub ck_version: String,
    /// Plugin description.
    pub description: Option<String>,
    /// Author information.
    pub author: Option<String>,
    /// Required permissions.
    #[serde(default)]
    pub permissions: Vec<PluginCapability>,
}

impl PluginManifest {
    /// Parse a manifest from TOML.
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Check if the plugin is compatible with a ck version.
    pub fn is_compatible(&self, ck_version: &str) -> bool {
        // Simple version check - just compare major.minor
        // A more complete implementation would use semver
        let required = parse_version_req(&self.ck_version);
        let current = parse_version(ck_version);

        match (required, current) {
            (Some((req_major, req_minor)), Some((cur_major, cur_minor))) => {
                cur_major > req_major || (cur_major == req_major && cur_minor >= req_minor)
            }
            _ => false,
        }
    }
}

/// Plugin capability/permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Read configuration.
    ReadConfig,
    /// Network access.
    Network,
    /// File system read.
    FsRead,
    /// File system write.
    FsWrite,
    /// Git operations.
    GitRead,
    /// Environment variables.
    Env,
}

impl PluginCapability {
    /// Get all capabilities.
    pub fn all() -> &'static [PluginCapability] {
        &[
            PluginCapability::ReadConfig,
            PluginCapability::Network,
            PluginCapability::FsRead,
            PluginCapability::FsWrite,
            PluginCapability::GitRead,
            PluginCapability::Env,
        ]
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            PluginCapability::ReadConfig => "Read CK configuration",
            PluginCapability::Network => "Make network requests",
            PluginCapability::FsRead => "Read files from disk",
            PluginCapability::FsWrite => "Write files to disk",
            PluginCapability::GitRead => "Read git repository data",
            PluginCapability::Env => "Access environment variables",
        }
    }
}

/// Parse a version string like "0.1.0".
fn parse_version(version: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some((major, minor))
    } else {
        None
    }
}

/// Parse a version requirement like ">=0.1.0".
fn parse_version_req(req: &str) -> Option<(u32, u32)> {
    let version = req
        .trim_start_matches(">=")
        .trim_start_matches(">")
        .trim_start_matches("=")
        .trim_start_matches("^")
        .trim_start_matches("~");
    parse_version(version)
}

/// Plugin ABI version.
#[allow(dead_code)]
pub const ABI_VERSION: u32 = 1;

/// Function signatures for plugin exports.
pub mod exports {
    /// Initialize the plugin.
    #[allow(dead_code)]
    pub const INIT: &str = "ck_plugin_init";
    /// Get plugin metadata.
    #[allow(dead_code)]
    pub const METADATA: &str = "ck_plugin_metadata";
    /// Validate a commit message.
    #[allow(dead_code)]
    pub const VALIDATE: &str = "ck_plugin_validate";
    /// Generate a commit message.
    #[allow(dead_code)]
    pub const GENERATE: &str = "ck_plugin_generate";
    /// Clean up the plugin.
    #[allow(dead_code)]
    pub const CLEANUP: &str = "ck_plugin_cleanup";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1)));
        assert_eq!(parse_version("1.2.3"), Some((1, 2)));
        assert_eq!(parse_version("invalid"), None);
    }

    #[test]
    fn test_version_compatibility() {
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            ck_version: ">=0.1.0".to_string(),
            description: None,
            author: None,
            permissions: vec![],
        };

        assert!(manifest.is_compatible("0.1.0"));
        assert!(manifest.is_compatible("0.2.0"));
        assert!(manifest.is_compatible("1.0.0"));
        assert!(!manifest.is_compatible("0.0.9"));
    }

    #[test]
    fn test_manifest_from_toml() {
        let toml = r#"
name = "test-plugin"
version = "1.0.0"
ck_version = ">=0.1.0"
description = "A test plugin"
permissions = ["read_config", "network"]
"#;

        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.permissions.len(), 2);
    }
}
