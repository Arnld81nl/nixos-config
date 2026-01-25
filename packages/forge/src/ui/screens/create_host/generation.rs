//! Configuration review, generation progress, and completion screens

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::helpers::{draw_footer, draw_header};
use crate::app::{App, NewHostConfig, StepStatus};
use crate::ui::layout::{centered_rect, progress_layout};
use crate::ui::theme;
use crate::ui::widgets::{LogView, ProgressSteps};

/// Draw review screen
pub fn draw_review(frame: &mut Frame, config: &NewHostConfig, _app: &App) {
    let area = frame.area();
    let center = centered_rect(70, 70, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(12),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(center);

    draw_header(frame, chunks[0], "Review Configuration");

    // Configuration summary
    let gpu_model = config.gpu.model.as_deref().unwrap_or("N/A");
    let disk_model = config.disk.model.as_deref().unwrap_or("Unknown");

    let summary = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Hostname:    ", theme::dim()),
            Span::styled(&config.hostname, theme::info()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CPU:         ", theme::dim()),
            Span::styled(format!("{}", config.cpu.vendor), theme::text()),
            Span::styled(format!(" ({})", config.cpu.model_name), theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("  GPU:         ", theme::dim()),
            Span::styled(format!("{}", config.gpu.vendor), theme::text()),
            Span::styled(format!(" ({})", gpu_model), theme::dim()),
        ]),
        Line::from(vec![
            Span::styled("  Form Factor: ", theme::dim()),
            Span::styled(format!("{}", config.form_factor), theme::text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Disk:        ", theme::dim()),
            Span::styled(&config.disk.path, theme::text()),
            Span::styled(format!(" ({}, {})", config.disk.size, disk_model), theme::dim()),
        ]),
        Line::from(""),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(Span::styled(" Configuration Summary ", theme::title())),
    );
    frame.render_widget(summary, chunks[1]);

    // Files to be created
    let files = Paragraph::new(vec![
        Line::from(Span::styled("Files to be created:", theme::dim())),
        Line::from(vec![
            Span::styled("  • ", theme::info()),
            Span::styled(
                format!("hosts/{}/default.nix", config.hostname),
                theme::text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • ", theme::info()),
            Span::styled(
                format!("hosts/{}/hardware-configuration.nix", config.hostname),
                theme::text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • ", theme::info()),
            Span::styled(
                format!("modules/disko/{}.nix", config.hostname),
                theme::text(),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border()),
    );
    frame.render_widget(files, chunks[2]);

    draw_footer(frame, chunks[3], &["Enter Create", "Esc Back"]);
}

/// Draw generating screen
pub fn draw_generating(
    frame: &mut Frame,
    config: &NewHostConfig,
    steps: &[StepStatus],
    output: &[String],
    app: &App,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" Creating Host Configuration | ", theme::title()),
        Span::styled("Host: ", theme::dim()),
        Span::styled(&config.hostname, theme::text()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active()),
    );
    frame.render_widget(header, chunks[0]);

    // Progress and output
    let (steps_area, output_area) = progress_layout(chunks[1]);

    let progress = ProgressSteps::new(steps, app.spinner_state).title(" Progress ");
    frame.render_widget(progress, steps_area);

    let log = LogView::new(output).title(" Output ");
    frame.render_widget(log, output_area);

    // Footer
    let footer = Paragraph::new(Line::from(Span::styled(
        "Creating configuration files...",
        theme::dim(),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}

/// Draw completion screen
pub fn draw_complete(
    frame: &mut Frame,
    success: bool,
    config: &crate::app::state::NewHostConfig,
    _app: &App,
) {
    let area = frame.area();
    let center = centered_rect(60, 50, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(center);

    // Header
    let (title, style) = if success {
        (" ✓ Host Created Successfully ", theme::success())
    } else {
        (" ✗ Host Creation Failed ", theme::error())
    };
    let header = Paragraph::new(Line::from(Span::styled(title, style)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style),
        );
    frame.render_widget(header, chunks[0]);

    if success {
        // Success message - auto-proceeding to install
        let message = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Host '", theme::text()),
                Span::styled(&config.hostname, theme::info()),
                Span::styled("' has been created.", theme::text()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Proceeding to installation...",
                theme::info(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to continue",
                theme::dim(),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border()),
        );
        frame.render_widget(message, chunks[1]);
        draw_footer(frame, chunks[2], &["Any key Continue", "q Quit"]);
    } else {
        // Failure message
        let message = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Failed to create host configuration.",
                theme::error(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Check the output for error details.",
                theme::dim(),
            )),
            Line::from(""),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::error()),
        );
        frame.render_widget(message, chunks[1]);
        draw_footer(frame, chunks[2], &["Enter Menu", "q Quit"]);
    }
}
