//! Command message handling for the application

use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

use super::state::{
    AppMode, AppProfileState, CommitInfo, CreateHostState, InstallState, KeysState, StepState,
    StepStatus, UpdateState,
};
use super::App;
use crate::commands::errors::ParsedError;
use crate::commands::executor::run_capture;
use crate::commands::CommandMessage;
use crate::constants::{nixos_config_dir, OUTPUT_BUFFER_SIZE};

/// Regex to match ANSI escape codes.
static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap());

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    ANSI_RE.replace_all(s, "").to_string()
}

impl App {
    /// Handle messages from running commands
    pub async fn handle_command_message(&mut self, msg: CommandMessage) -> Result<()> {
        match msg {
            CommandMessage::Stdout(line) | CommandMessage::Stderr(line) => {
                self.append_output(&line);
            }
            CommandMessage::StepComplete { step } => {
                self.mark_step_complete(&step);
            }
            CommandMessage::StepFailed { step, error } => {
                self.mark_step_failed(&step, error);
            }
            CommandMessage::StepSkipped { step } => {
                self.mark_step_skipped(&step);
            }
            CommandMessage::Done { success } => {
                self.handle_command_done(success).await;
            }
            CommandMessage::Cancelled => {
                self.handle_command_cancelled();
            }
            CommandMessage::UpdatesAvailable {
                nixos_config,
                app_profiles,
                commits,
            } => {
                self.startup_check_running = false;
                if (nixos_config || app_profiles) && matches!(self.mode, AppMode::MainMenu { .. }) {
                    self.pending_updates.nixos_config = nixos_config;
                    self.pending_updates.app_profiles = app_profiles;
                    self.pending_updates.commits = commits
                        .into_iter()
                        .map(|(hash, message)| CommitInfo { hash, message })
                        .collect();
                    self.pending_updates.selected = 0;
                    self.pending_updates.viewing_commits = false;
                    self.pending_updates.commit_scroll = 0;
                }
            }
            CommandMessage::RebootRecommended { reasons } => {
                self.show_reboot_confirm = true;
                self.reboot_reasons = reasons;
            }
            CommandMessage::CloneComplete { success } => {
                self.handle_clone_complete(success);
            }
        }
        Ok(())
    }

    fn handle_clone_complete(&mut self, success: bool) {
        if success {
            // Re-discover hosts from the newly cloned repository
            self.hosts = crate::system::config::discover_hosts();
            // Transition to host selection
            self.mode = AppMode::Install(InstallState::SelectHost { selected: 0 });
        } else {
            // Clone failed - show error screen
            use std::collections::VecDeque;
            let mut output = VecDeque::new();
            output.push_back("Failed to clone configuration repository.".to_string());
            output.push_back("".to_string());
            output.push_back("Please check:".to_string());
            output.push_back("  1. Internet connection (run 'nmtui' to configure WiFi)".to_string());
            output.push_back("  2. GitHub is accessible".to_string());
            output.push_back("".to_string());
            output.push_back("Press Enter to return to main menu.".to_string());
            self.mode = AppMode::Install(InstallState::Complete {
                success: false,
                output,
                scroll_offset: None,
            });
        }
    }

    fn append_output(&mut self, line: &str) {
        let clean_line = strip_ansi_codes(line);
        self.log_to_screen(&clean_line);

        match &mut self.mode {
            AppMode::Update(UpdateState::Running { output, .. })
            | AppMode::Update(UpdateState::Complete { output, .. }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            AppMode::Install(InstallState::CloneRepository { output }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            AppMode::Install(InstallState::Running { output, .. }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            AppMode::Apps(AppProfileState::Running { output, .. }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            AppMode::Apps(AppProfileState::Status { output }) => {
                output.push_back(clean_line);
            }
            AppMode::Keys(KeysState::Running { output, .. }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            AppMode::CreateHost(CreateHostState::Generating { output, .. }) => {
                output.push_back(clean_line);
                while output.len() > OUTPUT_BUFFER_SIZE {
                    output.pop_front();
                }
            }
            _ => {}
        }
    }

    /// Check if a step matches the given step name.
    fn step_matches(step: &StepStatus, step_name: &str) -> bool {
        let step_lower = step.name.to_lowercase();
        let name_lower = step_name.to_lowercase();

        if step_lower.contains(&name_lower) {
            return true;
        }

        if let Some(first_word) = step_lower.split_whitespace().next() {
            if first_word == name_lower || name_lower.contains(first_word) {
                return true;
            }
        }

        false
    }

    fn mark_step_complete(&mut self, step_name: &str) {
        self.log_to_screen(&format!("[✓] Step complete: {}", step_name));

        match &mut self.mode {
            AppMode::Update(UpdateState::Running { steps, step, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Complete;
                }
                *step = (*step + 1).min(steps.len());
                if *step < steps.len() {
                    steps[*step].status = StepState::Running;
                }
            }
            AppMode::Install(InstallState::Running { steps, step, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Complete;
                }
                *step = (*step + 1).min(steps.len());
                if *step < steps.len() {
                    steps[*step].status = StepState::Running;
                }
            }
            AppMode::CreateHost(CreateHostState::Generating { steps, step, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Complete;
                }
                *step = (*step + 1).min(steps.len());
                if *step < steps.len() {
                    steps[*step].status = StepState::Running;
                }
            }
            _ => {}
        }
    }

    fn mark_step_failed(&mut self, step_name: &str, error: ParsedError) {
        // Log formatted error to screen
        self.log_to_screen(&format!("[✗] Step failed: {}", step_name));
        self.log_to_screen("");
        self.log_to_screen(&format!("  Error: {}", error.summary));
        if let Some(ref detail) = error.detail {
            for line in detail.lines() {
                self.log_to_screen(&format!("  {}", line));
            }
        }
        self.log_to_screen("");
        self.log_to_screen(&format!("  Suggestion: {}", error.suggestion));

        match &mut self.mode {
            AppMode::Update(UpdateState::Running { steps, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Failed;
                }
                self.error = Some(error.summary);
            }
            AppMode::Install(InstallState::Running { steps, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Failed;
                }
                self.error = Some(error.summary);
            }
            AppMode::CreateHost(CreateHostState::Generating { steps, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Failed;
                }
                self.error = Some(error.summary);
            }
            _ => {}
        }
    }

    fn mark_step_skipped(&mut self, step_name: &str) {
        self.log_to_screen(&format!("[-] Step skipped: {}", step_name));

        match &mut self.mode {
            AppMode::Update(UpdateState::Running { steps, step, .. }) => {
                if let Some(s) = steps.iter_mut().find(|s| Self::step_matches(s, step_name)) {
                    s.status = StepState::Skipped;
                }
                *step = (*step + 1).min(steps.len());
                if *step < steps.len() {
                    steps[*step].status = StepState::Running;
                }
            }
            _ => {}
        }
    }

    async fn handle_command_done(&mut self, success: bool) {
        self.log_to_screen(&format!(
            "\n=== Operation {} ===\n",
            if success { "COMPLETED" } else { "FAILED" }
        ));

        match &mut self.mode {
            AppMode::Apps(AppProfileState::Running { output, .. }) => {
                self.mode = AppMode::Apps(AppProfileState::Complete {
                    success,
                    output: output.clone(),
                    scroll_offset: None, // None = auto-scroll continues
                });
            }
            AppMode::Keys(KeysState::Running { output, .. }) => {
                self.mode = AppMode::Keys(KeysState::Complete {
                    success,
                    output: output.clone(),
                    scroll_offset: None, // None = auto-scroll continues
                });
            }
            AppMode::Install(InstallState::Running { output, .. }) => {
                self.mode = AppMode::Install(InstallState::Complete {
                    success,
                    output: output.clone(),
                    scroll_offset: None, // None = auto-scroll continues
                });
            }
            AppMode::Update(UpdateState::Running {
                steps,
                output,
                stashed,
                ..
            }) => {
                let was_stashed = *stashed;
                let mut final_output = output.clone();

                // If we stashed changes and update succeeded, restore them
                if was_stashed && success {
                    let config_path = nixos_config_dir();
                    let config_str = config_path.to_string_lossy().to_string();

                    final_output.push_back("".to_string());
                    final_output.push_back("Restoring stashed changes...".to_string());

                    match run_capture("git", &["-C", &config_str, "stash", "pop"]).await {
                        Ok((pop_ok, stdout, stderr)) => {
                            if pop_ok {
                                final_output.push_back("  ✓ Stashed changes restored successfully".to_string());
                            } else {
                                final_output.push_back("  ✗ Failed to restore stashed changes".to_string());
                                if !stderr.is_empty() {
                                    final_output.push_back(format!("    {}", stderr.trim()));
                                }
                                final_output.push_back("    Run 'git stash pop' manually to restore".to_string());
                            }
                            if !stdout.is_empty() {
                                for line in stdout.lines() {
                                    final_output.push_back(format!("    {}", line));
                                }
                            }
                        }
                        Err(e) => {
                            final_output.push_back(format!("  ✗ Error restoring stash: {}", e));
                            final_output.push_back("    Run 'git stash pop' manually to restore".to_string());
                        }
                    }
                }

                self.mode = AppMode::Update(UpdateState::Complete {
                    success,
                    steps: steps.clone(),
                    output: final_output,
                    scroll_offset: None, // None = auto-scroll continues
                    stashed: was_stashed,
                });
            }
            AppMode::CreateHost(CreateHostState::Generating { config, .. }) => {
                self.mode = AppMode::CreateHost(CreateHostState::Complete {
                    success,
                    config: config.clone(),
                });
            }
            _ => {}
        }
    }

    fn handle_command_cancelled(&mut self) {
        self.log_to_screen("\n=== Operation CANCELLED ===\n");

        // Clear the cancellation token
        self.cancel_token = None;

        match &mut self.mode {
            AppMode::Apps(AppProfileState::Running { output, .. }) => {
                output.push_back("Operation cancelled by user.".to_string());
                self.mode = AppMode::Apps(AppProfileState::Complete {
                    success: false,
                    output: output.clone(),
                    scroll_offset: None,
                });
            }
            AppMode::Keys(KeysState::Running { output, .. }) => {
                output.push_back("Operation cancelled by user.".to_string());
                self.mode = AppMode::Keys(KeysState::Complete {
                    success: false,
                    output: output.clone(),
                    scroll_offset: None,
                });
            }
            AppMode::Install(InstallState::Running { output, .. }) => {
                output.push_back("Operation cancelled by user.".to_string());
                self.mode = AppMode::Install(InstallState::Complete {
                    success: false,
                    output: output.clone(),
                    scroll_offset: None,
                });
            }
            AppMode::Update(UpdateState::Running { steps, output, stashed, .. }) => {
                output.push_back("Operation cancelled by user.".to_string());
                self.mode = AppMode::Update(UpdateState::Complete {
                    success: false,
                    steps: steps.clone(),
                    output: output.clone(),
                    scroll_offset: None,
                    stashed: *stashed,
                });
            }
            AppMode::CreateHost(CreateHostState::Generating { config, output, .. }) => {
                output.push_back("Operation cancelled by user.".to_string());
                self.mode = AppMode::CreateHost(CreateHostState::Complete {
                    success: false,
                    config: config.clone(),
                });
            }
            _ => {}
        }
    }
}
