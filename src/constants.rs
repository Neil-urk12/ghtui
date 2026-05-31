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
}
