use color_eyre::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use crate::constants::{GITHUB_ACCESS_TOKEN_URL, GITHUB_DEVICE_CODE_URL};

pub const GHTUI_CLIENT_ID: &str = "Iv1.b507a08c1255a797";
pub const GHTUI_SCOPES: &str = "repo notifications read:org";

// --- Token Storage ---

#[derive(Serialize, Deserialize)]
struct StoredAuth {
    token: String,
    scopes: String,
}

fn auth_file_path_in(base_dir: &std::path::Path) -> PathBuf {
    base_dir.join("ghtui").join("auth.json")
}

fn config_base() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}


pub fn save_token(token: &str, scopes: &str) -> Result<()> {
    save_token_in(token, scopes, &config_base())
}

pub(crate) fn save_token_in(token: &str, scopes: &str, base_dir: &std::path::Path) -> Result<()> {
    let path = auth_file_path_in(base_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let auth = StoredAuth {
        token: token.to_string(),
        scopes: scopes.to_string(),
    };
    let json = serde_json::to_string_pretty(&auth)?;

    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&path)?;
    use std::io::Write;
    file.write_all(json.as_bytes())?;

    Ok(())
}

pub fn load_stored_token() -> Option<String> {
    load_stored_token_in(&config_base())
}

pub(crate) fn load_stored_token_in(base_dir: &std::path::Path) -> Option<String> {
    let path = auth_file_path_in(base_dir);
    let data = fs::read_to_string(path).ok()?;
    let auth: StoredAuth = serde_json::from_str(&data).ok()?;
    Some(auth.token)
}

pub fn delete_token() -> Result<()> {
    delete_token_in(&config_base())
}

pub(crate) fn delete_token_in(base_dir: &std::path::Path) -> Result<()> {
    let path = auth_file_path_in(base_dir);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

// --- Device Flow Structs ---

#[derive(Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct TokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub interval: Option<u64>,
}

// --- Device Flow Functions ---

pub async fn request_device_code(
    client: &reqwest::Client,
    client_id: &str,
    scopes: &str,
) -> Result<DeviceCodeResponse> {
    let resp = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", scopes)])
        .send()
        .await?;
    let device_code: DeviceCodeResponse = resp.json().await?;
    Ok(device_code)
}

pub async fn poll_for_token(
    client: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<TokenResponse> {
    let resp = client
        .post(GITHUB_ACCESS_TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?;
    let token_resp: TokenResponse = resp.json().await?;
    Ok(token_resp)
}

// --- resolve_token ---

fn try_gh_token() -> Result<String> {
    let output = Command::new("gh").arg("auth").arg("token").output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!("gh auth token failed: {}", stderr.trim()));
    }
    let token = String::from_utf8(output.stdout)?.trim().to_string();
    if token.is_empty() {
        return Err(color_eyre::eyre::eyre!("gh auth token returned empty"));
    }
    Ok(token)
}

/// Check auth methods in priority order:
/// 1. GITHUB_TOKEN env var
/// 2. GH_TOKEN env var
/// 3. gh auth token (gh CLI)
/// 4. Stored token (~/.config/ghtui/auth.json)
/// 5. None — triggers device flow
pub fn resolve_token() -> Result<Option<String>> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }
    if let Ok(token) = std::env::var("GH_TOKEN") {
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }
    if let Ok(token) = try_gh_token() {
        return Ok(Some(token));
    }
    if let Some(token) = load_stored_token() {
        return Ok(Some(token));
    }
    Ok(None)
}

/// Force device flow — skips env vars, gh CLI, and stored token.
pub fn resolve_token_force_device() -> Result<Option<String>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Global mutex to serialize env var tests (process-global state)
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        keys: Vec<&'static str>,
        originals: Vec<Option<String>>,
    }

    impl EnvGuard {
        fn new(keys: &[&'static str]) -> Self {
            let originals = keys.iter().map(|k| std::env::var(k).ok()).collect();
            Self {
                keys: keys.to_vec(),
                originals,
            }
        }
        fn set(&self, key: &str, val: &str) {
            std::env::set_var(key, val);
        }
        fn remove(&self, key: &str) {
            std::env::remove_var(key);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, orig) in self.keys.iter().zip(self.originals.iter()) {
                match orig {
                    Some(val) => std::env::set_var(key, val),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    // --- Serde Tests ---

    #[test]
    fn token_response_success_deserializes() {
        let json = r#"{
            "access_token": "gho_abc123",
            "token_type": "bearer",
            "scope": "repo notifications"
        }"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token.as_deref(), Some("gho_abc123"));
        assert_eq!(resp.token_type.as_deref(), Some("bearer"));
        assert_eq!(resp.scope.as_deref(), Some("repo notifications"));
        assert_eq!(resp.error, None);
    }

    #[test]
    fn token_response_error_deserializes() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "User has not yet authorized"
        }"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, None);
        assert_eq!(resp.error.as_deref(), Some("authorization_pending"));
        assert_eq!(
            resp.error_description.as_deref(),
            Some("User has not yet authorized")
        );
    }

    #[test]
    fn token_response_with_interval_deserializes() {
        let json = r#"{
            "error": "slow_down",
            "interval": 10
        }"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.interval, Some(10));
    }

    #[test]
    fn device_code_response_deserializes() {
        let json = r#"{
            "device_code": "abc123def456",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "interval": 5,
            "expires_in": 900
        }"#;
        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "abc123def456");
        assert_eq!(resp.user_code, "WDJB-MJHT");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.interval, 5);
        assert_eq!(resp.expires_in, 900);
    }

    // --- resolve_token_force_device ---

    #[test]
    fn resolve_token_force_device_returns_none() {
        let result = resolve_token_force_device();
        assert_eq!(result.unwrap(), None);
    }

    // --- resolve_token env var tests ---
    // NOTE: env vars are process-global; tests clean up after themselves.

    #[test]
    fn resolve_token_returns_github_token_when_set() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let guard = EnvGuard::new(&["GITHUB_TOKEN", "GH_TOKEN"]);
        guard.set("GITHUB_TOKEN", "test-token-123");
        guard.remove("GH_TOKEN");
        let result = resolve_token();
        assert_eq!(result.unwrap(), Some("test-token-123".to_string()));
    }

    #[test]
    fn resolve_token_returns_gh_token_when_github_token_unset() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let guard = EnvGuard::new(&["GITHUB_TOKEN", "GH_TOKEN"]);
        guard.remove("GITHUB_TOKEN");
        guard.set("GH_TOKEN", "gh-token-456");
        let result = resolve_token();
        assert_eq!(result.unwrap(), Some("gh-token-456".to_string()));
    }

    #[test]
    fn resolve_token_prefers_github_token_over_gh_token() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let guard = EnvGuard::new(&["GITHUB_TOKEN", "GH_TOKEN"]);
        guard.set("GITHUB_TOKEN", "github-val");
        guard.set("GH_TOKEN", "gh-val");
        let result = resolve_token();
        assert_eq!(result.unwrap(), Some("github-val".to_string()));
    }

    #[test]
    fn resolve_token_skips_empty_github_token() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let guard = EnvGuard::new(&["GITHUB_TOKEN", "GH_TOKEN"]);
        guard.set("GITHUB_TOKEN", "");
        guard.set("GH_TOKEN", "gh-val");
        let result = resolve_token();
        assert_eq!(result.unwrap(), Some("gh-val".to_string()));
    }

    #[test]
    fn resolve_token_skips_empty_gh_token() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let guard = EnvGuard::new(&["GITHUB_TOKEN", "GH_TOKEN"]);
        guard.set("GITHUB_TOKEN", "");
        guard.set("GH_TOKEN", "");
        let result = resolve_token();
        // Falls through to gh CLI or stored token or None — just verify no error
        result.unwrap();
    }

    // --- File I/O tests (tempdir-based) ---

    #[test]
    fn save_and_load_token_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        save_token_in("my-token", "repo notifications", tmp.path()).unwrap();
        let loaded = load_stored_token_in(tmp.path());
        assert_eq!(loaded, Some("my-token".to_string()));
    }

    #[test]
    fn load_stored_token_returns_none_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = load_stored_token_in(tmp.path());
        assert_eq!(loaded, None);
    }

    #[test]
    fn load_stored_token_returns_none_on_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let auth_dir = tmp.path().join("ghtui");
        std::fs::create_dir_all(&auth_dir).unwrap();
        std::fs::write(auth_dir.join("auth.json"), "not json").unwrap();
        let loaded = load_stored_token_in(tmp.path());
        assert_eq!(loaded, None);
    }

    #[test]
    fn load_stored_token_returns_none_on_missing_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let auth_dir = tmp.path().join("ghtui");
        std::fs::create_dir_all(&auth_dir).unwrap();
        std::fs::write(auth_dir.join("auth.json"), r#"{"wrong": "fields"}"#).unwrap();
        let loaded = load_stored_token_in(tmp.path());
        assert_eq!(loaded, None);
    }

    #[test]
    fn delete_token_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        save_token_in("tok", "repo", tmp.path()).unwrap();
        assert!(load_stored_token_in(tmp.path()).is_some());
        delete_token_in(tmp.path()).unwrap();
        assert_eq!(load_stored_token_in(tmp.path()), None);
    }

    #[test]
    fn delete_token_noop_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        // Should not error even when file doesn't exist
        delete_token_in(tmp.path()).unwrap();
    }

    #[test]
    fn save_token_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("deep").join("nested");
        save_token_in("tok", "repo", &nested).unwrap();
        let loaded = load_stored_token_in(&nested);
        assert_eq!(loaded, Some("tok".to_string()));
    }

    #[test]
    fn save_token_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        save_token_in("first", "repo", tmp.path()).unwrap();
        save_token_in("second", "repo notifications", tmp.path()).unwrap();
        let loaded = load_stored_token_in(tmp.path());
        assert_eq!(loaded, Some("second".to_string()));
    }
}
