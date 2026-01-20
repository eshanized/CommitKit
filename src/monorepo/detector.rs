// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Package detection for monorepos.

use crate::config::CkConfig;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Information about a detected package.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Path to the package root.
    pub path: PathBuf,
    /// Package name (from manifest or directory name).
    pub name: String,
    /// The scope to use for commits.
    pub scope: String,
    /// Package type/marker that was detected.
    pub marker: String,
}

/// Detect packages in a repository.
pub fn detect_packages(root: &Path, config: &CkConfig) -> Vec<PackageInfo> {
    if !config.monorepo.enabled {
        return Vec::new();
    }

    let mut packages = Vec::new();
    let mut seen_paths = HashSet::new();

    // First add explicitly configured packages
    for pkg in &config.monorepo.packages {
        let full_path = root.join(&pkg.path);
        if full_path.exists() {
            packages.push(PackageInfo {
                path: full_path.clone(),
                name: pkg.name.clone().unwrap_or_else(|| pkg.scope.clone()),
                scope: pkg.scope.clone(),
                marker: "configured".to_string(),
            });
            seen_paths.insert(full_path);
        }
    }

    // Then auto-detect packages
    for marker in &config.monorepo.package_markers {
        for entry in WalkDir::new(root)
            .max_depth(4) // Don't go too deep
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories and common non-package directories
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "vendor"
                    && name != "dist"
                    && name != "build"
            })
            .flatten()
        {
            if entry.file_name().to_string_lossy() == *marker {
                if let Some(parent) = entry.path().parent() {
                    let parent_path = parent.to_path_buf();

                    // Skip if already seen or is the root
                    if seen_paths.contains(&parent_path) || parent_path == root {
                        continue;
                    }

                    // Extract package name
                    let name = extract_package_name(entry.path());
                    let scope = parent
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| name.clone());

                    packages.push(PackageInfo {
                        path: parent_path.clone(),
                        name,
                        scope,
                        marker: marker.clone(),
                    });
                    seen_paths.insert(parent_path);
                }
            }
        }
    }

    packages
}

/// Extract package name from a manifest file.
fn extract_package_name(manifest_path: &Path) -> String {
    let file_name = manifest_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // Try to read the manifest and extract the name
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        match file_name {
            "Cargo.toml" => {
                // Parse TOML and extract package.name
                if let Ok(toml) = toml::from_str::<toml::Value>(&content) {
                    if let Some(name) = toml
                        .get("package")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        return name.to_string();
                    }
                }
            }
            "package.json" => {
                // Parse JSON and extract name
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                        return name.to_string();
                    }
                }
            }
            "go.mod" => {
                // Extract module name from first line
                if let Some(first_line) = content.lines().next() {
                    if first_line.starts_with("module ") {
                        let module = first_line.trim_start_matches("module ").trim();
                        // Get last component of module path
                        if let Some(name) = module.split('/').next_back() {
                            return name.to_string();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fallback to directory name
    manifest_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_package_name_cargo() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"
[package]
name = "my-package"
version = "0.1.0"
"#,
        )
        .unwrap();

        let name = extract_package_name(&cargo_toml);
        assert_eq!(name, "my-package");
    }

    #[test]
    fn test_extract_package_name_npm() {
        let dir = TempDir::new().unwrap();
        let package_json = dir.path().join("package.json");
        fs::write(
            &package_json,
            r#"{"name": "@scope/my-package", "version": "1.0.0"}"#,
        )
        .unwrap();

        let name = extract_package_name(&package_json);
        assert_eq!(name, "@scope/my-package");
    }
}
