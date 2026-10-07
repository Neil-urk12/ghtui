use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time;

use crate::api::client::{build_http_client, GitHubClient};
use crate::app::{App, AppState};
use crate::cmd::Cmd;
use crate::event::{self, ApiPayload, Event};
use crate::panels::login::{LoginState, LoginStatus};
use crate::update::{update, Message};
use crate::view::view;

const CHANNEL_BUFFER_SIZE: usize = 128;

pub async fn run(app: &mut App, terminal: &mut DefaultTerminal) -> Result<()> {
    tracing::debug!("entering run loop");
    let mut event_stream = EventStream::new();
    let mut tick_interval = time::interval(crate::constants::TICK_INTERVAL);
    let (tx, mut rx) = mpsc::channel::<Message>(CHANNEL_BUFFER_SIZE);
    let mut login_tasks = JoinSet::new();

    if matches!(app.state, AppState::Login) {
        spawn_login(None, &mut login_tasks, tx.clone());
    } else {
        spawn_cmd(Cmd::FetchRepos, &app.client, tx.clone());
    }

    loop {
        terminal.draw(|frame| view(frame, app))?;

        let messages = tokio::select! {
            crossterm_event = event_stream.next() => {
                let event = match crossterm_event {
                    Some(Ok(event)) => Event::Input(event),
                    Some(Err(e)) => {
                        tracing::error!(error = %e, "crossterm error");
                        Event::Tick
                    }
                    None => break,
                };
                event::process_event(event, app)
            }
            result = rx.recv() => {
                match result {
                    Some(Message::ApiLoaded(payload)) => {
                        event::process_event(Event::ApiResult { payload }, app)
                    }
                    Some(Message::ApiError(msg)) => {
                        event::process_event(Event::ApiError(msg), app)
                    }
                    Some(msg @ (Message::LoginInit(_) | Message::LoginSuccess(_) | Message::LoginError(_))) => {
                        // The request is done; retain no task while waiting for the next poll.
                        login_tasks.detach_all();
                        vec![msg]
                    }
                    Some(msg) => vec![msg],
                    None => vec![Message::Noop],
                }
            }
            _ = tick_interval.tick() => event::process_event(Event::Tick, app),
        };

        for msg in messages {
            let cmd = update(app, msg);
            if !matches!(cmd, Cmd::None) {
                spawn_cmd(cmd, &app.client, tx.clone());
            }
        }

        if app.should_quit {
            break;
        }
        schedule_login(app, &mut login_tasks, tx.clone());
    }
    Ok(())
}

fn schedule_login(app: &mut App, tasks: &mut JoinSet<()>, tx: mpsc::Sender<Message>) {
    if !matches!(app.state, AppState::Login) || !tasks.is_empty() {
        return;
    }
    if let Some(login) = &mut app.login {
        if !matches!(login.status, LoginStatus::Waiting | LoginStatus::Expired) {
            return;
        }
        if std::time::Instant::now() >= login.expires_at {
            login.status = LoginStatus::Expired;
            spawn_login(None, tasks, tx);
        } else if login.poll_due() {
            spawn_login(Some(login.clone()), tasks, tx);
        }
    }
}

fn spawn_login(login: Option<LoginState>, tasks: &mut JoinSet<()>, tx: mpsc::Sender<Message>) {
    // JoinSet aborts an outstanding login request when the run loop exits.
    tasks.spawn(async move {
        let result = match login {
            Some(mut login) => {
                login.tick().await;
                Ok(login)
            }
            None => match build_http_client() {
                Ok(http) => LoginState::new(http).await,
                Err(e) => Err(e),
            },
        };
        let msg = match result {
            Ok(login) => match &login.status {
                LoginStatus::Success(token) => Message::LoginSuccess(token.clone()),
                LoginStatus::Error(msg) => Message::LoginError(msg.clone()),
                _ => Message::LoginInit(login),
            },
            Err(e) => Message::LoginError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn spawn_cmd(cmd: Cmd, client: &Arc<GitHubClient>, tx: mpsc::Sender<Message>) {
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
        let msg = match result {
            Ok(payload) => Message::ApiLoaded(payload),
            Err(e) => Message::ApiError(e.to_string()),
        };
        let _ = tx.send(msg).await;
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
    async fn delayed_login_keeps_input_and_redraws_responsive_without_overlapping_polls() {
        use std::time::{Duration, Instant};
        use tokio::io::AsyncReadExt;

        // A local proxy accepts the connection but never answers the HTTPS tunnel request.
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(format!("http://{}", proxy.local_addr().unwrap())).unwrap())
            .build()
            .unwrap();
        let mut app = App::new(Config::default(), None).unwrap();
        app.login = Some(LoginState {
            user_code: "TEST-CODE".into(),
            verification_uri: "https://github.com/login/device".into(),
            device_code: "test-device".into(),
            expires_at: Instant::now() + Duration::from_secs(60),
            poll_interval: Duration::from_secs(5),
            last_poll: Instant::now(),
            status: LoginStatus::Waiting,
            http_client: http,
        });
        let (tx, mut rx) = mpsc::channel(1);
        let mut tasks = JoinSet::new();
        schedule_login(&mut app, &mut tasks, tx.clone());
        assert!(
            tasks.is_empty(),
            "must wait for the server's polling interval"
        );

        app.login.as_mut().unwrap().last_poll = Instant::now() - Duration::from_secs(6);
        schedule_login(&mut app, &mut tasks, tx.clone());
        let (mut connection, _) = time::timeout(Duration::from_secs(2), proxy.accept())
            .await
            .unwrap()
            .unwrap();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        for _ in 0..3 {
            for message in event::process_event(Event::Tick, &app) {
                update(&mut app, message);
            }
            schedule_login(&mut app, &mut tasks, tx.clone());
            terminal.draw(|frame| view(frame, &mut app)).unwrap();
        }
        assert_eq!(app.tick, 3);
        assert_eq!(tasks.len(), 1, "only one poll may be in flight");
        assert!(
            rx.try_recv().is_err(),
            "the proxy is still blocking the login request"
        );
        for message in event::process_event(
            Event::Input(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char('q'),
                    crossterm::event::KeyModifiers::NONE,
                ),
            )),
            &app,
        ) {
            update(&mut app, message);
        }
        assert!(app.should_quit);
        drop(tasks);
        time::timeout(
            Duration::from_secs(2),
            connection.read_to_end(&mut Vec::new()),
        )
        .await
        .unwrap()
        .unwrap();
    }

    #[tokio::test]
    async fn expired_login_starts_one_background_renewal_and_errors_stop_polling() {
        use std::time::{Duration, Instant};
        let mut app = App::new(Config::default(), None).unwrap();
        app.login = Some(LoginState {
            user_code: "old-code".into(),
            verification_uri: "https://github.com/login/device".into(),
            device_code: "old-device".into(),
            expires_at: Instant::now() - Duration::from_secs(1),
            poll_interval: Duration::from_secs(5),
            last_poll: Instant::now() - Duration::from_secs(6),
            status: LoginStatus::Waiting,
            http_client: build_http_client().unwrap(),
        });
        let (tx, _rx) = mpsc::channel(1);
        let mut tasks = JoinSet::new();
        schedule_login(&mut app, &mut tasks, tx.clone());
        schedule_login(&mut app, &mut tasks, tx.clone());
        assert_eq!(tasks.len(), 1);
        assert!(matches!(
            app.login.as_ref().unwrap().status,
            LoginStatus::Expired
        ));
        // Cancel before yielding so this test never contacts GitHub.
        tasks.abort_all();
        assert!(tasks.join_next().await.unwrap().unwrap_err().is_cancelled());
        update(&mut app, Message::LoginError("renewal failed".into()));
        schedule_login(&mut app, &mut tasks, tx);
        assert!(tasks.is_empty());
        assert!(matches!(app.login.unwrap().status, LoginStatus::Error(_)));
    }

    #[tokio::test]
    async fn pr_comment_timeout_preserves_successful_sections() {
        use std::time::Duration;
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
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(31))
                    .set_body_json(serde_json::json!([])),
            )
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
        let result = time::timeout(Duration::from_secs(35), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let Message::ApiLoaded(ApiPayload::PrComments {
            comments,
            reviews,
            code_comments,
            files,
            ..
        }) = result
        else {
            panic!("Expected partial PR details");
        };
        assert!(reviews.unwrap_err().contains("timed out"));
        assert!(comments.unwrap().is_empty());
        assert!(code_comments.unwrap().is_empty());
        assert!(files.unwrap().is_empty());
    }

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
            .unwrap();
        let Message::ApiLoaded(payload) = payload else {
            panic!("Expected API payload");
        };

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
        assert!(matches!(result, Message::ApiError(msg) if msg.contains("500")));
    }
}
