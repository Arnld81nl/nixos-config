//! Keyboard input handlers for the application

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::mem;

use super::state::*;
use super::App;
use crate::commands;
use crate::commands::executor::run_capture;
use crate::commands::update::{check_local_changes, get_default_branch};
use crate::constants::nixos_config_dir;
use crate::constants::MAX_INPUT_LENGTH;
use crate::system::hardware::{
    gpu_vendor_options, CpuInfo, CpuVendor, FormFactor, GpuInfo, GpuVendor,
};

/// Check if this is a Ctrl+C key event
fn is_ctrl_c(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Get total RAM size in GB (rounded up)
fn get_ram_size_gb() -> u64 {
    // Read from /proc/meminfo
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                // Format: "MemTotal:       16384000 kB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        // Convert KB to GB, rounded up
                        return (kb + 1024 * 1024 - 1) / (1024 * 1024);
                    }
                }
            }
        }
    }
    // Fallback to 8GB if detection fails
    8
}

impl App {
    /// Handle keyboard input
    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle reboot confirmation dialog
        if self.show_reboot_confirm {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let _ = crate::commands::executor::run_capture("sudo", &["reboot"]).await;
                    self.should_quit = true;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.show_reboot_confirm = false;
                    self.reboot_reasons.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle exit confirmation dialog
        if self.show_exit_confirm {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.should_quit = true;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.show_exit_confirm = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle commit list view
        if self.pending_updates.viewing_commits {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.pending_updates.selected_commit > 0 {
                        self.pending_updates.selected_commit -= 1;
                        // Scroll up if selection above visible area
                        if self.pending_updates.selected_commit < self.pending_updates.commit_scroll {
                            self.pending_updates.commit_scroll = self.pending_updates.selected_commit;
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self.pending_updates.commits.len().saturating_sub(1);
                    if self.pending_updates.selected_commit < max {
                        self.pending_updates.selected_commit += 1;
                        // Scroll down if selection below visible area (assume ~8 visible)
                        let visible = 8;
                        if self.pending_updates.selected_commit >= self.pending_updates.commit_scroll + visible {
                            self.pending_updates.commit_scroll = self.pending_updates.selected_commit - visible + 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    // Run system update
                    self.pending_updates.clear();
                    self.start_update_with_changes_check().await?;
                }
                KeyCode::Esc | KeyCode::Backspace => {
                    // Go back to main dialog
                    self.pending_updates.viewing_commits = false;
                    self.pending_updates.commit_scroll = 0;
                    self.pending_updates.selected_commit = 0;
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle update dialog
        if self.pending_updates.has_updates() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.pending_updates.selected > 0 {
                        self.pending_updates.selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = get_update_dialog_option_count(&self.pending_updates) - 1;
                    if self.pending_updates.selected < max {
                        self.pending_updates.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    self.handle_update_dialog_select().await?;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_updates.clear();
                }
                _ => {}
            }
            return Ok(());
        }

        // Global quit
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
            && matches!(
                self.mode,
                AppMode::MainMenu { .. }
                    | AppMode::Apps(AppProfileState::Menu { .. })
                    | AppMode::Apps(AppProfileState::Complete { .. })
                    | AppMode::Apps(AppProfileState::Status { .. })
                    | AppMode::Keys(KeysState::Complete { .. })
                    | AppMode::Update(UpdateState::Complete { .. })
                    | AppMode::Install(InstallState::Complete { .. })
                    | AppMode::CreateHost(CreateHostState::Complete { .. })
            )
        {
            self.show_exit_confirm = true;
            return Ok(());
        }

        // Escape to go back (show confirm if on main menu)
        if key.code == KeyCode::Esc {
            if matches!(self.mode, AppMode::MainMenu { .. }) {
                self.show_exit_confirm = true;
                return Ok(());
            }
            self.handle_back().await?;
            return Ok(());
        }

        // Handle Ctrl+C to cancel running operations
        if is_ctrl_c(&key) {
            if matches!(
                self.mode,
                AppMode::Update(UpdateState::Running { .. })
                    | AppMode::Install(InstallState::Running { .. })
                    | AppMode::Apps(AppProfileState::Running { .. })
                    | AppMode::Keys(KeysState::Running { .. })
                    | AppMode::CreateHost(CreateHostState::Generating { .. })
            ) {
                self.cancel_operation();
                return Ok(());
            }
        }

        // Handle local changes prompt dialog
        if let AppMode::Update(UpdateState::LocalChangesPrompt {
            changed_files,
            selected,
        }) = &mut self.mode
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1).min(2); // 3 options: 0, 1, 2
                }
                KeyCode::Enter => {
                    let resolution = match *selected {
                        0 => LocalChangesResolution::Overwrite,
                        1 => LocalChangesResolution::Stash,
                        _ => LocalChangesResolution::Cancel,
                    };
                    // Take ownership of changed_files before applying resolution
                    let _files = std::mem::take(changed_files);
                    self.apply_local_changes_resolution(resolution).await?;
                }
                KeyCode::Esc => {
                    // Cancel - return to main menu
                    self.mode = AppMode::MainMenu { selected: 1 };
                }
                _ => {}
            }
            return Ok(());
        }

        // Extract values from mode to avoid borrow conflicts
        let action = match &self.mode {
            AppMode::MainMenu { selected } => Some(("main_menu", *selected, None, None)),
            AppMode::Apps(AppProfileState::Menu { selected }) => {
                Some(("browser_menu", *selected, None, None))
            }
            AppMode::Install(InstallState::SelectHost { selected }) => {
                Some(("install_host", *selected, None, None))
            }
            AppMode::Install(InstallState::SelectDisk {
                host,
                disks,
                selected,
            }) => Some((
                "install_disk",
                *selected,
                Some(host.clone()),
                Some(disks.clone()),
            )),
            AppMode::Install(InstallState::EnterCredentials { host, .. }) => {
                Some(("install_credentials", 0, Some(host.clone()), None))
            }
            AppMode::Install(InstallState::SelectSwapMode { host, selected, .. }) => {
                Some(("install_swap_mode", *selected, Some(host.clone()), None))
            }
            AppMode::Install(InstallState::Overview { host, disk: _, .. }) => {
                Some(("install_overview", 0, Some(host.clone()), None))
            }
            AppMode::Install(InstallState::Complete { success, .. }) => match key.code {
                KeyCode::Enter => Some(("complete", 0, None, None)),
                KeyCode::Char('r') | KeyCode::Char('R') if *success => Some(("reboot", 0, None, None)),
                KeyCode::Up | KeyCode::Down => Some(("scroll", 0, None, None)),
                _ => None,
            },
            AppMode::Update(UpdateState::Complete { .. })
            | AppMode::Apps(AppProfileState::Complete { .. })
            | AppMode::Keys(KeysState::Complete { .. }) => match key.code {
                KeyCode::Enter => Some(("complete", 0, None, None)),
                KeyCode::Up | KeyCode::Down => Some(("scroll", 0, None, None)),
                _ => None,
            },
            AppMode::Apps(AppProfileState::Status { .. }) => {
                if key.code == KeyCode::Enter {
                    Some(("browser_done", 0, None, None))
                } else {
                    None
                }
            }
            AppMode::CreateHost(_) => Some(("create_host", 0, None, None)),
            _ => None,
        };

        match action {
            Some(("main_menu", selected, _, _)) => {
                self.handle_main_menu_key(key, selected).await?;
            }
            Some(("browser_menu", selected, _, _)) => {
                self.handle_app_menu_key(key, selected).await?;
            }
            Some(("install_host", selected, _, _)) => {
                self.handle_install_host_key(key, selected).await?;
            }
            Some(("install_disk", selected, Some(host), Some(disks))) => {
                self.handle_install_disk_key(key, &host, &disks, selected)
                    .await?;
            }
            Some(("install_credentials", _, Some(host), _)) => {
                self.handle_credentials_key(key, &host).await?;
            }
            Some(("install_swap_mode", selected, Some(_host), _)) => {
                self.handle_swap_mode_key(key, selected).await?;
            }
            Some(("install_overview", _, Some(host), _)) => {
                self.handle_overview_key_action(key, &host).await?;
            }
            Some(("complete", _, _, _)) => {
                self.mode = AppMode::MainMenu { selected: 0 };
            }
            Some(("reboot", _, _, _)) => {
                // Reboot the system
                self.should_quit = true;
                let _ = std::process::Command::new("reboot").spawn();
            }
            Some(("scroll", _, _, _)) => {
                self.handle_scroll(key);
            }
            Some(("browser_done", _, _, _)) => {
                self.mode = AppMode::Apps(AppProfileState::Menu { selected: 0 });
            }
            Some(("create_host", _, _, _)) => {
                self.handle_create_host_key(key).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle selection in the update dialog
    pub(super) async fn handle_update_dialog_select(&mut self) -> Result<()> {
        let has_nixos = self.pending_updates.nixos_config;
        let has_apps = self.pending_updates.app_profiles;
        let both = has_nixos && has_apps;
        let selected = self.pending_updates.selected;

        let mut idx = 0;

        // Check if View NixOS updates was selected
        if has_nixos {
            if selected == idx {
                self.pending_updates.viewing_commits = true;
                self.pending_updates.commit_scroll = 0;
                return Ok(());
            }
            idx += 1;
        }

        // Check if Update app profiles was selected
        if has_apps {
            if selected == idx {
                self.pending_updates.clear();
                self.mode = AppMode::Apps(AppProfileState::new_restore(false));
                self.start_initial_command().await?;
                return Ok(());
            }
            idx += 1;
        }

        // Check if Update all was selected
        if both && selected == idx {
            self.pending_updates.clear();
            self.start_update_with_changes_check().await?;
            return Ok(());
        }

        // Dismiss selected (or fallback)
        self.pending_updates.clear();
        Ok(())
    }

    async fn handle_main_menu_key(&mut self, key: KeyEvent, current_selected: usize) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let AppMode::MainMenu { selected } = &mut self.mode {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let AppMode::MainMenu { selected } = &mut self.mode {
                    *selected = (*selected + 1).min(MAIN_MENU_ITEMS.len() - 1);
                }
            }
            KeyCode::Enter => {
                self.handle_main_menu_select(current_selected).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_main_menu_select(&mut self, selected: usize) -> Result<()> {
        match selected {
            0 => {
                // Install - check if running from Live ISO
                if !crate::system::is_live_iso_environment() {
                    use std::collections::VecDeque;
                    let mut output = VecDeque::new();
                    output.push_back(
                        "Error: Install can only be run from a NixOS Live ISO.".to_string(),
                    );
                    output.push_back("".to_string());
                    output.push_back(
                        "You appear to be running on an installed system.".to_string(),
                    );
                    output.push_back("".to_string());
                    output.push_back("To install NixOS:".to_string());
                    output.push_back("  1. Boot from a NixOS minimal ISO".to_string());
                    output.push_back("  2. Connect to WiFi: nmtui".to_string());
                    output.push_back("  3. Run: nix run github:Arnld81nl/nixos-config".to_string());
                    output.push_back("  4. Select 'Install NixOS' from the menu".to_string());
                    self.mode = AppMode::Install(InstallState::Complete {
                        success: false,
                        output,
                        scroll_offset: None,
                    });
                } else {
                    // On live ISO, clone repository first to discover hosts
                    self.mode = AppMode::Install(InstallState::CloneRepository {
                        output: std::collections::VecDeque::new(),
                    });
                    self.start_initial_command().await?;
                }
            }
            1 => {
                // Update
                self.start_update_with_changes_check().await?;
            }
            2 => {
                // App profiles
                self.mode = AppMode::Apps(AppProfileState::Menu { selected: 0 });
            }
            3 => {
                // Exit
                self.should_quit = true;
            }
            _ => {}
        }
        Ok(())
    }

    /// Start update, checking for local changes first
    /// If local changes exist, shows a prompt dialog
    /// Otherwise, starts the update directly
    async fn start_update_with_changes_check(&mut self) -> Result<()> {
        let changed_files = check_local_changes();
        if changed_files.is_empty() {
            // No local changes, start update directly
            self.mode = AppMode::Update(UpdateState::new());
            self.start_initial_command().await?;
        } else {
            // Local changes detected, show prompt
            self.mode = AppMode::Update(UpdateState::LocalChangesPrompt {
                changed_files,
                selected: 0,
            });
        }
        Ok(())
    }

    /// Apply the selected resolution for local changes and start the update
    async fn apply_local_changes_resolution(
        &mut self,
        resolution: LocalChangesResolution,
    ) -> Result<()> {
        let config_path = nixos_config_dir();
        let config_str = config_path.to_string_lossy().to_string();

        match resolution {
            LocalChangesResolution::Overwrite => {
                // Get the default branch name
                let branch = get_default_branch();

                // Fetch from remote first
                let _ = run_capture("git", &["-C", &config_str, "fetch", "origin"]).await;

                // Reset to remote branch
                let (reset_ok, _, _) = run_capture(
                    "git",
                    &[
                        "-C",
                        &config_str,
                        "reset",
                        "--hard",
                        &format!("origin/{}", branch),
                    ],
                )
                .await?;

                if reset_ok {
                    // Also clean untracked files
                    let _ = run_capture("git", &["-C", &config_str, "clean", "-fd"]).await;
                }

                // Start update (no stash needed)
                self.mode = AppMode::Update(UpdateState::new_with_stash(false));
                self.start_initial_command().await?;
            }
            LocalChangesResolution::Stash => {
                // Stash changes
                let (stash_ok, _, _) = run_capture(
                    "git",
                    &[
                        "-C",
                        &config_str,
                        "stash",
                        "push",
                        "-m",
                        "forge-update-autostash",
                    ],
                )
                .await?;

                if stash_ok {
                    // Start update with stash flag
                    self.mode = AppMode::Update(UpdateState::new_with_stash(true));
                    self.start_initial_command().await?;
                } else {
                    // Stash failed, go back to main menu
                    self.error = Some("Failed to stash changes".to_string());
                    self.mode = AppMode::MainMenu { selected: 1 };
                }
            }
            LocalChangesResolution::Cancel => {
                // Just return to main menu
                self.mode = AppMode::MainMenu { selected: 1 };
            }
        }
        Ok(())
    }

    /// Handle scroll keys for complete screens
    fn handle_scroll(&mut self, key: KeyEvent) {
        // Calculate visible height from terminal size
        // Layout: header(3) + steps(10) + output(rest) + footer(2), output has borders(2)
        let visible_height = crossterm::terminal::size()
            .map(|(_, h)| (h as usize).saturating_sub(17)) // 3+10+2+2 = 17 lines of chrome
            .unwrap_or(20)
            .max(5); // Minimum 5 lines visible

        match &mut self.mode {
            AppMode::Install(InstallState::Complete {
                output,
                scroll_offset,
                ..
            })
            | AppMode::Update(UpdateState::Complete {
                output,
                scroll_offset,
                ..
            })
            | AppMode::Apps(AppProfileState::Complete {
                output,
                scroll_offset,
                ..
            })
            | AppMode::Keys(KeysState::Complete {
                output,
                scroll_offset,
                ..
            }) => {
                // Calculate max scroll position (can't scroll past where last line is visible)
                let max_scroll = output.len().saturating_sub(visible_height);

                // If in auto-scroll mode (None), calculate what the start position would be
                let current = scroll_offset.unwrap_or(max_scroll);

                match key.code {
                    KeyCode::Up => {
                        *scroll_offset = Some(current.saturating_sub(1));
                    }
                    KeyCode::Down => {
                        // Don't scroll past the point where last line is at bottom
                        if current < max_scroll {
                            *scroll_offset = Some(current + 1);
                        } else {
                            // Already at bottom, stay in auto-scroll or at max
                            *scroll_offset = Some(max_scroll);
                        }
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    }

    async fn handle_app_menu_key(&mut self, key: KeyEvent, selected: usize) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let AppMode::Apps(AppProfileState::Menu { selected }) = &mut self.mode {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let AppMode::Apps(AppProfileState::Menu { selected }) = &mut self.mode {
                    *selected = (*selected + 1).min(APP_MENU_ITEMS.len() - 1);
                }
            }
            KeyCode::Enter => match selected {
                0 => {
                    // Backup
                    self.mode = AppMode::Apps(AppProfileState::new_backup(false));
                    self.start_initial_command().await?;
                }
                1 => {
                    // Restore
                    self.mode = AppMode::Apps(AppProfileState::new_restore(false));
                    self.start_initial_command().await?;
                }
                2 => {
                    // Status
                    self.mode = AppMode::Apps(AppProfileState::new_status());
                    self.start_initial_command().await?;
                }
                3 => {
                    // Back
                    self.mode = AppMode::MainMenu { selected: 2 };
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    async fn handle_install_host_key(&mut self, key: KeyEvent, selected: usize) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let AppMode::Install(InstallState::SelectHost { selected }) = &mut self.mode {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let AppMode::Install(InstallState::SelectHost { selected }) = &mut self.mode {
                    // +1 for "New host configuration" option
                    *selected = (*selected + 1).min(self.hosts.len());
                }
            }
            KeyCode::Enter => {
                if selected == 0 {
                    // "New host configuration" selected
                    self.mode = AppMode::CreateHost(CreateHostState::new());
                    self.start_initial_command().await?;
                } else {
                    // Existing host selected (index - 1 because of "New host" option)
                    let host = self.hosts[selected - 1].name.clone();
                    self.mode = AppMode::Install(InstallState::SelectDisk {
                        host,
                        disks: Vec::new(),
                        selected: 0,
                    });
                    self.start_initial_command().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_install_disk_key(
        &mut self,
        key: KeyEvent,
        host: &str,
        disks: &[crate::system::disk::DiskInfo],
        selected: usize,
    ) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let AppMode::Install(InstallState::SelectDisk { selected, .. }) = &mut self.mode
                {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let AppMode::Install(InstallState::SelectDisk { selected, disks, .. }) =
                    &mut self.mode
                {
                    if !disks.is_empty() {
                        *selected = (*selected + 1).min(disks.len() - 1);
                    }
                }
            }
            KeyCode::Enter => {
                if !disks.is_empty() {
                    self.mode = AppMode::Install(InstallState::EnterCredentials {
                        host: host.to_string(),
                        disk: disks[selected].clone(),
                        credentials: InstallCredentials::default(),
                        active_field: CredentialField::Username,
                        error: None,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_credentials_key(&mut self, key: KeyEvent, _host: &str) -> Result<()> {
        if let AppMode::Install(InstallState::EnterCredentials {
            host,
            disk,
            credentials,
            active_field,
            error,
        }) = &mut self.mode
        {
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    // Move to next field
                    *active_field = match active_field {
                        CredentialField::Username => CredentialField::Password,
                        CredentialField::Password => CredentialField::ConfirmPassword,
                        CredentialField::ConfirmPassword => CredentialField::Username,
                    };
                    *error = None;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    // Move to previous field
                    *active_field = match active_field {
                        CredentialField::Username => CredentialField::ConfirmPassword,
                        CredentialField::Password => CredentialField::Username,
                        CredentialField::ConfirmPassword => CredentialField::Password,
                    };
                    *error = None;
                }
                KeyCode::Char(c) => {
                    let field = match active_field {
                        CredentialField::Username => &mut credentials.username,
                        CredentialField::Password => &mut credentials.password,
                        CredentialField::ConfirmPassword => &mut credentials.confirm_password,
                    };
                    if field.len() < MAX_INPUT_LENGTH {
                        // Auto-convert username to lowercase
                        let c = if *active_field == CredentialField::Username {
                            c.to_ascii_lowercase()
                        } else {
                            c
                        };
                        field.push(c);
                    }
                    *error = None;
                }
                KeyCode::Backspace => {
                    let field = match active_field {
                        CredentialField::Username => &mut credentials.username,
                        CredentialField::Password => &mut credentials.password,
                        CredentialField::ConfirmPassword => &mut credentials.confirm_password,
                    };
                    field.pop();
                    *error = None;
                }
                KeyCode::Enter => {
                    // Validate and proceed to swap mode selection
                    if let Some(err) = validate_username(&credentials.username) {
                        *error = Some(err);
                    } else if let Some(err) = validate_password(&credentials.password, &credentials.confirm_password) {
                        *error = Some(err);
                    } else {
                        // All valid, proceed to swap mode selection
                        // Get RAM size for display
                        let ram_gb = get_ram_size_gb();
                        self.mode = AppMode::Install(InstallState::SelectSwapMode {
                            host: host.clone(),
                            disk: disk.clone(),
                            credentials: credentials.clone(),
                            selected: 0,
                            ram_gb,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_swap_mode_key(&mut self, key: KeyEvent, _current_selected: usize) -> Result<()> {
        if let AppMode::Install(InstallState::SelectSwapMode {
            host,
            disk,
            credentials,
            selected,
            ram_gb: _,
        }) = &mut self.mode
        {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1).min(1); // 2 options: 0, 1
                }
                KeyCode::Enter => {
                    // Set swap mode based on selection
                    let swap_mode = if *selected == 0 {
                        SwapMode::ZramOnly
                    } else {
                        SwapMode::HibernateSupport
                    };

                    // Update credentials with selected swap mode
                    let mut creds = credentials.clone();
                    creds.swap_mode = swap_mode;

                    // Proceed to overview
                    self.mode = AppMode::Install(InstallState::Overview {
                        host: host.clone(),
                        disk: disk.clone(),
                        credentials: creds,
                        hardware_config: None,
                        input: String::new(),
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_overview_key_action(&mut self, key: KeyEvent, host: &str) -> Result<()> {
        let (disk, credentials, should_start) = if let AppMode::Install(InstallState::Overview {
            disk,
            credentials,
            input,
            ..
        }) = &mut self.mode
        {
            match key.code {
                KeyCode::Char(c) => {
                    if input.len() < MAX_INPUT_LENGTH {
                        input.push(c);
                    }
                    (None, None, false)
                }
                KeyCode::Backspace => {
                    input.pop();
                    (None, None, false)
                }
                KeyCode::Enter => {
                    if input.trim().eq_ignore_ascii_case("yes") {
                        (Some(disk.clone()), Some(credentials.clone()), true)
                    } else {
                        (None, None, false)
                    }
                }
                _ => (None, None, false),
            }
        } else {
            (None, None, false)
        };

        if should_start {
            if let (Some(disk), Some(creds)) = (disk, credentials) {
                let mut steps = vec![
                    StepStatus::new("Checking network connectivity"),
                    StepStatus::new("Enabling Nix flakes"),
                    StepStatus::new("Cloning configuration repository"),
                    StepStatus::new("Configuring disk device"),
                    StepStatus::new("Running disko (partitioning)"),
                    StepStatus::new("Installing NixOS"),
                    StepStatus::new("Setting up user account"),
                ];
                steps[0].status = StepState::Running;

                self.mode = AppMode::Install(InstallState::Running {
                    host: host.to_string(),
                    disk: disk.clone(),
                    credentials: creds.clone(),
                    step: 0,
                    steps,
                    output: std::collections::VecDeque::new(),
                });
                if let Some(tx) = &self.cmd_tx {
                    commands::install::start_install(
                        tx.clone(),
                        host,
                        &disk.path,
                        &creds.username,
                        &creds.password,
                        creds.swap_mode.clone(),
                    ).await?;
                }
            }
        }
        Ok(())
    }

    /// Handle keyboard input for create host wizard
    async fn handle_create_host_key(&mut self, key: KeyEvent) -> Result<()> {
        // For keys that don't transition state, handle them with mutable borrow
        let should_transition = match &mut self.mode {
            AppMode::CreateHost(CreateHostState::ConfirmCpu {
                override_menu,
                selected,
                ..
            }) => {
                if *override_menu {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            *selected = selected.saturating_sub(1);
                            false
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            *selected = (*selected + 1).min(1);
                            false
                        }
                        KeyCode::Enter => true,
                        _ => false,
                    }
                } else {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => true,
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            *override_menu = true;
                            false
                        }
                        _ => false,
                    }
                }
            }
            AppMode::CreateHost(CreateHostState::ConfirmGpu {
                override_menu,
                selected,
                gpu,
                ..
            }) => {
                if *override_menu {
                    let options = gpu_vendor_options(gpu.hybrid.is_some());
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            *selected = selected.saturating_sub(1);
                            false
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max_index = options.len().saturating_sub(1);
                            *selected = (*selected + 1).min(max_index);
                            false
                        }
                        KeyCode::Enter => true,
                        _ => false,
                    }
                } else {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => true,
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('o') => {
                            *override_menu = true;
                            false
                        }
                        _ => false,
                    }
                }
            }
            AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                override_menu,
                selected,
                ..
            }) => {
                if *override_menu {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            *selected = selected.saturating_sub(1);
                            false
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            *selected = (*selected + 1).min(1);
                            false
                        }
                        KeyCode::Enter => true,
                        _ => false,
                    }
                } else {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => true,
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            *override_menu = true;
                            false
                        }
                        _ => false,
                    }
                }
            }
            AppMode::CreateHost(CreateHostState::SelectDisk { disks, selected, .. }) => {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        *selected = selected.saturating_sub(1);
                        false
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !disks.is_empty() {
                            *selected = (*selected + 1).min(disks.len() - 1);
                        }
                        false
                    }
                    KeyCode::Enter if !disks.is_empty() => true,
                    _ => false,
                }
            }
            AppMode::CreateHost(CreateHostState::EnterHostname { input, error, .. }) => {
                match key.code {
                    KeyCode::Char(c) => {
                        if input.len() < MAX_INPUT_LENGTH && (c.is_alphanumeric() || c == '-') {
                            input.push(c.to_ascii_lowercase());
                            *error = None;
                        }
                        false
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        *error = None;
                        false
                    }
                    KeyCode::Enter => true,
                    _ => false,
                }
            }
            AppMode::CreateHost(CreateHostState::Review { .. }) => key.code == KeyCode::Enter,
            AppMode::CreateHost(CreateHostState::Complete { success, .. }) => {
                // Auto-proceed on any key for success, Enter for failure
                *success || key.code == KeyCode::Enter
            }
            _ => false,
        };

        if !should_transition {
            return Ok(());
        }

        // Take ownership for state transitions to avoid cloning
        let old_mode = mem::replace(&mut self.mode, AppMode::MainMenu { selected: 0 });
        let mut needs_initial_command = false;

        self.mode = match old_mode {
            AppMode::CreateHost(CreateHostState::ConfirmCpu {
                cpu,
                detected_gpu,
                detected_form_factor,
                override_menu,
                selected,
            }) => {
                let gpu_override = detected_gpu.vendor == GpuVendor::None;
                if override_menu {
                    let new_vendor = if selected == 0 {
                        CpuVendor::AMD
                    } else {
                        CpuVendor::Intel
                    };
                    AppMode::CreateHost(CreateHostState::ConfirmGpu {
                        cpu: CpuInfo {
                            vendor: new_vendor,
                            model_name: format!("{} (manually selected)", new_vendor),
                        },
                        gpu: detected_gpu,
                        detected_form_factor,
                        override_menu: gpu_override,
                        selected: 0,
                    })
                } else {
                    AppMode::CreateHost(CreateHostState::ConfirmGpu {
                        cpu,
                        gpu: detected_gpu,
                        detected_form_factor,
                        override_menu: gpu_override,
                        selected: 0,
                    })
                }
            }
            AppMode::CreateHost(CreateHostState::ConfirmGpu {
                cpu,
                gpu,
                detected_form_factor,
                override_menu,
                selected,
            }) => {
                if override_menu {
                    let options = gpu_vendor_options(gpu.hybrid.is_some());
                    let new_vendor = options
                        .get(selected)
                        .copied()
                        .unwrap_or(GpuVendor::None);
                    let hybrid = if new_vendor == GpuVendor::HybridNvidiaAmd {
                        gpu.hybrid.clone()
                    } else {
                        None
                    };
                    let model = if new_vendor == GpuVendor::HybridNvidiaAmd {
                        gpu.model.clone().or_else(|| {
                            Some(format!("{} (manually selected)", new_vendor))
                        })
                    } else {
                        Some(format!("{} (manually selected)", new_vendor))
                    };
                    AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                        cpu,
                        gpu: GpuInfo {
                            vendor: new_vendor,
                            model,
                            hybrid,
                        },
                        form_factor: detected_form_factor,
                        override_menu: false,
                        selected: 0,
                    })
                } else {
                    AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                        cpu,
                        gpu,
                        form_factor: detected_form_factor,
                        override_menu: false,
                        selected: 0,
                    })
                }
            }
            AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                cpu,
                gpu,
                form_factor,
                override_menu,
                selected,
            }) => {
                needs_initial_command = true;
                let ff = if override_menu {
                    if selected == 0 {
                        FormFactor::Desktop
                    } else {
                        FormFactor::Laptop
                    }
                } else {
                    form_factor
                };
                AppMode::CreateHost(CreateHostState::SelectDisk {
                    cpu,
                    gpu,
                    form_factor: ff,
                    disks: Vec::new(),
                    selected: 0,
                })
            }
            AppMode::CreateHost(CreateHostState::SelectDisk {
                cpu,
                gpu,
                form_factor,
                disks,
                selected,
            }) => {
                // Validate bounds before accessing - return to disk selection if invalid
                let Some(disk) = disks.into_iter().nth(selected) else {
                    return Ok(());
                };
                AppMode::CreateHost(CreateHostState::EnterHostname {
                    cpu,
                    gpu,
                    form_factor,
                    disk,
                    input: String::new(),
                    error: None,
                })
            }
            AppMode::CreateHost(CreateHostState::EnterHostname {
                cpu,
                gpu,
                form_factor,
                disk,
                input,
                ..
            }) => {
                let hostname = input.trim().to_string();
                if let Some(err) = validate_hostname(&hostname, &self.hosts) {
                    AppMode::CreateHost(CreateHostState::EnterHostname {
                        cpu,
                        gpu,
                        form_factor,
                        disk,
                        input,
                        error: Some(err),
                    })
                } else {
                    AppMode::CreateHost(CreateHostState::Review {
                        config: NewHostConfig {
                            hostname,
                            cpu,
                            gpu,
                            form_factor,
                            disk,
                        },
                    })
                }
            }
            AppMode::CreateHost(CreateHostState::Review { config }) => {
                let mut steps = if crate::system::is_live_iso_environment() {
                    vec![
                        StepStatus::new("Cloning configuration repository"),
                        StepStatus::new("Creating host directory"),
                        StepStatus::new("Generating hardware configuration"),
                        StepStatus::new("Creating host configuration"),
                        StepStatus::new("Creating disko configuration"),
                        StepStatus::new("Updating flake.nix"),
                        StepStatus::new("Generating host metadata"),
                    ]
                } else {
                    vec![
                        StepStatus::new("Creating host directory"),
                        StepStatus::new("Generating hardware configuration"),
                        StepStatus::new("Creating host configuration"),
                        StepStatus::new("Creating disko configuration"),
                        StepStatus::new("Updating flake.nix"),
                        StepStatus::new("Generating host metadata"),
                    ]
                };
                steps[0].status = StepState::Running;

                let new_mode = AppMode::CreateHost(CreateHostState::Generating {
                    config,
                    step: 0,
                    steps,
                    output: std::collections::VecDeque::new(),
                });

                if let Some(tx) = &self.cmd_tx {
                    commands::create_host::start_create_host(tx.clone(), new_mode.clone()).await?;
                }
                new_mode
            }
            AppMode::CreateHost(CreateHostState::Complete { config, success }) => {
                if success {
                    // Auto-proceed to install credentials entry
                    AppMode::Install(InstallState::EnterCredentials {
                        host: config.hostname.clone(),
                        disk: config.disk.clone(),
                        credentials: InstallCredentials::default(),
                        active_field: CredentialField::Username,
                        error: None,
                    })
                } else {
                    AppMode::Install(InstallState::SelectHost { selected: 0 })
                }
            }
            other => other,
        };

        if needs_initial_command {
            self.start_initial_command().await?;
        }

        Ok(())
    }

    pub(super) async fn handle_back(&mut self) -> Result<()> {
        // Take ownership of the mode to avoid cloning
        let old_mode = mem::replace(&mut self.mode, AppMode::MainMenu { selected: 0 });

        let needs_disk_refresh = matches!(
            old_mode,
            AppMode::Install(InstallState::EnterCredentials { .. })
                | AppMode::Install(InstallState::SelectSwapMode { .. })
                | AppMode::Install(InstallState::Overview { .. })
                | AppMode::CreateHost(CreateHostState::EnterHostname { .. })
        );

        self.mode = match old_mode {
            AppMode::Apps(AppProfileState::Menu { .. }) => AppMode::MainMenu { selected: 2 },
            AppMode::Apps(AppProfileState::Complete { .. })
            | AppMode::Apps(AppProfileState::Status { .. }) => {
                AppMode::Apps(AppProfileState::Menu { selected: 0 })
            }
            AppMode::Keys(KeysState::Complete { .. }) => AppMode::MainMenu { selected: 2 },
            AppMode::Install(InstallState::SelectHost { .. }) => {
                AppMode::MainMenu { selected: 0 }
            }
            AppMode::Install(InstallState::SelectDisk { .. }) => {
                AppMode::Install(InstallState::SelectHost { selected: 0 })
            }
            AppMode::Install(InstallState::EnterCredentials { host, disk, .. }) => {
                // Go back to disk selection
                AppMode::Install(InstallState::SelectDisk {
                    host,
                    disks: vec![disk], // Keep the selected disk
                    selected: 0,
                })
            }
            AppMode::Install(InstallState::SelectSwapMode { host, disk, credentials, .. }) => {
                // Go back to credentials entry
                AppMode::Install(InstallState::EnterCredentials {
                    host,
                    disk,
                    credentials,
                    active_field: CredentialField::Username,
                    error: None,
                })
            }
            AppMode::Install(InstallState::Overview { host, disk, credentials, .. }) => {
                // Go back to swap mode selection
                let ram_gb = get_ram_size_gb();
                AppMode::Install(InstallState::SelectSwapMode {
                    host,
                    disk,
                    credentials,
                    selected: 0,
                    ram_gb,
                })
            }
            AppMode::Install(InstallState::Complete { .. }) => AppMode::MainMenu { selected: 0 },
            AppMode::Update(UpdateState::Complete { .. }) => AppMode::MainMenu { selected: 1 },
            // CreateHost back navigation - take ownership to avoid clones
            AppMode::CreateHost(CreateHostState::DetectingHardware) => {
                AppMode::Install(InstallState::SelectHost { selected: 0 })
            }
            AppMode::CreateHost(CreateHostState::ConfirmCpu { .. }) => {
                AppMode::Install(InstallState::SelectHost { selected: 0 })
            }
            AppMode::CreateHost(CreateHostState::ConfirmGpu {
                cpu,
                gpu,
                detected_form_factor,
                ..
            }) => AppMode::CreateHost(CreateHostState::ConfirmCpu {
                cpu,
                detected_gpu: gpu,
                detected_form_factor,
                override_menu: false,
                selected: 0,
            }),
            AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                cpu,
                gpu,
                form_factor,
                ..
            }) => AppMode::CreateHost(CreateHostState::ConfirmGpu {
                cpu,
                gpu,
                detected_form_factor: form_factor,
                override_menu: false,
                selected: 0,
            }),
            AppMode::CreateHost(CreateHostState::SelectDisk {
                cpu,
                gpu,
                form_factor,
                ..
            }) => AppMode::CreateHost(CreateHostState::ConfirmFormFactor {
                cpu,
                gpu,
                form_factor,
                override_menu: false,
                selected: 0,
            }),
            AppMode::CreateHost(CreateHostState::EnterHostname {
                cpu,
                gpu,
                form_factor,
                ..
            }) => AppMode::CreateHost(CreateHostState::SelectDisk {
                cpu,
                gpu,
                form_factor,
                disks: Vec::new(),
                selected: 0,
            }),
            AppMode::CreateHost(CreateHostState::Review { config }) => {
                // Destructure to take ownership of all fields
                let NewHostConfig {
                    hostname,
                    cpu,
                    gpu,
                    form_factor,
                    disk,
                } = config;
                AppMode::CreateHost(CreateHostState::EnterHostname {
                    cpu,
                    gpu,
                    form_factor,
                    disk,
                    input: hostname,
                    error: None,
                })
            }
            AppMode::CreateHost(CreateHostState::Complete { .. }) => {
                AppMode::Install(InstallState::SelectHost { selected: 0 })
            }
            other => {
                // Restore the original mode if no match
                self.mode = other;
                return Ok(());
            }
        };

        if needs_disk_refresh {
            self.start_initial_command().await?;
        }

        Ok(())
    }
}
