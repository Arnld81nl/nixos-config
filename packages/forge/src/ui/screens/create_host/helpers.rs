//! Shared helper functions for create host screens

use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme;

/// Draw a centered header with title
pub fn draw_header(frame: &mut Frame, area: Rect, title: &str) {
    let header = Paragraph::new(Line::from(Span::styled(title, theme::title())))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border_active()),
        );
    frame.render_widget(header, area);
}

/// Draw a footer with key hints
pub fn draw_footer(frame: &mut Frame, area: Rect, hints: &[&str]) {
    let spans: Vec<Span> = hints
        .iter()
        .enumerate()
        .flat_map(|(i, hint)| {
            let mut v = vec![];
            if i > 0 {
                v.push(Span::styled("  ", theme::dim()));
            }
            v.push(Span::styled("[", theme::dim()));
            let parts: Vec<&str> = hint.splitn(2, ' ').collect();
            if parts.len() == 2 {
                v.push(Span::styled(parts[0], theme::key_hint()));
                v.push(Span::styled(format!("] {}", parts[1]), theme::dim()));
            } else {
                v.push(Span::styled(*hint, theme::key_hint()));
                v.push(Span::styled("]", theme::dim()));
            }
            v
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    frame.render_widget(footer, area);
}
