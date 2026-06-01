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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use ratatui::style::Style;

    fn test_theme() -> Theme {
        Theme {
            diff_header: Style::default(),
            diff_addition: Style::default(),
            diff_deletion: Style::default(),
            muted: Style::default(),
            ..Default::default()
        }
    }

    #[test]
    fn parse_hunk_header_standard_format() {
        let result = parse_hunk_header("@@ -10,5 +20,8 @@");
        assert_eq!(result, Some((10, 20)));
    }

    #[test]
    fn parse_hunk_header_no_counts() {
        let result = parse_hunk_header("@@ -3 +7 @@");
        assert_eq!(result, Some((3, 7)));
    }

    #[test]
    fn parse_hunk_header_with_function_context() {
        let result = parse_hunk_header("@@ -1,3 +1,5 @@ fn main()");
        assert_eq!(result, Some((1, 1)));
    }

    #[test]
    fn parse_hunk_header_invalid_returns_none() {
        assert_eq!(parse_hunk_header("not a hunk"), None);
        assert_eq!(parse_hunk_header("@@"), None);
        assert_eq!(parse_hunk_header("@@ -1"), None);
        assert_eq!(parse_hunk_header(""), None);
    }

    #[test]
    fn parse_diff_patch_empty_string() {
        let theme = test_theme();
        let lines = parse_diff_patch("", &theme);
        assert!(lines.is_empty());
    }

    #[test]
    fn parse_diff_patch_with_additions_and_deletions() {
        let theme = test_theme();
        let patch = "@@ -1,3 +1,4 @@\n context\n+added\n-removed\n final";
        let lines = parse_diff_patch(patch, &theme);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn parse_diff_patch_file_headers_are_muted() {
        let theme = test_theme();
        let patch = "--- a/file.rs\n+++ b/file.rs\n@@ -1 +1 @@\n+new";
        let lines = parse_diff_patch(patch, &theme);
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn parse_diff_patch_no_newline_at_eof() {
        let theme = test_theme();
        let patch = "@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file";
        let lines = parse_diff_patch(patch, &theme);
        assert_eq!(lines.len(), 4);
    }
}
