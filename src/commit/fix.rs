// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit fixing functionality.

use crate::error::Result;

/// Commit fixer for interactive commit repair.
pub struct CommitFixer;

impl CommitFixer {
    /// Create a new commit fixer.
    pub fn new() -> Self {
        Self
    }

    /// Fix commits in a range.
    pub fn fix(&self, target: &str, count: Option<usize>, dry_run: bool, auto: bool) -> Result<()> {
        // Determine the actual range
        let range = if let Some(n) = count {
            format!("HEAD~{}..HEAD", n)
        } else if target.contains("..") {
            target.to_string()
        } else {
            format!("{}~1..{}", target, target)
        };

        // Get commits in range
        let commits = crate::git::get_commit_range(&range)?;

        if commits.is_empty() {
            println!("No commits to fix");
            return Ok(());
        }

        println!("Found {} commit(s) to analyze", commits.len());

        for (sha, message) in &commits {
            let short_sha = &sha[..7.min(sha.len())];
            let first_line = message.lines().next().unwrap_or("");

            // Try to parse and validate
            match crate::commit::CommitMessage::parse(message) {
                Ok(parsed) => {
                    let engine = crate::rules::RuleEngine::new(
                        crate::config::CkConfig::load().unwrap_or_default(),
                    );
                    let result = engine.validate(&parsed);

                    if result.is_valid() {
                        println!("✓ {} {}", short_sha, first_line);
                    } else {
                        println!("✗ {} {}", short_sha, first_line);
                        for error in &result.errors {
                            println!("  → {}", error.message);
                            if let Some(ref suggestion) = error.suggestion {
                                println!("    Suggestion: {}", suggestion);
                            }
                        }

                        if !dry_run && auto {
                            // Auto-fix logic would go here
                            println!("  [auto-fix not yet implemented]");
                        }
                    }
                }
                Err(e) => {
                    println!("✗ {} {} (parse error: {})", short_sha, first_line, e);
                }
            }
        }

        if dry_run {
            println!("\n[dry-run] No changes made");
        }

        Ok(())
    }
}

impl Default for CommitFixer {
    fn default() -> Self {
        Self::new()
    }
}
