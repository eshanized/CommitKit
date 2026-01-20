// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Configuration loading and merging.

use crate::error::{CkError, ConfigError, Result};
use std::path::{Path, PathBuf};

use super::schema::CkConfig;

/// Configuration file names to search for, in order of priority.
const CONFIG_FILES: &[&str] = &["ck.toml", ".ck.toml", ".config/ck.toml"];

/// Find the configuration file in the current directory or parent directories.
pub fn find_config_file() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_config_file_from(&current_dir)
}

/// Find the configuration file starting from a specific directory.
pub fn find_config_file_from(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        for config_name in CONFIG_FILES {
            let config_path = current.join(config_name);
            if config_path.exists() {
                return Some(config_path);
            }
        }

        // Try parent directory
        if !current.pop() {
            break;
        }
    }

    // Also check user's home directory
    if let Some(home) = dirs::home_dir() {
        for config_name in CONFIG_FILES {
            let config_path = home.join(config_name);
            if config_path.exists() {
                return Some(config_path);
            }
        }

        // Check XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let ck_config = config_dir.join("ck").join("config.toml");
            if ck_config.exists() {
                return Some(ck_config);
            }
        }
    }

    None
}

/// Load configuration from the default locations.
pub fn load_config() -> Result<CkConfig> {
    match find_config_file() {
        Some(path) => load_config_from(&path),
        None => {
            tracing::debug!("No configuration file found, using defaults");
            Ok(CkConfig::default())
        }
    }
}

/// Load configuration from a specific path.
pub fn load_config_from(path: &Path) -> Result<CkConfig> {
    tracing::debug!("Loading configuration from: {:?}", path);

    if !path.exists() {
        return Err(CkError::Config(ConfigError::NotFound {
            path: path.to_path_buf(),
        }));
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        CkError::Config(ConfigError::ParseError {
            message: format!("Failed to read config file: {}", e),
        })
    })?;

    parse_config(&content)
}

/// Parse configuration from a TOML string.
pub fn parse_config(content: &str) -> Result<CkConfig> {
    toml::from_str(content).map_err(|e| {
        CkError::Config(ConfigError::ParseError {
            message: format!("Failed to parse TOML: {}", e),
        })
    })
}

/// Merge two configurations, with the overlay taking precedence.
pub fn merge_configs(base: CkConfig, overlay: CkConfig) -> CkConfig {
    // For now, we do a simple overlay where non-default values from overlay
    // take precedence. In a more complete implementation, we'd do field-by-field
    // merging with proper defaults detection.
    CkConfig {
        rules: merge_rules_config(base.rules, overlay.rules),
        monorepo: overlay.monorepo,
        security: overlay.security,
        hooks: overlay.hooks,
        plugins: overlay.plugins,
        ui: overlay.ui,
    }
}

fn merge_rules_config(
    base: super::schema::RulesConfig,
    overlay: super::schema::RulesConfig,
) -> super::schema::RulesConfig {
    super::schema::RulesConfig {
        max_subject_length: if overlay.max_subject_length != 72 {
            overlay.max_subject_length
        } else {
            base.max_subject_length
        },
        min_subject_length: if overlay.min_subject_length != 10 {
            overlay.min_subject_length
        } else {
            base.min_subject_length
        },
        require_scope: overlay.require_scope || base.require_scope,
        require_body: overlay.require_body || base.require_body,
        allowed_types: if !overlay.allowed_types.is_empty() {
            overlay.allowed_types
        } else {
            base.allowed_types
        },
        forbidden_types: if !overlay.forbidden_types.is_empty() {
            overlay.forbidden_types
        } else {
            base.forbidden_types
        },
        scope: overlay.scope,
        paths: {
            let mut merged = base.paths;
            merged.extend(overlay.paths);
            merged
        },
        branch: {
            let mut merged = base.branch;
            merged.extend(overlay.branch);
            merged
        },
        ci: overlay.ci,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let config = parse_config("").unwrap();
        assert_eq!(config.rules.max_subject_length, 72);
    }

    #[test]
    fn test_parse_custom_config() {
        let toml = r#"
[rules]
max_subject_length = 50
require_scope = true
allowed_types = ["feat", "fix"]

[security]
enabled = false
"#;
        let config = parse_config(toml).unwrap();
        assert_eq!(config.rules.max_subject_length, 50);
        assert!(config.rules.require_scope);
        assert_eq!(config.rules.allowed_types, vec!["feat", "fix"]);
        assert!(!config.security.enabled);
    }

    #[test]
    fn test_parse_path_rules() {
        let toml = r#"
[rules.paths]
"src/core/**" = { type = "feat", require_scope = true, scope = "core" }
"docs/**" = { type = "docs" }
"#;
        let config = parse_config(toml).unwrap();
        assert!(config.rules.paths.contains_key("src/core/**"));
        assert!(config.rules.paths.contains_key("docs/**"));
    }

    #[test]
    fn test_parse_branch_rules() {
        let toml = r#"
[rules.branch]
"main" = { forbid = ["wip"], require_body = true }
"release/*" = { require_signed = true }
"#;
        let config = parse_config(toml).unwrap();
        assert!(config.rules.branch.contains_key("main"));

        let main_rules = &config.rules.branch["main"];
        assert_eq!(main_rules.forbid, vec!["wip"]);
        assert_eq!(main_rules.require_body, Some(true));
    }

    #[test]
    fn test_parse_monorepo_config() {
        let toml = r#"
[monorepo]
enabled = true
root_scope = "workspace"

[[monorepo.packages]]
path = "crates/core"
scope = "core"

[[monorepo.packages]]
path = "crates/cli"
scope = "cli"
"#;
        let config = parse_config(toml).unwrap();
        assert!(config.monorepo.enabled);
        assert_eq!(config.monorepo.root_scope, "workspace");
        assert_eq!(config.monorepo.packages.len(), 2);
    }

    #[test]
    fn test_merge_configs() {
        let base = CkConfig::default();
        let overlay_toml = r#"
[rules]
max_subject_length = 50
require_scope = true
"#;
        let overlay = parse_config(overlay_toml).unwrap();
        let merged = merge_configs(base, overlay);

        assert_eq!(merged.rules.max_subject_length, 50);
        assert!(merged.rules.require_scope);
    }
}
