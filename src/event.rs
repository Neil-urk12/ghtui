use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::App;
use crate::keys;
use crate::update::Message;

#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Tick,
    ApiResult { payload: ApiPayload },
    ApiError(String),
}

#[derive(Debug)]
pub enum ApiPayload {
    Repos(Vec<crate::domain::Repo>),
    Pulls(Vec<crate::domain::PullRequest>),
    Issues(Vec<crate::domain::Issue>),
    Notifications(Vec<crate::domain::Notification>),
    Actions(Vec<crate::domain::WorkflowRun>),
    Success(String),
}

pub fn process_event(event: Event, app: &App) -> Vec<Message> {
    match event {
        Event::Input(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return vec![Message::Quit];
            }

            if app.show_help {
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('?') => vec![Message::ToggleHelp],
                    _ => vec![Message::Noop],
                };
            }

            if app.search.active {
                return match key.code {
                    KeyCode::Esc => vec![Message::SearchClear],
                    KeyCode::Enter => vec![Message::SearchSubmit],
                    KeyCode::Backspace => vec![Message::SearchBackspace],
                    KeyCode::Char(c) => vec![Message::SearchInput(c)],
                    _ => vec![Message::Noop],
                };
            }

            if app.error_toast.is_some() {
                return vec![Message::DismissError];
            }
            if app.success_toast.is_some() {
                return vec![Message::DismissSuccess];
            }

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
            } else if key.code == KeyCode::Char('J') {
                vec![Message::DetailScrollDown]
            } else if key.code == KeyCode::Char('K') {
                vec![Message::DetailScrollUp]
            } else if let KeyCode::Char(c) = key.code {
                vec![Message::PanelKeyInput(c)]
            } else if let KeyCode::Enter = key.code {
                vec![Message::PanelKeyInput('\n')]
            } else {
                vec![Message::Noop]
            }
        }
        Event::ApiResult { payload } => {
            let is_success = matches!(payload, ApiPayload::Success(_));
            let mut msgs = vec![Message::ApiLoaded(payload)];
            if is_success {
                msgs.push(Message::Refresh);
            }
            msgs
        }
        Event::ApiError(msg) => vec![Message::ApiError(msg)],
        Event::Tick => {
            let (remaining, reset) = app.client.rate_info();
            let reset_secs = reset.map(|inst| {
                let dur = inst.saturating_duration_since(std::time::Instant::now());
                dur.as_secs()
            });
            vec![Message::RateLimitUpdate { remaining, reset_secs }, Message::Tick]
        }
        _ => vec![Message::Noop],
    }
}