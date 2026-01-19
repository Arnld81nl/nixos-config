//! Multi-step progress widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{StepState, StepStatus};
use crate::ui::theme;
use crate::ui::widgets::Spinner;

pub struct ProgressSteps<'a> {
    steps: &'a [StepStatus],
    spinner_state: usize,
    title: Option<&'a str>,
}

impl<'a> ProgressSteps<'a> {
    pub fn new(steps: &'a [StepStatus], spinner_state: usize) -> Self {
        Self {
            steps,
            spinner_state,
            title: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl Widget for ProgressSteps<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = self
            .steps
            .iter()
            .enumerate()
            .map(|(_i, step)| {
                let (icon, style) = match step.status {
                    StepState::Pending => ("[ ]", theme::dim()),
                    StepState::Running => {
                        let spinner = Spinner::new(self.spinner_state);
                        // We'll handle this specially
                        return Line::from(vec![
                            Span::styled(format!(" [{}] ", spinner.char()), theme::info()),
                            Span::styled(&step.name, theme::text()),
                            Span::styled("...", theme::dim()),
                        ]);
                    }
                    StepState::Complete => ("[✓]", theme::success()),
                    StepState::Failed => ("[✗]", theme::error()),
                    StepState::Skipped => ("[-]", theme::dim()),
                };
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), style),
                    Span::styled(&step.name, theme::text()),
                ])
            })
            .collect();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border());

        if let Some(title) = self.title {
            block = block.title(Span::styled(title, theme::title()));
        }

        let paragraph = Paragraph::new(lines).block(block);
        paragraph.render(area, buf);
    }
}
