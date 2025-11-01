//! The core state machine bridging the diagnostics and interactive editor.
//!
//! A TUI needs a single source of truth that can be interrogated and mutated as the user navigates
//! and edits. We achieve this by syncing the editor save state not only with the edit plan but with
//! the files on disk too. We keep track of the cumulative total number of lines that have been
//! added to the file during the session so that we can determine the correct offset to insert
//! docstrings at without re-computing the diagnostics from cargo.
//!
//! If the missing_docs diagnostic is for a module, we try to find the file (either {name}.rs or
//! {name}/mod.rs) and insert it at the start as a `//!` style rather than `///` style comment.
use crate::edit_plan::{Edit, EditPlan};
use crate::types::{Coordinate, Span};
use std::collections::HashMap;
use std::{fs, io};

#[derive(Clone)]
/// Each missing documentation warning needs to be tracked individually as it moves through states.
/// It starts out undocumented (missing), becomes edited (gets an edit plan entry), then is saved.
/// Hallelujah! It is the atomic unit of work in the application. The dirty flag is transactional
/// state to prevent data loss. We store the docstring as ready-made lines rather to avoid surprises
/// from word-wrapping later (what you see is what you get).
pub struct DiagnosticEntry {
    pub id: usize,
    pub coord: Coordinate,
    pub doc_comment: Option<Vec<String>>,
    pub dirty: bool,
    pub is_external_module: bool,
    pub module_file_path: Option<String>,
}

impl DiagnosticEntry {
    pub fn new(id: usize, coord: Coordinate) -> Self {
        let (is_external_module, module_file_path) = check_external_module(&coord);
        Self {
            id,
            coord,
            doc_comment: None,
            dirty: false,
            is_external_module,
            module_file_path,
        }
    }

    pub fn lines_added(&self) -> usize {
        self.doc_comment.as_ref().map_or(0, |v| v.len())
    }

    pub fn is_external_module_with_file(&self) -> bool {
        self.is_external_module && Some(self.module_file_path)
    }

    pub fn doc_prefix(&self) -> &'static str {
        if self.is_external_module_with_file {
            "//!"
        } else {
            "///"
        }
    }
}

fn check_external_module(coord: &Coordinate) -> (bool, Option<String>) {
    if let Some(ref msg) = coord.message {
        if msg.message == "missing documentation for a module" {
            if let Some(span) = msg.spans.iter().find(|s| s.is_primary) {
                if let Some(text) = span.text.first() {
                    if text.text.trim().ends_with(';') {
                        let module_file = get_module_file_path(span);
                        return (true, module_file);
                    }
                }
            }
        }
    }
    (false, None)
}

fn get_module_file_path(span: &Span) -> Option<String> {
    let text = span.text.first()?.text.trim();
    let module_name = text
        .split_whitespace()
        .skip_while(|&word| word != "mod")
        .nth(1)?  // Get the word after "mod"
        .trim_end_matches(';');

    let dir = std::path::Path::new(&span.file_name).parent()?;

    let direct_path = dir.join(format!("{}.rs", module_name));
    if direct_path.exists() {
        return Some(direct_path.to_string_lossy().to_string());
    }

    let mod_path = dir.join(module_name).join("mod.rs");
    if mod_path.exists() {
        return Some(mod_path.to_string_lossy().to_string());
    }

    None
}

pub struct AppState {
    pub entries: Vec<DiagnosticEntry>,
    pub current_view: View,
    pub list_index: usize,
    pub detail_lines: Vec<String>,
    pub detail_saved_lines: Vec<String>,
    pub detail_cursor_line: usize,
    pub detail_cursor_col: usize,
    pub command_buffer: String,
    pub message: Option<String>,
    pub max_width: usize,
}

#[derive(PartialEq)]
pub enum View {
    List,
    Detail,
    Command,
}

impl AppState {
    pub fn new(coords: Vec<Coordinate>, max_width: usize) -> Self {
        let entries = coords
            .into_iter()
            .enumerate()
            .map(|(id, coord)| DiagnosticEntry::new(id, coord))
            .collect();

        Self {
            entries,
            current_view: View::List,
            list_index: 0,
            detail_lines: Vec::new(),
            detail_saved_lines: Vec::new(),
            detail_cursor_line: 0,
            detail_cursor_col: 0,
            command_buffer: String::new(),
            message: None,
            max_width,
        }
    }

    pub fn cumulative_offset(&self, index: usize) -> usize {
        self.entries[0..index].iter().map(|e| e.lines_added()).sum()
    }

    pub fn load_docs(&mut self, plan: EditPlan) {
        let mut doc_map: HashMap<String, Vec<String>> = HashMap::new();
        for edit in plan.edits {
            let key = format!("{}:{}:{}", edit.file_name, edit.line_start, edit.column_start);
            let lines: Vec<String> = edit.doc_comment.lines().map(|s| s.to_string()).collect();
            doc_map.insert(key, lines);
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
            if let Some(ref doc_lines) = entry.doc_comment {
                if let Some(ref msg) = entry.coord.message {
                    for span in &msg.spans {
                        if span.is_primary {
                            let item_name = extract_item_name(span);
                            let doc_comment = doc_lines.join("\n");
                            edits.push(Edit {
                                file_name: span.file_name.clone(),
                                line_start: span.line_start,
                                line_end: span.line_end,
                                column_start: span.column_start,
                                column_end: span.column_end,
                                doc_comment,
                                item_name,
                                span: span.clone(),
                                is_module_doc: entry.is_external_module_with_file,
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
        self.detail_lines = entry.doc_comment.clone().unwrap_or_default();
        self.detail_saved_lines = self.detail_lines.clone();
        self.detail_cursor_line = 0;
        self.detail_cursor_col = 0;
        if self.detail_lines.is_empty() {
            self.detail_lines.push(String::new());
        }
        self.current_view = View::Detail;
    }

    pub fn exit_detail_view(&mut self, save: bool) {
        if save {
            self.entries[self.list_index].doc_comment = Some(self.detail_lines.clone());
            self.entries[self.list_index].dirty = false;
            self.detail_saved_lines = self.detail_lines.clone();
        } else {
            self.detail_lines = self.detail_saved_lines.clone();
        }
        self.current_view = View::List;
    }

    pub fn save_current(&mut self) -> io::Result<()> {
        self.entries[self.list_index].doc_comment = Some(self.detail_lines.clone());
        self.entries[self.list_index].dirty = false;
        self.detail_saved_lines = self.detail_lines.clone();

        let entry = &self.entries[self.list_index];

        if entry.is_external_module_with_file {
            // the ...with_file variant ensures module_file_path is Some
            self.apply_module_doc(entry.module_file_path)?;
        } else if let Some(ref msg) = entry.coord.message {
            for span in &msg.spans {
                if span.is_primary {
                    let doc_comment = self.detail_lines.join("\n");
                    let edit = Edit {
                        file_name: span.file_name.clone(),
                        line_start: span.line_start,
                        line_end: span.line_end,
                        column_start: span.column_start,
                        column_end: span.column_end,
                        doc_comment,
                        item_name: extract_item_name(span),
                        span: span.clone(),
                        is_module_doc: false,
                    };

                    self.apply_single_edit(&edit)?;
                    break;
                }
            }
        }

        self.message = Some("Saved".to_string());
        Ok(())
    }

    fn apply_module_doc(&self, module_path: &str) -> io::Result<()> {
        let content = fs::read_to_string(module_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let doc_lines: Vec<String> = self.detail_lines.iter()
            .map(|line| {
                if line.is_empty() {
                    "//!".to_string()
                } else {
                    format!("//! {}", line)
                }
            })
            .collect();

        for (i, doc_line) in doc_lines.iter().enumerate() {
            lines.insert(i, doc_line.clone());
        }

        fs::write(module_path, lines.join("\n") + "\n")?;
        Ok(())
    }


    fn apply_single_edit(&self, edit: &Edit) -> io::Result<()> {
        let content = fs::read_to_string(&edit.file_name)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let offset = self.cumulative_offset(self.list_index);
        let insert_pos = (edit.line_start as usize - 1) + offset;
        let doc_lines = edit.format_doc_lines(self.max_width);

        for (i, doc_line) in doc_lines.iter().enumerate() {
            lines.insert(insert_pos + i, doc_line.clone());
        }

        fs::write(&edit.file_name, lines.join("\n") + "\n")?;
        Ok(())
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

    pub fn get_indent(&self) -> usize {
        if self.entries.is_empty() {
            return 0;
        }
        let entry = &self.entries[self.list_index];
        if entry.is_external_module_with_file {
            return 0;
        }
        if let Some(ref msg) = entry.coord.message {
            for span in &msg.spans {
                if span.is_primary {
                    return (span.column_start - 1) as usize;
                }
            }
        }
        0
    }

    pub fn get_max_line_width(&self) -> usize {
        let indent = self.get_indent();
        let entry = &self.entries[self.list_index];
        let prefix_len = entry.doc_prefix().len() + 1; // "/// " or "//! "
        self.max_width.saturating_sub(indent + prefix_len)
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
