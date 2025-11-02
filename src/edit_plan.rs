//! The edit plan keeps a record of the contract between the interactive session and the file system
//!
//! This module defines the transformation that work in the TUI manifests as actual edits on disk.
//! The word 'plan' is used not in the sense of intent for future work, but more like blueprints,
//! or floor plan. It represents the total set of edits that are applied over the session. They are
//! not deferred until the end of the session, but executed upon 'save' (saving to the plan will
//! save to the corresponding source file too).
//!
//! There are only 2 comment styles, `//!` module docstrings and `///` regular docstrings. They are
//! always written for the user (the user never writes them, and the TUI shows them for all lines.
//! The edit plan keeps a record of the contract between the interactive session and the file system
//!
//! This module defines the transformation that work in the TUI manifests as actual edits on disk.
//! The word 'plan' is used not in the sense of intent for future work, but more like blueprints,
//! or floor plan. It represents the total set of edits that are applied over the session. They are
//! not deferred until the end of the session, but executed upon 'save' (saving to the plan will
//! save to the corresponding source file too).
//!
//! There are only 2 comment styles
//! The edit plan keeps a record of the contract between the interactive session and the file system
//!
//! This module defines the transformation that work in the TUI manifests as actual edits on disk.
//! The word 'plan' is used not in the sense of intent for future work, but more like blueprints,
//! or floor plan. It represents the total set of edits that are applied over the session. They are
//! not deferred until the end of the session, but executed upon 'save' (saving to the plan will
//! save to the corresponding source file too).
//!
//! There are only 2 comment styles
use crate::types::Span;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct EditPlan {
    pub edits: Vec<Edit>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Edit {
    pub file_name: String,
    pub line_start: i64,
    pub line_end: i64,
    pub column_start: i64,
    pub column_end: i64,
    pub doc_comment: String,
    pub item_name: String,
    pub span: Span,
    pub is_module_doc: bool,
}

impl Edit {
    #[must_use]
    pub fn format_doc_lines(&self, max_width: usize) -> Vec<String> {
        let indent = " ".repeat(usize::try_from(self.column_start - 1).unwrap_or(0));
        let prefix = if self.is_module_doc { "//!" } else { "///" };
        let available_width = max_width.saturating_sub(indent.len() + prefix.len() + 1);

        let mut lines = Vec::new();

        for line in self.doc_comment.split('\n') {
            if line.trim().is_empty() {
                // Empty line - no trailing space
                lines.push(format!("{indent}{prefix}"));
                continue;
            }

            // Preserve leading whitespace
            let leading_spaces = line.len() - line.trim_start().len();
            let leading = " ".repeat(leading_spaces);
            let content = line.trim_start();

            let mut current_line = String::new();
            let mut is_first_line = true;

            for word in content.split_whitespace() {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.len() + 1 + word.len() <= available_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    // Flush current line
                    if is_first_line {
                        lines.push(format!("{indent}{prefix} {leading}{current_line}"));
                        is_first_line = false;
                    } else {
                        lines.push(format!("{indent}{prefix} {current_line}"));
                    }
                    current_line = word.to_string();
                }
            }

            if !current_line.is_empty() {
                if is_first_line {
                    lines.push(format!("{indent}{prefix} {leading}{current_line}"));
                } else {
                    lines.push(format!("{indent}{prefix} {current_line}"));
                }
            }
        }

        if std::env::var("WHICHDOC_DEBUG").is_ok() {
            use std::io::Write;
            let mut debug_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/whichdoc_debug.log")
                .unwrap();
            writeln!(debug_file, "format_doc_lines input: {:?}", self.doc_comment).ok();
            writeln!(debug_file, "format_doc_lines output: {:?}", lines).ok();
        }

        lines
    }
}
