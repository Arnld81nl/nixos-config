//! Application state management
//!
//! This module contains the core application state and is split into:
//! - `state.rs` - State type definitions (AppMode, InstallState, etc.)
//! - `handlers.rs` - Keyboard input handlers
//! - `messages.rs` - Command message handling

mod handlers;
mod messages;
pub mod state;

use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::commands::{self, CommandMessage};
use crate::constants::SPINNER_TICK_MS;
use crate::system::config::{discover_hosts, HostConfig};
use crate::system::hardware::{CpuVendor, GpuInfo, GpuVendor};

// Re-export commonly used types
pub use state::{
    AppMode, AppOp, AppProfileState, CreateHostState, CredentialField, InstallCredentials,
    InstallState, KeysOp, KeysState, NewHostConfig, PendingUpdates, StepState, StepStatus,
    SwapMode, UpdateState, UpdateSummary, APP_MENU_ITEMS, MAIN_MENU_ITEMS,
};

/// Main application state
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub show_exit_confirm: bool,
    pub show_reboot_confirm: bool,
    pub reboot_reasons: Vec<String>,
    /// Available updates detected during startup check
    pub pending_updates: PendingUpdates,
    /// Whether the startup update check is in progress
    pub startup_check_running: bool,
    pub spinner_state: usize,
    pub last_tick: Instant,
    pub error: Option<String>,
    pub hosts: Vec<HostConfig>,
    pub(crate) cmd_tx: Option<mpsc::Sender<CommandMessage>>,
    /// Cancellation token for running operations
    pub(crate) cancel_token: Option<CancellationToken>,
    screen_log: Option<File>,
    pub screen_log_path: PathBuf,
}

impl App {
    pub fn new(initial_mode: AppMode) -> Self {
        // Set up screen log file
        let log_dir = crate::constants::forge_data_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let screen_log_path = log_dir.join(crate::constants::SCREEN_LOG_FILE);

        // Open log file (truncate existing)
        let mut screen_log = match File::create(&screen_log_path) {
            Ok(file) => Some(file),
            Err(e) => {
                tracing::warn!("Failed to create screen log file: {}", e);
                None
            }
        };

        // Write header to log
        if let Some(ref mut file) = screen_log {
            let _ = writeln!(file, "=== Forge Screen Log ===\n");
            let _ = file.flush();
        }

        Self {
            mode: initial_mode,
            should_quit: false,
            show_exit_confirm: false,
            show_reboot_confirm: false,
            reboot_reasons: Vec::new(),
            pending_updates: PendingUpdates::default(),
            startup_check_running: false,
            spinner_state: 0,
            last_tick: Instant::now(),
            error: None,
            hosts: discover_hosts(),
            cmd_tx: None,
            cancel_token: None,
            screen_log,
            screen_log_path,
        }
    }

    pub fn set_command_sender(&mut self, tx: mpsc::Sender<CommandMessage>) {
        self.cmd_tx = Some(tx);
    }

    /// Write a line to the screen log file
    pub fn log_to_screen(&mut self, line: &str) {
        if let Some(ref mut file) = self.screen_log {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
        }
    }

    /// Cancel any running operation
    pub fn cancel_operation(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
    }

    /// Create a new cancellation token for an operation
    pub fn new_cancel_token(&mut self) -> CancellationToken {
        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        token
    }

    /// Called on each tick to update animations
    pub fn tick(&mut self) {
        if self.last_tick.elapsed().as_millis() >= SPINNER_TICK_MS {
            self.spinner_state = (self.spinner_state + 1) % 10;
            self.last_tick = Instant::now();
        }
    }

    /// Start initial command if mode requires it
    pub async fn start_initial_command(&mut self) -> Result<()> {
        match &mut self.mode {
            AppMode::Update(UpdateState::Running { steps, .. }) => {
                if !steps.is_empty() {
                    steps[0].status = StepState::Running;
                }
                let tx = self.cmd_tx.clone();
                if let Some(tx) = tx {
                    let cancel = self.new_cancel_token();
                    commands::update::start_update(tx, cancel).await?;
                }
            }
            AppMode::Apps(AppProfileState::Running {
                operation, force, ..
            }) => {
                if let Some(tx) = &self.cmd_tx {
                    match operation {
                        AppOp::Backup => {
                            commands::apps::start_backup(tx.clone(), *force).await?;
                        }
                        AppOp::Restore => {
                            commands::apps::start_restore(tx.clone(), *force).await?;
                        }
                    }
                }
            }
            AppMode::Apps(AppProfileState::Status { .. }) => {
                if let Some(tx) = &self.cmd_tx {
                    commands::apps::start_status(tx.clone()).await?;
                }
            }
            AppMode::Keys(KeysState::Running {
                operation, force, ..
            }) => {
                if let Some(tx) = &self.cmd_tx {
                    match operation {
                        KeysOp::Setup => {
                            commands::keys::start_setup(tx.clone()).await?;
                        }
                        KeysOp::Backup => {
                            commands::keys::start_backup(tx.clone()).await?;
                        }
                        KeysOp::Restore => {
                            commands::keys::start_restore(tx.clone(), *force).await?;
                        }
                        KeysOp::Status => {
                            commands::keys::start_status(tx.clone()).await?;
                        }
                    }
                }
            }
            AppMode::Install(InstallState::CloneRepository { .. }) => {
                if let Some(tx) = &self.cmd_tx {
                    commands::install::start_clone_repository(tx.clone()).await?;
                }
            }
            AppMode::Install(InstallState::SelectDisk { disks, .. }) => {
                *disks = crate::system::disk::get_available_disks()?;
            }
            AppMode::CreateHost(CreateHostState::DetectingHardware) => {
                match crate::system::hardware::detect_all() {
                    Ok(hw) => {
                        let cpu_override = hw.cpu.vendor == CpuVendor::Unknown;
                        self.mode = AppMode::CreateHost(CreateHostState::ConfirmCpu {
                            cpu: hw.cpu,
                            detected_gpu: hw.gpu,
                            detected_form_factor: hw.form_factor,
                            override_menu: cpu_override,
                            selected: 0,
                        });
                    }
                    Err(e) => {
                        tracing::error!("Hardware detection failed: {}", e);
                        self.mode = AppMode::CreateHost(CreateHostState::ConfirmCpu {
                            cpu: crate::system::hardware::CpuInfo {
                                vendor: CpuVendor::Unknown,
                                model_name: "Unknown (detection failed)".to_string(),
                            },
                            detected_gpu: GpuInfo {
                                vendor: GpuVendor::None,
                                model: None,
                                hybrid: None,
                            },
                            detected_form_factor: crate::system::hardware::FormFactor::Desktop,
                            override_menu: true,
                            selected: 0,
                        });
                    }
                }
            }
            AppMode::CreateHost(CreateHostState::SelectDisk { disks, .. }) => {
                *disks = crate::system::disk::get_available_disks()?;
            }
            AppMode::MainMenu { .. } => {
                if let Some(tx) = &self.cmd_tx {
                    self.startup_check_running = true;
                    commands::apps::start_quick_update_check(tx.clone()).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
