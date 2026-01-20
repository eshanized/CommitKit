// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Configuration schema definitions.
//!
//! Defines all configuration structures that can be loaded from ck.toml.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// The main configuration structure for ck.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CkConfig {
    /// Rule configuration.
    pub rules: RulesConfig,

    /// Monorepo configuration.
    pub monorepo: MonorepoConfig,

    /// Security configuration.
    pub security: SecurityConfig,

    /// Hook configuration.
    pub hooks: HooksConfig,

    /// Plugin configuration.
    pub plugins: PluginsConfig,

    /// UI/UX configuration.
    pub ui: UiConfig,
}

impl CkConfig {
    /// Load configuration from the default locations.
    pub fn load() -> crate::error::Result<Self> {
        super::loader::load_config()
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &std::path::Path) -> crate::error::Result<Self> {
        super::loader::load_config_from(path)
    }
}

/// Rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RulesConfig {
    /// Maximum length of the subject line.
    pub max_subject_length: usize,

    /// Minimum length of the subject line.
    pub min_subject_length: usize,

    /// Whether scope is required.
    pub require_scope: bool,

    /// Whether body is required.
    pub require_body: bool,

    /// Allowed commit types.
    pub allowed_types: Vec<String>,

    /// Forbidden commit types.
    pub forbidden_types: Vec<String>,

    /// Scope configuration.
    pub scope: ScopeConfig,

    /// Path-based rules.
    #[serde(default)]
    pub paths: HashMap<String, PathRuleConfig>,

    /// Branch-based rules.
    #[serde(default)]
    pub branch: HashMap<String, BranchRuleConfig>,

    /// CI-specific rules.
    pub ci: CiRulesConfig,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            max_subject_length: 72,
            min_subject_length: 10,
            require_scope: false,
            require_body: false,
            allowed_types: vec![
                "feat".to_string(),
                "fix".to_string(),
                "docs".to_string(),
                "style".to_string(),
                "refactor".to_string(),
                "perf".to_string(),
                "test".to_string(),
                "chore".to_string(),
                "revert".to_string(),
                "build".to_string(),
                "ci".to_string(),
            ],
            forbidden_types: vec!["wip".to_string()],
            scope: ScopeConfig::default(),
            paths: HashMap::new(),
            branch: HashMap::new(),
            ci: CiRulesConfig::default(),
        }
    }
}

/// Scope configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ScopeConfig {
    /// Whether scope is required.
    pub require: bool,

    /// Allowed scopes (empty means all allowed).
    pub allowed: Vec<String>,

    /// Forbidden scopes.
    pub forbidden: Vec<String>,
}

/// Path-based rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PathRuleConfig {
    /// Suggested commit type for this path.
    #[serde(rename = "type")]
    pub commit_type: Option<String>,

    /// Whether scope is required for this path.
    pub require_scope: Option<bool>,

    /// Suggested scope for this path.
    pub scope: Option<String>,

    /// Whether body is required for this path.
    pub require_body: Option<bool>,
}

/// Branch-based rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BranchRuleConfig {
    /// Forbidden types on this branch.
    pub forbid: Vec<String>,

    /// Allowed types on this branch (overrides forbidden).
    pub allow: Vec<String>,

    /// Whether body is required on this branch.
    pub require_body: Option<bool>,

    /// Whether signed commits are required on this branch.
    pub require_signed: Option<bool>,
}

/// CI-specific rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CiRulesConfig {
    /// Whether to enable strict mode in CI.
    pub strict: bool,

    /// Whether to fail on warnings in CI.
    pub fail_on_warning: bool,
}

impl Default for CiRulesConfig {
    fn default() -> Self {
        Self {
            strict: true,
            fail_on_warning: false,
        }
    }
}

/// Monorepo configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MonorepoConfig {
    /// Whether monorepo support is enabled.
    pub enabled: bool,

    /// Package marker files to detect packages.
    pub package_markers: Vec<String>,

    /// Scope to use for root-level changes.
    pub root_scope: String,

    /// Explicit package definitions.
    pub packages: Vec<PackageConfig>,
}

impl Default for MonorepoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            package_markers: vec![
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "go.mod".to_string(),
                "pyproject.toml".to_string(),
                "pom.xml".to_string(),
            ],
            root_scope: "root".to_string(),
            packages: Vec::new(),
        }
    }
}

/// Package configuration for monorepo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Path to the package.
    pub path: PathBuf,

    /// Scope to use for this package.
    pub scope: String,

    /// Optional name (defaults to scope).
    pub name: Option<String>,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Whether security scanning is enabled.
    pub enabled: bool,

    /// Whether to block commits with detected secrets.
    pub block_on_secret: bool,

    /// Custom secret patterns.
    pub patterns: Vec<SecretPattern>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            block_on_secret: true,
            patterns: Vec::new(),
        }
    }
}

/// Secret pattern definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPattern {
    /// Name of the pattern.
    pub name: String,

    /// Regex pattern to match.
    pub pattern: String,

    /// Optional description.
    pub description: Option<String>,
}

/// Hooks configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HooksConfig {
    /// Whether hooks are enabled.
    pub enabled: bool,

    /// commit-msg hook settings.
    pub commit_msg: HookSettings,

    /// prepare-commit-msg hook settings.
    pub prepare_commit_msg: HookSettings,

    /// pre-push hook settings.
    pub pre_push: HookSettings,
}

/// Settings for a specific hook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HookSettings {
    /// Whether this hook is enabled.
    pub enabled: bool,

    /// Additional arguments to pass to ck.
    pub args: Vec<String>,
}

/// Plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PluginsConfig {
    /// Whether plugins are enabled.
    pub enabled: bool,

    /// Directory containing plugins.
    pub directory: Option<PathBuf>,

    /// List of enabled plugins.
    pub enabled_plugins: Vec<String>,
}

/// UI/UX configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Whether to use colors.
    pub color: bool,

    /// Whether to use emoji.
    pub emoji: bool,

    /// Whether to show hints.
    pub hints: bool,

    /// Theme name.
    pub theme: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: true,
            emoji: true,
            hints: true,
            theme: "default".to_string(),
        }
    }
}

/// Commit type definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitType {
    Feat,
    Fix,
    Docs,
    Style,
    Refactor,
    Perf,
    Test,
    Chore,
    Revert,
    Build,
    Ci,
    Wip,
}

impl CommitType {
    /// Get the string representation of the commit type.
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Docs => "docs",
            CommitType::Style => "style",
            CommitType::Refactor => "refactor",
            CommitType::Perf => "perf",
            CommitType::Test => "test",
            CommitType::Chore => "chore",
            CommitType::Revert => "revert",
            CommitType::Build => "build",
            CommitType::Ci => "ci",
            CommitType::Wip => "wip",
        }
    }

    /// Get a description of the commit type.
    pub fn description(&self) -> &'static str {
        match self {
            CommitType::Feat => "A new feature",
            CommitType::Fix => "A bug fix",
            CommitType::Docs => "Documentation only changes",
            CommitType::Style => "Code style changes (formatting, whitespace)",
            CommitType::Refactor => "Code refactoring (no feature/fix)",
            CommitType::Perf => "Performance improvements",
            CommitType::Test => "Adding or updating tests",
            CommitType::Chore => "Build process or auxiliary tool changes",
            CommitType::Revert => "Reverting a previous commit",
            CommitType::Build => "Build system or dependency changes",
            CommitType::Ci => "CI configuration changes",
            CommitType::Wip => "Work in progress",
        }
    }

    /// Get all commit types.
    pub fn all() -> &'static [CommitType] {
        &[
            CommitType::Feat,
            CommitType::Fix,
            CommitType::Docs,
            CommitType::Style,
            CommitType::Refactor,
            CommitType::Perf,
            CommitType::Test,
            CommitType::Chore,
            CommitType::Revert,
            CommitType::Build,
            CommitType::Ci,
            CommitType::Wip,
        ]
    }
}

impl std::str::FromStr for CommitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "feat" | "feature" => Ok(CommitType::Feat),
            "fix" | "bugfix" => Ok(CommitType::Fix),
            "docs" | "doc" => Ok(CommitType::Docs),
            "style" => Ok(CommitType::Style),
            "refactor" => Ok(CommitType::Refactor),
            "perf" | "performance" => Ok(CommitType::Perf),
            "test" | "tests" => Ok(CommitType::Test),
            "chore" => Ok(CommitType::Chore),
            "revert" => Ok(CommitType::Revert),
            "build" => Ok(CommitType::Build),
            "ci" => Ok(CommitType::Ci),
            "wip" => Ok(CommitType::Wip),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for CommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CkConfig::default();
        assert_eq!(config.rules.max_subject_length, 72);
        assert!(!config.rules.require_scope);
        assert!(config.security.enabled);
    }

    #[test]
    fn test_commit_type_from_str() {
        assert_eq!("feat".parse::<CommitType>(), Ok(CommitType::Feat));
        assert_eq!("FIX".parse::<CommitType>(), Ok(CommitType::Fix));
        assert!("unknown".parse::<CommitType>().is_err());
    }

    #[test]
    fn test_commit_type_display() {
        assert_eq!(CommitType::Feat.to_string(), "feat");
        assert_eq!(CommitType::Refactor.to_string(), "refactor");
    }

    #[test]
    fn test_config_serialization() {
        let config = CkConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("max_subject_length"));
    }
}
