//! Key management screens (Age and SSH keys)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, KeysOp};
use crate::ui::theme;
use crate::ui::widgets::LogView;

/// Draw running operation screen
pub fn draw_running(frame: &mut Frame, operation: &KeysOp, output: &[String], app: &App) {
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
    let title = match operation {
        KeysOp::Setup => " Setting Up Keys (from 1Password) ",
        KeysOp::Backup => " Backing Up Keys ",
        KeysOp::Restore => " Restoring Keys ",
        KeysOp::Status => " Key Status ",
    };
    let header = Paragraph::new(Line::from(Span::styled(title, theme::title())))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border_active()),
        );
    frame.render_widget(header, chunks[0]);

    // Output with spinner
    let spinner_char = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
        [app.spinner_state % 10];
    let title = format!(" {} Running... ", spinner_char);
    let log = LogView::new(output).title(&title);
    frame.render_widget(log, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("Ctrl+C", theme::key_hint()),
        Span::styled("] Cancel", theme::dim()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}

/// Draw completion screen (shows output log)
pub fn draw_complete(
    frame: &mut Frame,
    success: bool,
    output: &[String],
    scroll_offset: Option<usize>,
    _app: &App,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let (title, style) = if success {
        (" Keys Operation Complete ", theme::success())
    } else {
        (" Keys Operation Failed ", theme::error())
    };
    let header = Paragraph::new(Line::from(Span::styled(title, style)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style),
        );
    frame.render_widget(header, chunks[0]);

    // Output log
    let mut log = LogView::new(output).title(" Output ");
    if let Some(offset) = scroll_offset {
        log = log.scroll_offset(offset);
    }
    frame.render_widget(log, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑↓", theme::key_hint()),
        Span::styled("] Scroll  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Done  [", theme::dim()),
        Span::styled("q", theme::key_hint()),
        Span::styled("] Quit", theme::dim()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
