// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Scope resolution for monorepos.

use crate::config::CkConfig;
use std::path::{Path, PathBuf};

use super::detector::{detect_packages, PackageInfo};

/// Scope resolver for monorepo commits.
pub struct ScopeResolver {
    packages: Vec<PackageInfo>,
    root_scope: String,
}

impl ScopeResolver {
    /// Create a new scope resolver.
    pub fn new(root: &Path, config: &CkConfig) -> Self {
        let packages = detect_packages(root, config);
        let root_scope = config.monorepo.root_scope.clone();

        Self {
            packages,
            root_scope,
        }
    }

    /// Resolve the scope for a set of files.
    pub fn resolve(&self, files: &[PathBuf]) -> Option<String> {
        if files.is_empty() {
            return None;
        }

        // Find which packages the files belong to
        let mut package_scopes: Vec<&str> = Vec::new();

        for file in files {
            if let Some(pkg) = self.find_package_for_file(file) {
                if !package_scopes.contains(&&pkg.scope[..]) {
                    package_scopes.push(&pkg.scope);
                }
            }
        }

        match package_scopes.len() {
            0 => {
                // No package matched - use root scope or try common dir
                if let Some(common_path) = find_common_prefix(files) {
                    if let Some(name) = common_path.file_name() {
                        if let Some(s) = name.to_str() {
                            return Some(s.to_string());
                        }
                    }
                }
                Some(self.root_scope.clone())
            }
            1 => Some(package_scopes[0].to_string()),
            _ => {
                // Multiple packages - no single scope
                None
            }
        }
    }

    /// Check if files span multiple packages.
    pub fn is_multi_package(&self, files: &[PathBuf]) -> bool {
        let mut seen_scopes = std::collections::HashSet::new();

        for file in files {
            if let Some(pkg) = self.find_package_for_file(file) {
                seen_scopes.insert(&pkg.scope);
            }
        }

        seen_scopes.len() > 1
    }

    /// Get all packages that have changes.
    pub fn changed_packages(&self, files: &[PathBuf]) -> Vec<&PackageInfo> {
        let mut result = Vec::new();

        for pkg in &self.packages {
            if files.iter().any(|f| f.starts_with(&pkg.path)) {
                result.push(pkg);
            }
        }

        result
    }

    /// Find the package that contains a file.
    fn find_package_for_file(&self, file: &Path) -> Option<&PackageInfo> {
        // Find the most specific (deepest) package that contains this file
        self.packages
            .iter()
            .filter(|pkg| file.starts_with(&pkg.path))
            .max_by_key(|pkg| pkg.path.components().count())
    }
}

/// Resolve scope for a set of files.
pub fn resolve_scope(files: &[PathBuf], root: &Path, config: &CkConfig) -> Option<String> {
    let resolver = ScopeResolver::new(root, config);
    resolver.resolve(files)
}

/// Find the common prefix path for a set of files.
fn find_common_prefix(files: &[PathBuf]) -> Option<PathBuf> {
    if files.is_empty() {
        return None;
    }

    let first = &files[0];
    let mut common: Vec<_> = first.components().collect();

    for file in files.iter().skip(1) {
        let components: Vec<_> = file.components().collect();
        let min_len = common.len().min(components.len());

        common.truncate(
            (0..min_len)
                .take_while(|&i| common[i] == components[i])
                .count(),
        );
    }

    if common.is_empty() {
        None
    } else {
        Some(common.iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_common_prefix() {
        let files = vec![
            PathBuf::from("src/core/mod.rs"),
            PathBuf::from("src/core/lib.rs"),
        ];

        let common = find_common_prefix(&files);
        assert_eq!(common, Some(PathBuf::from("src/core")));
    }

    #[test]
    fn test_find_common_prefix_different() {
        let files = vec![
            PathBuf::from("src/core/mod.rs"),
            PathBuf::from("tests/test.rs"),
        ];

        let common = find_common_prefix(&files);
        assert!(common.is_none() || common == Some(PathBuf::new()));
    }
}
