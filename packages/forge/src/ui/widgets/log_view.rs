//! Scrollable log output widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::ui::theme;

pub struct LogView<'a> {
    lines: &'a [String],
    title: Option<&'a str>,
    auto_scroll: bool,
    scroll_offset: Option<usize>,
}

impl<'a> LogView<'a> {
    pub fn new(lines: &'a [String]) -> Self {
        Self {
            lines,
            title: None,
            auto_scroll: true,
            scroll_offset: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    #[allow(dead_code)]
    pub fn auto_scroll(mut self, auto_scroll: bool) -> Self {
        self.auto_scroll = auto_scroll;
        self
    }

    /// Set a manual scroll offset (disables auto_scroll)
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = Some(offset);
        self.auto_scroll = false;
        self
    }
}

impl Widget for LogView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border());

        if let Some(title) = self.title {
            block = block.title(Span::styled(title, theme::title()));
        }

        // Calculate visible lines
        let inner_height = area.height.saturating_sub(2) as usize; // Account for borders
        let start = if let Some(offset) = self.scroll_offset {
            // Manual scroll mode: use provided offset
            offset.min(self.lines.len().saturating_sub(1))
        } else if self.auto_scroll && self.lines.len() > inner_height && inner_height > 0 {
            // Auto-scroll mode: show most recent lines
            self.lines.len().saturating_sub(inner_height)
        } else {
            0
        };
        let end = (start + inner_height).min(self.lines.len());

        let visible_lines: Vec<Line> = self.lines[start..end]
            .iter()
            .map(|line| {
                // Simple color parsing for common patterns
                let style = if line.contains("[ERROR]") || line.contains("error:") {
                    theme::error()
                } else if line.contains("[WARN]") || line.contains("warning:") {
                    theme::warning()
                } else if line.contains("[SUCCESS]") || line.starts_with("✓") {
                    theme::success()
                } else if line.starts_with('>') || line.starts_with("  >") {
                    theme::info()
                } else {
                    theme::text()
                };
                Line::from(Span::styled(line.as_str(), style))
            })
            .collect();

        let paragraph = Paragraph::new(visible_lines)
            .block(block)
            .wrap(Wrap { trim: false });

        paragraph.render(area, buf);
    }
}
