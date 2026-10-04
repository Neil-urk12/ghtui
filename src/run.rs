use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::time;

use crate::api::client::{build_http_client, GitHubClient};
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
        let http = build_http_client()?;
        match crate::panels::login::LoginState::new(http).await {
            Ok(login) => {
                // Intentionally discarded: always returns Cmd::None
                let _ = update(app, Message::LoginInit(login));
            }
            Err(e) => {
                // Intentionally discarded: always returns Cmd::None
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
                        tracing::error!(error = %e, "crossterm error");
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
                        let cmd = update(app, Message::LoginSuccess(token));
                        if !matches!(cmd, Cmd::None) {
                            spawn_cmd(cmd, &app.client, tx.clone());
                        }
                    }
                    crate::panels::login::LoginStatus::Error(msg) => {
                        let msg = msg.clone();
                        // Intentionally discarded: always returns Cmd::None
                        let _ = update(app, Message::LoginError(msg));
                    }
                    crate::panels::login::LoginStatus::Expired => {
                        let http = build_http_client()?;
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
    if matches!(
        cmd,
        Cmd::None | Cmd::OpenRepoDetail { .. } | Cmd::CycleRepoSort
    ) {
        // Local commands are handled in update().
        return;
    }
    let client = Arc::clone(client);
    tokio::spawn(async move {
        let result = match cmd {
            Cmd::None | Cmd::OpenRepoDetail { .. } | Cmd::CycleRepoSort => return,
            Cmd::FetchRepos => client.get_repos().await.map(ApiPayload::Repos),
            Cmd::FetchPulls { ctx, state } => client
                .get_pulls(&ctx.owner, &ctx.repo, &state)
                .await
                .map(ApiPayload::Pulls),
            Cmd::FetchIssues { ctx, state } => client
                .get_issues(&ctx.owner, &ctx.repo, &state)
                .await
                .map(ApiPayload::Issues),
            Cmd::FetchNotifications { all } => client
                .get_notifications(all)
                .await
                .map(ApiPayload::Notifications),
            Cmd::MarkNotificationRead { id } => client
                .mark_notification_read(&id)
                .await
                .map(|_| ApiPayload::Success(format!("Notification {} marked as read", id))),
            Cmd::MarkAllNotificationsRead => client
                .mark_all_notifications_read()
                .await
                .map(|_| ApiPayload::Success("All notifications marked as read".into())),
            Cmd::FetchActions { ctx } => client
                .get_actions(&ctx.owner, &ctx.repo)
                .await
                .map(ApiPayload::Actions),
            Cmd::ApprovePr { ctx, pr_number } => client
                .approve_pr(&ctx.owner, &ctx.repo, pr_number)
                .await
                .map(|_| ApiPayload::Success(format!("PR #{} approved", pr_number))),
            Cmd::MergePr { ctx, pr_number } => client
                .merge_pr(&ctx.owner, &ctx.repo, pr_number)
                .await
                .map(|_| ApiPayload::Success(format!("PR #{} merged", pr_number))),
            Cmd::RerunWorkflow { ctx, run_id } => client
                .rerun_workflow(&ctx.owner, &ctx.repo, run_id)
                .await
                .map(|_| ApiPayload::Success(format!("Workflow #{} rerun triggered", run_id))),
            Cmd::CancelWorkflow { ctx, run_id } => client
                .cancel_workflow(&ctx.owner, &ctx.repo, run_id)
                .await
                .map(|_| ApiPayload::Success(format!("Workflow #{} cancelled", run_id))),
            Cmd::FetchPrComments { ctx, pr_number } => {
                let (comments, reviews, code_comments, files) = tokio::join!(
                    client.get_issue_comments(&ctx.owner, &ctx.repo, pr_number),
                    client.get_reviews(&ctx.owner, &ctx.repo, pr_number),
                    client.get_review_comments(&ctx.owner, &ctx.repo, pr_number),
                    client.get_pr_files(&ctx.owner, &ctx.repo, pr_number),
                );
                Ok(ApiPayload::PrComments {
                    pr_number,
                    comments: comments.map_err(|e| format!("{:#}", e)),
                    reviews: reviews.map_err(|e| format!("{:#}", e)),
                    code_comments: code_comments.map_err(|e| format!("{:#}", e)),
                    files: files.map_err(|e| format!("{:#}", e)),
                })
            }
            Cmd::ClosePr { ctx, pr_number } => client
                .close_pr(&ctx.owner, &ctx.repo, pr_number)
                .await
                .map(|_| ApiPayload::Success(format!("PR #{} closed", pr_number))),
            Cmd::ReopenPr { ctx, pr_number } => client
                .reopen_pr(&ctx.owner, &ctx.repo, pr_number)
                .await
                .map(|_| ApiPayload::Success(format!("PR #{} reopened", pr_number))),
        };
        let _ = tx.send(result).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{PanelKind, PanelState};
    use crate::cmd::RepoContext;
    use crate::config::Config;
    use crate::domain::{PrReview, PullRequest};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn pr_comment_results_preserve_partial_success_and_retry() {
        let server = MockServer::start().await;
        for endpoint in [
            "/repos/o/r/issues/1/comments",
            "/repos/o/r/pulls/1/comments",
            "/repos/o/r/pulls/1/files",
        ] {
            Mock::given(method("GET"))
                .and(path(endpoint))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
                .expect(1)
                .mount(&server)
                .await;
        }
        Mock::given(method("GET"))
            .and(path("/repos/o/r/pulls/1/reviews"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;

        let client = Arc::new(GitHubClient::new_for_test("tok".into(), server.uri()).unwrap());
        let (tx, mut rx) = mpsc::channel(1);
        spawn_cmd(
            Cmd::FetchPrComments {
                ctx: RepoContext::new("o", "r"),
                pr_number: 1,
            },
            &client,
            tx,
        );
        let payload = time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let mut app = App::new(Config::default(), Some("tok".into())).unwrap();
        update(
            &mut app,
            Message::ApiLoaded(ApiPayload::Pulls(vec![PullRequest {
                number: 1,
                reviews: Some(vec![PrReview::default()]),
                comment_errors: Some("previous failure".into()),
                code_comment_errors: Some("previous failure".into()),
                file_errors: Some("previous failure".into()),
                ..Default::default()
            }])),
        );
        update(&mut app, Message::ApiLoaded(payload));
        assert_eq!(app.active_panel, PanelKind::Repos);
        let PanelState::Pulls(panel) = app.panels.get(&PanelKind::Pulls).unwrap() else {
            panic!("Expected Pulls panel");
        };
        let pr = &panel.data.items[0];
        assert!(pr.comments.as_ref().unwrap().is_empty());
        assert!(pr.code_comments.as_ref().unwrap().is_empty());
        assert!(pr.files.as_ref().unwrap().is_empty());
        assert!(pr.comment_errors.is_none());
        assert!(pr.code_comment_errors.is_none());
        assert!(pr.file_errors.is_none());
        assert!(pr.reviews.is_none());
        assert!(pr.review_errors.as_ref().unwrap().contains("500"));

        update(
            &mut app,
            Message::ApiLoaded(ApiPayload::PrComments {
                pr_number: 1,
                comments: Err("comments failed".into()),
                reviews: Ok(vec![PrReview::default()]),
                code_comments: Err("code comments failed".into()),
                files: Err("files failed".into()),
            }),
        );
        let PanelState::Pulls(panel) = app.panels.get(&PanelKind::Pulls).unwrap() else {
            panic!("Expected Pulls panel");
        };
        let pr = &panel.data.items[0];
        assert_eq!(pr.reviews.as_ref().unwrap().len(), 1);
        assert!(pr.review_errors.is_none());
        assert!(pr.comments.is_none());
        assert!(pr.code_comments.is_none());
        assert!(pr.files.is_none());
        assert_eq!(pr.comment_errors.as_deref(), Some("comments failed"));
        assert_eq!(
            pr.code_comment_errors.as_deref(),
            Some("code comments failed")
        );
        assert_eq!(pr.file_errors.as_deref(), Some("files failed"));
    }

    #[tokio::test]
    async fn api_failure_is_sent_through_channel() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/repos"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;
        let client = Arc::new(GitHubClient::new_for_test("tok".into(), server.uri()).unwrap());
        let (tx, mut rx) = mpsc::channel(1);
        spawn_cmd(Cmd::FetchRepos, &client, tx);
        let result = time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(result.unwrap_err().to_string().contains("500"));
    }
}
