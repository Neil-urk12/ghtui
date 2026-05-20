use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::api::client::GitHubClient;
use crate::api::cache::CacheKey;
use crate::app::{update, view, App, Message, PanelKind, PanelState};
use crate::cmd::Cmd;
use crate::event::{ApiPayload, Event};
use crate::keys;

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    eprintln!("ghtui: entering run loop");
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(Duration::from_millis(250));
    let (tx, mut rx) = mpsc::channel::<Result<ApiPayload>>(128);

    // Auto-fetch repos on startup
    spawn_cmd(Cmd::FetchRepos, &app.client, tx.clone());

    loop {
        terminal.draw(|frame| view(frame, app))?;

        let event = tokio::select! {
            crossterm_event = event_stream.next() => {
                match crossterm_event {
                    Some(Ok(event)) => Event::Input(event),
                    Some(Err(e)) => {
                        eprintln!("crossterm error: {e}");
                        Event::Tick
                    }
                    None => break,
                }
            }
            api_result = rx.recv() => {
                match api_result {
                    Some(Ok(payload)) => Event::ApiResult { payload },
                    Some(Err(e)) => Event::ApiError(e.to_string()),
                    None => Event::Tick,
                }
            }
            _ = tick_interval.tick() => Event::Tick,
        };

        let messages = process_event(event, app);

        for msg in messages {
            let cmd = update(app, msg);
            if !matches!(cmd, Cmd::None) {
                spawn_cmd(cmd, &app.client, tx.clone());
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn spawn_cmd(cmd: Cmd, client: &Arc<GitHubClient>, tx: mpsc::Sender<Result<ApiPayload>>) {
    let client = Arc::clone(client);
    match cmd {
        Cmd::None => {}
        Cmd::FetchRepos => {
            tokio::spawn(async move {
                let result = client.get_repos().await.map(ApiPayload::Repos);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchPulls { owner, repo, state } => {
            tokio::spawn(async move {
                let result = client
                    .get_pulls(&owner, &repo, &state)
                    .await
                    .map(ApiPayload::Pulls);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchIssues { owner, repo, state } => {
            tokio::spawn(async move {
                let result = client
                    .get_issues(&owner, &repo, &state)
                    .await
                    .map(ApiPayload::Issues);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchNotifications { all } => {
            tokio::spawn(async move {
                let result = client.get_notifications(all).await.map(ApiPayload::Notifications);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchActions { owner, repo } => {
            tokio::spawn(async move {
                let result = client
                    .get_actions(&owner, &repo)
                    .await
                    .map(ApiPayload::Actions);
                let _ = tx.send(result).await;
            });
        }
        Cmd::MarkNotificationRead { id } => {
            tokio::spawn(async move {
                let result = client
                    .mark_notification_read(&id)
                    .await;
                if result.is_ok() {
                    client.invalidate_cache(&CacheKey::Notifications);
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success("Notification marked as read".into()))).await;
            });
        }
        Cmd::MarkAllNotificationsRead => {
            tokio::spawn(async move {
                let result = client
                    .mark_all_notifications_read()
                    .await;
                if result.is_ok() {
                    client.invalidate_cache(&CacheKey::Notifications);
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success("All notifications marked as read".into()))).await;
            });
        }
        Cmd::ApprovePr { owner, repo, pr_number } => {
            tokio::spawn(async move {
                let result = client
                    .approve_pr(&owner, &repo, pr_number)
                    .await;
                if result.is_ok() {
                    for state in ["open", "closed", "all"] {
                        client.invalidate_cache(&CacheKey::Pulls { owner: owner.clone(), repo: repo.clone(), state: state.into() });
                    }
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success(format!("PR #{} approved", pr_number)))).await;
            });
        }
        Cmd::MergePr { owner, repo, pr_number } => {
            tokio::spawn(async move {
                let result = client
                    .merge_pr(&owner, &repo, pr_number)
                    .await;
                if result.is_ok() {
                    for state in ["open", "closed", "all"] {
                        client.invalidate_cache(&CacheKey::Pulls { owner: owner.clone(), repo: repo.clone(), state: state.into() });
                    }
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success(format!("PR #{} merged", pr_number)))).await;
            });
        }
        Cmd::RerunWorkflow { owner, repo, run_id } => {
            tokio::spawn(async move {
                let result = client
                    .rerun_workflow(&owner, &repo, run_id)
                    .await;
                if result.is_ok() {
                    client.invalidate_cache(&CacheKey::Actions { owner: owner.clone(), repo: repo.clone() });
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success(format!("Workflow #{} rerun triggered", run_id)))).await;
            });
        }
        Cmd::CancelWorkflow { owner, repo, run_id } => {
            tokio::spawn(async move {
                let result = client
                    .cancel_workflow(&owner, &repo, run_id)
                    .await;
                if result.is_ok() {
                    client.invalidate_cache(&CacheKey::Actions { owner: owner.clone(), repo: repo.clone() });
                }
                let _ = tx.send(result.map(|_| ApiPayload::Success(format!("Workflow #{} cancelled", run_id)))).await;
            });
        }
    }
}

fn process_event(event: Event, app: &mut App) -> Vec<Message> {
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
                // Trigger refresh of current panel after successful mutation
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
            // Sync rate limits from client to app state
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
