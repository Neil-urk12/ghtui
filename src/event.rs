use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::{App, PanelKind, PanelState};
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

pub fn process_event(event: Event, app: &mut App) -> Vec<Message> {
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
            handle_api_result(app, payload);
            if is_success {
                vec![Message::Refresh]
            } else {
                vec![Message::Noop]
            }
        }
        Event::ApiError(msg) => {
            app.error_toast = Some(msg);
            vec![Message::Noop]
        }
        Event::Tick => {
            if let Ok(remaining) = app.client.rate_remaining.lock() {
                app.rate_remaining = *remaining;
            }
            if let Ok(reset) = app.client.rate_reset.lock() {
                app.rate_reset_secs = reset.map(|inst| {
                    let dur = inst.saturating_duration_since(std::time::Instant::now());
                    dur.as_secs()
                });
            }
            app.tick += 1;
            vec![Message::Noop]
        }
        _ => vec![Message::Noop],
    }
}

fn handle_api_result(app: &mut App, payload: ApiPayload) {
    match payload {
        ApiPayload::Repos(repos) => {
            if let Some(PanelState::Repos(ref mut panel)) = app.panels.get_mut(&PanelKind::Repos) {
                panel.set_repos(repos);
            }
        }
        ApiPayload::Pulls(pulls) => {
            if let Some(PanelState::Pulls(ref mut panel)) = app.panels.get_mut(&PanelKind::Pulls) {
                panel.set_pulls(pulls);
            }
        }
        ApiPayload::Issues(issues) => {
            if let Some(PanelState::Issues(ref mut panel)) = app.panels.get_mut(&PanelKind::Issues) {
                panel.set_issues(issues);
            }
        }
        ApiPayload::Notifications(notifs) => {
            if let Some(PanelState::Notifications(ref mut panel)) =
                app.panels.get_mut(&PanelKind::Notifications)
            {
                panel.set_notifications(notifs);
            }
        }
        ApiPayload::Actions(runs) => {
            if let Some(PanelState::Actions(ref mut panel)) =
                app.panels.get_mut(&PanelKind::Actions)
            {
                panel.runs = runs;
                panel.loading = false;
            }
        }
        ApiPayload::Success(msg) => {
            app.success_toast = Some(msg);
        }
    }
}