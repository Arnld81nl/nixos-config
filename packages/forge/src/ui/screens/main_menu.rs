//! Main menu screen

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, MAIN_MENU_ITEMS};
use crate::ui::layout::centered_rect;
use crate::ui::theme;
use crate::ui::widgets::{MenuList, Spinner};

/// ASCII logo for Cybex (all lines padded to same width for proper centering)
const LOGO: &[&str] = &[
    r#"                     $$a.                                       "#,
    r#"                      `$$$                                      "#,
    r#" .a&$$$&a, a$$a..a$$a. `$$bd$$$&a,    .a&$""$&a     .a$$a..a$$a."#,
    r#"d#7^' `^^' `Q$$bd$$$^   1$#7^' `^Q$, d#7@Qbd@'' d$   Q$$$$$$$$P "#,
    r#"Y$b,. .,,.    Q$$$$'   .$$$b.. .,d7' Q$&a,..,a&$P'  .d$$$PQ$$$b "#,
    r#" `@Q$$$P@'    d$$$'    `^@Q$$$$$@"'   `^@Q$$$P@^'   @Q$P@  @Q$P@"#,
    r#"             @$$P                                               "#,
];

pub fn draw(frame: &mut Frame, selected: usize, app: &App) {
    let area = frame.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Header with logo
            Constraint::Min(10),    // Menu
            Constraint::Length(3),  // Footer
        ])
        .split(centered_rect(80, 90, area));

    // Header with logo
    draw_header(frame, chunks[0]);

    // Menu
    let menu = MenuList::new(MAIN_MENU_ITEMS.to_vec(), selected);
    frame.render_widget(menu, chunks[1]);

    // Footer with key hints
    draw_footer(frame, chunks[2], app);

    // Exit confirmation popup
    if app.show_exit_confirm {
        draw_exit_confirm(frame, area);
    }
}

fn draw_exit_confirm(frame: &mut Frame, area: Rect) {
    // Center the popup
    let popup_width = 40;
    let popup_height = 7;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Draw popup content
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("Are you sure you want to exit?", theme::text())),
        Line::from(""),
        Line::from(vec![
            Span::styled("[", theme::dim()),
            Span::styled("Enter/Y", theme::key_hint()),
            Span::styled("] Yes  [", theme::dim()),
            Span::styled("Esc/N", theme::key_hint()),
            Span::styled("] No", theme::dim()),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::warning())
            .title(Span::styled(" Exit ", theme::warning())),
    );

    frame.render_widget(content, popup_area);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    // Build logo lines
    let mut lines: Vec<Line> = LOGO
        .iter()
        .map(|line| Line::from(Span::styled(*line, theme::title())))
        .collect();

    // Add spacing and title
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "FORGE - NixOS Toolkit",
        theme::title(),
    )));
    lines.push(Line::from(Span::styled(
        "Copyright Cybex B.V.",
        theme::dim(),
    )));

    let header = Paragraph::new(lines).alignment(Alignment::Center);

    frame.render_widget(header, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled("[", theme::dim()),
        Span::styled("↑↓", theme::key_hint()),
        Span::styled("] Navigate  [", theme::dim()),
        Span::styled("Enter", theme::key_hint()),
        Span::styled("] Select  [", theme::dim()),
        Span::styled("q", theme::key_hint()),
        Span::styled("] Quit", theme::dim()),
    ];

    // Show checking indicator when startup check is running
    if app.startup_check_running {
        let spinner_char = Spinner::new(app.spinner_state).char();
        spans.push(Span::styled("     ", theme::dim()));
        spans.push(Span::styled(format!("{} ", spinner_char), theme::dim()));
        spans.push(Span::styled("Checking for updates...", theme::dim()));
    }

    let hints = Line::from(spans);
    let footer = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(footer, area);
}
