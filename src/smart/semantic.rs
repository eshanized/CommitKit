// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Semantic analysis for smart commit generation.

use crate::analysis::{DiffAnalysis, RepositoryContext};
use crate::config::CommitType;
use crate::git::DiffInfo;
use std::collections::HashSet;

/// Semantic analyzer for understanding code changes.
pub struct SemanticAnalyzer {
    diff_analysis: DiffAnalysis,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer from a diff.
    pub fn from_diff(diff: &DiffInfo) -> Self {
        let diff_analysis = DiffAnalysis::from_diff(diff);
        Self { diff_analysis }
    }

    /// Create from repository context.
    pub fn from_context(context: &RepositoryContext) -> Self {
        Self::from_diff(&context.diff_info)
    }

    /// Get the primary intent of the changes.
    pub fn primary_intent(&self) -> ChangeIntent {
        if self.diff_analysis.is_docs_change {
            ChangeIntent::Documentation
        } else if self.diff_analysis.is_test_change {
            ChangeIntent::Testing
        } else if self.diff_analysis.is_config_change {
            ChangeIntent::Configuration
        } else if self.diff_analysis.is_refactoring {
            ChangeIntent::Refactoring
        } else if self.diff_analysis.is_fix {
            ChangeIntent::BugFix
        } else if self.diff_analysis.adds_functionality {
            ChangeIntent::Feature
        } else {
            ChangeIntent::Update
        }
    }

    /// Get the suggested commit type.
    pub fn suggested_type(&self) -> CommitType {
        match self.primary_intent() {
            ChangeIntent::Feature => CommitType::Feat,
            ChangeIntent::BugFix => CommitType::Fix,
            ChangeIntent::Documentation => CommitType::Docs,
            ChangeIntent::Testing => CommitType::Test,
            ChangeIntent::Refactoring => CommitType::Refactor,
            ChangeIntent::Configuration => CommitType::Chore,
            ChangeIntent::Update => CommitType::Chore,
        }
    }

    /// Extract key actions from the changes.
    pub fn extract_actions(&self) -> Vec<ChangeAction> {
        let mut actions = Vec::new();

        for key_change in &self.diff_analysis.key_changes {
            let parts: Vec<&str> = key_change.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let verb = match parts[0] {
                    "add" => ActionVerb::Add,
                    "remove" => ActionVerb::Remove,
                    "update" => ActionVerb::Update,
                    "rename" => ActionVerb::Rename,
                    "modify" => ActionVerb::Modify,
                    _ => ActionVerb::Modify,
                };

                actions.push(ChangeAction {
                    verb,
                    target: parts[1].to_string(),
                    details: None,
                });
            }
        }

        // Deduplicate similar actions
        let mut seen = HashSet::new();
        actions.retain(|a| seen.insert(a.target.clone()));

        actions
    }

    /// Generate a summary of the changes.
    pub fn generate_summary(&self) -> String {
        self.diff_analysis.summary()
    }

    /// Get affected areas.
    pub fn affected_areas(&self) -> Vec<String> {
        let mut areas = Vec::new();

        for (category, files) in &self.diff_analysis.categories {
            if !files.is_empty() {
                let area = match category {
                    crate::analysis::diff::ChangeCategory::NewFiles => "new files",
                    crate::analysis::diff::ChangeCategory::DeletedFiles => "deleted files",
                    crate::analysis::diff::ChangeCategory::Tests => "tests",
                    crate::analysis::diff::ChangeCategory::Documentation => "documentation",
                    crate::analysis::diff::ChangeCategory::Configuration => "configuration",
                    crate::analysis::diff::ChangeCategory::Source => "source code",
                    crate::analysis::diff::ChangeCategory::Build => "build system",
                    crate::analysis::diff::ChangeCategory::Assets => "assets",
                };
                areas.push(area.to_string());
            }
        }

        areas
    }
}

/// Intent behind the changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeIntent {
    Feature,
    BugFix,
    Documentation,
    Testing,
    Refactoring,
    Configuration,
    Update,
}

impl ChangeIntent {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            ChangeIntent::Feature => "new functionality",
            ChangeIntent::BugFix => "bug fix",
            ChangeIntent::Documentation => "documentation update",
            ChangeIntent::Testing => "test changes",
            ChangeIntent::Refactoring => "code refactoring",
            ChangeIntent::Configuration => "configuration update",
            ChangeIntent::Update => "general update",
        }
    }
}

/// Action verb for changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionVerb {
    Add,
    Remove,
    Update,
    Rename,
    Modify,
    Fix,
    Implement,
    Refactor,
}

impl ActionVerb {
    /// Get the imperative form.
    pub fn imperative(&self) -> &'static str {
        match self {
            ActionVerb::Add => "add",
            ActionVerb::Remove => "remove",
            ActionVerb::Update => "update",
            ActionVerb::Rename => "rename",
            ActionVerb::Modify => "modify",
            ActionVerb::Fix => "fix",
            ActionVerb::Implement => "implement",
            ActionVerb::Refactor => "refactor",
        }
    }
}

/// A single change action.
#[derive(Debug, Clone)]
pub struct ChangeAction {
    /// The verb for this action.
    pub verb: ActionVerb,
    /// The target of the action.
    pub target: String,
    /// Optional details.
    pub details: Option<String>,
}

impl ChangeAction {
    /// Format as a bullet point.
    pub fn as_bullet(&self) -> String {
        if let Some(ref details) = self.details {
            format!("- {} {} ({})", self.verb.imperative(), self.target, details)
        } else {
            format!("- {} {}", self.verb.imperative(), self.target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{ChangeType, DiffStats, FileChange};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_diff_info(files: Vec<(&str, ChangeType)>) -> DiffInfo {
        DiffInfo {
            files: files
                .iter()
                .map(|(p, ct)| FileChange {
                    path: PathBuf::from(p),
                    change_type: *ct,
                    lines_added: 10,
                    lines_removed: 5,
                    is_binary: false,
                    old_path: None,
                })
                .collect(),
            stats: DiffStats {
                files_changed: files.len(),
                lines_added: 10,
                lines_removed: 5,
                binary_files: 0,
            },
            patches: HashMap::new(),
        }
    }

    #[test]
    fn test_semantic_analyzer_docs() {
        let diff = make_diff_info(vec![("README.md", ChangeType::Modified)]);
        let analyzer = SemanticAnalyzer::from_diff(&diff);

        assert_eq!(analyzer.primary_intent(), ChangeIntent::Documentation);
        assert_eq!(analyzer.suggested_type(), CommitType::Docs);
    }

    #[test]
    fn test_semantic_analyzer_tests() {
        let diff = make_diff_info(vec![("tests/test_main.rs", ChangeType::Added)]);
        let analyzer = SemanticAnalyzer::from_diff(&diff);

        assert_eq!(analyzer.primary_intent(), ChangeIntent::Testing);
        assert_eq!(analyzer.suggested_type(), CommitType::Test);
    }

    #[test]
    fn test_change_action_bullet() {
        let action = ChangeAction {
            verb: ActionVerb::Add,
            target: "new feature".to_string(),
            details: None,
        };

        assert_eq!(action.as_bullet(), "- add new feature");
    }
}
