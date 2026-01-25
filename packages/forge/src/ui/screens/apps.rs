//! App profile management screens (browser sessions and logins)

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, AppOp, APP_MENU_ITEMS};
use crate::ui::layout::centered_rect;
use crate::ui::theme;
use crate::ui::widgets::{LogView, MenuList};

/// Draw app profiles menu
pub fn draw_menu(frame: &mut Frame, selected: usize, _app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(centered_rect(60, 80, area));

    // Header
    let header = Paragraph::new(Line::from(Span::styled(
        " App Profiles ",
        theme::title(),
    )))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active()),
    );
    frame.render_widget(header, chunks[0]);

    // Menu
    let menu = MenuList::new(APP_MENU_ITEMS.to_vec(), selected);
    frame.render_widget(menu, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑↓", theme::key_hint()),
        Span::styled("] Navigate  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Select  [", theme::dim()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled("] Back", theme::dim()),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}

/// Draw running operation screen
pub fn draw_running(frame: &mut Frame, operation: &AppOp, output: &[String], app: &App) {
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
        AppOp::Backup => " Backing Up App Profiles ",
        AppOp::Restore => " Restoring App Profiles ",
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

/// Draw status screen
pub fn draw_status(frame: &mut Frame, output: &[String], _app: &App) {
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
    let header = Paragraph::new(Line::from(Span::styled(
        " App Profile Status ",
        theme::title(),
    )))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_active()),
    );
    frame.render_widget(header, chunks[0]);

    // Output
    let log = LogView::new(output).title(" Status ");
    frame.render_widget(log, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("[", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Done  [", theme::dim()),
        Span::styled("Esc", theme::key_hint()),
        Span::styled("] Back", theme::dim()),
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
        (" Operation Complete ", theme::success())
    } else {
        (" Operation Failed ", theme::error())
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
