use serde::{Deserialize, Serialize};
use crate::types::Span;

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
}

impl Edit {
    pub fn format_doc_lines(&self, max_width: usize) -> Vec<String> {
        let indent = " ".repeat((self.column_start - 1) as usize);
        let available_width = max_width.saturating_sub(indent.len() + 4); // "/// "

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in self.doc_comment.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= available_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(format!("{}/// {}", indent, current_line));
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(format!("{}/// {}", indent, current_line));
        }

        lines
    }
}
