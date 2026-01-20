// Author: Eshan Roy
// SPDX-License-Identifier: MIT

//! Commit message preview.

use console::{style, Term};

use super::message::CommitMessage;

/// Commit preview renderer.
pub struct CommitPreview<'a> {
    message: &'a CommitMessage,
}

impl<'a> CommitPreview<'a> {
    /// Create a new preview for a commit message.
    pub fn new(message: &'a CommitMessage) -> Self {
        Self { message }
    }

    /// Print the preview to stderr.
    pub fn print(&self) {
        let term = Term::stderr();
        let _ = self.render(&term);
    }

    /// Render the preview to a terminal.
    fn render(&self, term: &Term) -> std::io::Result<()> {
        // Box top
        term.write_line(&format!(
            "{}",
            style("┌─ Commit Preview ─────────────────────────────────────────────┐").dim()
        ))?;

        // Header line
        let header = self.format_header();
        term.write_line(&format!(
            "{} {}{}",
            style("│").dim(),
            header,
            self.padding(header.len())
        ))?;

        // Body if present
        if let Some(ref body) = self.message.body {
            term.write_line(&format!("{} {}", style("│").dim(), style("").dim()))?;

            for line in body.lines() {
                let visible_len = line.len().min(60);
                term.write_line(&format!(
                    "{} {}{}",
                    style("│").dim(),
                    style(line).dim(),
                    self.padding(visible_len)
                ))?;
            }
        }

        // Box bottom
        term.write_line(&format!(
            "{}",
            style("└──────────────────────────────────────────────────────────────┘").dim()
        ))?;

        Ok(())
    }

    /// Format the header with syntax highlighting.
    fn format_header(&self) -> String {
        let mut result = String::new();

        // Type (colored)
        let type_style = match self.message.commit_type.as_str() {
            "feat" => style(self.message.commit_type.as_str()).green().bold(),
            "fix" => style(self.message.commit_type.as_str()).red().bold(),
            "docs" => style(self.message.commit_type.as_str()).blue().bold(),
            "style" => style(self.message.commit_type.as_str()).magenta().bold(),
            "refactor" => style(self.message.commit_type.as_str()).yellow().bold(),
            "perf" => style(self.message.commit_type.as_str()).cyan().bold(),
            "test" => style(self.message.commit_type.as_str()).white().bold(),
            _ => style(self.message.commit_type.as_str()).white().bold(),
        };
        result.push_str(&type_style.to_string());

        // Scope
        if let Some(ref scope) = self.message.scope {
            result.push_str(&format!("({})", style(scope).cyan()));
        }

        // Breaking indicator
        if self.message.is_breaking {
            result.push_str(&style("!").red().bold().to_string());
        }

        // Separator
        result.push_str(": ");

        // Subject
        result.push_str(&self.message.subject);

        result
    }

    /// Create padding to align the right border.
    fn padding(&self, content_len: usize) -> String {
        let box_width: usize = 62;
        let padding_needed = box_width.saturating_sub(content_len + 2);
        format!("{}{}", " ".repeat(padding_needed), style("│").dim())
    }

    /// Get a formatted string representation.
    pub fn to_string(&self) -> String {
        self.message.format()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CommitType;

    #[test]
    fn test_preview_new() {
        let message = CommitMessage::new(CommitType::Feat, "test");
        let preview = CommitPreview::new(&message);
        assert_eq!(preview.to_string(), "feat: test");
    }

    #[test]
    fn test_format_header() {
        let message = CommitMessage::new(CommitType::Feat, "add feature").with_scope("core");
        let preview = CommitPreview::new(&message);
        let header = preview.format_header();
        assert!(header.contains("feat"));
        assert!(header.contains("core"));
        assert!(header.contains("add feature"));
    }

    #[test]
    fn test_format_header_breaking() {
        let message = CommitMessage::new(CommitType::Feat, "change").with_breaking(true);
        let preview = CommitPreview::new(&message);
        let header = preview.format_header();
        assert!(header.contains("!"));
    }
}
