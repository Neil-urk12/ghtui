pub mod format;
pub mod markdown;

pub fn open_in_browser(item_number: impl std::fmt::Display, owner: &str, repo: &str) {
    let url = format!("{}", item_number);
    let repo_arg = format!("{}/{}", owner, repo);
    match std::process::Command::new("gh")
        .args(["browse", &url, "--repo", &repo_arg])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => {}
        Err(e) => tracing::warn!("failed to open browser: {e}"),
    }
}
