//! UI rendering module

mod layout;
mod screens;
pub mod theme;
pub mod widgets;

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, AppMode, AppProfileState, CreateHostState, InstallState, KeysState, PendingUpdates, UpdateState};

/// Main draw function - dispatches to appropriate screen
pub fn draw(frame: &mut Frame, app: &App) {
    match &app.mode {
        AppMode::MainMenu { selected } => {
            screens::main_menu::draw(frame, *selected, app);
        }
        AppMode::Install(state) => match state {
            InstallState::CloneRepository { output } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::install::draw_clone_repository(frame, &output_vec, app);
            }
            InstallState::SelectHost { selected } => {
                screens::install::draw_host_selection(frame, *selected, &app.hosts, app);
            }
            InstallState::SelectDisk {
                host,
                disks,
                selected,
            } => {
                screens::install::draw_disk_selection(frame, host, disks, *selected, app);
            }
            InstallState::EnterCredentials {
                host,
                disk,
                credentials,
                active_field,
                error,
            } => {
                screens::install::draw_enter_credentials(
                    frame, host, disk, credentials, active_field, error.as_deref(), app,
                );
            }
            InstallState::SelectSwapMode {
                host,
                disk,
                selected,
                ram_gb,
                ..
            } => {
                screens::install::draw_select_swap_mode(frame, host, disk, *selected, *ram_gb, app);
            }
            InstallState::Overview { host, disk, credentials, input, hardware_config, .. } => {
                screens::install::draw_overview(frame, host, disk, credentials, input, hardware_config.as_ref(), app);
            }
            InstallState::Running {
                host,
                disk,
                steps,
                output,
                ..
            } => {
                // Convert VecDeque to Vec for UI rendering
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::install::draw_running(frame, host, disk, steps, &output_vec, app);
            }
            InstallState::Complete {
                success,
                output,
                scroll_offset,
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::install::draw_complete(frame, *success, &output_vec, *scroll_offset, app);
            }
        },
        AppMode::Update(state) => match state {
            UpdateState::LocalChangesPrompt {
                changed_files,
                selected,
            } => {
                screens::update::draw_local_changes_prompt(frame, changed_files, *selected);
            }
            UpdateState::Running {
                steps, output, ..
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::update::draw_running(frame, steps, &output_vec, false, None, app);
            }
            UpdateState::Complete {
                steps,
                output,
                scroll_offset,
                ..
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::update::draw_running(frame, steps, &output_vec, true, *scroll_offset, app);
            }
        },
        AppMode::Apps(state) => match state {
            AppProfileState::Menu { selected } => {
                screens::apps::draw_menu(frame, *selected, app);
            }
            AppProfileState::Running {
                operation, output, ..
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::apps::draw_running(frame, operation, &output_vec, app);
            }
            AppProfileState::Status { output } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::apps::draw_status(frame, &output_vec, app);
            }
            AppProfileState::Complete {
                success,
                output,
                scroll_offset,
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::apps::draw_complete(frame, *success, &output_vec, *scroll_offset, app);
            }
        },
        AppMode::Keys(state) => match state {
            KeysState::Running {
                operation, output, ..
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::keys::draw_running(frame, operation, &output_vec, app);
            }
            KeysState::Complete {
                success,
                output,
                scroll_offset,
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::keys::draw_complete(frame, *success, &output_vec, *scroll_offset, app);
            }
        },
        AppMode::CreateHost(state) => match state {
            CreateHostState::DetectingHardware => {
                screens::create_host::draw_detecting_hardware(frame, app);
            }
            CreateHostState::ConfirmCpu {
                cpu,
                override_menu,
                selected,
                ..
            } => {
                screens::create_host::draw_confirm_cpu(frame, cpu, *override_menu, *selected, app);
            }
            CreateHostState::ConfirmGpu {
                cpu,
                gpu,
                override_menu,
                selected,
                ..
            } => {
                screens::create_host::draw_confirm_gpu(frame, cpu, gpu, *override_menu, *selected, app);
            }
            CreateHostState::ConfirmFormFactor {
                cpu,
                gpu,
                form_factor,
                override_menu,
                selected,
            } => {
                screens::create_host::draw_confirm_form_factor(
                    frame, cpu, gpu, form_factor, *override_menu, *selected, app,
                );
            }
            CreateHostState::SelectDisk {
                cpu,
                gpu,
                form_factor,
                disks,
                selected,
            } => {
                screens::create_host::draw_select_disk(
                    frame, cpu, gpu, form_factor, disks, *selected, app,
                );
            }
            CreateHostState::EnterHostname {
                cpu,
                gpu,
                form_factor,
                disk,
                input,
                error,
            } => {
                screens::create_host::draw_enter_hostname(
                    frame, cpu, gpu, form_factor, disk, input, error.as_deref(), app,
                );
            }
            CreateHostState::Review { config } => {
                screens::create_host::draw_review(frame, config, app);
            }
            CreateHostState::Generating {
                config,
                steps,
                output,
                ..
            } => {
                let output_vec: Vec<String> = output.iter().cloned().collect();
                screens::create_host::draw_generating(frame, config, steps, &output_vec, app);
            }
            CreateHostState::Complete { success, config } => {
                screens::create_host::draw_complete(frame, *success, config, app);
            }
        },
        AppMode::Quit => {}
    }

    // Render update dialog or commit list on top of any screen (but below exit confirm)
    if !app.show_exit_confirm {
        if app.pending_updates.viewing_commits {
            draw_commit_list(frame, &app.pending_updates);
        } else if app.pending_updates.has_updates() {
            draw_update_dialog(frame, &app.pending_updates);
        }
    }

    if app.show_reboot_confirm {
        draw_reboot_confirm(frame, &app.reboot_reasons);
    }

    // Render exit confirmation popup on top of any screen
    if app.show_exit_confirm {
        draw_exit_confirm(frame);
    }
}

/// Draw the exit confirmation popup centered on screen
fn draw_exit_confirm(frame: &mut Frame) {
    let area = frame.area();
    let popup_width = 40;
    let popup_height = 7;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Draw popup content
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("Are you sure you want to exit?", theme::text())),
        Line::from(""),
        Line::from(vec![
            Span::styled("[", theme::dim()),
            Span::styled("Enter/Y", theme::key_hint()),
            Span::styled("] Yes  [", theme::dim()),
            Span::styled("Esc/N", theme::key_hint()),
            Span::styled("] No", theme::dim()),
        ]),
    ])
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active())
            .title(Span::styled(" Exit ", theme::title())),
    );
    frame.render_widget(content, popup_area);
}

/// Draw the reboot confirmation popup centered on screen
fn draw_reboot_confirm(frame: &mut Frame, reasons: &[String]) {
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Reboot recommended to apply updates:",
        theme::text(),
    )));
    lines.push(Line::from(""));

    if reasons.is_empty() {
        lines.push(Line::from(Span::styled(
            "- System update requires a reboot",
            theme::text(),
        )));
    } else {
        for reason in reasons {
            lines.push(Line::from(Span::styled(format!("- {}", reason), theme::text())));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("Enter/Y", theme::key_hint()),
        Span::styled("] Reboot  [", theme::dim()),
        Span::styled("Esc/N", theme::key_hint()),
        Span::styled("] Later", theme::dim()),
    ]));

    let popup_height = (lines.len() as u16 + 2)
        .min(area.height.saturating_sub(4))
        .max(7);
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Draw popup content
    let content = Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::warning())
                .title(Span::styled(" Reboot ", theme::warning())),
        );
    frame.render_widget(content, popup_area);
}

/// Draw the combined update available dialog centered on screen
fn draw_update_dialog(frame: &mut Frame, updates: &PendingUpdates) {
    let area = frame.area();
    let both = updates.nixos_config && updates.app_profiles;

    // Calculate dialog size based on content
    let popup_width = 55;
    let popup_height = if both { 13 } else { 10 };
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Build content lines
    let mut lines = vec![Line::from("")];

    // Header message
    if both {
        lines.push(Line::from(Span::styled(
            "System and app profile updates available.",
            theme::text(),
        )));
    } else if updates.nixos_config {
        lines.push(Line::from(Span::styled(
            "NixOS configuration updates available.",
            theme::text(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "App profile updates available.",
            theme::text(),
        )));
    }

    lines.push(Line::from(""));

    // Build menu options in order:
    // 1. View NixOS updates (when nixos_config)
    // 2. Update app profiles (when app_profiles)
    // 3. Update all (when both)
    // 4. Dismiss (always)
    let mut option_idx = 0usize;

    if updates.nixos_config {
        let style = if updates.selected == option_idx {
            theme::selected()
        } else {
            theme::text()
        };
        let prefix = if updates.selected == option_idx { "> " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}View NixOS updates", prefix),
            style,
        )));
        option_idx += 1;
    }

    if updates.app_profiles {
        let style = if updates.selected == option_idx {
            theme::selected()
        } else {
            theme::text()
        };
        let prefix = if updates.selected == option_idx { "> " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}Update app profiles", prefix),
            style,
        )));
        option_idx += 1;
    }

    if both {
        let style = if updates.selected == option_idx {
            theme::selected()
        } else {
            theme::text()
        };
        let prefix = if updates.selected == option_idx { "> " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}Update all", prefix),
            style,
        )));
        option_idx += 1;
    }

    // Dismiss option
    let style = if updates.selected == option_idx {
        theme::selected()
    } else {
        theme::dim()
    };
    let prefix = if updates.selected == option_idx { "> " } else { "  " };
    lines.push(Line::from(Span::styled(format!("{}Dismiss", prefix), style)));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑/↓", theme::key_hint()),
        Span::styled("] Navigate  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Select  [", theme::dim()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled("] Dismiss", theme::dim()),
    ]));

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active())
            .title(Span::styled(" Updates Available ", theme::title())),
    );
    frame.render_widget(content, popup_area);
}

/// Draw the commit list view for NixOS config updates
fn draw_commit_list(frame: &mut Frame, updates: &PendingUpdates) {
    let area = frame.area();

    // Calculate popup size - larger to show commits
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Build content lines
    let mut lines = vec![Line::from("")];

    // Show commit count
    let commit_count = updates.commits.len();
    lines.push(Line::from(Span::styled(
        format!("{} new commit{} available:", commit_count, if commit_count == 1 { "" } else { "s" }),
        theme::text(),
    )));
    lines.push(Line::from(""));

    // Calculate visible area for commits (popup height - header - footer)
    let visible_commits = (popup_height as usize).saturating_sub(8);

    // Show commits with scroll
    let start = updates.commit_scroll;
    let end = (start + visible_commits).min(updates.commits.len());

    for (i, commit) in updates.commits.iter().enumerate().skip(start).take(visible_commits) {
        let is_current = i == updates.selected_commit;
        let style = if is_current {
            theme::selected()
        } else {
            theme::text()
        };

        // Truncate message to fit
        let max_msg_len = (popup_width as usize).saturating_sub(14);
        let msg = if commit.message.len() > max_msg_len {
            format!("{}...", &commit.message[..max_msg_len.saturating_sub(3)])
        } else {
            commit.message.clone()
        };

        let prefix = if is_current { "> " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}{}  {}", prefix, commit.hash, msg),
            style,
        )));
    }

    // Show scroll indicator if there are more commits
    if end < updates.commits.len() {
        lines.push(Line::from(Span::styled(
            format!("  ... and {} more", updates.commits.len() - end),
            theme::dim(),
        )));
    }

    // Add padding to fill space
    while lines.len() < (popup_height as usize).saturating_sub(4) {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑/↓", theme::key_hint()),
        Span::styled("] Scroll  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Update now  [", theme::dim()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled("] Back", theme::dim()),
    ]));

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active())
            .title(Span::styled(" Pending NixOS Config Updates ", theme::title())),
    );
    frame.render_widget(content, popup_area);
}
