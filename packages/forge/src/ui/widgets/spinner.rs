//! Animated spinner widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

use crate::ui::theme;

/// Braille spinner characters (same as install.sh)
const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct Spinner {
    state: usize,
    style: Style,
}

impl Spinner {
    pub fn new(state: usize) -> Self {
        Self {
            state,
            style: theme::info(),
        }
    }

    #[allow(dead_code)]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn char(&self) -> char {
        SPINNER_CHARS[self.state % SPINNER_CHARS.len()]
    }
}

impl Widget for Spinner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 1 || area.height < 1 {
            return;
        }
        buf.set_string(area.x, area.y, self.char().to_string(), self.style);
    }
}
