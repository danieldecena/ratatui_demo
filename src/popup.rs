use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;

/// Center a rect of the given size inside `area`.
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [h] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [v] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(h);
    v
}

/// Draw a modal help dialog over the rest of the UI.
pub fn render_help(frame: &mut Frame, theme: &Theme) {
    let area = centered(frame.area(), 54, 16);
    frame.render_widget(Clear, area); // clear whatever is underneath

    let lines = vec![
        Line::from("Keyboard & mouse").style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("  Tab / ← →      switch tabs"),
        Line::from("  ↑ ↓ / scroll   adjust active tab"),
        Line::from("  Enter          details (List/Table/System)"),
        Line::from("  k              kill process (System tab)"),
        Line::from("  r              reset the counter"),
        Line::from("  t              cycle color theme"),
        Line::from("  ?              toggle this help"),
        Line::from("  q / Esc        quit"),
        Line::from(""),
        Line::from("  click rows to select; click the top bar"),
        Line::from("  to advance tabs"),
    ];

    let popup = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(popup, area);
}

/// Draw a modal yes/no confirmation prompt.
pub fn render_confirm(frame: &mut Frame, theme: &Theme, prompt: &str) {
    let area = centered(frame.area(), 50, 7);
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(prompt).style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y", Style::default().fg(theme.quit).add_modifier(Modifier::BOLD)),
            Span::raw(" = confirm     "),
            Span::styled("n / Esc", Style::default().fg(theme.hint)),
            Span::raw(" = cancel  "),
        ]),
    ];
    let popup = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.quit)),
        );
    frame.render_widget(popup, area);
}

/// Draw a modal detail box showing arbitrary text about a selected item.
pub fn render_detail(frame: &mut Frame, theme: &Theme, body: &str) {
    let area = centered(frame.area(), 46, 11);
    frame.render_widget(Clear, area);

    let popup = Paragraph::new(body)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        );
    frame.render_widget(popup, area);
}
