#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! ratatui = "0.28"
//! crossterm = "0.28"
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! atty = "0.2"
//! ```

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Coordinate {
        pub reason: String,
        pub message: Option<Message>,
        pub package_id: Option<String>,
        pub manifest_path: Option<String>,
        pub target: Option<Target>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Message {
        #[serde(rename = "$message_type")]
        pub message_type: String,
        pub children: Vec<Child>,
        pub code: Code,
        pub level: String,
        pub message: String,
        pub rendered: String,
        pub spans: Vec<Span>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Child {
        pub children: Vec<Option<serde_json::Value>>,
        pub code: Option<serde_json::Value>,
        pub level: String,
        pub message: String,
        pub rendered: Option<serde_json::Value>,
        pub spans: Vec<Option<serde_json::Value>>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Code {
        pub code: String,
        pub explanation: Option<serde_json::Value>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Span {
        pub byte_end: i64,
        pub byte_start: i64,
        pub column_end: i64,
        pub column_start: i64,
        pub expansion: Option<serde_json::Value>,
        pub file_name: String,
        pub is_primary: bool,
        pub label: Option<serde_json::Value>,
        pub line_end: i64,
        pub line_start: i64,
        pub suggested_replacement: Option<serde_json::Value>,
        pub suggestion_applicability: Option<serde_json::Value>,
        pub text: Vec<Text>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Text {
        pub highlight_end: i64,
        pub highlight_start: i64,
        pub text: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Target {
        pub crate_types: Vec<String>,
        pub doc: bool,
        pub doctest: bool,
        pub edition: String,
        pub kind: Vec<String>,
        pub name: String,
        pub src_path: String,
        pub test: bool,
    }
}

mod edit_plan {
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
}

mod input {
    use crate::types::Coordinate;
    use std::io::{self, BufRead};
    use std::process::{Command, Stdio};

    pub fn read_cargo_diagnostics() -> io::Result<Vec<Coordinate>> {
        let input: Box<dyn BufRead> = if atty::is(atty::Stream::Stdin) {
            let child = Command::new("cargo")
                .args(&["doc", "--message-format=json"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;
            Box::new(io::BufReader::new(child.stdout.unwrap()))
        } else {
            Box::new(io::BufReader::new(io::stdin()))
        };

        let mut diagnostics = Vec::new();
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(coord) = serde_json::from_str::<Coordinate>(&line) {
                if let Some(ref msg) = coord.message {
                    if msg.code.code == "missing_docs" {
                        diagnostics.push(coord);
                    }
                }
            }
        }
        Ok(diagnostics)
    }
}

mod app_state {
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
}

mod ui {
    use crate::app_state::{AppState, View};
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
        Frame,
    };

    pub fn draw(f: &mut Frame, app: &AppState) {
        match app.current_view {
            View::List => draw_list(f, app),
            View::Detail => draw_detail(f, app),
            View::Command => draw_detail(f, app), // Command overlay on detail
        }
    }

    fn draw_list(f: &mut Frame, app: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        let items: Vec<ListItem> = app
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let style = if entry.doc_comment.is_some() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };

                let item_name = if let Some(ref msg) = entry.coord.message {
                    msg.spans
                        .iter()
                        .find(|s| s.is_primary)
                        .and_then(|s| s.text.first())
                        .map(|t| {
                            t.text
                                .split('{')
                                .next()
                                .unwrap_or("unknown")
                                .trim()
                                .to_string()
                        })
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    "unknown".to_string()
                };

                let status = if entry.doc_comment.is_some() { "✓" } else { " " };
                let text = format!("[{}] #{}: {}", status, entry.id, item_name);

                ListItem::new(text).style(if i == app.list_index {
                    style.add_modifier(Modifier::REVERSED)
                } else {
                    style
                })
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Missing Docs"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(list, chunks[0]);

        let help = Paragraph::new("↑/↓: Navigate | Enter: Edit | q: Quit")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[1]);
    }

    fn draw_detail(f: &mut Frame, app: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        let entry = &app.entries[app.list_index];
        let info_text = if let Some(ref msg) = entry.coord.message {
            let span = msg.spans.iter().find(|s| s.is_primary);
            if let Some(s) = span {
                format!(
                    "File: {}\nLine: {}\n\n{}",
                    s.file_name, s.line_start, msg.rendered
                )
            } else {
                msg.rendered.clone()
            }
        } else {
            "No message".to_string()
        };

        let info = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL).title("Diagnostic"))
            .wrap(Wrap { trim: true });
        f.render_widget(info, chunks[0]);

        let doc_input = Paragraph::new(app.detail_text.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Doc Comment (Type your documentation here)"),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(doc_input, chunks[1]);

        let help_text = if app.current_view == View::Command {
            format!(":{}", app.command_buffer)
        } else if let Some(ref msg) = app.message {
            msg.clone()
        } else {
            ":w Save | :x Save & Exit | :q Quit | :q! Force Quit | :wn Save & Next | :wp Save & Prev".to_string()
        };

        let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let coords = if args.len() > 2 && args[1] == "--load-docs" {
        let file_content = std::fs::read_to_string(&args[2])?;
        let plan: edit_plan::EditPlan = serde_json::from_str(&file_content)?;
        let coords = input::read_cargo_diagnostics()?;
        let mut state = app_state::AppState::new(coords);
        state.load_docs(plan);
        return run_tui(state);
    } else {
        input::read_cargo_diagnostics()?
    };

    if coords.is_empty() {
        eprintln!("No missing_docs diagnostics found");
        return Ok(());
    }

    let state = app_state::AppState::new(coords);
    run_tui(state)
}

fn run_tui(mut app: app_state::AppState) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    } else {
        let plan = app.generate_edit_plan();
        let json = serde_json::to_string_pretty(&plan)?;
        println!("{}", json);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut app_state::AppState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.current_view {
                app_state::View::List => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => {
                            if app.list_index > 0 {
                                app.list_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.list_index < app.entries.len() - 1 {
                                app.list_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            app.enter_detail_view();
                        }
                        _ => {}
                    }
                }
                app_state::View::Detail => {
                    match key.code {
                        KeyCode::Char(':') => {
                            app.current_view = app_state::View::Command;
                            app.command_buffer.clear();
                            app.message = None;
                        }
                        KeyCode::Char(c) => {
                            app.detail_text.push(c);
                            app.entries[app.list_index].dirty = app.detail_text != app.detail_saved_text;
                        }
                        KeyCode::Backspace => {
                            app.detail_text.pop();
                            app.entries[app.list_index].dirty = app.detail_text != app.detail_saved_text;
                        }
                        KeyCode::Enter => {
                            app.detail_text.push('\n');
                        }
                        KeyCode::Esc => {
                            if app.entries[app.list_index].dirty {
                                app.message = Some("Unsaved changes! Use :q to discard or :x to save".to_string());
                            } else {
                                app.exit_detail_view(false);
                            }
                        }
                        _ => {}
                    }
                }
                app_state::View::Command => {
                    match key.code {
                        KeyCode::Char(c) => {
                            app.command_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            app.command_buffer.pop();
                        }
                        KeyCode::Enter => {
                            let cmd = app.command_buffer.clone();
                            app.current_view = app_state::View::Detail;

                            match cmd.as_str() {
                                "w" => {
                                    app.save_current();
                                }
                                "x" => {
                                    app.save_current();
                                    app.exit_detail_view(true);
                                }
                                "q" => {
                                    if app.entries[app.list_index].dirty {
                                        app.message = Some("Unsaved changes! Use :q! to force quit".to_string());
                                        app.current_view = app_state::View::Detail;
                                    } else {
                                        app.exit_detail_view(false);
                                    }
                                }
                                "q!" => {
                                    app.exit_detail_view(false);
                                }
                                "wn" => {
                                    app.save_current();
                                    if let Some(next) = app.find_next_undocumented() {
                                        app.exit_detail_view(true);
                                        app.list_index = next;
                                        app.enter_detail_view();
                                    } else {
                                        app.message = Some("No more undocumented items".to_string());
                                    }
                                }
                                "wp" => {
                                    app.save_current();
                                    if let Some(prev) = app.find_prev_undocumented() {
                                        app.exit_detail_view(true);
                                        app.list_index = prev;
                                        app.enter_detail_view();
                                    } else {
                                        app.message = Some("No previous undocumented items".to_string());
                                    }
                                }
                                _ => {
                                    app.message = Some(format!("Unknown command: {}", cmd));
                                }
                            }
                            app.command_buffer.clear();
                        }
                        KeyCode::Esc => {
                            app.current_view = app_state::View::Detail;
                            app.command_buffer.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
