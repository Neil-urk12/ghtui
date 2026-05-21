use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::api::client::GitHubClient;
use crate::api::cache::CacheKey;
use crate::app::{view, App, AppState};
use crate::update::{update, Message};
use crate::cmd::Cmd;
use crate::event::{self, ApiPayload, Event};

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    eprintln!("ghtui: entering run loop");
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(Duration::from_millis(250));
    let (tx, mut rx) = mpsc::channel::<Result<ApiPayload>>(128);

    // Initialize login if no token found
    if matches!(app.state, AppState::Login) {
        let http = reqwest::Client::new();
        match crate::panels::login::LoginState::new(http).await {
            Ok(login) => {
                let _ = update(app, Message::LoginInit(login));
            }
            Err(e) => {
                let _ = update(app, Message::LoginError(e.to_string()));
            }
        }
    } else {
        // Auto-fetch repos on startup (only when authenticated)
        spawn_cmd(Cmd::FetchRepos, &app.client, tx.clone());
    }

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

        let messages = event::process_event(event, app);

        // Login tick — poll for auth on each tick
        if matches!(app.state, AppState::Login) {
            if let Some(ref mut login) = app.login {
                login.tick().await;
                match &login.status {
                    crate::panels::login::LoginStatus::Success(token) => {
                        let token = token.clone();
                        let _ = update(app, Message::LoginSuccess(token));
                        // Start fetching repos now that we're authenticated
                        spawn_cmd(Cmd::FetchRepos, &app.client, tx.clone());
                    }
                    crate::panels::login::LoginStatus::Error(msg) => {
                        let msg = msg.clone();
                        let _ = update(app, Message::LoginError(msg));
                    }
                    crate::panels::login::LoginStatus::Expired => {
                        // Auto-restart device flow
                        let http = reqwest::Client::new();
                        if let Ok(new_login) = crate::panels::login::LoginState::new(http).await {
                            app.login = Some(new_login);
                        }
                    }
                    _ => {}
                }
            }
        }

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
        Cmd::OpenRepoDetail { .. } => {
            // Handled in update(), should not reach spawn_cmd
        }
    }
}
