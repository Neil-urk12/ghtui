use color_eyre::Result;
use std::process::Command;

pub fn resolve_token() -> Result<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    if let Ok(token) = std::env::var("GH_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    try_gh_token().map_err(|_| {
        color_eyre::eyre::eyre!(
            "No GitHub token found. Set GITHUB_TOKEN or run 'gh auth login'"
        )
    })
}

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
