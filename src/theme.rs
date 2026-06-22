use ratatui::style::{Color, Modifier, Style};

/// A small color palette the whole UI reads from, so theming lives in one place.
#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub value: Color,
    pub series: Color,
    pub axis: Color,
    pub hint: Color,
    pub quit: Color,
    pub highlight_bg: Color,
}

pub const THEMES: [Theme; 3] = [
    Theme {
        name: "Magenta",
        accent: Color::Magenta,
        value: Color::Cyan,
        series: Color::Green,
        axis: Color::Gray,
        hint: Color::Yellow,
        quit: Color::Red,
        highlight_bg: Color::Cyan,
    },
    Theme {
        name: "Ocean",
        accent: Color::Cyan,
        value: Color::LightBlue,
        series: Color::LightCyan,
        axis: Color::DarkGray,
        hint: Color::LightBlue,
        quit: Color::LightRed,
        highlight_bg: Color::Blue,
    },
    Theme {
        name: "Amber",
        accent: Color::Yellow,
        value: Color::LightYellow,
        series: Color::LightGreen,
        axis: Color::DarkGray,
        hint: Color::LightYellow,
        quit: Color::Red,
        highlight_bg: Color::Yellow,
    },
];

impl Theme {
    /// Style used for selected rows in lists/tables.
    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(self.highlight_bg)
            .add_modifier(Modifier::BOLD)
    }
}
