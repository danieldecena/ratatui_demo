mod app;
mod claude;
mod data;
mod popup;
mod theme;

use std::io::stdout;

use color_eyre::Result;
use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    init_panic_hook();

    let mut app = App::new();
    app.load_config();

    let terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    let result = app.run(terminal).await;
    restore_terminal();
    app.save_config();
    result.map_err(Into::into)
}

/// Restore the terminal even if the app panics, so a crash never leaves the
/// terminal in raw mode with the alternate screen and mouse capture still on.
fn init_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original(info);
    }));
}

/// Leave mouse-capture/alternate-screen/raw-mode. Safe to call more than once.
fn restore_terminal() {
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
}
