use ratatui::prelude::*;
use ratatui::text::{Line, Span};

/// Convert markdown text to ratatui Lines for display in a paragraph.
/// Handles: # headings, **bold**, *italic*, `code`, ```code blocks```, - lists
pub fn render_markdown(input: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in input.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray).bg(Color::Black),
            )));
            continue;
        }

        if let Some(heading) = line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
        } else if let Some(heading) = line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            )));
        } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            let content = parse_inline(item);
            let mut spans = vec![Span::raw("  • ")];
            spans.extend(content);
            lines.push(Line::from(spans));
        } else if line.trim().is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(parse_inline(line)));
        }
    }

    lines
}

/// Parse inline markdown: **bold**, *italic*, `code`
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Bold: **text**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            let end = text[i + 2..].find("**").map(|e| e + i + 2);
            if let Some(end) = end {
                spans.push(Span::styled(
                    text[i + 2..end].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                i = end + 2;
                continue;
            }
        }

        // Italic: *text*
        if chars[i] == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            if let Some(end) = text[i + 1..].find('*').map(|e| e + i + 1) {
                spans.push(Span::styled(
                    text[i + 1..end].to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
                i = end + 1;
                continue;
            }
        }

        // Inline code: `text`
        if chars[i] == '`' {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            if let Some(end) = text[i + 1..].find('`').map(|e| e + i + 1) {
                spans.push(Span::styled(
                    format!(" {} ", &text[i + 1..end]),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
                i = end + 1;
                continue;
            }
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        spans.push(Span::raw(current));
    }

    spans
}
