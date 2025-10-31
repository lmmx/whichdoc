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
