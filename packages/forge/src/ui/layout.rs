//! Common layout helpers

#![allow(dead_code)]

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Create a centered box with specified percentage width and height
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

/// Create a centered box with fixed width and height
pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Split area into header, content, and footer
pub fn main_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Header
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

/// Split content area for progress screen (steps + output)
pub fn progress_layout(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Steps
            Constraint::Min(5),     // Output
        ])
        .split(area);
    (chunks[0], chunks[1])
}

/// Split content area for host selection (list + preview)
pub fn host_selection_layout(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Host list
            Constraint::Percentage(65), // Preview panel
        ])
        .split(area);
    (chunks[0], chunks[1])
}
