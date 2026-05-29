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

fn auth_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ghtui")
        .join("auth.json")
}

pub fn save_token(token: &str, scopes: &str) -> Result<()> {
    let path = auth_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let auth = StoredAuth {
        token: token.to_string(),
        scopes: scopes.to_string(),
    };
    let json = serde_json::to_string_pretty(&auth)?;
    use std::os::unix::fs::OpenOptionsExt;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true).mode(0o600);
    let mut file = opts.open(&path)?;
    use std::io::Write;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn load_stored_token() -> Option<String> {
    let path = auth_file_path();
    let data = fs::read_to_string(path).ok()?;
    let auth: StoredAuth = serde_json::from_str(&data).ok()?;
    Some(auth.token)
}

pub fn delete_token() -> Result<()> {
    let path = auth_file_path();
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
        return Err(color_eyre::eyre::eyre!("gh auth token failed"));
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
