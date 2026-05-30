use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Panel-specific hints
pub const HINT_REPOS: &str = "Enter: PRs | s: sort";
pub const HINT_PULLS: &str = "1:open 2:closed 3:all | o:browser";
pub const HINT_ISSUES: &str = "1:open 2:closed 3:all | o:browser";
pub const HINT_NOTIFICATIONS: &str = "a:all/unread";
pub const HINT_ACTIONS: &str = "Enter: expand | v: logs";

// Global hints
pub const HINT_GLOBAL: &str = "j/k: nav | Tab: switch | /: search | ?: help | q: quit";

/// Check if a key event matches a configured binding string.
/// Supports: single chars ("q"), special keys ("Tab", "Enter", "Esc", "BackTab"),
/// and modifiers ("Ctrl+c", "Shift-Tab").
pub fn matches_key(event: KeyEvent, binding: &str) -> bool {
    let binding = binding.trim();

    // Handle modifier+key combos
    if let Some(rest) = binding.strip_prefix("Ctrl+") {
        let Some(c) = rest.chars().next() else {
            return false;
        };
        return event.code == KeyCode::Char(c) && event.modifiers.contains(KeyModifiers::CONTROL);
    }

    // Handle Shift-Tab explicitly
    if binding == "Shift-Tab" {
        return event.code == KeyCode::BackTab;
    }

    // Handle special keys
    match binding {
        "Tab" => event.code == KeyCode::Tab,
        "Enter" => event.code == KeyCode::Enter,
        "Esc" | "Escape" => event.code == KeyCode::Esc,
        "Backspace" => event.code == KeyCode::Backspace,
        "Delete" => event.code == KeyCode::Delete,
        "Up" => event.code == KeyCode::Up,
        "Down" => event.code == KeyCode::Down,
        "Left" => event.code == KeyCode::Left,
        "Right" => event.code == KeyCode::Right,
        "Home" => event.code == KeyCode::Home,
        "End" => event.code == KeyCode::End,
        "PageUp" => event.code == KeyCode::PageUp,
        "PageDown" => event.code == KeyCode::PageDown,
        "Space" => event.code == KeyCode::Char(' '),
        "/" => event.code == KeyCode::Char('/'),
        "?" => event.code == KeyCode::Char('?'),
        _ => {
            // Single char binding
            if let Some(c) = binding.chars().next() {
                event.code == KeyCode::Char(c) && event.modifiers.is_empty()
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_mod(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn single_char_binding() {
        assert!(matches_key(key(KeyCode::Char('q')), "q"));
        assert!(matches_key(key(KeyCode::Char('r')), "r"));
        assert!(!matches_key(key(KeyCode::Char('x')), "q"));
    }

    #[test]
    fn single_char_rejects_modifiers() {
        assert!(!matches_key(
            key_mod(KeyCode::Char('q'), KeyModifiers::CONTROL),
            "q"
        ));
    }

    #[test]
    fn ctrl_combo() {
        assert!(matches_key(
            key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL),
            "Ctrl+c"
        ));
        assert!(!matches_key(key(KeyCode::Char('c')), "Ctrl+c"));
    }

    #[test]
    fn shift_tab() {
        assert!(matches_key(key(KeyCode::BackTab), "Shift-Tab"));
        assert!(!matches_key(key(KeyCode::Tab), "Shift-Tab"));
    }

    #[test]
    fn special_keys() {
        assert!(matches_key(key(KeyCode::Tab), "Tab"));
        assert!(matches_key(key(KeyCode::Enter), "Enter"));
        assert!(matches_key(key(KeyCode::Esc), "Esc"));
        assert!(matches_key(key(KeyCode::Esc), "Escape"));
        assert!(matches_key(key(KeyCode::Backspace), "Backspace"));
        assert!(matches_key(key(KeyCode::Up), "Up"));
        assert!(matches_key(key(KeyCode::Down), "Down"));
        assert!(matches_key(key(KeyCode::PageUp), "PageUp"));
    }

    #[test]
    fn slash_and_question() {
        assert!(matches_key(key(KeyCode::Char('/')), "/"));
        assert!(matches_key(key(KeyCode::Char('?')), "?"));
    }

    #[test]
    fn space_binding() {
        assert!(matches_key(key(KeyCode::Char(' ')), "Space"));
    }

    #[test]
    fn empty_binding() {
        assert!(!matches_key(key(KeyCode::Char('q')), ""));
    }

    #[test]
    fn whitespace_trimmed() {
        assert!(matches_key(key(KeyCode::Char('q')), "  q  "));
    }
}
