//! System update command implementation
//!
//! This module handles the full NixOS system update process:
//! - Flake input updates
//! - System rebuild
//! - Package comparison
//! - CLI tool updates (Claude Code, Codex)
//! - Browser profile status check

pub mod flake;
mod packages;
mod shell;
mod tools;

use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::UpdateSummary;
use crate::commands::errors::{ErrorContext, ParsedError};
use crate::commands::executor::{command_exists, get_output, run_capture, run_command_cancellable, run_command_cancellable_transformed, CommandResult};
use crate::commands::CommandMessage;

use flake::{get_flake_lock_hash, parse_flake_changes, save_flake_lock_backup};
use packages::{parse_package_changes_from_history, PackageCompareResult};
use tools::{check_browser_status, clean_version, get_npm_package_version};

use crate::constants::nixos_config_dir;

/// Check for local uncommitted changes in the NixOS config directory.
/// Returns a list of changed files (empty if no changes).
pub fn check_local_changes() -> Vec<String> {
    let config_path = nixos_config_dir();

    // Check if this is a git repository
    if !config_path.join(".git").exists() {
        return Vec::new();
    }

    // Run git status --porcelain to get changed files
    let output = std::process::Command::new("git")
        .args(["-C", &config_path.to_string_lossy(), "status", "--porcelain"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    // Format: "XY filename" where XY is the status code
                    // Extract just the filename (skip first 3 chars: status + space)
                    if line.len() > 3 {
                        line[3..].to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Get the default branch name (main or master) for the remote
pub fn get_default_branch() -> String {
    let config_path = nixos_config_dir();

    // Try to get the default branch from remote HEAD
    let output = std::process::Command::new("git")
        .args(["-C", &config_path.to_string_lossy(), "symbolic-ref", "refs/remotes/origin/HEAD"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Format: refs/remotes/origin/main
            if let Some(branch) = stdout.trim().strip_prefix("refs/remotes/origin/") {
                return branch.to_string();
            }
        }
    }

    // Fall back to checking if origin/main exists
    let check_main = std::process::Command::new("git")
        .args(["-C", &config_path.to_string_lossy(), "rev-parse", "--verify", "origin/main"])
        .output();

    if let Ok(output) = check_main {
        if output.status.success() {
            return "main".to_string();
        }
    }

    // Default to master
    "master".to_string()
}

/// Regex to extract "message" from JSON error responses
static JSON_MESSAGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""message"\s*:\s*"([^"]+)""#).unwrap());

/// Transform nix command output to remove noise and extract useful info from errors
/// Returns None to skip the line, Some(line) to include it (possibly transformed)
fn transform_nix_output(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Keep intentional empty lines
    if trimmed.is_empty() {
        return Some(line.to_string());
    }

    // Skip "is dirty" warnings
    if line.contains("is dirty") {
        return None;
    }

    // Skip HTML content (error pages from GitHub)
    if trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<head")
        || trimmed.starts_with("<body")
        || trimmed.starts_with("<style")
        || trimmed.starts_with("<div")
        || trimmed.starts_with("<title")
        || trimmed.starts_with("<meta")
        || trimmed.starts_with("<link")
        || trimmed.starts_with("<p>")
        || trimmed.starts_with("<ul")
        || trimmed.starts_with("<li")
        || trimmed.starts_with("<a ")
        || trimmed.starts_with("<img")
        || trimmed.starts_with("</")
        || trimmed.starts_with("<!--")
        || trimmed.starts_with("-->")
        || trimmed == "{"
        || trimmed == "}"
        || trimmed == "("
        || trimmed == ")"
    {
        return None;
    }

    // Skip CSS content
    if trimmed.contains("background-color:")
        || trimmed.contains("font-family:")
        || trimmed.contains("text-align:")
        || trimmed.contains("margin:")
        || trimmed.contains("padding:")
        || (trimmed.starts_with(".") && trimmed.contains("{"))
        || (trimmed.starts_with("@media") && trimmed.contains("{"))
    {
        return None;
    }

    // Skip base64 data (long strings without spaces, typically image data)
    if trimmed.len() > 100 && !trimmed.contains(' ') && !trimmed.contains(':') {
        return None;
    }

    // Skip lines that are just closing HTML tags or whitespace with special chars
    if trimmed.starts_with("*/") || trimmed.ends_with("*/") {
        return None;
    }

    // Extract message from JSON error responses
    // e.g., {"message":"API rate limit exceeded..."} -> "  API: API rate limit exceeded..."
    if trimmed.starts_with("{\"") && trimmed.contains("\"message\"") {
        if let Some(caps) = JSON_MESSAGE_RE.captures(trimmed) {
            if let Some(msg) = caps.get(1) {
                // Truncate long messages and clean them up
                let message = msg.as_str();
                let clean_msg = if message.len() > 80 {
                    format!("{}...", &message[..77])
                } else {
                    message.to_string()
                };
                return Some(format!("       → {}", clean_msg));
            }
        }
        // Skip JSON lines we can't parse nicely
        return None;
    }

    // Keep the line as-is
    Some(line.to_string())
}

/// Start the update process
pub async fn start_update(tx: mpsc::Sender<CommandMessage>, cancel: CancellationToken) -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = run_update(&tx, cancel).await {
            tracing::error!("Update failed: {}", e);
            let _ = tx
                .send(CommandMessage::StepFailed {
                    step: "Update".to_string(),
                    error: ParsedError::from_stderr(
                        &e.to_string(),
                        ErrorContext {
                            operation: "Update".to_string(),
                        },
                    ),
                })
                .await;
            let _ = tx.send(CommandMessage::Done { success: false }).await;
        }
    });
    Ok(())
}

async fn run_update(tx: &mpsc::Sender<CommandMessage>, cancel: CancellationToken) -> Result<()> {
    let mut summary = UpdateSummary::default();

    // Find the flake directory
    let flake_dir = crate::constants::nixos_config_dir();

    // Get hostname
    let hostname = match get_output("hostname", &[]).await {
        Ok(h) if !h.is_empty() => h,
        _ => {
            tracing::warn!("Could not get hostname, using 'localhost'");
            "localhost".to_string()
        }
    };

    // Print header
    out(tx, "").await;
    out(tx, "==============================================").await;
    out(tx, "  NixOS System Update").await;
    out(tx, "==============================================").await;
    out(tx, "").await;

    let flake_path = flake_dir.to_str().unwrap_or(".");

    // Step 1: Pull configuration updates
    let pull_result = pull_config_updates(tx, flake_path).await;
    if let Err(e) = pull_result {
        tracing::warn!("Failed to check for config updates: {}", e);
        // Non-fatal - continue with update even if pull check fails
    }

    // Save flake.lock hash and backup before update
    let lock_before = get_flake_lock_hash(&flake_dir).await;
    save_flake_lock_backup(&flake_dir).await;

    // Step 2: Flake update (with streaming output)
    out(tx, "").await;
    out(tx, "══════════════════════════════════════════════").await;
    out(tx, "  Updating Flake Inputs").await;
    out(tx, "══════════════════════════════════════════════").await;
    out(tx, "").await;

    // Transform output: filter noise and extract useful info from errors
    let result = run_command_cancellable_transformed(tx, "nix", &["flake", "update", "--flake", flake_path], cancel.clone(), transform_nix_output).await?;

    out(tx, "").await;
    match result {
        CommandResult::Cancelled => {
            out(tx, "  ⊘ Flake update cancelled").await;
            tx.send(CommandMessage::Cancelled).await?;
            return Ok(());
        }
        CommandResult::Completed(false) => {
            out(tx, "  ✗ Flake update failed").await;
            let error = ParsedError::from_stderr(
                "Flake update failed - see output above for details",
                ErrorContext {
                    operation: "Flake update".to_string(),
                },
            );
            tx.send(CommandMessage::StepFailed {
                step: "flake".to_string(),
                error,
            })
            .await?;
            tx.send(CommandMessage::Done { success: false }).await?;
            return Ok(());
        }
        CommandResult::Completed(true) => {}
    }
    out(tx, "  ✓ Flake inputs updated").await;
    tx.send(CommandMessage::StepComplete {
        step: "flake".to_string(),
    })
    .await?;

    // Check if flake.lock changed
    let lock_after = get_flake_lock_hash(&flake_dir).await;
    let needs_rebuild = lock_before != lock_after;

    if needs_rebuild {
        summary.flake_changes = parse_flake_changes(&flake_dir).await.unwrap_or_default();
    }

    // Step 3: Rebuild (only if needed)
    if needs_rebuild {
        out(tx, "").await;
        out(tx, "══════════════════════════════════════════════").await;
        out(tx, "  Rebuilding System").await;
        out(tx, "══════════════════════════════════════════════").await;
        out(tx, "").await;

        let config_name = hostname.clone();
        let flake_ref = format!("{}#{}", flake_path, config_name);
        let result =
            run_command_cancellable(tx, "sudo", &["nixos-rebuild", "switch", "--flake", &flake_ref], cancel.clone()).await?;

        out(tx, "").await;
        match result {
            CommandResult::Cancelled => {
                out(tx, "  ⊘ System rebuild cancelled").await;
                tx.send(CommandMessage::Cancelled).await?;
                return Ok(());
            }
            CommandResult::Completed(true) => {
                out(tx, "  ✓ System rebuilt successfully").await;
                tx.send(CommandMessage::StepComplete {
                    step: "Rebuild".to_string(),
                })
                .await?;

                // Check if shell needs restart due to store path change
                if let Ok(Some(shell_name)) = shell::restart_shell_if_needed(tx).await {
                    out(tx, &format!("  ✓ Restarted {} shell", shell_name)).await;
                }
            }
            CommandResult::Completed(false) => {
                out(tx, "  ✗ System rebuild failed").await;
                summary.rebuild_failed = true;
                let error = ParsedError::from_stderr(
                    "System rebuild failed - see output above for details",
                    ErrorContext {
                        operation: "System rebuild".to_string(),
                    },
                );
                tx.send(CommandMessage::StepFailed {
                    step: "Rebuild".to_string(),
                    error,
                })
                .await?;
            }
        }
    } else {
        out(tx, "").await;
        out(tx, "  - Skipping rebuild (no changes)").await;
        summary.rebuild_skipped = true;
        tx.send(CommandMessage::StepSkipped {
            step: "Rebuild".to_string(),
        })
        .await?;
    }

    // Step 3: Compare packages
    out(tx, "").await;
    out(tx, "  Comparing packages...").await;
    let pkg_result = parse_package_changes_from_history(tx)
        .await
        .unwrap_or_else(|_| PackageCompareResult::default());
    summary.package_changes = pkg_result.changes;
    summary.closure_summary = pkg_result.closure_summary;

    if summary.package_changes.is_empty() {
        out(tx, "  - No package version changes").await;
    } else {
        out(
            tx,
            &format!("  ✓ {} packages updated", summary.package_changes.len()),
        )
        .await;
    }
    tx.send(CommandMessage::StepComplete {
        step: "Packages".to_string(),
    })
    .await?;

    if !summary.rebuild_failed && !summary.rebuild_skipped {
        summary.reboot_reasons = detect_reboot_reasons(&summary.package_changes).await;
    }

    // Step 4: Update Claude Code
    update_claude_code(tx, &mut summary).await?;

    // Step 5: Update Codex CLI
    update_codex_cli(tx, &mut summary).await?;

    // Step 6: Check app profiles
    check_app_profiles(tx, &mut summary).await?;

    // Output summary
    output_summary(tx, &summary).await?;

    if !summary.reboot_reasons.is_empty() {
        tx.send(CommandMessage::RebootRecommended {
            reasons: summary.reboot_reasons.clone(),
        })
        .await?;
    }

    tx.send(CommandMessage::Done {
        success: !summary.rebuild_failed,
    })
    .await?;

    Ok(())
}

async fn update_claude_code(
    tx: &mpsc::Sender<CommandMessage>,
    summary: &mut UpdateSummary,
) -> Result<()> {
    let claude_path = crate::constants::claude_cli_path();

    if claude_path.exists() {
        let claude_cmd = claude_path.to_str().unwrap_or("claude");
        summary.claude_old = get_output(claude_cmd, &["--version"])
            .await
            .ok()
            .map(|v| clean_version(&v));

        let (success, _stdout, _stderr) = run_capture(claude_cmd, &["update"]).await?;

        if success {
            out(tx, "  ✓ Updating Claude Code").await;
        } else {
            out(tx, "  ✗ Updating Claude Code").await;
        }

        summary.claude_new = get_output(claude_cmd, &["--version"])
            .await
            .ok()
            .map(|v| clean_version(&v));

        tx.send(CommandMessage::StepComplete {
            step: "Claude".to_string(),
        })
        .await?;
    } else {
        out(tx, "  - Claude Code not installed").await;
        tx.send(CommandMessage::StepSkipped {
            step: "Claude".to_string(),
        })
        .await?;
    }

    Ok(())
}

async fn update_codex_cli(
    tx: &mpsc::Sender<CommandMessage>,
    summary: &mut UpdateSummary,
) -> Result<()> {
    let codex_path = crate::constants::codex_cli_path();

    if codex_path.exists() {
        summary.codex_old = get_npm_package_version("@openai/codex").await;

        let (success, _stdout, _stderr) =
            run_capture("npm", &["update", "-g", "@openai/codex"]).await?;

        if success {
            out(tx, "  ✓ Updating Codex CLI").await;
        } else {
            out(tx, "  ✗ Updating Codex CLI").await;
        }

        summary.codex_new = get_npm_package_version("@openai/codex").await;

        tx.send(CommandMessage::StepComplete {
            step: "Codex".to_string(),
        })
        .await?;
    } else {
        out(tx, "  - Codex CLI not installed").await;
        tx.send(CommandMessage::StepSkipped {
            step: "Codex".to_string(),
        })
        .await?;
    }

    Ok(())
}

async fn check_app_profiles(
    tx: &mpsc::Sender<CommandMessage>,
    summary: &mut UpdateSummary,
) -> Result<()> {
    if command_exists("app-restore").await {
        let config_path = crate::constants::app_backup_config_path();

        if config_path.exists() {
            summary.browser_status =
                check_browser_status().await.unwrap_or_else(|_| "unknown".to_string());
            out(tx, "  ✓ Browser profiles up to date").await;
        } else {
            summary.browser_status = "not configured".to_string();
            out(tx, "  - Browser profiles not configured").await;
        }

        tx.send(CommandMessage::StepComplete {
            step: "browser".to_string(),
        })
        .await?;
    } else {
        summary.browser_status = "not configured".to_string();
        out(tx, "  - App backup not configured").await;
        tx.send(CommandMessage::StepSkipped {
            step: "browser".to_string(),
        })
        .await?;
    }

    Ok(())
}

async fn detect_reboot_reasons(
    package_changes: &[(String, String, String)],
) -> Vec<String> {
    let mut reasons = Vec::new();

    if let (Ok(booted), Ok(current)) = (
        get_output("readlink", &["/run/booted-system/kernel"]).await,
        get_output("readlink", &["/run/current-system/kernel"]).await,
    ) {
        if booted.trim() != current.trim() {
            reasons.push("Kernel updated".to_string());
        }
    }

    let mut bootloader_changed = false;
    let mut firmware_changed = false;

    for (pkg, _, _) in package_changes {
        let name = pkg.to_lowercase();
        if name.contains("limine") || name.contains("grub") || name.contains("refind") {
            bootloader_changed = true;
        }
        if name.contains("linux-firmware") || name.contains("firmware") || name == "fwupd" {
            firmware_changed = true;
        }
    }

    if bootloader_changed {
        reasons.push("Bootloader updated".to_string());
    }
    if firmware_changed {
        reasons.push("Firmware updated".to_string());
    }

    reasons
}

async fn output_summary(tx: &mpsc::Sender<CommandMessage>, summary: &UpdateSummary) -> Result<()> {
    out(tx, "").await;
    out(tx, "╔══════════════════════════════════════════════╗").await;
    out(tx, "║            Update Summary                    ║").await;
    out(tx, "╚══════════════════════════════════════════════╝").await;

    // Flake changes with commit messages
    if !summary.flake_changes.is_empty() {
        out(tx, "").await;
        out(tx, "  Flake inputs updated:").await;
        for change in &summary.flake_changes {
            out(tx, "").await;
            if change.total_commits > 0 {
                out(
                    tx,
                    &format!(
                        "  {} ({} commit{}):",
                        change.name,
                        change.total_commits,
                        if change.total_commits == 1 { "" } else { "s" }
                    ),
                )
                .await;

                // Show commit messages
                for commit in &change.commits {
                    out(tx, &format!("    {} {}", commit.hash, commit.message)).await;
                }

                // If there are more commits than shown, add a link
                if change.total_commits > change.commits.len() {
                    if let Some(ref url) = change.compare_url {
                        out(
                            tx,
                            &format!(
                                "    ... and {} more → {}",
                                change.total_commits - change.commits.len(),
                                url
                            ),
                        )
                        .await;
                    }
                }
            } else {
                // No commits fetched (API failed or other issue)
                out(
                    tx,
                    &format!(
                        "  {}: {} → {}",
                        change.name,
                        &change.old_rev[..7.min(change.old_rev.len())],
                        &change.new_rev[..7.min(change.new_rev.len())]
                    ),
                )
                .await;
                if let Some(ref url) = change.compare_url {
                    out(tx, &format!("    → {}", url)).await;
                }
            }
        }
    }

    // CLI tool updates
    let claude_updated = summary.claude_old.is_some()
        && summary.claude_new.is_some()
        && summary.claude_old != summary.claude_new;
    let codex_updated = summary.codex_old.is_some()
        && summary.codex_new.is_some()
        && summary.codex_old != summary.codex_new;

    if claude_updated || codex_updated {
        out(tx, "").await;
        out(tx, "  CLI tools updated:").await;
        if claude_updated {
            out(
                tx,
                &format!(
                    "    Claude Code: {} → {}",
                    summary.claude_old.as_deref().unwrap_or(""),
                    summary.claude_new.as_deref().unwrap_or("")
                ),
            )
            .await;
        }
        if codex_updated {
            out(
                tx,
                &format!(
                    "    Codex CLI: {} → {}",
                    summary.codex_old.as_deref().unwrap_or(""),
                    summary.codex_new.as_deref().unwrap_or("")
                ),
            )
            .await;
        }
    }

    // Package changes and closure summary
    if !summary.package_changes.is_empty() {
        out(tx, "").await;
        out(tx, "  Packages changed:").await;
        for (pkg, old, new) in &summary.package_changes {
            out(tx, &format!("    {}: {} → {}", pkg, old, new)).await;
        }
    }

    // Show closure summary (especially useful when no version changes)
    if let Some(ref closure) = summary.closure_summary {
        out(tx, "").await;
        out(tx, &format!("  Closure: {}", closure)).await;
    }

    // Status section
    out(tx, "").await;
    out(tx, "  ─────────────────────────────────────────").await;
    out(tx, "").await;

    // System status
    if summary.rebuild_failed {
        out(tx, "  System:      Rebuild failed").await;
    } else if summary.rebuild_skipped {
        out(tx, "  System:      Already up to date").await;
    }

    // Show versions that weren't updated
    if summary.claude_old.is_some() && !claude_updated {
        out(
            tx,
            &format!(
                "  Claude Code: {}",
                summary.claude_new.as_deref().unwrap_or("")
            ),
        )
        .await;
    }
    if summary.codex_old.is_some() && !codex_updated {
        out(
            tx,
            &format!(
                "  Codex CLI:   {}",
                summary.codex_new.as_deref().unwrap_or("")
            ),
        )
        .await;
    }

    // Browser status
    if !summary.browser_status.is_empty() {
        out(tx, &format!("  Browser:     {}", summary.browser_status)).await;
    }

    out(tx, "").await;
    out(tx, "══════════════════════════════════════════════").await;

    Ok(())
}

/// Pull configuration updates from remote repository
async fn pull_config_updates(tx: &mpsc::Sender<CommandMessage>, config_path: &str) -> Result<()> {
    // Check if this is a git repository
    let git_dir = std::path::Path::new(config_path).join(".git");
    if !git_dir.exists() {
        out(tx, "  - Not a git repository, skipping pull").await;
        tx.send(CommandMessage::StepSkipped {
            step: "pull".to_string(),
        })
        .await?;
        return Ok(());
    }

    // Fetch from remote
    let (fetch_ok, _, _) = run_capture("git", &["-C", config_path, "fetch", "origin"]).await?;

    if !fetch_ok {
        out(tx, "  - Unable to fetch from remote").await;
        tx.send(CommandMessage::StepSkipped {
            step: "pull".to_string(),
        })
        .await?;
        return Ok(());
    }

    // Check if there are unpulled commits
    let (count_ok, count_str, _) = run_capture(
        "git",
        &["-C", config_path, "rev-list", "HEAD..origin/main", "--count"],
    )
    .await?;

    let count: usize = if count_ok {
        count_str.trim().parse().unwrap_or(0)
    } else {
        // Try origin/master as fallback
        let (master_ok, master_count, _) = run_capture(
            "git",
            &[
                "-C",
                config_path,
                "rev-list",
                "HEAD..origin/master",
                "--count",
            ],
        )
        .await?;

        if master_ok {
            master_count.trim().parse().unwrap_or(0)
        } else {
            0
        }
    };

    if count == 0 {
        out(tx, "  - No configuration updates to pull").await;
        tx.send(CommandMessage::StepSkipped {
            step: "pull".to_string(),
        })
        .await?;
        return Ok(());
    }

    // Pull the updates
    let (pull_ok, _, stderr) =
        run_capture("git", &["-C", config_path, "pull", "--ff-only"]).await?;

    if pull_ok {
        out(tx, &format!("  ✓ Pulled {} commit(s)", count)).await;
        tx.send(CommandMessage::StepComplete {
            step: "pull".to_string(),
        })
        .await?;
    } else {
        out(tx, "  ✗ Failed to pull configuration updates").await;
        let error = ParsedError::from_stderr(
            &stderr,
            ErrorContext {
                operation: "Git pull".to_string(),
            },
        );
        tx.send(CommandMessage::StepFailed {
            step: "pull".to_string(),
            error,
        })
        .await?;
    }

    Ok(())
}

/// Helper to send stdout message
pub(crate) async fn out(tx: &mpsc::Sender<CommandMessage>, msg: &str) {
    let _ = tx.send(CommandMessage::Stdout(msg.to_string())).await;
}
