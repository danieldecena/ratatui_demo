mod app;
mod popup;
mod theme;

use std::io::stdout;

use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

use app::App;

fn main() -> std::io::Result<()> {
    let terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    let result = App::new().run(terminal);
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}
