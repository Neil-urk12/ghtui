use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::api::auth;

#[derive(Clone)]
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

#[derive(Clone)]
pub enum LoginStatus {
    Waiting,
    Success(String),
    Expired,
    Error(String),
}

// --- Pure state logic (testable) ---

/// Check if the device code has expired.
fn is_expired(expires_at: Instant) -> bool {
    Instant::now() >= expires_at
}

/// Check if enough time has passed since last poll.
fn should_poll(last_poll: Instant, poll_interval: Duration) -> bool {
    last_poll.elapsed() >= poll_interval
}

/// Calculate remaining time until expiry, clamped to ZERO.
fn remaining_until(expires_at: Instant) -> Duration {
    let now = Instant::now();
    if expires_at > now {
        expires_at - now
    } else {
        Duration::ZERO
    }
}

/// Format a duration as "M:SS" countdown string.
fn format_countdown(remaining: Duration) -> String {
    let total_secs = remaining.as_secs();
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

/// Apply a token response to determine the new login status.
/// Returns (new_status, new_poll_interval) if state should change.
fn apply_token_response(
    access_token: Option<String>,
    interval: Option<u64>,
    scope: Option<String>,
) -> TokenEffect {
    if let Some(token) = access_token {
        TokenEffect::Success(token, scope.unwrap_or_default())
    } else if let Some(interval) = interval {
        TokenEffect::UpdateInterval(Duration::from_secs(interval))
    } else {
        TokenEffect::NoChange
    }
}

enum TokenEffect {
    Success(String, String),
    UpdateInterval(Duration),
    NoChange,
}

impl LoginState {
    pub async fn new(http_client: reqwest::Client) -> Result<Self> {
        let resp =
            auth::request_device_code(&http_client, auth::GHTUI_CLIENT_ID, auth::GHTUI_SCOPES)
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

    pub fn poll_due(&self) -> bool {
        matches!(self.status, LoginStatus::Waiting)
            && should_poll(self.last_poll, self.poll_interval)
    }

    pub async fn tick(&mut self) {
        match &self.status {
            LoginStatus::Success(_) | LoginStatus::Error(_) => return,
            _ => {}
        }

        if is_expired(self.expires_at) {
            self.status = LoginStatus::Expired;
            return;
        }

        if !should_poll(self.last_poll, self.poll_interval) {
            return;
        }
        self.last_poll = Instant::now();

        match auth::poll_for_token(&self.http_client, auth::GHTUI_CLIENT_ID, &self.device_code)
            .await
        {
            Ok(resp) => match apply_token_response(resp.access_token, resp.interval, resp.scope) {
                TokenEffect::Success(token, _scopes) => {
                    self.status = LoginStatus::Success(token);
                }
                TokenEffect::UpdateInterval(interval) => {
                    self.poll_interval = interval;
                }
                TokenEffect::NoChange => {}
            },
            Err(e) => {
                self.status = LoginStatus::Error(e.to_string());
            }
        }
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ]);
    frame.render_widget(Paragraph::new(instruction), chunks[1]);

    let code_label = Line::from("Enter code:");
    frame.render_widget(Paragraph::new(code_label), chunks[3]);

    let code_value = Line::from(vec![Span::styled(
        &state.user_code,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(code_value), chunks[4]);

    let remaining = remaining_until(state.expires_at);
    let countdown = format_countdown(remaining);

    let status_line = match &state.status {
        LoginStatus::Waiting => Line::from(vec![
            "Waiting for authorization... ".gray(),
            countdown.white(),
        ]),
        LoginStatus::Expired => Line::from("Code expired. Requesting new one...".red()),
        LoginStatus::Error(e) => Line::from(vec!["Error: ".red(), e.as_str().white()]),
        LoginStatus::Success(_) => Line::from("Authorized! Redirecting...".green()),
    };
    frame.render_widget(Paragraph::new(status_line), chunks[6]);

    let hint = Line::from("[q] Quit".dark_gray());
    frame.render_widget(Paragraph::new(hint), chunks[7]);
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_expired ---

    #[test]
    fn is_expired_returns_false_when_future() {
        let expires_at = Instant::now() + crate::constants::AUTH_EXPIRE_TTL;
        assert!(!is_expired(expires_at));
    }

    #[test]
    fn is_expired_returns_true_when_past() {
        // Instant::now() is already past if we don't add anything
        // Use a zero duration to ensure it's in the past or present
        let expires_at = Instant::now() - Duration::from_secs(1);
        assert!(is_expired(expires_at));
    }

    // --- should_poll ---

    #[test]
    fn should_poll_returns_false_when_interval_not_elapsed() {
        let last_poll = Instant::now();
        let poll_interval = crate::constants::AUTH_POLL_INTERVAL;
        assert!(!should_poll(last_poll, poll_interval));
    }

    #[test]
    fn should_poll_returns_true_when_interval_elapsed() {
        let last_poll = Instant::now() - Duration::from_secs(11);
        let poll_interval = crate::constants::AUTH_POLL_INTERVAL;
        assert!(should_poll(last_poll, poll_interval));
    }

    #[test]
    fn should_poll_returns_true_at_exact_boundary() {
        let last_poll = Instant::now() - Duration::from_secs(10);
        let poll_interval = crate::constants::AUTH_POLL_INTERVAL;
        assert!(should_poll(last_poll, poll_interval));
    }

    // --- remaining_until ---

    #[test]
    fn remaining_until_returns_duration_when_future() {
        let expires_at = Instant::now() + crate::constants::AUTH_MAX_WAIT;
        let remaining = remaining_until(expires_at);
        // Should be close to 120s (allow 1s tolerance for test execution)
        assert!(remaining.as_secs() >= 119);
        assert!(remaining.as_secs() <= 120);
    }

    #[test]
    fn remaining_until_returns_zero_when_past() {
        let expires_at = Instant::now() - crate::constants::AUTH_INITIAL_WAIT;
        let remaining = remaining_until(expires_at);
        assert_eq!(remaining, Duration::ZERO);
    }

    // --- format_countdown ---

    #[test]
    fn format_countdown_zero() {
        assert_eq!(format_countdown(Duration::ZERO), "0:00");
    }

    #[test]
    fn format_countdown_seconds_only() {
        assert_eq!(format_countdown(Duration::from_secs(45)), "0:45");
    }

    #[test]
    fn format_countdown_minutes_and_seconds() {
        assert_eq!(format_countdown(Duration::from_secs(125)), "2:05");
    }

    #[test]
    fn format_countdown_exact_minute() {
        assert_eq!(format_countdown(Duration::from_secs(60)), "1:00");
    }

    #[test]
    fn format_countdown_large_value() {
        assert_eq!(format_countdown(Duration::from_secs(3661)), "61:01");
    }

    // --- apply_token_response ---

    #[test]
    fn apply_token_response_with_access_token_returns_success() {
        let result = apply_token_response(
            Some("gho_abc123".into()),
            None,
            Some("repo notifications".into()),
        );
        match result {
            TokenEffect::Success(token, scopes) => {
                assert_eq!(token, "gho_abc123");
                assert_eq!(scopes, "repo notifications");
            }
            _ => panic!("expected Success"),
        }
    }

    #[test]
    fn apply_token_response_with_access_token_no_scope_uses_empty() {
        let result = apply_token_response(Some("gho_abc123".into()), None, None);
        match result {
            TokenEffect::Success(token, scopes) => {
                assert_eq!(token, "gho_abc123");
                assert_eq!(scopes, "");
            }
            _ => panic!("expected Success"),
        }
    }

    #[test]
    fn apply_token_response_with_interval_returns_update() {
        let result = apply_token_response(None, Some(10), None);
        match result {
            TokenEffect::UpdateInterval(d) => {
                assert_eq!(d, Duration::from_secs(10));
            }
            _ => panic!("expected UpdateInterval"),
        }
    }

    #[test]
    fn apply_token_response_with_nothing_returns_no_change() {
        let result = apply_token_response(None, None, None);
        assert!(matches!(result, TokenEffect::NoChange));
    }

    #[test]
    fn apply_token_response_token_takes_priority_over_interval() {
        let result = apply_token_response(Some("gho_token".into()), Some(5), Some("scope".into()));
        match result {
            TokenEffect::Success(token, _) => assert_eq!(token, "gho_token"),
            _ => panic!("expected Success, token should take priority"),
        }
    }
}
