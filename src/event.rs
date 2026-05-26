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
    PrComments {
        pr_number: u64,
        comments: Result<Vec<crate::domain::PrComment>, String>,
        reviews: Result<Vec<crate::domain::PrReview>, String>,
        code_comments: Result<Vec<crate::domain::PrCodeComment>, String>,
    },
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

            if app.error_toast.is_some() || app.success_toast.is_some() {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('x')) {
                    let mut msgs = Vec::new();
                    if app.error_toast.is_some() {
                        msgs.push(Message::DismissError);
                    }
                    if app.success_toast.is_some() {
                        msgs.push(Message::DismissSuccess);
                    }
                    return msgs;
                }
                return vec![Message::Noop];
            }


            if app.confirmation.is_some() {
                return match key.code {
                    KeyCode::Char('y') => vec![Message::ConfirmYes],
                    KeyCode::Char('n') | KeyCode::Esc => vec![Message::ConfirmNo],
                    _ => vec![Message::Noop],
                };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;

    fn test_app() -> App {
        App::new(Config::default(), Some("test-token".into()))
    }

    fn key_event(code: KeyCode) -> CrosstermEvent {
        CrosstermEvent::Key(crossterm::event::KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn key_event_mod(code: KeyCode, mods: KeyModifiers) -> CrosstermEvent {
        CrosstermEvent::Key(crossterm::event::KeyEvent::new(code, mods))
    }

    #[test]
    fn ctrl_c_quits() {
        let app = test_app();
        let msgs = process_event(Event::Input(key_event_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)), &app);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], Message::Quit));
    }

    #[test]
    fn help_overlay_captures_keys() {
        let mut app = test_app();
        app.show_help = true;

        // Any key other than Esc/? -> Noop
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('j'))), &app);
        assert!(matches!(msgs[0], Message::Noop));

        // Esc dismisses help
        let msgs = process_event(Event::Input(key_event(KeyCode::Esc)), &app);
        assert!(matches!(msgs[0], Message::ToggleHelp));

        // ? dismisses help
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('?'))), &app);
        assert!(matches!(msgs[0], Message::ToggleHelp));
    }

    #[test]
    fn search_mode_captures_keys() {
        let mut app = test_app();
        app.search.active = true;

        let msgs = process_event(Event::Input(key_event(KeyCode::Char('a'))), &app);
        assert!(matches!(msgs[0], Message::SearchInput('a')));

        let msgs = process_event(Event::Input(key_event(KeyCode::Backspace)), &app);
        assert!(matches!(msgs[0], Message::SearchBackspace));

        let msgs = process_event(Event::Input(key_event(KeyCode::Enter)), &app);
        assert!(matches!(msgs[0], Message::SearchSubmit));

        let msgs = process_event(Event::Input(key_event(KeyCode::Esc)), &app);
        assert!(matches!(msgs[0], Message::SearchClear));
    }

    #[test]
    fn toast_dismissal() {
        let mut app = test_app();

        app.error_toast = Some("err".into());
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('x'))), &app);
        assert!(matches!(msgs[0], Message::DismissError));

        // Esc also dismisses
        let msgs = process_event(Event::Input(key_event(KeyCode::Esc)), &app);
        assert!(matches!(msgs[0], Message::DismissError));

        // Other keys blocked while toast visible
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('j'))), &app);
        assert!(matches!(msgs[0], Message::Noop));

        app.error_toast = None;
        app.success_toast = Some("ok".into());
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('x'))), &app);
        assert!(matches!(msgs[0], Message::DismissSuccess));
    }

    #[test]
    fn j_k_detail_scroll() {
        let app = test_app();

        let msgs = process_event(Event::Input(key_event(KeyCode::Char('J'))), &app);
        assert!(matches!(msgs[0], Message::DetailScrollDown));

        let msgs = process_event(Event::Input(key_event(KeyCode::Char('K'))), &app);
        assert!(matches!(msgs[0], Message::DetailScrollUp));
    }

    #[test]
    fn regular_char_becomes_panel_input() {
        let app = test_app();
        let msgs = process_event(Event::Input(key_event(KeyCode::Char('j'))), &app);
        assert!(matches!(msgs[0], Message::PanelKeyInput('j')));
    }

    #[test]
    fn enter_becomes_panel_input_newline() {
        let app = test_app();
        let msgs = process_event(Event::Input(key_event(KeyCode::Enter)), &app);
        assert!(matches!(msgs[0], Message::PanelKeyInput('\n')));
    }

    #[test]
    fn api_result_success_produces_loaded_and_refresh() {
        let app = test_app();
        let msgs = process_event(Event::ApiResult { payload: ApiPayload::Success("ok".into()) }, &app);
        assert_eq!(msgs.len(), 2);
        assert!(matches!(&msgs[0], Message::ApiLoaded(_)));
        assert!(matches!(msgs[1], Message::Refresh));
    }

    #[test]
    fn api_result_data_produces_loaded_only() {
        let app = test_app();
        let msgs = process_event(Event::ApiResult { payload: ApiPayload::Repos(vec![]) }, &app);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0], Message::ApiLoaded(_)));
    }

    #[test]
    fn api_error_produces_message() {
        let app = test_app();
        let msgs = process_event(Event::ApiError("fail".into()), &app);
        assert!(matches!(&msgs[0], Message::ApiError(msg) if msg == "fail"));
    }

    #[test]
    fn tick_produces_rate_limit_and_tick() {
        let app = test_app();
        let msgs = process_event(Event::Tick, &app);
        assert_eq!(msgs.len(), 2);
        assert!(matches!(&msgs[0], Message::RateLimitUpdate { .. }));
        assert!(matches!(msgs[1], Message::Tick));
    }
}