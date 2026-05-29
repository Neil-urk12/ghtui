use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::api::client::GitHubClient;
use crate::app::{App, AppState};
use crate::cmd::Cmd;
use crate::event::{self, ApiPayload, Event};
use crate::update::{update, Message};
use crate::view::view;

const CHANNEL_BUFFER_SIZE: usize = 128;

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    tracing::debug!("entering run loop");
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(crate::constants::TICK_INTERVAL);
    let (tx, mut rx) = mpsc::channel::<Result<ApiPayload>>(CHANNEL_BUFFER_SIZE);

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
                        tracing::error!("crossterm error: {e}");
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

        // Login tick — poll for auth on each tick (async, stays in run loop)
        if matches!(app.state, AppState::Login) {
            if let Some(ref mut login) = app.login {
                login.tick().await;
                match &login.status {
                    crate::panels::login::LoginStatus::Success(token) => {
                        let token = token.clone();
                        let _ = update(app, Message::LoginSuccess(token));
                        spawn_cmd(Cmd::FetchRepos, &app.client, tx.clone());
                    }
                    crate::panels::login::LoginStatus::Error(msg) => {
                        let msg = msg.clone();
                        let _ = update(app, Message::LoginError(msg));
                    }
                    crate::panels::login::LoginStatus::Expired => {
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
        Cmd::FetchPulls { ctx, state } => {
            tokio::spawn(async move {
                let result = client
                    .get_pulls(&ctx.owner, &ctx.repo, &state)
                    .await
                    .map(ApiPayload::Pulls);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchIssues { ctx, state } => {
            tokio::spawn(async move {
                let result = client
                    .get_issues(&ctx.owner, &ctx.repo, &state)
                    .await
                    .map(ApiPayload::Issues);
                let _ = tx.send(result).await;
            });
        }
        Cmd::FetchNotifications { all } => {
            tokio::spawn(async move {
                let result = client
                    .get_notifications(all)
                    .await
                    .map(ApiPayload::Notifications);
                let _ = tx.send(result).await;
            });
        }
        Cmd::MarkNotificationRead { id } => {
            tokio::spawn(async move {
                let result = client.mark_notification_read(&id).await;
                let _ = tx
                    .send(result.map(|_| {
                        ApiPayload::Success(format!("Notification {} marked as read", id))
                    }))
                    .await;
            });
        }
        Cmd::MarkAllNotificationsRead => {
            tokio::spawn(async move {
                let result = client.mark_all_notifications_read().await;
                let _ = tx
                    .send(result.map(|_| ApiPayload::Success("All notifications marked as read".into())))
                    .await;
            });
        }
        Cmd::FetchActions { ctx } => {
            tokio::spawn(async move {
                let result = client
                    .get_actions(&ctx.owner, &ctx.repo)
                    .await
                    .map(ApiPayload::Actions);
                let _ = tx.send(result).await;
            });
        }
        Cmd::ApprovePr { ctx, pr_number } => {
            tokio::spawn(async move {
                let result = client.approve_pr(&ctx.owner, &ctx.repo, pr_number).await;
                let _ = tx
                    .send(
                        result.map(|_| ApiPayload::Success(format!("PR #{} approved", pr_number))),
                    )
                    .await;
            });
        }
        Cmd::MergePr { ctx, pr_number } => {
            tokio::spawn(async move {
                let result = client.merge_pr(&ctx.owner, &ctx.repo, pr_number).await;
                let _ = tx
                    .send(result.map(|_| ApiPayload::Success(format!("PR #{} merged", pr_number))))
                    .await;
            });
        }
        Cmd::RerunWorkflow { ctx, run_id } => {
            tokio::spawn(async move {
                let result = client.rerun_workflow(&ctx.owner, &ctx.repo, run_id).await;
                let _ = tx
                    .send(result.map(|_| {
                        ApiPayload::Success(format!("Workflow #{} rerun triggered", run_id))
                    }))
                    .await;
            });
        }
        Cmd::CancelWorkflow { ctx, run_id } => {
            tokio::spawn(async move {
                let result = client.cancel_workflow(&ctx.owner, &ctx.repo, run_id).await;
                let _ = tx
                    .send(result.map(|_| {
                            ApiPayload::Success(format!("Workflow #{} cancelled", run_id))
                        }),
                    )
                    .await;
            });
        }
        Cmd::OpenRepoDetail { .. } => {
            // Handled in update(), should not reach spawn_cmd
        }
        Cmd::CycleRepoSort => {
            // Handled in update(), should not reach spawn_cmd
        }

        Cmd::FetchPrComments { ctx, pr_number } => {
            tokio::spawn(async move {
                let (comments, reviews, code_comments, files) = tokio::join!(
                    client.get_issue_comments(&ctx.owner, &ctx.repo, pr_number),
                    client.get_reviews(&ctx.owner, &ctx.repo, pr_number),
                    client.get_review_comments(&ctx.owner, &ctx.repo, pr_number),
                    client.get_pr_files(&ctx.owner, &ctx.repo, pr_number),
                );
                let _ = tx
                    .send(Ok(ApiPayload::PrComments {
                        pr_number,
                        comments: comments.map_err(|e| format!("{:#}", e)),
                        reviews: reviews.map_err(|e| format!("{:#}", e)),
                        code_comments: code_comments.map_err(|e| format!("{:#}", e)),
                        files: files.map_err(|e| format!("{:#}", e)),
                    }))
                    .await;
            });
        }
        Cmd::ClosePr { ctx, pr_number } => {
            tokio::spawn(async move {
                let result = client.close_pr(&ctx.owner, &ctx.repo, pr_number).await;
                let _ = tx
                    .send(result.map(|_| ApiPayload::Success(format!("PR #{} closed", pr_number))))
                    .await;
            });
        }
        Cmd::ReopenPr { ctx, pr_number } => {
            tokio::spawn(async move {
                let result = client.reopen_pr(&ctx.owner, &ctx.repo, pr_number).await;
                let _ = tx
                    .send(
                        result.map(|_| ApiPayload::Success(format!("PR #{} reopened", pr_number))),
                    )
                    .await;
            });
        }
    }
}
