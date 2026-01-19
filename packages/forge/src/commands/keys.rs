//! Key management commands (Age and SSH keys)

use anyhow::Result;
use tokio::sync::mpsc;

use super::runner::{spawn_with_error_handling, CommandRunner};
use super::CommandMessage;

/// Start key setup from 1Password
pub async fn start_setup(tx: mpsc::Sender<CommandMessage>) -> Result<()> {
    spawn_with_error_handling(tx, "Key setup", "Setup", |tx| async move {
        let runner = CommandRunner::new(&tx);
        runner
            .run_simple_operation(
                "Key Setup (from 1Password)",
                "keys-setup",
                &[],
                "Keys set up successfully",
                "Setup failed",
            )
            .await?;
        Ok(())
    })
}

/// Start key backup
pub async fn start_backup(tx: mpsc::Sender<CommandMessage>) -> Result<()> {
    spawn_with_error_handling(tx, "Key backup", "Backup", |tx| async move {
        let runner = CommandRunner::new(&tx);
        runner
            .run_simple_operation(
                "Key Backup",
                "keys-backup",
                &["--push"],
                "Keys backed up successfully",
                "Backup failed",
            )
            .await?;
        Ok(())
    })
}

/// Start key restore
pub async fn start_restore(tx: mpsc::Sender<CommandMessage>, force: bool) -> Result<()> {
    spawn_with_error_handling(tx, "Key restore", "Restore", move |tx| async move {
        let runner = CommandRunner::new(&tx);
        let args: Vec<&str> = if force {
            vec!["--pull", "--force"]
        } else {
            vec!["--pull"]
        };
        runner
            .run_simple_operation(
                "Key Restore",
                "keys-restore",
                &args,
                "Keys restored successfully",
                "Restore failed",
            )
            .await?;
        Ok(())
    })
}

/// Start key status check
pub async fn start_status(tx: mpsc::Sender<CommandMessage>) -> Result<()> {
    tokio::spawn(async move {
        let runner = CommandRunner::new(&tx);
        runner.header("Key Status").await;

        let success = match runner.run("keys-status", &[]).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Key status check failed: {}", e);
                runner.err(&e.to_string()).await;
                false
            }
        };

        runner.footer().await;
        let _ = runner.done(success).await;
    });
    Ok(())
}
