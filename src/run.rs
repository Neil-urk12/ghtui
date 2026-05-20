use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::time;

use crate::app::{update, view, App, Message};
use crate::event::Event;

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(Duration::from_millis(250));

    loop {
        terminal.draw(|frame| view(frame, app))?;

        let event = tokio::select! {
            Some(Ok(crossterm_event)) = event_stream.next() => Event::Input(crossterm_event),
            _ = tick_interval.tick() => Event::Tick,
        };

        let messages = process_event(event);

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

fn process_event(event: Event) -> Vec<Message> {
    match event {
        Event::Input(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
            // Global key handling
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return vec![Message::Quit];
            }
            match key.code {
                KeyCode::Char('q') => vec![Message::Quit],
                KeyCode::Tab => vec![Message::NextTab],
                KeyCode::BackTab => vec![Message::PrevTab],
                KeyCode::Char('r') => vec![Message::Refresh],
                _ => vec![Message::Noop],
            }
        }
        Event::Tick => vec![Message::Noop],
        _ => vec![Message::Noop],
    }
}
