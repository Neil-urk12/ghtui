use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::api::auth;

pub struct LoginState {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub expires_at: Instant,
    pub poll_interval: Duration,
    pub last_poll: Instant,
    pub status: LoginStatus,
    pub http_client: reqwest::Client,
}

pub enum LoginStatus {
    Waiting,
    #[allow(dead_code)]
    Polling,
    Success(String),
    Expired,
    Error(String),
}

impl LoginState {
    pub async fn new(http_client: reqwest::Client) -> Result<Self> {
        let resp = auth::request_device_code(
            &http_client,
            auth::GHTUI_CLIENT_ID,
            auth::GHTUI_SCOPES,
        )
        .await?;

        Ok(Self {
            user_code: resp.user_code,
            verification_uri: resp.verification_uri,
            device_code: resp.device_code,
            expires_at: Instant::now() + Duration::from_secs(resp.expires_in),
            poll_interval: Duration::from_secs(resp.interval),
            last_poll: Instant::now(),
            status: LoginStatus::Waiting,
            http_client,
        })
    }

    pub async fn tick(&mut self) {
        match &self.status {
            LoginStatus::Success(_) | LoginStatus::Error(_) => return,
            _ => {}
        }

        if Instant::now() >= self.expires_at {
            self.status = LoginStatus::Expired;
            return;
        }

        if self.last_poll.elapsed() < self.poll_interval {
            return;
        }
        self.last_poll = Instant::now();

        match auth::poll_for_token(
            &self.http_client,
            auth::GHTUI_CLIENT_ID,
            &self.device_code,
        )
        .await
        {
            Ok(resp) => {
                if let Some(token) = resp.access_token {
                    let _ = auth::save_token(&token, &resp.scope.unwrap_or_default());
                    self.status = LoginStatus::Success(token);
                } else if let Some(interval) = resp.interval {
                    self.poll_interval = Duration::from_secs(interval);
                }
            }
            Err(e) => {
                self.status = LoginStatus::Error(e.to_string());
            }
        }
    }

    #[allow(dead_code)]
    pub async fn restart(&mut self) -> Result<()> {
        let resp = auth::request_device_code(
            &self.http_client,
            auth::GHTUI_CLIENT_ID,
            auth::GHTUI_SCOPES,
        )
        .await?;

        self.user_code = resp.user_code;
        self.verification_uri = resp.verification_uri;
        self.device_code = resp.device_code;
        self.expires_at = Instant::now() + Duration::from_secs(resp.expires_in);
        self.poll_interval = Duration::from_secs(resp.interval);
        self.last_poll = Instant::now();
        self.status = LoginStatus::Waiting;
        Ok(())
    }
}

pub fn render_login(frame: &mut Frame, state: &LoginState) {
    let area = frame.area();

    let block = Block::default()
        .title(" GitHub Login ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .margin(1)
    .split(inner);

    let instruction = Line::from(vec![
        "Open: ".white(),
        Span::styled(
            &state.verification_uri,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
        ),
    ]);
    frame.render_widget(Paragraph::new(instruction), chunks[1]);

    let code_label = Line::from("Enter code:");
    frame.render_widget(Paragraph::new(code_label), chunks[3]);

    let code_value = Line::from(vec![Span::styled(
        &state.user_code,
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(code_value), chunks[4]);

    let now = Instant::now();
    let remaining = if state.expires_at > now {
        state.expires_at - now
    } else {
        Duration::ZERO
    };
    let mins = remaining.as_secs() / 60;
    let secs = remaining.as_secs() % 60;
    let countdown = format!("{}:{:02}", mins, secs);

    let status_line = match &state.status {
        LoginStatus::Waiting | LoginStatus::Polling => Line::from(vec![
            "Waiting for authorization... ".gray(),
            countdown.white(),
        ]),
        LoginStatus::Expired => {
            Line::from("Code expired. Requesting new one...".red())
        }
        LoginStatus::Error(e) => {
            Line::from(vec!["Error: ".red(), e.as_str().white()])
        }
        LoginStatus::Success(_) => {
            Line::from("Authorized! Redirecting...".green())
        }
    };
    frame.render_widget(Paragraph::new(status_line), chunks[6]);

    let hint = Line::from("[q] Quit".dark_gray());
    frame.render_widget(Paragraph::new(hint), chunks[7]);
}
