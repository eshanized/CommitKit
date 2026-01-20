// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Hook manager for installing and managing git hooks.

use crate::error::{CkError, HookError, Result};
use crate::git;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::templates::HookTemplate;

/// Manager for git hooks.
pub struct HookManager {
    hooks_dir: PathBuf,
}

impl HookManager {
    /// Create a new hook manager for the current repository.
    pub fn new() -> Result<Self> {
        let repo = git::open_repo()?;
        let hooks_dir = repo.git_dir().join("hooks");

        // Ensure hooks directory exists
        if !hooks_dir.exists() {
            fs::create_dir_all(&hooks_dir).map_err(|e| {
                CkError::Hook(HookError::InstallFailed {
                    hook: "all".to_string(),
                    message: format!("Failed to create hooks directory: {}", e),
                })
            })?;
        }

        Ok(Self { hooks_dir })
    }

    /// Install a specific hook.
    pub fn install_hook(&self, hook_name: &str, force: bool) -> Result<()> {
        let template = hook_name.parse::<HookTemplate>().ok().ok_or_else(|| {
            CkError::Hook(HookError::NotFound {
                hook: hook_name.to_string(),
            })
        })?;

        self.install_template(&template, force)
    }

    /// Install all hooks.
    pub fn install_all(&self, force: bool) -> Result<()> {
        for template in HookTemplate::all() {
            self.install_template(template, force)?;
        }
        Ok(())
    }

    /// Install a hook from a template.
    fn install_template(&self, template: &HookTemplate, force: bool) -> Result<()> {
        let hook_path = self.hooks_dir.join(template.filename());
        let backup_path = self
            .hooks_dir
            .join(format!("{}.backup", template.filename()));

        // Check if hook already exists
        if hook_path.exists() && !force {
            // Check if it's our hook
            if !self.is_ck_hook(&hook_path)? {
                return Err(CkError::Hook(HookError::AlreadyExists {
                    hook: template.filename().to_string(),
                }));
            }
        }

        // Backup existing hook if it's not ours
        if hook_path.exists() && !self.is_ck_hook(&hook_path)? {
            fs::rename(&hook_path, &backup_path).map_err(|e| {
                CkError::Hook(HookError::InstallFailed {
                    hook: template.filename().to_string(),
                    message: format!("Failed to backup existing hook: {}", e),
                })
            })?;
        }

        // Generate and write hook
        let script = template.generate();
        fs::write(&hook_path, &script).map_err(|e| {
            CkError::Hook(HookError::InstallFailed {
                hook: template.filename().to_string(),
                message: format!("Failed to write hook: {}", e),
            })
        })?;

        // Make executable
        let mut perms = fs::metadata(&hook_path)
            .map_err(|e| {
                CkError::Hook(HookError::InstallFailed {
                    hook: template.filename().to_string(),
                    message: format!("Failed to get permissions: {}", e),
                })
            })?
            .permissions();

        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).map_err(|e| {
            CkError::Hook(HookError::InstallFailed {
                hook: template.filename().to_string(),
                message: format!("Failed to set permissions: {}", e),
            })
        })?;

        Ok(())
    }

    /// Uninstall a specific hook.
    pub fn uninstall_hook(&self, hook_name: &str) -> Result<()> {
        let template = hook_name.parse::<HookTemplate>().ok().ok_or_else(|| {
            CkError::Hook(HookError::NotFound {
                hook: hook_name.to_string(),
            })
        })?;

        let hook_path = self.hooks_dir.join(template.filename());
        let backup_path = self
            .hooks_dir
            .join(format!("{}.backup", template.filename()));

        if !hook_path.exists() {
            return Ok(()); // Nothing to uninstall
        }

        // Only remove if it's our hook
        if !self.is_ck_hook(&hook_path)? {
            return Err(CkError::Hook(HookError::RemoveFailed {
                hook: hook_name.to_string(),
                message: "Hook was not installed by ck".to_string(),
            }));
        }

        fs::remove_file(&hook_path).map_err(|e| {
            CkError::Hook(HookError::RemoveFailed {
                hook: hook_name.to_string(),
                message: format!("Failed to remove hook: {}", e),
            })
        })?;

        // Restore backup if exists
        if backup_path.exists() {
            fs::rename(&backup_path, &hook_path).ok();
        }

        Ok(())
    }

    /// Uninstall all hooks.
    pub fn uninstall_all(&self) -> Result<()> {
        for template in HookTemplate::all() {
            self.uninstall_hook(template.filename())?;
        }
        Ok(())
    }

    /// Get the status of all hooks.
    pub fn status(&self) -> Result<Vec<(String, bool)>> {
        let mut status = Vec::new();

        for template in HookTemplate::all() {
            let hook_path = self.hooks_dir.join(template.filename());
            let installed = hook_path.exists() && self.is_ck_hook(&hook_path).unwrap_or(false);
            status.push((template.filename().to_string(), installed));
        }

        Ok(status)
    }

    /// Run a hook manually.
    pub fn run_hook(&self, hook_name: &str, args: &[String]) -> Result<()> {
        let template = hook_name.parse::<HookTemplate>().ok().ok_or_else(|| {
            CkError::Hook(HookError::NotFound {
                hook: hook_name.to_string(),
            })
        })?;

        let hook_path = self.hooks_dir.join(template.filename());

        if !hook_path.exists() {
            return Err(CkError::Hook(HookError::NotFound {
                hook: hook_name.to_string(),
            }));
        }

        let output = std::process::Command::new(&hook_path)
            .args(args)
            .output()
            .map_err(|e| {
                CkError::Hook(HookError::ExecutionFailed {
                    hook: hook_name.to_string(),
                    message: format!("Failed to run hook: {}", e),
                })
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CkError::Hook(HookError::ExecutionFailed {
                hook: hook_name.to_string(),
                message: stderr.to_string(),
            }));
        }

        Ok(())
    }

    /// Check if a hook was installed by ck.
    fn is_ck_hook(&self, path: &Path) -> Result<bool> {
        let content = fs::read_to_string(path).map_err(|e| {
            CkError::Hook(HookError::ExecutionFailed {
                hook: path.display().to_string(),
                message: format!("Failed to read hook: {}", e),
            })
        })?;

        Ok(content.contains("CK Git Hook") || content.contains("Generated by ck"))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_is_ck_hook_detection() {
        let content = "#!/bin/sh\n# CK Git Hook\n# Generated by ck v0.1.0\n";
        assert!(content.contains("CK Git Hook"));
    }
}
