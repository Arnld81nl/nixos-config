//! Cybex color theme

#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};

// Cybex brand colors - Cyan/Blue theme
pub const PRIMARY: Color = Color::Cyan;
pub const SECONDARY: Color = Color::Blue;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const TEXT: Color = Color::White;
pub const DIM: Color = Color::DarkGray;
pub const BG: Color = Color::Reset;

/// Title style (headers)
pub fn title() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

/// Normal text
pub fn text() -> Style {
    Style::default().fg(TEXT)
}

/// Dimmed/inactive text
pub fn dim() -> Style {
    Style::default().fg(DIM)
}

/// Selected/highlighted item
pub fn selected() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(PRIMARY)
        .add_modifier(Modifier::BOLD)
}

/// Success message
pub fn success() -> Style {
    Style::default().fg(SUCCESS)
}

/// Warning message
pub fn warning() -> Style {
    Style::default().fg(WARNING)
}

/// Error message
pub fn error() -> Style {
    Style::default().fg(ERROR)
}

/// Border style
pub fn border() -> Style {
    Style::default().fg(PRIMARY)
}

/// Active border (focused)
pub fn border_active() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

/// Key hint style
pub fn key_hint() -> Style {
    Style::default().fg(SECONDARY)
}

/// Version/info style
pub fn info() -> Style {
    Style::default().fg(SECONDARY)
}
