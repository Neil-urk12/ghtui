pub mod format;
pub mod markdown;
pub mod diff;

/// Spawn a background process with stdout/stderr suppressed.
///
/// Uses `tokio::process::Command` (no shell) which reaps child processes
/// via `.wait()`, preventing zombie accumulation on Unix.
///
/// Returns a `JoinHandle` that callers can await or drop (fire-and-forget).
/// Spawn errors and non-zero exits are logged via `tracing`.
pub fn spawn_background(cmd: &str, args: &[&str]) -> tokio::task::JoinHandle<()> {
    let cmd = cmd.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::spawn(async move {
        match tokio::process::Command::new(&cmd)
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(mut child) => match child.wait().await {
                Ok(status) if !status.success() => {
                    tracing::debug!(status = %status, "background process exited");
                }
                Err(e) => tracing::warn!(error = %e, "failed to wait for background process"),
                _ => {}
            },
            Err(e) => tracing::warn!(error = %e, "failed to spawn background process"),
        }
    })
}

/// Open a GitHub item (issue/PR) in the browser via `gh browse`.
pub fn open_in_browser(item_number: impl std::fmt::Display, owner: &str, repo: &str) {
    let url = format!("{}", item_number);
    let repo_arg = format!("{}/{}", owner, repo);
    spawn_background("gh", &["browse", &url, "--repo", &repo_arg]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_spawn_background_valid_command() {
        let handle = spawn_background("echo", &["hello"]);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_background_invalid_command() {
        let handle = spawn_background("nonexistent_command_xyz", &[]);
        // Should complete without panic — error is logged
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_background_with_args() {
        let handle = spawn_background("echo", &["-n", "test"]);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_spawn_background_actually_runs() {
        // Verify the child process actually executes
        let tmp = std::env::temp_dir().join(format!("ghtui_test_{}", std::process::id()));
        let path = tmp.to_str().unwrap().to_string();
        let handle = spawn_background("touch", &[&path]);
        handle.await.unwrap();
        // Give filesystem a moment
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(tmp.exists(), "child process should have created file");
        let _ = std::fs::remove_file(&tmp);
    }
}
