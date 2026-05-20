use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::time;

use crate::app::{update, view, App, Message};
use crate::event::Event;
use crate::keys;

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(Duration::from_millis(250));

    loop {
        terminal.draw(|frame| view(frame, app))?;

        let event = tokio::select! {
            Some(Ok(crossterm_event)) = event_stream.next() => Event::Input(crossterm_event),
            _ = tick_interval.tick() => Event::Tick,
        };

        let messages = process_event(event, app);

        for msg in messages {
            let cmd = update(app, msg);
            // TODO: execute commands (spawn async tasks, send results back)
            let _ = cmd;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn process_event(event: Event, app: &App) -> Vec<Message> {
    match event {
        Event::Input(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
            // Ctrl+C always quits
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return vec![Message::Quit];
            }

            // Help overlay takes precedence
            if app.show_help {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('?') => vec![Message::ToggleHelp],
                    _ => vec![Message::Noop],
                };
            }

            // Search mode: capture all input
            if app.search.active {
                return match key.code {
                    KeyCode::Esc => vec![Message::SearchClear],
                    KeyCode::Enter => vec![Message::SearchSubmit],
                    KeyCode::Backspace => vec![Message::SearchBackspace],
                    KeyCode::Char(c) => vec![Message::SearchInput(c)],
                    _ => vec![Message::Noop],
                };
            }

            // Dismiss error toast on any key
            if app.error_toast.is_some() {
                return vec![Message::DismissError];
            }

            // Normal mode
            let bindings = &app.config.keybindings;
            if keys::matches_key(key, &bindings.quit) {
                vec![Message::Quit]
            } else if keys::matches_key(key, &bindings.refresh) {
                vec![Message::Refresh]
            } else if keys::matches_key(key, &bindings.next_tab) {
                vec![Message::NextTab]
            } else if keys::matches_key(key, &bindings.prev_tab) {
                vec![Message::PrevTab]
            } else if keys::matches_key(key, &bindings.search) {
                vec![Message::SearchActivate]
            } else if keys::matches_key(key, &bindings.help) {
                vec![Message::ToggleHelp]
            } else if let KeyCode::Char(c) = key.code {
                vec![Message::PanelKeyInput(c)]
            } else if let KeyCode::Enter = key.code {
                vec![Message::PanelKeyInput('\n')]
            } else {
                vec![Message::Noop]
            }
        }
        Event::Tick => vec![Message::Noop],
        _ => vec![Message::Noop],
    }
}
