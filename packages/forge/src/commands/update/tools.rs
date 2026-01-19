//! CLI tool update utilities (Claude Code, Codex, browser profiles)

use anyhow::Result;

use crate::commands::executor::run_capture;

/// Clean version strings (remove duplicate labels)
pub fn clean_version(v: &str) -> String {
    v.lines()
        .next()
        .unwrap_or("")
        .replace(" (Claude Code)", "")
        .replace(" (Codex)", "")
        .trim()
        .to_string()
}

/// Get version of an npm package installed globally
pub async fn get_npm_package_version(package: &str) -> Option<String> {
    let (success, stdout, _) = run_capture("npm", &["list", "-g", "--depth=0", package])
        .await
        .ok()?;

    if !success {
        tracing::debug!("npm list failed for package: {}", package);
        return None;
    }

    // Parse version from output like "@openai/codex@1.0.0" or "codex@1.0.0"
    // Handle both scoped (@scope/package) and unscoped packages
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.contains(package) {
            // Split from the right to handle scoped packages correctly
            // "@openai/codex@1.2.3" -> split at last '@' gives "1.2.3"
            if let Some(at_pos) = trimmed.rfind('@') {
                let before = &trimmed[..at_pos];
                // Make sure this '@' is for the version, not the scope
                // For "@openai/codex@1.2.3", before="@openai/codex", after="1.2.3"
                if before.contains(package) || before.ends_with(package) {
                    let version = trimmed[at_pos + 1..].trim();
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    tracing::debug!("Could not parse version for package: {}", package);
    None
}

/// Check browser/app profile sync status
pub async fn check_browser_status() -> Result<String> {
    let local_repo = crate::constants::app_backup_data_dir();

    if !local_repo.join(".git").exists() {
        return Ok("not synced".to_string());
    }

    // Fetch and compare
    let _ = run_capture(
        "git",
        &["-C", local_repo.to_str().unwrap_or("."), "fetch", "origin"],
    )
    .await;

    let (_, local_head, _) = run_capture(
        "git",
        &["-C", local_repo.to_str().unwrap_or("."), "rev-parse", "HEAD"],
    )
    .await?;

    let (_, remote_head, _) = run_capture(
        "git",
        &[
            "-C",
            local_repo.to_str().unwrap_or("."),
            "rev-parse",
            "origin/main",
        ],
    )
    .await
    .unwrap_or((false, String::new(), String::new()));

    if local_head.trim() == remote_head.trim() {
        Ok("up to date".to_string())
    } else {
        Ok("updates available".to_string())
    }
}
