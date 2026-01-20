// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Default configuration values.

use super::schema::CkConfig;

/// Get the default configuration.
pub fn default_config() -> CkConfig {
    CkConfig::default()
}

/// Generate an example configuration file.
pub fn example_config() -> &'static str {
    r#"# CK Configuration File
# Author: Eshan Roy
# SPDX-License-Identifier: MIT

# Rule configuration
[rules]
max_subject_length = 72
min_subject_length = 10
require_scope = true
require_body = false
allowed_types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "revert", "build", "ci"]
forbidden_types = ["wip"]

# Scope configuration
[rules.scope]
require = true
allowed = ["core", "cli", "config", "git", "rules", "hooks"]

# Path-based rules
[rules.paths]
"src/core/**" = { type = "feat", require_scope = true, scope = "core" }
"src/cli/**" = { type = "feat", scope = "cli" }
"docs/**" = { type = "docs", require_body = false }
"tests/**" = { type = "test" }

# Branch-based rules
[rules.branch]
"main" = { forbid = ["wip", "fixup"], require_body = true }
"release/*" = { forbid = ["wip"], require_signed = true }
"feature/*" = { allow = ["wip"] }

# CI-specific rules
[rules.ci]
strict = true
fail_on_warning = true

# Monorepo configuration
[monorepo]
enabled = true
package_markers = ["Cargo.toml", "package.json", "go.mod"]
root_scope = "root"

[[monorepo.packages]]
path = "crates/core"
scope = "core"

[[monorepo.packages]]
path = "crates/cli"
scope = "cli"

# Security configuration
[security]
enabled = true
block_on_secret = true

[[security.patterns]]
name = "AWS Access Key"
pattern = "AKIA[0-9A-Z]{16}"

[[security.patterns]]
name = "Generic API Key"
pattern = "(?i)(api[_-]?key|apikey)\\s*[:=]\\s*['\"]?[a-zA-Z0-9]{16,}['\"]?"

[[security.patterns]]
name = "Private Key"
pattern = "-----BEGIN (RSA|DSA|EC|OPENSSH|PGP) PRIVATE KEY-----"

# Hook configuration
[hooks]
enabled = true

[hooks.commit_msg]
enabled = true

[hooks.prepare_commit_msg]
enabled = false

[hooks.pre_push]
enabled = true

# Plugin configuration
[plugins]
enabled = false
directory = ".ck/plugins"
enabled_plugins = []

# UI configuration
[ui]
color = true
emoji = true
hints = true
theme = "default"
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.rules.max_subject_length, 72);
        assert!(config.security.enabled);
    }

    #[test]
    fn test_example_config_parseable() {
        let example = example_config();
        let _config: CkConfig = toml::from_str(example).expect("Example config should parse");
    }
}
