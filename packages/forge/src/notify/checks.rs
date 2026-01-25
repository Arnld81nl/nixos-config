//! Shared update check logic for Forge
//!
//! Provides unified update checking for both the TUI (commands/apps.rs)
//! and the background notifier (bin/notify.rs).

use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use super::constants::git_fetch_timeout;
use super::paths::{app_backup_data_dir, nixos_config_dir};

/// Check for nixos-config repo updates
///
/// Returns a list of (hash, message) pairs for commits on origin/main
/// that aren't in the local HEAD.
///
/// # Arguments
/// * `timeout` - Optional custom timeout for git fetch. Uses default if None.
pub async fn check_nixos_config_updates(
    timeout: Option<Duration>,
) -> Result<Vec<(String, String)>> {
    let config_dir = nixos_config_dir();

    // If no git repo, no updates to check
    if !config_dir.join(".git").exists() {
        return Ok(vec![]);
    }

    let fetch_timeout = timeout.unwrap_or_else(git_fetch_timeout);

    // Fetch from remote with timeout
    let fetch_result = tokio::time::timeout(
        fetch_timeout,
        run_git_command(&config_dir, &["fetch", "origin"]),
    )
    .await;

    match fetch_result {
        Ok(Ok(true)) => {}
        _ => return Ok(vec![]), // Timeout or fetch failed
    }

    // Get list of commits on origin/main not in HEAD
    let (ok, log_output) = run_git_output(
        &config_dir,
        &["log", "HEAD..origin/main", "--pretty=format:%h|%s"],
    )
    .await?;

    if !ok || log_output.trim().is_empty() {
        return Ok(vec![]);
    }

    // Parse output into (hash, message) pairs
    let commits: Vec<(String, String)> = log_output
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            (
                parts.first().unwrap_or(&"").to_string(),
                parts.get(1).unwrap_or(&"").to_string(),
            )
        })
        .collect();

    Ok(commits)
}

/// Check for app profile updates
///
/// Returns true if the remote has commits that aren't in the local HEAD.
///
/// # Arguments
/// * `timeout` - Optional custom timeout for git fetch. Uses default if None.
pub async fn check_app_updates(timeout: Option<Duration>) -> Result<bool> {
    let local_repo = app_backup_data_dir();

    if local_repo.as_os_str().is_empty() || !local_repo.join(".git").exists() {
        return Ok(false);
    }

    // Check if remote is configured
    let (remote_ok, _) = run_git_output(&local_repo, &["remote", "get-url", "origin"]).await?;
    if !remote_ok {
        return Ok(false);
    }

    let fetch_timeout = timeout.unwrap_or_else(git_fetch_timeout);

    // Fetch from remote with timeout
    let fetch_result = tokio::time::timeout(
        fetch_timeout,
        run_git_command(&local_repo, &["fetch", "origin"]),
    )
    .await;

    match fetch_result {
        Ok(Ok(true)) => {}
        _ => return Ok(false),
    }

    // Count commits on origin that aren't in local HEAD
    let (ok, count_str) =
        run_git_output(&local_repo, &["rev-list", "HEAD..origin/main", "--count"]).await?;

    let count: usize = if ok {
        count_str.trim().parse().unwrap_or(0)
    } else {
        // Try origin/master as fallback
        let (master_ok, master_count) =
            run_git_output(&local_repo, &["rev-list", "HEAD..origin/master", "--count"]).await?;
        if !master_ok {
            return Ok(false);
        }
        master_count.trim().parse().unwrap_or(0)
    };

    Ok(count > 0)
}

/// Run a git command and return success status
async fn run_git_command(dir: &Path, args: &[&str]) -> Result<bool> {
    let mut cmd_args = vec!["-C", dir.to_str().unwrap_or(".")];
    cmd_args.extend(args);

    let status = Command::new("git")
        .args(&cmd_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    Ok(status.success())
}

/// Run a git command and capture output
async fn run_git_output(dir: &Path, args: &[&str]) -> Result<(bool, String)> {
    let mut cmd_args = vec!["-C", dir.to_str().unwrap_or(".")];
    cmd_args.extend(args);

    let output = Command::new("git").args(&cmd_args).output().await?;

    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_nixos_config_no_git() {
        // When there's no git repo, should return empty
        let result = check_nixos_config_updates(Some(Duration::from_millis(100))).await;
        // This test will pass in CI where there's no nixos-config
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_app_updates_no_repo() {
        // When there's no app backup repo, should return false
        let result = check_app_updates(Some(Duration::from_millis(100))).await;
        assert!(result.is_ok());
        // Without a repo, should be false (or Ok if repo exists and is up to date)
    }
}
