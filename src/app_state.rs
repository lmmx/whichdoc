use crate::edit_plan::{Edit, EditPlan};
use crate::types::{Coordinate, Span};
use std::collections::HashMap;

#[derive(Clone)]
pub struct DiagnosticEntry {
    pub id: usize,
    pub coord: Coordinate,
    pub doc_comment: Option<String>,
    pub dirty: bool,
}

pub struct AppState {
    pub entries: Vec<DiagnosticEntry>,
    pub current_view: View,
    pub list_index: usize,
    pub detail_text: String,
    pub detail_saved_text: String,
    pub command_buffer: String,
    pub message: Option<String>,
}

#[derive(PartialEq)]
pub enum View {
    List,
    Detail,
    Command,
}

impl AppState {
    pub fn new(coords: Vec<Coordinate>) -> Self {
        let entries = coords
            .into_iter()
            .enumerate()
            .map(|(id, coord)| DiagnosticEntry {
                id,
                coord,
                doc_comment: None,
                dirty: false,
            })
            .collect();

        Self {
            entries,
            current_view: View::List,
            list_index: 0,
            detail_text: String::new(),
            detail_saved_text: String::new(),
            command_buffer: String::new(),
            message: None,
        }
    }

    pub fn load_docs(&mut self, plan: EditPlan) {
        let mut doc_map: HashMap<String, String> = HashMap::new();
        for edit in plan.edits {
            let key = format!("{}:{}:{}", edit.file_name, edit.line_start, edit.column_start);
            doc_map.insert(key, edit.doc_comment);
        }

        for entry in &mut self.entries {
            if let Some(ref msg) = entry.coord.message {
                for span in &msg.spans {
                    if span.is_primary {
                        let key = format!("{}:{}:{}", span.file_name, span.line_start, span.column_start);
                        if let Some(doc) = doc_map.get(&key) {
                            entry.doc_comment = Some(doc.clone());
                        }
                    }
                }
            }
        }
    }

    pub fn generate_edit_plan(&self) -> EditPlan {
        let mut edits = Vec::new();
        for entry in &self.entries {
            if let Some(ref doc) = entry.doc_comment {
                if let Some(ref msg) = entry.coord.message {
                    for span in &msg.spans {
                        if span.is_primary {
                            let item_name = extract_item_name(span);
                            edits.push(Edit {
                                file_name: span.file_name.clone(),
                                line_start: span.line_start,
                                line_end: span.line_end,
                                column_start: span.column_start,
                                column_end: span.column_end,
                                doc_comment: doc.clone(),
                                item_name,
                                span: span.clone(),
                            });
                        }
                    }
                }
            }
        }
        EditPlan { edits }
    }

    pub fn enter_detail_view(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let entry = &self.entries[self.list_index];
        self.detail_text = entry.doc_comment.clone().unwrap_or_default();
        self.detail_saved_text = self.detail_text.clone();
        self.current_view = View::Detail;
    }

    pub fn exit_detail_view(&mut self, save: bool) {
        if save {
            self.entries[self.list_index].doc_comment = Some(self.detail_text.clone());
            self.entries[self.list_index].dirty = false;
            self.detail_saved_text = self.detail_text.clone();
        } else {
            self.detail_text = self.detail_saved_text.clone();
        }
        self.current_view = View::List;
    }

    pub fn save_current(&mut self) {
        self.entries[self.list_index].doc_comment = Some(self.detail_text.clone());
        self.entries[self.list_index].dirty = false;
        self.detail_saved_text = self.detail_text.clone();
        self.message = Some("Saved".to_string());
    }

    pub fn find_next_undocumented(&self) -> Option<usize> {
        for i in (self.list_index + 1)..self.entries.len() {
            if self.entries[i].doc_comment.is_none() {
                return Some(i);
            }
        }
        None
    }

    pub fn find_prev_undocumented(&self) -> Option<usize> {
        for i in (0..self.list_index).rev() {
            if self.entries[i].doc_comment.is_none() {
                return Some(i);
            }
        }
        None
    }
}

fn extract_item_name(span: &Span) -> String {
    if !span.text.is_empty() {
        span.text[0]
            .text
            .split('{')
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string()
    } else {
        "unknown".to_string()
    }
}
