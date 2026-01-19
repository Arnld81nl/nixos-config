//! Async command execution with output streaming

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::CommandMessage;
use crate::constants::DEFAULT_COMMAND_TIMEOUT_SECS;

/// Execute a command and stream output to the channel
pub async fn run_command(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
) -> Result<bool> {
    run_command_filtered(tx, cmd, args, |_| true).await
}

/// Execute a command and stream output, filtering lines with a predicate
/// Lines where the filter returns false will be skipped
pub async fn run_command_filtered<F>(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    filter: F,
) -> Result<bool>
where
    F: Fn(&str) -> bool + Send + Sync + 'static,
{
    run_command_filtered_with_timeout(tx, cmd, args, None, filter).await
}

/// Execute a command with explicit timeout
pub async fn run_command_with_timeout(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    timeout_secs: Option<u64>,
) -> Result<bool> {
    run_command_filtered_with_timeout(tx, cmd, args, timeout_secs, |_| true).await
}

/// Execute a command and transform/filter output lines
/// The transform function returns Option<String>:
/// - None: skip the line
/// - Some(line): output the (possibly modified) line
pub async fn run_command_transformed<F>(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    transform: F,
) -> Result<bool>
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    run_command_transformed_with_timeout(tx, cmd, args, None, transform).await
}

/// Execute a command with transform and timeout
pub async fn run_command_transformed_with_timeout<F>(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    timeout_secs: Option<u64>,
    transform: F,
) -> Result<bool>
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    use std::sync::Arc;
    let transform = Arc::new(transform);

    tracing::info!("Running command: {} {:?}", cmd, args);

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for command: {}", cmd))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr for command: {}", cmd))?;

    let tx_out = tx.clone();
    let transform_out = Arc::clone(&transform);
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Apply transform - skip if None, output transformed line if Some
            if let Some(transformed) = transform_out(&line) {
                if let Err(e) = tx_out.send(CommandMessage::Stdout(transformed)).await {
                    tracing::warn!("Failed to send stdout to channel: {}", e);
                    break;
                }
            }
        }
    });

    let tx_err = tx.clone();
    let transform_err = Arc::clone(&transform);
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Apply transform - skip if None, output transformed line if Some
            if let Some(transformed) = transform_err(&line) {
                if let Err(e) = tx_err.send(CommandMessage::Stderr(transformed)).await {
                    tracing::warn!("Failed to send stderr to channel: {}", e);
                    break;
                }
            }
        }
    });

    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS));
    let status = tokio::time::timeout(timeout, child.wait())
        .await
        .with_context(|| format!("Command timed out after {}s: {}", timeout.as_secs(), cmd))?
        .with_context(|| format!("Failed to wait for command: {}", cmd))?;

    match tokio::time::timeout(Duration::from_secs(5), stdout_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stdout reader task panicked: {}", e),
        Err(_) => tracing::warn!("stdout reader task timed out for command: {}", cmd),
    }
    match tokio::time::timeout(Duration::from_secs(5), stderr_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stderr reader task panicked: {}", e),
        Err(_) => tracing::warn!("stderr reader task timed out for command: {}", cmd),
    }

    let success = status.success();
    tracing::info!("Command completed with success={}", success);
    Ok(success)
}

/// Execute a command with explicit timeout and output filtering
pub async fn run_command_filtered_with_timeout<F>(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    timeout_secs: Option<u64>,
    filter: F,
) -> Result<bool>
where
    F: Fn(&str) -> bool + Send + Sync + 'static,
{
    use std::sync::Arc;
    let filter = Arc::new(filter);

    tracing::info!("Running command: {} {:?}", cmd, args);

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    // Use proper error handling instead of .expect()
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for command: {}", cmd))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr for command: {}", cmd))?;

    let tx_out = tx.clone();
    let filter_out = Arc::clone(&filter);
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Apply filter - skip lines that don't pass
            if !filter_out(&line) {
                continue;
            }
            // Log channel send failures but don't propagate - channel may be closed
            if let Err(e) = tx_out.send(CommandMessage::Stdout(line)).await {
                tracing::warn!("Failed to send stdout to channel: {}", e);
                break;
            }
        }
    });

    let tx_err = tx.clone();
    let filter_err = Arc::clone(&filter);
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            // Apply filter - skip lines that don't pass
            if !filter_err(&line) {
                continue;
            }
            if let Err(e) = tx_err.send(CommandMessage::Stderr(line)).await {
                tracing::warn!("Failed to send stderr to channel: {}", e);
                break;
            }
        }
    });

    // Apply timeout
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS));
    let status = tokio::time::timeout(timeout, child.wait())
        .await
        .with_context(|| format!("Command timed out after {}s: {}", timeout.as_secs(), cmd))?
        .with_context(|| format!("Failed to wait for command: {}", cmd))?;

    // Wait for output tasks with short timeout (they should complete quickly after process exits)
    match tokio::time::timeout(Duration::from_secs(5), stdout_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stdout reader task panicked: {}", e),
        Err(_) => tracing::warn!("stdout reader task timed out for command: {}", cmd),
    }
    match tokio::time::timeout(Duration::from_secs(5), stderr_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stderr reader task panicked: {}", e),
        Err(_) => tracing::warn!("stderr reader task timed out for command: {}", cmd),
    }

    let success = status.success();
    tracing::info!("Command completed with success={}", success);
    Ok(success)
}

/// Execute a command without logging arguments (for sensitive data like passwords)
pub async fn run_command_sensitive(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
) -> Result<bool> {
    // Only log command name, not arguments (which may contain passwords)
    tracing::info!("Running command: {} [args hidden]", cmd);

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for command: {}", cmd))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr for command: {}", cmd))?;

    let tx_out = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Err(e) = tx_out.send(CommandMessage::Stdout(line)).await {
                tracing::warn!("Failed to send stdout to channel: {}", e);
                break;
            }
        }
    });

    let tx_err = tx.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Err(e) = tx_err.send(CommandMessage::Stderr(line)).await {
                tracing::warn!("Failed to send stderr to channel: {}", e);
                break;
            }
        }
    });

    let timeout = Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS);
    let status = tokio::time::timeout(timeout, child.wait())
        .await
        .with_context(|| format!("Command timed out after {}s: {}", timeout.as_secs(), cmd))?
        .with_context(|| format!("Failed to wait for command: {}", cmd))?;

    match tokio::time::timeout(Duration::from_secs(5), stdout_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stdout reader task panicked: {}", e),
        Err(_) => tracing::warn!("stdout reader task timed out for command: {}", cmd),
    }
    match tokio::time::timeout(Duration::from_secs(5), stderr_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stderr reader task panicked: {}", e),
        Err(_) => tracing::warn!("stderr reader task timed out for command: {}", cmd),
    }

    let success = status.success();
    tracing::info!("Command completed with success={}", success);
    Ok(success)
}

/// Execute a command with sudo
#[allow(dead_code)]
pub async fn run_sudo(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
) -> Result<bool> {
    let mut sudo_args = vec![cmd];
    sudo_args.extend(args);
    run_command(tx, "sudo", &sudo_args).await
}

/// Execute a command and capture output (no streaming)
pub async fn run_capture(cmd: &str, args: &[&str]) -> Result<(bool, String, String)> {
    tracing::info!("Capturing command: {} {:?}", cmd, args);

    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to execute command: {}", cmd))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((output.status.success(), stdout, stderr))
}

/// Check if a command exists
pub async fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get output of a command as a string
pub async fn get_output(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to get output from command: {}", cmd))?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Result of a cancellable command execution
#[derive(Debug)]
pub enum CommandResult {
    /// Command completed (success status)
    Completed(bool),
    /// Command was cancelled
    Cancelled,
}

/// Execute a command with cancellation support
/// Returns Cancelled if the token is triggered, Completed otherwise
pub async fn run_command_cancellable(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    cancel: CancellationToken,
) -> Result<CommandResult> {
    run_command_cancellable_with_timeout(tx, cmd, args, None, cancel).await
}

/// Execute a command with cancellation and timeout support
pub async fn run_command_cancellable_with_timeout(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    timeout_secs: Option<u64>,
    cancel: CancellationToken,
) -> Result<CommandResult> {
    tracing::info!("Running cancellable command: {} {:?}", cmd, args);

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for command: {}", cmd))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr for command: {}", cmd))?;

    let tx_out = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Err(e) = tx_out.send(CommandMessage::Stdout(line)).await {
                tracing::warn!("Failed to send stdout to channel: {}", e);
                break;
            }
        }
    });

    let tx_err = tx.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Err(e) = tx_err.send(CommandMessage::Stderr(line)).await {
                tracing::warn!("Failed to send stderr to channel: {}", e);
                break;
            }
        }
    });

    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS));

    // Race between: command completion, timeout, and cancellation
    let result = tokio::select! {
        biased;

        // Cancellation takes priority
        _ = cancel.cancelled() => {
            tracing::info!("Command cancelled: {} {:?}", cmd, args);
            // Kill the child process
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }
            CommandResult::Cancelled
        }

        // Command completion with timeout
        status = tokio::time::timeout(timeout, child.wait()) => {
            match status {
                Ok(Ok(exit_status)) => {
                    let success = exit_status.success();
                    tracing::info!("Command completed with success={}", success);
                    CommandResult::Completed(success)
                }
                Ok(Err(e)) => {
                    tracing::error!("Command wait failed: {}", e);
                    CommandResult::Completed(false)
                }
                Err(_) => {
                    tracing::warn!("Command timed out after {}s: {}", timeout.as_secs(), cmd);
                    if let Err(e) = child.kill().await {
                        tracing::warn!("Failed to kill timed out process: {}", e);
                    }
                    CommandResult::Completed(false)
                }
            }
        }
    };

    // Clean up output tasks
    match tokio::time::timeout(Duration::from_secs(2), stdout_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stdout reader task panicked: {}", e),
        Err(_) => tracing::warn!("stdout reader task timed out for command: {}", cmd),
    }
    match tokio::time::timeout(Duration::from_secs(2), stderr_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stderr reader task panicked: {}", e),
        Err(_) => tracing::warn!("stderr reader task timed out for command: {}", cmd),
    }

    Ok(result)
}

/// Execute a command with transform, timeout, and cancellation support
pub async fn run_command_cancellable_transformed<F>(
    tx: &mpsc::Sender<CommandMessage>,
    cmd: &str,
    args: &[&str],
    cancel: CancellationToken,
    transform: F,
) -> Result<CommandResult>
where
    F: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    use std::sync::Arc;
    let transform = Arc::new(transform);

    tracing::info!("Running cancellable command: {} {:?}", cmd, args);

    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", cmd))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout for command: {}", cmd))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stderr for command: {}", cmd))?;

    let tx_out = tx.clone();
    let transform_out = Arc::clone(&transform);
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(transformed) = transform_out(&line) {
                if let Err(e) = tx_out.send(CommandMessage::Stdout(transformed)).await {
                    tracing::warn!("Failed to send stdout to channel: {}", e);
                    break;
                }
            }
        }
    });

    let tx_err = tx.clone();
    let transform_err = Arc::clone(&transform);
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(transformed) = transform_err(&line) {
                if let Err(e) = tx_err.send(CommandMessage::Stderr(transformed)).await {
                    tracing::warn!("Failed to send stderr to channel: {}", e);
                    break;
                }
            }
        }
    });

    let timeout = Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS);

    // Race between: command completion, timeout, and cancellation
    let result = tokio::select! {
        biased;

        // Cancellation takes priority
        _ = cancel.cancelled() => {
            tracing::info!("Command cancelled: {} {:?}", cmd, args);
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }
            CommandResult::Cancelled
        }

        // Command completion with timeout
        status = tokio::time::timeout(timeout, child.wait()) => {
            match status {
                Ok(Ok(exit_status)) => {
                    let success = exit_status.success();
                    tracing::info!("Command completed with success={}", success);
                    CommandResult::Completed(success)
                }
                Ok(Err(e)) => {
                    tracing::error!("Command wait failed: {}", e);
                    CommandResult::Completed(false)
                }
                Err(_) => {
                    tracing::warn!("Command timed out after {}s: {}", timeout.as_secs(), cmd);
                    if let Err(e) = child.kill().await {
                        tracing::warn!("Failed to kill timed out process: {}", e);
                    }
                    CommandResult::Completed(false)
                }
            }
        }
    };

    // Clean up output tasks
    match tokio::time::timeout(Duration::from_secs(2), stdout_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stdout reader task panicked: {}", e),
        Err(_) => tracing::warn!("stdout reader task timed out for command: {}", cmd),
    }
    match tokio::time::timeout(Duration::from_secs(2), stderr_task).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("stderr reader task panicked: {}", e),
        Err(_) => tracing::warn!("stderr reader task timed out for command: {}", cmd),
    }

    Ok(result)
}
