//! Selectable menu list widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

use crate::ui::theme;

pub struct MenuList<'a> {
    items: Vec<&'a str>,
    selected: usize,
    title: Option<&'a str>,
}

impl<'a> MenuList<'a> {
    pub fn new(items: Vec<&'a str>, selected: usize) -> Self {
        // Clamp selected index to valid range to prevent out-of-bounds access
        let clamped_selected = if items.is_empty() {
            0
        } else {
            selected.min(items.len() - 1)
        };

        Self {
            items,
            selected: clamped_selected,
            title: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl Widget for MenuList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let prefix = if i == self.selected { "> " } else { "  " };
                let style = if i == self.selected {
                    theme::selected()
                } else {
                    theme::text()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(*item, style),
                ]))
            })
            .collect();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border());

        if let Some(title) = self.title {
            block = block.title(Span::styled(title, theme::title()));
        }

        let list = List::new(items).block(block);

        // Use StatefulWidget to highlight selected item
        let mut state = ListState::default().with_selected(Some(self.selected));
        StatefulWidget::render(list, area, buf, &mut state);
    }
}
