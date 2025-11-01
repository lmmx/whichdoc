use once_cell::sync::Lazy;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub fn highlight_source_lines(
    lines: &[&str],
    start: usize,
    end: usize,
    target_line: usize,
) -> Vec<Line<'static>> {
    let theme = &THEME_SET.themes["base16-eighties.dark"];
    let syntax_ref = SYNTAX_SET
        .find_syntax_by_extension("rs")
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let mut highlight_lines = HighlightLines::new(syntax_ref, theme);
    let mut display_lines: Vec<Line<'static>> = Vec::new();

    for (i, line_text) in lines[start..end].iter().enumerate() {
        let line_num = start + i + 1;
        let marker = if line_num == target_line + 1 {
            "→"
        } else {
            " "
        };
        let line_num_text = format!("{} {:4} | ", marker, line_num);

        let highlighted = highlight_lines
            .highlight_line(line_text, &SYNTAX_SET)
            .unwrap();

        let mut spans = vec![Span::raw(line_num_text)];
        for (style, text) in highlighted {
            spans.push(Span::styled(
                text.to_string(),
                Style::default().fg(Color::Rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                )),
            ));
        }

        display_lines.push(Line::from(spans));
    }

    display_lines
}
