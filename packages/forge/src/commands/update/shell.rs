//! Shell restart logic for Quickshell-based desktop shells
//!
//! After a NixOS rebuild, Quickshell may continue running with an old store path
//! while the shell commands point to a new path. This module detects when a restart
//! is needed and handles it automatically.

use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::commands::executor::{get_output, run_capture};
use crate::commands::CommandMessage;

use super::out;

/// Types of Quickshell-based desktop shells
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellType {
    Noctalia,
    Illogical,
}

impl ShellType {
    /// Display name for the shell
    pub fn name(&self) -> &'static str {
        match self {
            ShellType::Noctalia => "Noctalia",
            ShellType::Illogical => "Illogical Impulse",
        }
    }

    /// Command to restart this shell
    pub fn restart_command(&self) -> (&'static str, Vec<&'static str>) {
        match self {
            ShellType::Noctalia => ("noctalia-shell", vec![]),
            ShellType::Illogical => ("quickshell", vec!["-c", "~/.config/quickshell/ii"]),
        }
    }

    /// Path to check for expected store path (symlink target)
    pub fn config_symlink_path(&self) -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        match self {
            ShellType::Noctalia => Some(PathBuf::from(format!(
                "{}/.config/quickshell/noctalia-shell",
                home
            ))),
            ShellType::Illogical => Some(PathBuf::from(format!(
                "{}/.config/quickshell/ii",
                home
            ))),
        }
    }
}

/// Information about a running Quickshell process
#[derive(Debug)]
pub struct RunningShellInfo {
    pub shell_type: ShellType,
    pub running_path: String,
    pub pid: u32,
}

/// Detect ALL running Quickshell-based shells and get their store paths
pub async fn get_all_running_quickshell_info() -> Vec<RunningShellInfo> {
    // Get all quickshell processes with full command line
    let output = match get_output("pgrep", &["-a", "quickshell"]).await {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if output.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    // Parse ALL quickshell processes (there may be duplicates after updates)
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }

        let pid: u32 = match parts[0].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let cmd = parts[1];

        // Detect shell type and extract path from command line
        if let Some(info) = parse_quickshell_command(pid, cmd) {
            results.push(info);
        }
    }

    results
}

/// Parse a quickshell command line to determine shell type and path
fn parse_quickshell_command(pid: u32, cmd: &str) -> Option<RunningShellInfo> {
    // Noctalia: quickshell -p /nix/store/.../noctalia-shell/share/noctalia-shell
    if cmd.contains("/noctalia-shell") {
        if let Some(path) = extract_path_arg(cmd, "-p") {
            return Some(RunningShellInfo {
                shell_type: ShellType::Noctalia,
                running_path: path,
                pid,
            });
        }
    }

    // Illogical: quickshell -c ~/.config/quickshell/ii
    // The -c points to a config dir, but we need to check the quickshell binary's store path
    if cmd.contains("quickshell/ii") || cmd.contains("-c") && cmd.contains("/ii") {
        // For illogical, the path comparison is different - check the quickshell binary itself
        if let Some(path) = extract_quickshell_binary_path(cmd) {
            return Some(RunningShellInfo {
                shell_type: ShellType::Illogical,
                running_path: path,
                pid,
            });
        }
    }

    None
}

/// Extract a path argument from a command line (e.g., -p /path/to/something)
fn extract_path_arg(cmd: &str, flag: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == flag && i + 1 < parts.len() {
            return Some(parts[i + 1].to_string());
        }
    }
    None
}

/// Extract the quickshell binary path from the command
fn extract_quickshell_binary_path(cmd: &str) -> Option<String> {
    // The command starts with the binary path
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if let Some(binary) = parts.first() {
        if binary.contains("/nix/store/") {
            return Some(binary.to_string());
        }
    }
    None
}

/// Get the expected store path for a shell after rebuild
pub async fn get_expected_shell_path(shell: ShellType) -> Option<String> {
    let symlink_path = shell.config_symlink_path()?;

    // Read the symlink target
    match std::fs::read_link(&symlink_path) {
        Ok(target) => Some(target.to_string_lossy().to_string()),
        Err(_) => {
            // For illogical, the config dir might not be a symlink
            // In that case, check if the quickshell command exists and get its path
            if shell == ShellType::Illogical {
                get_output("which", &["quickshell"]).await.ok()
            } else {
                None
            }
        }
    }
}

/// Check if shell needs restart and restart if necessary
/// Returns Some(shell_name) if restarted/cleaned up, None if not needed
pub async fn restart_shell_if_needed(
    tx: &mpsc::Sender<CommandMessage>,
) -> Result<Option<String>> {
    // Get info about ALL running quickshell processes
    let running_shells = get_all_running_quickshell_info().await;

    if running_shells.is_empty() {
        tracing::debug!("No Quickshell process running, skipping restart check");
        return Ok(None);
    }

    tracing::info!("Found {} running Quickshell process(es)", running_shells.len());
    for info in &running_shells {
        tracing::info!(
            "  PID {}: {} at {}",
            info.pid,
            info.shell_type.name(),
            info.running_path
        );
    }

    // Use the first shell's type to determine expected path
    // (all shells should be the same type in normal operation)
    let shell_type = running_shells[0].shell_type;

    // Get expected path after rebuild
    let expected_path = match get_expected_shell_path(shell_type).await {
        Some(path) => path,
        None => {
            tracing::warn!(
                "Could not determine expected path for {} shell",
                shell_type.name()
            );
            return Ok(None);
        }
    };

    tracing::info!("Expected shell path: {}", expected_path);

    // Categorize processes: correct path vs wrong path
    let mut correct_pids: Vec<u32> = Vec::new();
    let mut wrong_pids: Vec<u32> = Vec::new();

    for info in &running_shells {
        let is_correct = match info.shell_type {
            ShellType::Noctalia => info.running_path == expected_path,
            ShellType::Illogical => {
                info.running_path.contains(&expected_path)
                    || expected_path.contains(&info.running_path)
            }
        };

        if is_correct {
            correct_pids.push(info.pid);
        } else {
            wrong_pids.push(info.pid);
        }
    }

    tracing::info!(
        "Correct path: {} process(es), wrong path: {} process(es)",
        correct_pids.len(),
        wrong_pids.len()
    );

    // If there are no wrong processes, nothing to do
    if wrong_pids.is_empty() {
        tracing::info!("All shells have correct path, no cleanup needed");
        return Ok(None);
    }

    // Kill only the processes with wrong paths
    out(tx, "").await;
    if correct_pids.is_empty() {
        out(
            tx,
            &format!(
                "  Restarting {} shell (store path changed)...",
                shell_type.name()
            ),
        )
        .await;
    } else {
        out(
            tx,
            &format!(
                "  Cleaning up {} stale {} shell process(es)...",
                wrong_pids.len(),
                shell_type.name()
            ),
        )
        .await;
    }

    // Kill processes with wrong paths
    for pid in &wrong_pids {
        tracing::info!("Killing stale quickshell PID {}", pid);
        let _ = run_capture("kill", &[&pid.to_string()]).await;
    }

    // Wait a moment for processes to die
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Only start a new shell if there wasn't already a correct one running
    if correct_pids.is_empty() {
        // Restart using hyprctl if available, otherwise direct launch
        let (cmd, args) = shell_type.restart_command();

        // Try hyprctl dispatch exec first (preferred for Wayland)
        let hyprctl_available = run_capture("which", &["hyprctl"])
            .await
            .map(|(ok, _, _)| ok)
            .unwrap_or(false);

        if hyprctl_available {
            let exec_cmd = if args.is_empty() {
                cmd.to_string()
            } else {
                format!("{} {}", cmd, args.join(" "))
            };

            let _ = run_capture("hyprctl", &["dispatch", "exec", &exec_cmd]).await;
        } else {
            // Direct launch as fallback
            let mut launch_args: Vec<&str> = vec![cmd];
            launch_args.extend(args.iter());

            // Use nohup to detach the process
            let _ = run_capture("nohup", &launch_args).await;
        }

        // Wait for shell to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify restart succeeded
        let new_shells = get_all_running_quickshell_info().await;
        if !new_shells.is_empty() {
            tracing::info!("Shell restarted successfully");
        } else {
            tracing::warn!("Shell may not have restarted properly");
        }

        Ok(Some(shell_type.name().to_string()))
    } else {
        // A correct shell was already running, we just cleaned up stale ones
        tracing::info!("Kept existing correct shell, cleaned up {} stale process(es)", wrong_pids.len());
        Ok(Some(format!("{} (cleanup)", shell_type.name())))
    }
}
