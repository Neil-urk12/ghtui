use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
