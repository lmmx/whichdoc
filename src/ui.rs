use crate::app_state::{AppState, View};
use crate::config::Config;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &AppState, cfg: &Config) {
    match app.current_view {
        View::List => draw_list(f, app),
        View::Detail => draw_detail(f, app, cfg),
        View::Command => draw_detail(f, app, cfg),
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

fn draw_detail(f: &mut Frame, app: &AppState, _cfg: &Config) {
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
        .block(Block::default().borders(Borders::ALL).title("Diagnostic"));
    f.render_widget(info, chunks[0]);

    let indent = app.get_indent();
    let max_width = app.get_max_line_width();
    let indent_str = " ".repeat(indent);

    let doc_prefix = app.entries[app.list_index].doc_prefix();
    let mut display_text = String::new();
    for (i, line) in app.detail_lines.iter().enumerate() {
        display_text.push_str(&format!("{}{} {}", indent_str, doc_prefix, line));
        if i < app.detail_lines.len() - 1 {
            display_text.push('\n');
        }
    }

    let title = format!("Doc Comment (max line: {} chars)", max_width);
    let doc_input = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        );
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
