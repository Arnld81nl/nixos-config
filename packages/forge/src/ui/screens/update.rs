//! Update screens

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, StepStatus};
use crate::ui::layout::progress_layout;
use crate::ui::theme;
use crate::ui::widgets::{LogView, ProgressSteps};

/// Draw running/complete update screen
pub fn draw_running(
    frame: &mut Frame,
    steps: &[StepStatus],
    output: &[String],
    complete: bool,
    scroll_offset: Option<usize>,
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
    let title = if complete {
        " Update Complete "
    } else {
        " NixOS System Update "
    };
    let header = Paragraph::new(Line::from(Span::styled(title, theme::title())))
        .alignment(Alignment::Center)
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

    let mut log = LogView::new(output).title(" Output ");
    if let Some(offset) = scroll_offset {
        log = log.scroll_offset(offset);
    }
    frame.render_widget(log, output_area);

    // Footer
    let footer = if complete {
        Paragraph::new(Line::from(vec![
            Span::styled("[", theme::dim()),
            Span::styled("↑↓", theme::key_hint()),
            Span::styled("] Scroll  [", theme::dim()),
            Span::styled("Enter", theme::key_hint()),
            Span::styled("] Done  [", theme::dim()),
            Span::styled("q", theme::key_hint()),
            Span::styled("] Quit", theme::dim()),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[", theme::dim()),
            Span::styled("Ctrl+C", theme::key_hint()),
            Span::styled("] Cancel", theme::dim()),
        ]))
    }
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}

/// Options for local changes resolution
const LOCAL_CHANGES_OPTIONS: &[&str] = &[
    "Overwrite - Discard all local changes",
    "Stash - Save changes, restore after update",
    "Cancel - Keep changes, abort update",
];

/// Draw local changes prompt dialog
pub fn draw_local_changes_prompt(frame: &mut Frame, changed_files: &[String], selected: usize) {
    let area = frame.area();

    // Calculate popup dimensions
    let popup_width = 60.min(area.width.saturating_sub(4));
    let file_list_height = changed_files.len().min(8) as u16;
    let popup_height = (12 + file_list_height).min(area.height.saturating_sub(4));

    // Center the popup
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Create layout inside popup
    let inner_area = Rect::new(
        popup_area.x + 1,
        popup_area.y + 1,
        popup_area.width.saturating_sub(2),
        popup_area.height.saturating_sub(2),
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),              // Title
            Constraint::Length(file_list_height + 2), // File list
            Constraint::Length(5),              // Options
            Constraint::Length(2),              // Footer
        ])
        .split(inner_area);

    // Draw border
    let border = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::warning())
        .title(Span::styled(" Local Changes Detected ", theme::warning()));
    frame.render_widget(border, popup_area);

    // Title/description
    let title = Paragraph::new(Line::from(vec![
        Span::styled("Your local repository has uncommitted changes:", theme::text()),
    ]))
    .alignment(Alignment::Left);
    frame.render_widget(title, chunks[0]);

    // File list
    let files: Vec<ListItem> = changed_files
        .iter()
        .take(8)
        .map(|f| {
            ListItem::new(Line::from(vec![
                Span::styled("  ", theme::dim()),
                Span::styled(f.clone(), theme::info()),
            ]))
        })
        .collect();

    let more_indicator = if changed_files.len() > 8 {
        format!("  ... and {} more", changed_files.len() - 8)
    } else {
        String::new()
    };

    let mut file_items = files;
    if !more_indicator.is_empty() {
        file_items.push(ListItem::new(Line::from(Span::styled(
            more_indicator,
            theme::dim(),
        ))));
    }

    let file_list = List::new(file_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border())
            .title(" Changed Files "),
    );
    frame.render_widget(file_list, chunks[1]);

    // Options
    let options: Vec<ListItem> = LOCAL_CHANGES_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let style = if i == selected {
                theme::selected()
            } else {
                theme::text()
            };
            let prefix = if i == selected { "▶ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{}{}", prefix, opt), style)))
        })
        .collect();

    let options_list = List::new(options);
    frame.render_widget(options_list, chunks[2]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑↓", theme::key_hint()),
        Span::styled("] Navigate  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Select  [", theme::dim()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled("] Cancel", theme::dim()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[3]);
}
