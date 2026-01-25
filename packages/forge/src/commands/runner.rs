//! Command runner abstraction to reduce boilerplate in command modules
//!
//! Provides a unified way to:
//! - Spawn async commands with error handling
//! - Format output with headers/footers
//! - Handle step failures consistently

use anyhow::Result;
use tokio::sync::mpsc;

use super::errors::{ErrorContext, ParsedError};
use super::executor::{run_command, run_command_with_timeout};
use super::CommandMessage;

/// A helper for running commands with consistent formatting and error handling
pub struct CommandRunner<'a> {
    tx: &'a mpsc::Sender<CommandMessage>,
}

impl<'a> CommandRunner<'a> {
    /// Create a new command runner
    pub fn new(tx: &'a mpsc::Sender<CommandMessage>) -> Self {
        Self { tx }
    }

    /// Get a reference to the underlying sender
    pub fn tx(&self) -> &mpsc::Sender<CommandMessage> {
        self.tx
    }

    /// Send a stdout message
    pub async fn out(&self, msg: &str) {
        let _ = self.tx.send(CommandMessage::Stdout(msg.to_string())).await;
    }

    /// Send a stderr message
    pub async fn err(&self, msg: &str) {
        let _ = self.tx.send(CommandMessage::Stderr(msg.to_string())).await;
    }

    /// Print a header with title
    pub async fn header(&self, title: &str) {
        self.out("").await;
        self.out("==============================================").await;
        self.out(&format!("  {}", title)).await;
        self.out("==============================================").await;
        self.out("").await;
    }

    /// Print a footer
    pub async fn footer(&self) {
        self.out("").await;
        self.out("==============================================").await;
    }

    /// Run a command and return success status
    pub async fn run(&self, cmd: &str, args: &[&str]) -> Result<bool> {
        run_command(self.tx, cmd, args).await
    }

    /// Run a command with a custom timeout (in seconds)
    pub async fn run_with_timeout(&self, cmd: &str, args: &[&str], timeout_secs: u64) -> Result<bool> {
        run_command_with_timeout(self.tx, cmd, args, Some(timeout_secs)).await
    }

    /// Send a step complete message
    pub async fn step_complete(&self, step: &str) -> Result<()> {
        self.tx
            .send(CommandMessage::StepComplete {
                step: step.to_string(),
            })
            .await?;
        Ok(())
    }

    /// Send a step failed message
    pub async fn step_failed(&self, step: &str, error_msg: &str, operation: &str) -> Result<()> {
        self.tx
            .send(CommandMessage::StepFailed {
                step: step.to_string(),
                error: ParsedError::from_stderr(
                    error_msg,
                    ErrorContext {
                        operation: operation.to_string(),
                    },
                ),
            })
            .await?;
        Ok(())
    }

    /// Send done message
    pub async fn done(&self, success: bool) -> Result<()> {
        self.tx.send(CommandMessage::Done { success }).await?;
        Ok(())
    }

    /// Run a simple operation with header, command execution, and footer
    /// Returns the success status
    pub async fn run_simple_operation(
        &self,
        title: &str,
        cmd: &str,
        args: &[&str],
        success_msg: &str,
        failure_msg: &str,
    ) -> Result<bool> {
        self.header(title).await;

        let success = self.run(cmd, args).await?;

        if success {
            self.out("").await;
            self.out(&format!("  {}", success_msg)).await;
        } else {
            self.out("").await;
            self.out(&format!("  {}", failure_msg)).await;
        }

        self.footer().await;
        self.done(success).await?;

        Ok(success)
    }
}

/// Spawn an async command with standard error handling
/// This macro reduces boilerplate for the common pattern of spawning a task
/// that runs a command and handles errors consistently.
#[macro_export]
macro_rules! spawn_command {
    ($tx:expr, $operation:expr, $step:expr, $body:expr) => {{
        let tx = $tx;
        tokio::spawn(async move {
            if let Err(e) = $body(&tx).await {
                tracing::error!("{} failed: {}", $operation, e);
                let _ = tx
                    .send($crate::commands::CommandMessage::StepFailed {
                        step: $step.to_string(),
                        error: $crate::commands::errors::ParsedError::from_stderr(
                            &e.to_string(),
                            $crate::commands::errors::ErrorContext {
                                operation: $operation.to_string(),
                            },
                        ),
                    })
                    .await;
                let _ = tx
                    .send($crate::commands::CommandMessage::Done { success: false })
                    .await;
            }
        });
        Ok(())
    }};
}

/// Helper function to spawn a command task with error handling
pub fn spawn_with_error_handling<F, Fut>(
    tx: mpsc::Sender<CommandMessage>,
    operation: &'static str,
    step: &'static str,
    f: F,
) -> Result<()>
where
    F: FnOnce(mpsc::Sender<CommandMessage>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        if let Err(e) = f(tx.clone()).await {
            tracing::error!("{} failed: {}", operation, e);
            let _ = tx
                .send(CommandMessage::StepFailed {
                    step: step.to_string(),
                    error: ParsedError::from_stderr(
                        &e.to_string(),
                        ErrorContext {
                            operation: operation.to_string(),
                        },
                    ),
                })
                .await;
            let _ = tx.send(CommandMessage::Done { success: false }).await;
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_runner_out() {
        let (tx, mut rx) = mpsc::channel(10);
        let runner = CommandRunner::new(&tx);

        runner.out("test message").await;
        drop(tx);

        let msg = rx.recv().await.unwrap();
        match msg {
            CommandMessage::Stdout(s) => assert_eq!(s, "test message"),
            _ => panic!("Expected Stdout message"),
        }
    }

    #[tokio::test]
    async fn test_command_runner_header() {
        let (tx, mut rx) = mpsc::channel(10);
        let runner = CommandRunner::new(&tx);

        runner.header("Test Title").await;
        drop(tx);

        // Should receive: empty, separator, title, separator, empty
        let mut messages = Vec::new();
        while let Some(msg) = rx.recv().await {
            messages.push(msg);
        }
        assert_eq!(messages.len(), 5);
    }
}
