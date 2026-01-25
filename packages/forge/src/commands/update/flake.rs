//! Flake-related utilities for the update command

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::commands::executor::run_capture;

/// Maximum number of commits to fetch per input (to avoid huge responses)
const MAX_COMMITS_TO_FETCH: usize = 10;

/// Commit info from GitHub API
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
}

/// Information about a changed flake input
#[derive(Debug, Clone)]
pub struct FlakeInputChange {
    pub name: String,
    pub owner: String,
    pub repo: String,
    pub old_rev: String,
    pub new_rev: String,
    pub commits: Vec<CommitInfo>,
    pub total_commits: usize,
    pub compare_url: Option<String>,
}

/// Flake.lock JSON structure
#[derive(Debug, Deserialize)]
struct FlakeLock {
    nodes: HashMap<String, FlakeNode>,
}

#[derive(Debug, Deserialize)]
struct FlakeNode {
    locked: Option<LockedInfo>,
}

#[derive(Debug, Deserialize)]
struct LockedInfo {
    owner: Option<String>,
    repo: Option<String>,
    rev: Option<String>,
    #[serde(rename = "type")]
    source_type: Option<String>,
}

/// GitHub API compare response
#[derive(Debug, Deserialize)]
struct GitHubCompareResponse {
    total_commits: usize,
    commits: Vec<GitHubCommit>,
}

#[derive(Debug, Deserialize)]
struct GitHubCommit {
    sha: String,
    commit: GitHubCommitInfo,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitInfo {
    message: String,
}

/// Get the SHA256 hash of flake.lock file
pub async fn get_flake_lock_hash(dir: &Path) -> Option<String> {
    let lock_path = dir.join("flake.lock");
    if lock_path.exists() {
        let (_, stdout, _) = run_capture("sha256sum", &[lock_path.to_str()?])
            .await
            .ok()?;
        Some(stdout.split_whitespace().next()?.to_string())
    } else {
        None
    }
}

/// Save a copy of flake.lock before updating
pub async fn save_flake_lock_backup(dir: &Path) -> Option<String> {
    let lock_path = dir.join("flake.lock");
    if !lock_path.exists() {
        return None;
    }

    let backup_path = "/tmp/forge-flake.lock.old";
    let (success, _, _) = run_capture("cp", &[lock_path.to_str()?, backup_path])
        .await
        .ok()?;

    if success {
        Some(backup_path.to_string())
    } else {
        None
    }
}

/// Parse changes in flake.lock between old backup and current
pub async fn parse_flake_changes(dir: &Path) -> Result<Vec<FlakeInputChange>> {
    let lock_path = dir.join("flake.lock");
    let backup_path = Path::new("/tmp/forge-flake.lock.old");

    if !lock_path.exists() || !backup_path.exists() {
        return Ok(Vec::new());
    }

    // Read both files
    let old_content = tokio::fs::read_to_string(&backup_path).await?;
    let new_content = tokio::fs::read_to_string(&lock_path).await?;

    // Parse JSON
    let old_lock: FlakeLock = serde_json::from_str(&old_content)?;
    let new_lock: FlakeLock = serde_json::from_str(&new_content)?;

    // Find changed inputs
    let mut changes = Vec::new();

    for (name, new_node) in &new_lock.nodes {
        // Skip the root node
        if name == "root" {
            continue;
        }

        let Some(new_locked) = &new_node.locked else {
            continue;
        };

        // Only handle GitHub sources
        if new_locked.source_type.as_deref() != Some("github") {
            continue;
        }

        let Some(new_rev) = &new_locked.rev else {
            continue;
        };
        let Some(owner) = &new_locked.owner else {
            continue;
        };
        let Some(repo) = &new_locked.repo else {
            continue;
        };

        // Check if this input existed before and has changed
        if let Some(old_node) = old_lock.nodes.get(name) {
            if let Some(old_locked) = &old_node.locked {
                if let Some(old_rev) = &old_locked.rev {
                    if old_rev != new_rev {
                        changes.push(FlakeInputChange {
                            name: name.clone(),
                            owner: owner.clone(),
                            repo: repo.clone(),
                            old_rev: old_rev.clone(),
                            new_rev: new_rev.clone(),
                            commits: Vec::new(),
                            total_commits: 0,
                            compare_url: Some(format!(
                                "https://github.com/{}/{}/compare/{}...{}",
                                owner, repo, &old_rev[..7.min(old_rev.len())], &new_rev[..7.min(new_rev.len())]
                            )),
                        });
                    }
                }
            }
        }
    }

    // Fetch commit messages from GitHub API
    fetch_commits_for_changes(&mut changes).await;

    // Clean up backup file
    let _ = tokio::fs::remove_file(backup_path).await;

    Ok(changes)
}

/// Fetch commit messages from GitHub API for each changed input
async fn fetch_commits_for_changes(changes: &mut Vec<FlakeInputChange>) {
    let client = match reqwest::Client::builder()
        .user_agent("forge-nixos-tool")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to create HTTP client: {}", e);
            return;
        }
    };

    for change in changes.iter_mut() {
        match fetch_github_commits(&client, change).await {
            Ok((commits, total)) => {
                change.commits = commits;
                change.total_commits = total;
            }
            Err(e) => {
                tracing::debug!(
                    "Failed to fetch commits for {}/{}: {}",
                    change.owner,
                    change.repo,
                    e
                );
            }
        }
    }
}

/// Fetch commits between two revisions from GitHub API
async fn fetch_github_commits(
    client: &reqwest::Client,
    change: &FlakeInputChange,
) -> Result<(Vec<CommitInfo>, usize)> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/compare/{}...{}",
        change.owner, change.repo, change.old_rev, change.new_rev
    );

    let response = client
        .get(&url)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("GitHub API returned {}", response.status());
    }

    let compare: GitHubCompareResponse = response.json().await?;

    // Take only the most recent commits (they come in chronological order, oldest first)
    let commits: Vec<CommitInfo> = compare
        .commits
        .iter()
        .rev() // Reverse to show newest first
        .take(MAX_COMMITS_TO_FETCH)
        .map(|c| CommitInfo {
            hash: c.sha[..7.min(c.sha.len())].to_string(),
            message: c.commit.message.lines().next().unwrap_or("").to_string(),
        })
        .collect();

    Ok((commits, compare.total_commits))
}
