//! The UI renders the application state into something visible and vim-able.
//!
//! The draw function dispatches to `draw_list` or `draw_detail` based on the `app.current_view`.
//! The rendering primitives use ratatui which arranges the page using Cassowary solver (cool!)
//!
//! The list view is the main menu, I think of it as a "diagnostic picker".
//! The detail view is what you get when you "open" or "pick" a diagnostic, it doesn't add much info
//! in practice because we always have `missing_docs` but in future it might be more like PDB++'s
//! sticky mode (but for now it's a simple file/line number read-out followed by the rendered error.
//!
//! A footer at the bottom of the view shows hints for the vim-like save/exit/quit/next/prev keys.
//!
//! The UI is all read-only, mutation happens in the event loop, draw takes immutable references.
//! This separation means rendering can never corrupt application state.
use crate::app_state::{AppState, View};
use crate::config::Config;
use crate::highlight;
use edtui::{EditorTheme, EditorView, SyntaxHighlighter};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut AppState, cfg: &Config) {
    match app.current_view {
        View::List => draw_list(f, app),
        View::Detail | View::Command => draw_detail(f, app, cfg),
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
                    .map_or_else(
                        || "unknown".to_string(),
                        |t| {
                            t.text
                                .split('{')
                                .next()
                                .unwrap_or("unknown")
                                .trim()
                                .to_string()
                        },
                    )
            } else {
                "unknown".to_string()
            };

            let status = if entry.doc_comment.is_some() {
                "✓"
            } else {
                " "
            };
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

fn draw_detail(f: &mut Frame, app: &mut AppState, _cfg: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),     // Diagnostic info
            Constraint::Percentage(40), // Source code viewer
            Constraint::Percentage(40), // Doc comment editor
            Constraint::Length(3),      // Help
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

    let info =
        Paragraph::new(info_text).block(Block::default().borders(Borders::ALL).title("Diagnostic"));
    f.render_widget(info, chunks[0]);

    let max_width = app.get_max_line_width();
    let max_width_u16: u16 = match max_width.try_into() {
        Ok(value) => value, // Successfully converted
        Err(_) => u16::MAX,
    };

    let title = format!("Doc Comment (max line: {max_width} chars)");

    if let Some(ref msg) = entry.coord.message {
        if let Some(span) = msg.spans.iter().find(|s| s.is_primary) {
            if let Ok(content) = std::fs::read_to_string(&span.file_name) {
                let lines: Vec<&str> = content.lines().collect();

                // Add the cumulative offset here
                let offset = app.cumulative_offset(app.list_index);
                let target_line = usize::try_from(span.line_start)
                    .unwrap_or(0)
                    .saturating_sub(1)
                    + offset;
                let start = target_line.saturating_sub(5);
                let end = (target_line + 15).min(lines.len());

                let display_lines =
                    highlight::highlight_source_lines(&lines, start, end, target_line);

                let viewer = Paragraph::new(display_lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Source: {}", span.file_name)),
                );

                f.render_widget(viewer, chunks[1]);
            }
        }
    }

    if let Some(ref mut editor_state) = app.editor_state {
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(chunks[2]);
        f.render_widget(block, chunks[2]);

        let syntax_highlighter = SyntaxHighlighter::new("dracula", "rs");
        let editor = EditorView::new(editor_state)
            .theme(EditorTheme::default())
            .syntax_highlighter(Some(syntax_highlighter))
            .wrap(true);

        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(max_width_u16), Constraint::Min(0)])
            .split(inner);

        f.render_widget(editor, editor_chunks[0]);
    }

    let help_text = if app.current_view == View::Command {
        format!(":{}", app.command_buffer)
    } else if let Some(ref msg) = app.message {
        msg.clone()
    } else {
        ":w Save | :x Save & Exit | :q Quit | :q! Force Quit | :wn Save & Next | :wp Save & Prev"
            .to_string()
    };

    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}
