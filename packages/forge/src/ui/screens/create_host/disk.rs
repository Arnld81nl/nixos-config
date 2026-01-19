//! Disk selection and hostname entry screens

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use super::helpers::{draw_footer, draw_header};
use crate::app::App;
use crate::system::disk::DiskInfo;
use crate::system::hardware::{CpuInfo, FormFactor, GpuInfo};
use crate::ui::layout::centered_rect;
use crate::ui::theme;

/// Draw hostname entry screen (comes after disk selection)
pub fn draw_enter_hostname(
    frame: &mut Frame,
    cpu: &CpuInfo,
    gpu: &GpuInfo,
    form_factor: &FormFactor,
    disk: &DiskInfo,
    input: &str,
    error: Option<&str>,
    _app: &App,
) {
    let area = frame.area();
    let center = centered_rect(65, 65, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(center);

    // Header
    draw_header(frame, chunks[0], "Enter Hostname");

    // Hardware summary
    let disk_model = disk.model.as_deref().unwrap_or("Unknown");
    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  CPU: ", theme::dim()),
            Span::styled(format!("{}", cpu.vendor), theme::text()),
            Span::styled(" | GPU: ", theme::dim()),
            Span::styled(format!("{}", gpu.vendor), theme::text()),
            Span::styled(" | Form: ", theme::dim()),
            Span::styled(format!("{}", form_factor), theme::text()),
        ]),
        Line::from(vec![
            Span::styled("  Disk: ", theme::dim()),
            Span::styled(&disk.path, theme::text()),
            Span::styled(format!(" ({}, {})", disk.size, disk_model), theme::dim()),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(Span::styled(" Configuration ", theme::title())),
    );
    frame.render_widget(summary, chunks[1]);

    // Instructions and input
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter a hostname for this configuration:",
            theme::text(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", theme::info()),
            Span::styled(input, theme::text()),
            Span::styled("_", theme::info()),
        ]),
    ];

    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(err, theme::error())));
    }

    let input_block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(Span::styled(" Hostname ", theme::title())),
    );
    frame.render_widget(input_block, chunks[2]);

    // Hint
    let hint = Paragraph::new(Line::from(Span::styled(
        "Use lowercase letters, numbers, and hyphens only",
        theme::dim(),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(hint, chunks[3]);

    // Footer
    draw_footer(frame, chunks[4], &["Enter Continue", "Esc Back"]);
}

/// Draw disk selection screen with partition tree view
pub fn draw_select_disk(
    frame: &mut Frame,
    cpu: &CpuInfo,
    gpu: &GpuInfo,
    form_factor: &FormFactor,
    disks: &[DiskInfo],
    selected: usize,
    _app: &App,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(centered_rect(80, 85, area));

    draw_header(frame, chunks[0], "Select Target Disk");

    // Hardware summary
    let summary = Paragraph::new(vec![Line::from(vec![
        Span::styled(" CPU: ", theme::dim()),
        Span::styled(format!("{}", cpu.vendor), theme::text()),
        Span::styled(" | GPU: ", theme::dim()),
        Span::styled(format!("{}", gpu.vendor), theme::text()),
        Span::styled(" | Form: ", theme::dim()),
        Span::styled(format!("{}", form_factor), theme::text()),
    ])])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border()),
    );
    frame.render_widget(summary, chunks[1]);

    if disks.is_empty() {
        let message = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No disks found!", theme::warning())),
            Line::from(""),
            Line::from(Span::styled(
                "Please check that disks are properly connected.",
                theme::dim(),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::warning())
                .title(Span::styled(" Available Disks ", theme::title())),
        );
        frame.render_widget(message, chunks[2]);
        draw_footer(frame, chunks[3], &["Esc Back"]);
        return;
    }

    // Build rows with disks and their partitions
    let mut rows: Vec<Row> = Vec::new();

    for (i, disk) in disks.iter().enumerate() {
        let prefix = if i == selected { ">" } else { " " };
        let style = if i == selected {
            theme::selected()
        } else {
            theme::text()
        };

        // Disk row
        rows.push(
            Row::new(vec![
                prefix.to_string(),
                disk.path.clone(),
                disk.size.clone(),
                disk.model.clone().unwrap_or_default(),
            ])
            .style(style),
        );

        // Partition rows (indented tree view)
        let part_count = disk.partitions.len();
        for (j, part) in disk.partitions.iter().enumerate() {
            let is_last = j == part_count - 1;
            let tree_char = if is_last { "└─" } else { "├─" };

            // Format: short device name, size, fstype (OS)
            let device_short = part.path.replace("/dev/", "");
            let os_str = part
                .os_type
                .as_ref()
                .map(|os| format!(" ({})", os))
                .unwrap_or_default();

            rows.push(
                Row::new(vec![
                    "".to_string(),
                    format!("   {} {}", tree_char, device_short),
                    part.size.clone(),
                    format!("{}{}", part.fstype, os_str),
                ])
                .style(theme::dim()),
            );
        }
    }

    // Disk table with partitions
    let header = Row::new(vec!["", "Device", "Size", "Type/Model"])
        .style(theme::title())
        .bottom_margin(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Min(25),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(Span::styled(" Select Target Disk ", theme::title())),
    );
    frame.render_widget(table, chunks[2]);

    draw_footer(frame, chunks[3], &["↑↓ Navigate", "Enter Select", "Esc Back"]);
}
