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

        // Split by lines first to preserve explicit line breaks
        for paragraph in self.doc_comment.split('\n') {
            if paragraph.trim().is_empty() {
                // Empty line - preserve it as an empty comment
                lines.push(format!("{indent}{prefix}"));
                continue;
            }

            let mut current_line = String::new();
            for word in paragraph.split_whitespace() {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.len() + 1 + word.len() <= available_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    lines.push(format!("{indent}{prefix} {current_line}"));
                    current_line = word.to_string();
                }
            }

            if !current_line.is_empty() {
                lines.push(format!("{indent}{prefix} {current_line}"));
            }
        }

        lines
    }
}
