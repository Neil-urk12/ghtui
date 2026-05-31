use std::time::Duration;

/// Terminal redraw interval
pub const TICK_INTERVAL: Duration = Duration::from_millis(250);

/// Debounce window for panel fetch on selection change
pub const FETCH_DEBOUNCE: Duration = Duration::from_millis(500);

/// API cache time-to-live (test default)
#[cfg(test)]
pub const TEST_CACHE_TTL: Duration = Duration::from_secs(60);

/// GitHub device flow polling interval (test default)
#[cfg(test)]
pub const AUTH_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Token expiration TTL (test default)
#[cfg(test)]
pub const AUTH_EXPIRE_TTL: Duration = Duration::from_secs(60);

/// GitHub OAuth device code endpoint
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// GitHub OAuth access token endpoint
pub const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Initial wait before first auth poll (test default)
#[cfg(test)]
pub const AUTH_INITIAL_WAIT: Duration = Duration::from_secs(1);

/// Maximum wait between auth polls (test default)
#[cfg(test)]
pub const AUTH_MAX_WAIT: Duration = Duration::from_secs(120);

/// Cache TTL for repository list (seconds)
pub const REPO_CACHE_TTL_SECS: u64 = 60;
/// Cache TTL for pull requests (seconds)
pub const PULLS_CACHE_TTL_SECS: u64 = 30;
/// Cache TTL for issues (seconds)
pub const ISSUES_CACHE_TTL_SECS: u64 = 30;
/// Cache TTL for notifications (seconds)
pub const NOTIFICATIONS_CACHE_TTL_SECS: u64 = 15;
/// Cache TTL for workflow runs (seconds)
pub const ACTIONS_CACHE_TTL_SECS: u64 = 30;
/// Cache TTL for PR comments (seconds)
pub const PR_COMMENTS_CACHE_TTL_SECS: u64 = 60;

/// Toast auto-dismiss interval in ticks (~4 seconds at 250ms/tick)
pub const AUTO_DISMISS_TICKS: u64 = 16;

/// Error toast width as percentage of terminal width
pub const ERROR_TOAST_WIDTH_PCT: u16 = 60;
/// Error toast height in terminal rows
pub const ERROR_TOAST_HEIGHT: u16 = 8;
/// Success toast width as percentage of terminal width
pub const SUCCESS_TOAST_WIDTH_PCT: u16 = 50;
/// Success toast height in terminal rows
pub const SUCCESS_TOAST_HEIGHT: u16 = 6;
/// Help popup width as percentage of terminal width
pub const HELP_POPUP_WIDTH_PCT: u16 = 70;
/// Help popup height as percentage of terminal height
pub const HELP_POPUP_HEIGHT_PCT: u16 = 60;
/// Confirmation modal width as percentage of terminal width
pub const CONFIRM_MODAL_WIDTH_PCT: u16 = 50;
/// Confirmation modal height as percentage of terminal height
pub const CONFIRM_MODAL_HEIGHT_PCT: u16 = 20;

/// Default page size for GitHub API pagination
pub const API_PER_PAGE: usize = 100;
/// Page size for Actions API (smaller due to heavier responses)
pub const ACTIONS_PER_PAGE: usize = 50;
/// Maximum pages to fetch for Actions API
pub const ACTIONS_MAX_PAGES: usize = 3;

/// Device flow OAuth timeout
pub const DEVICE_FLOW_TIMEOUT: Duration = Duration::from_secs(30);

/// Tokio worker thread stack size (16 MiB)
pub const TOKIO_STACK_SIZE: usize = 16 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_interval_is_250ms() {
        assert_eq!(TICK_INTERVAL, Duration::from_millis(250));
    }

    #[test]
    fn fetch_debounce_is_500ms() {
        assert_eq!(FETCH_DEBOUNCE, Duration::from_millis(500));
    }

    #[test]
    fn test_cache_ttl_is_60s() {
        assert_eq!(TEST_CACHE_TTL, Duration::from_secs(60));
    }

    #[test]
    fn auth_poll_interval_is_10s() {
        assert_eq!(AUTH_POLL_INTERVAL, Duration::from_secs(10));
    }

    #[test]
    fn auth_expire_ttl_is_60s() {
        assert_eq!(AUTH_EXPIRE_TTL, Duration::from_secs(60));
    }

    #[test]
    fn auth_initial_wait_is_1s() {
        assert_eq!(AUTH_INITIAL_WAIT, Duration::from_secs(1));
    }

    #[test]
    fn auth_max_wait_is_120s() {
        assert_eq!(AUTH_MAX_WAIT, Duration::from_secs(120));
    }

    #[test]
    fn cache_ttl_values() {
        assert_eq!(REPO_CACHE_TTL_SECS, 60);
        assert_eq!(PULLS_CACHE_TTL_SECS, 30);
        assert_eq!(ISSUES_CACHE_TTL_SECS, 30);
        assert_eq!(NOTIFICATIONS_CACHE_TTL_SECS, 15);
        assert_eq!(ACTIONS_CACHE_TTL_SECS, 30);
        assert_eq!(PR_COMMENTS_CACHE_TTL_SECS, 60);
    }

    #[test]
    fn popup_layout_constants() {
        assert_eq!(ERROR_TOAST_WIDTH_PCT, 60);
        assert_eq!(ERROR_TOAST_HEIGHT, 8);
        assert_eq!(SUCCESS_TOAST_WIDTH_PCT, 50);
        assert_eq!(SUCCESS_TOAST_HEIGHT, 6);
        assert_eq!(HELP_POPUP_WIDTH_PCT, 70);
        assert_eq!(HELP_POPUP_HEIGHT_PCT, 60);
        assert_eq!(CONFIRM_MODAL_WIDTH_PCT, 50);
        assert_eq!(CONFIRM_MODAL_HEIGHT_PCT, 20);
    }

    #[test]
    fn api_pagination_constants() {
        assert_eq!(API_PER_PAGE, 100);
        assert_eq!(ACTIONS_PER_PAGE, 50);
        assert_eq!(ACTIONS_MAX_PAGES, 3);
    }

    #[test]
    fn tokio_stack_size_is_16mib() {
        assert_eq!(TOKIO_STACK_SIZE, 16 * 1024 * 1024);
    }

    #[test]
    fn device_flow_timeout_is_30s() {
        assert_eq!(DEVICE_FLOW_TIMEOUT, Duration::from_secs(30));
    }

    #[test]
    fn github_urls_are_valid() {
        assert!(GITHUB_DEVICE_CODE_URL.starts_with("https://github.com/"));
        assert!(GITHUB_ACCESS_TOKEN_URL.starts_with("https://github.com/"));
    }
}
