use ratatui::prelude::*;

use crate::theme::Theme;

/// Parse unified diff patch text into styled lines with line numbers.
/// Colors: `+` addition, `-` deletion, `@@` header, context default.
pub fn parse_diff_patch(patch: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    for raw in patch.lines() {
        if raw.starts_with("@@") {
            if let Some((old, new)) = parse_hunk_header(raw) {
                old_line = old;
                new_line = new;
            }
            lines.push(Line::from(Span::styled(
                raw.to_string(),
                theme.diff_header,
            )));
        } else if raw.starts_with("+++") || raw.starts_with("---") {
            // File header lines (--- a/file, +++ b/file) — render as muted
            lines.push(Line::from(Span::styled(raw.to_string(), theme.muted)));
        } else if raw.starts_with('+') {
            let prefix = format!("{:>4} | ", new_line);
            new_line += 1;
            lines.push(Line::from(vec![
                Span::styled(prefix, theme.muted),
                Span::styled(raw.to_string(), theme.diff_addition),
            ]));
        } else if raw.starts_with('-') {
            let prefix = format!("{:>4} | ", old_line);
            old_line += 1;
            lines.push(Line::from(vec![
                Span::styled(prefix, theme.muted),
                Span::styled(raw.to_string(), theme.diff_deletion),
            ]));
        } else if raw.starts_with('\\') {
            lines.push(Line::from(Span::styled(raw.to_string(), theme.muted)));
        } else {
            let prefix = format!("{:>4} | ", new_line);
            if !raw.is_empty() {
                old_line += 1;
                new_line += 1;
            }
            lines.push(Line::from(vec![
                Span::styled(prefix, theme.muted),
                Span::raw(raw.to_string()),
            ]));
        }
    }

    lines
}

/// Parse @@ -old_start,count +new_start,count @@ header.
/// Returns (old_start, new_start) line numbers.
pub fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let old = parts[1].strip_prefix('-').unwrap_or(parts[1]);
    let new = parts[2].strip_prefix('+').unwrap_or(parts[2]);
    let old_start = old.split(',').next()?.parse::<u32>().ok()?;
    let new_start = new.split(',').next()?.parse::<u32>().ok()?;
    Some((old_start, new_start))
}
