use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
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

/// A modal help dialog drawn over the rest of the UI.
pub struct HelpModal<'a> {
    theme: &'a Theme,
}

impl<'a> HelpModal<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl<'a> Widget for HelpModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = centered(area, 54, 16);
        Clear.render(popup_area, buf); // clear whatever is underneath

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

        let popup = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(self.theme.accent)),
        );
        popup.render(popup_area, buf);
    }
}

/// A modal yes/no confirmation prompt.
pub struct ConfirmModal<'a> {
    theme: &'a Theme,
    prompt: &'a str,
}

impl<'a> ConfirmModal<'a> {
    pub fn new(theme: &'a Theme, prompt: &'a str) -> Self {
        Self { theme, prompt }
    }
}

impl<'a> Widget for ConfirmModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = centered(area, 50, 7);
        Clear.render(popup_area, buf);

        let lines = vec![
            Line::from(self.prompt).style(Style::default().add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  y",
                    Style::default()
                        .fg(self.theme.quit)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" = confirm     "),
                Span::styled("n / Esc", Style::default().fg(self.theme.hint)),
                Span::raw(" = cancel  "),
            ]),
        ];
        let popup = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(self.theme.quit)),
        );
        popup.render(popup_area, buf);
    }
}

/// A modal detail box showing arbitrary text about a selected item.
pub struct DetailModal<'a> {
    theme: &'a Theme,
    body: &'a str,
}

impl<'a> DetailModal<'a> {
    pub fn new(theme: &'a Theme, body: &'a str) -> Self {
        Self { theme, body }
    }
}

impl<'a> Widget for DetailModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = centered(area, 46, 11);
        Clear.render(popup_area, buf);

        let popup = Paragraph::new(self.body).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(self.theme.accent)),
        );
        popup.render(popup_area, buf);
    }
}
