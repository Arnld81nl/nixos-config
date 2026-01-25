//! Forge Background Update Checker
//!
//! A lightweight daemon that checks for updates and sends desktop notifications.
//! Designed to run as a systemd user service.
//!
//! Usage:
//!   forge-notify [--once]     Run check once and exit
//!   forge-notify --help       Show help

use anyhow::Result;
use clap::Parser;
use forge::notify;
use forge::notify::constants::NOTIFICATION_TIMEOUT_MS;
use forge::notify::paths::{forge_data_dir, FORGE_LOG_FILE};
use notify_rust::{Notification, Urgency};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Forge Background Update Checker
#[derive(Parser)]
#[command(name = "forge-notify")]
#[command(author = "Cybex B.V.")]
#[command(version = "1.0.0")]
#[command(about = "Background update checker for Forge")]
struct Cli {
    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    setup_logging(cli.verbose)?;

    tracing::info!("forge-notify starting");

    // Run the update check
    match run_check().await {
        Ok(notified) => {
            if notified {
                tracing::info!("Notification sent");
            } else {
                tracing::info!("No new updates to notify about");
            }
        }
        Err(e) => {
            tracing::error!("Check failed: {}", e);
            // Don't return error - we want the service to succeed even if checks fail
            // This prevents systemd from marking the service as failed
        }
    }

    tracing::info!("forge-notify complete");
    Ok(())
}

/// Run the update check and send notification if needed
async fn run_check() -> Result<bool> {
    // Load existing state
    let mut state = notify::state::NotifyState::load().unwrap_or_default();

    // Check for all updates
    let status = notify::check_all_updates().await?;

    tracing::debug!(
        "Check results: config={}, apps={}, flake={}",
        status.config_updates.len(),
        status.app_updates,
        status.flake_updates.len()
    );

    // Check if we should notify
    if !state.should_notify(&status) {
        return Ok(false);
    }

    // Send notification
    send_notification(&status)?;

    // Update state
    state.mark_notified(&status);
    state.save()?;

    Ok(true)
}

/// Send a desktop notification
fn send_notification(status: &notify::UpdateStatus) -> Result<()> {
    let summary = status.summary();

    Notification::new()
        .summary("Forge Updates Available")
        .body(&format!("{}\n\nRun 'forge update' to apply.", summary))
        .icon("software-update-available")
        .urgency(Urgency::Normal)
        .timeout(NOTIFICATION_TIMEOUT_MS)
        .show()?;

    Ok(())
}

/// Set up logging to file
fn setup_logging(verbose: bool) -> Result<()> {
    let log_dir = forge_data_dir();
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, FORGE_LOG_FILE);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(level.into()))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    // Keep the guard alive for the duration of the program
    // Note: The guard is leaked to prevent the logger from being dropped
    // when the function returns. This is intentional for a long-running daemon.
    std::mem::forget(_guard);

    Ok(())
}
