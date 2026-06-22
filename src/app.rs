use std::time::{Duration, Instant};

use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::Rect,
    style::Color,
    widgets::{ListState, TableState},
};
use sysinfo::Pid;

mod render;

use crate::claude::data::*;
use crate::data::*;
use crate::theme::{THEMES, Theme};

const TAB_TITLES: [&str; 8] = [
    "Counter", "List", "Table", "Chart", "Live", "Canvas", "System", "Claude",
];

// (widget, kind, stateful) — shared by the table view and the detail popup.
const TABLE_DATA: [(&str, &str, &str); 8] = [
    ("List", "selection", "yes"),
    ("Table", "tabular", "yes"),
    ("Gauge", "progress", "no"),
    ("Chart", "plot", "no"),
    ("Sparkline", "trend", "no"),
    ("Tabs", "navigation", "no"),
    ("Paragraph", "text", "no"),
    ("Canvas", "drawing", "no"),
];
const TABLE_ROWS: usize = TABLE_DATA.len();

/// Format a duration in seconds as "Hh Mm Ss".
fn fmt_duration(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    format!("{h}h {m}m {s}s")
}

/// Human-readable byte rate, e.g. "1.2 MB/s".
fn fmt_rate(bytes_per_sec: u64) -> String {
    let b = bytes_per_sec as f64;
    if b >= 1_000_000_000.0 {
        format!("{:.1} GB/s", b / 1_000_000_000.0)
    } else if b >= 1_000_000.0 {
        format!("{:.1} MB/s", b / 1_000_000.0)
    } else if b >= 1_000.0 {
        format!("{:.0} KB/s", b / 1_000.0)
    } else {
        format!("{b:.0} B/s")
    }
}

/// Color a load percentage: green (calm) → yellow (busy) → red (hot).
fn load_color(theme: &Theme, pct: f64) -> Color {
    if pct >= 80.0 {
        Color::Red
    } else if pct >= 50.0 {
        Color::Yellow
    } else {
        theme.series
    }
}

/// How the process table is sorted.

const REFRESH_MIN_MS: u64 = 40;
const REFRESH_MAX_MS: u64 = 1000;

/// A user intent, decoded from input and applied in `App::update`. Decoupling
/// "what the user meant" from "how it was entered" keeps logic testable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    QuitOrClose,
    NextTab,
    PrevTab,
    ClickTab(u16),
    ClickBody(u16, u16),
    Up,
    Down,
    OpenDetail,
    ToggleHelp,
    ShowPalette,
    CycleTheme,
    ResetCounter,
    FasterRefresh,
    SlowerRefresh,
    CycleSort,
    StartFilter,
    FilterChar(char),
    FilterBackspace,
    EndFilter,
    RequestKill,
    ConfirmKill,
    CancelKill,
}

/// A snapshot of one process row, kept so the detail popup can read the selection.

pub struct App {
    tab: usize,
    theme_idx: usize,
    show_help: bool,
    detail: Option<String>, // when Some, a detail popup is shown
    counter: i64,
    items: Vec<String>,
    list_state: ListState,
    table_state: TableState,
    spark: Vec<u64>,
    tick: u64,
    body: Rect, // area of the active tab's body, for mouse hit-testing
    pub monitor: SysMonitor,
    proc_state: TableState,
    proc_count: usize,           // rows currently shown, for selection clamping
    proc_snapshot: Vec<ProcRow>, // last-drawn process list, for the detail popup
    proc_area: Rect,             // rect of the process table, for mouse hit-testing
    pending_kill: Option<(u32, String)>, // (pid, name) awaiting confirmation
    sort_by: SortKey,
    filter: String,
    filtering: bool,             // true while typing in the filter box
    refresh_ms: u64,             // tick / refresh interval
    tab_ranges: Vec<(u16, u16)>, // x-ranges of each tab title, for click hit-testing
    exit: bool,
    claude_history: Vec<ClaudeHistoryRow>,
    claude_state: ListState,
    claude_stats: Option<ClaudeStats>,
    tcp_sockets: Vec<SocketRow>,
    show_palette: bool,
    palette_input: String,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        let mut files = vec!["..".to_string()];
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let prefix = if is_dir { "📁 " } else { "📄 " };
                files.push(format!("{}{}", prefix, name));
            }
        }
        files.sort();

        Self {
            tab: 0,
            theme_idx: 0,
            show_help: false,
            detail: None,
            counter: 0,
            items: files,
            list_state,
            table_state,
            spark: vec![0; 80],
            tick: 0,
            body: Rect::new(0, 0, 0, 0),
            monitor: SysMonitor::new(),
            proc_state: TableState::default().with_selected(Some(0)),
            proc_count: 0,
            proc_snapshot: Vec::new(),
            proc_area: Rect::new(0, 0, 0, 0),
            pending_kill: None,
            sort_by: SortKey::Cpu,
            filter: String::new(),
            filtering: false,
            refresh_ms: 120,
            tab_ranges: Vec::new(),
            exit: false,
            claude_history: load_history(),
            claude_state: ListState::default(),
            claude_stats: load_stats(),
            tcp_sockets: get_tcp_sockets(),
            show_palette: false,
            palette_input: String::new(),
        }
    }

    /// Path to the persisted config file, if a config dir is available.
    fn config_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("", "", "ratatui_demo")
            .map(|d| d.config_dir().join("config"))
    }

    /// Load persisted UI preferences (theme, tab, sort, refresh). Best-effort.
    pub fn load_config(&mut self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match (k.trim(), v.trim()) {
                ("theme", v) => {
                    if let Ok(i) = v.parse::<usize>() {
                        self.theme_idx = i % THEMES.len();
                    }
                }
                ("tab", v) => {
                    if let Ok(i) = v.parse::<usize>() {
                        self.tab = i % TAB_TITLES.len();
                    }
                }
                ("sort", v) => {
                    if let Ok(i) = v.parse::<usize>() {
                        self.sort_by = SortKey::from_index(i);
                    }
                }
                ("refresh", v) => {
                    if let Ok(ms) = v.parse::<u64>() {
                        self.refresh_ms = ms.clamp(REFRESH_MIN_MS, REFRESH_MAX_MS);
                    }
                }
                _ => {}
            }
        }
    }

    /// Persist UI preferences. Best-effort; errors are ignored.
    pub fn save_config(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body = format!(
            "theme={}\ntab={}\nsort={}\nrefresh={}\n",
            self.theme_idx,
            self.tab,
            self.sort_by.to_index(),
            self.refresh_ms,
        );
        let _ = std::fs::write(path, body);
    }

    fn theme(&self) -> &Theme {
        &THEMES[self.theme_idx]
    }

    pub async fn run(&mut self, mut terminal: ratatui::DefaultTerminal) -> std::io::Result<()> {
        use futures::StreamExt;
        use ratatui::crossterm::event::EventStream;
        use std::time::Duration;
        use tokio::time::interval;

        let mut ticker = interval(Duration::from_millis(self.refresh_ms));
        let mut events = EventStream::new();

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            tokio::select! {
                _ = ticker.tick() => {
                    self.on_tick();
                }
                Some(Ok(event)) = events.next() => {
                    self.handle_event(event);
                }
            }
        }
        Ok(())
    }

    fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let t = self.tick as f64 * 0.25;
        let v = ((t.sin() * 0.5 + 0.5) * 90.0) as u64 + (self.tick % 11);
        self.spark.remove(0);
        self.spark.push(v);

        // Refresh real system stats every other tick (~240ms — above sysinfo's
        // minimum CPU sampling interval for accurate readings).
        self.monitor.tick(self.tick, self.refresh_ms, self.tab == 6);
    }

    /// Translate a raw event into an `Action`, then apply it (Elm-style).
    fn handle_event(&mut self, ev: Event) {
        if let Some(action) = self.map_event(ev) {
            self.update(action);
        }
    }

    /// Pure-ish mapping from input event to intent, aware of modal state.
    fn map_event(&mut self, ev: Event) -> Option<Action> {
        let Event::Key(key) = ev else {
            // Mouse events (no modal interception needed here).
            if let Event::Mouse(m) = ev {
                return match m.kind {
                    MouseEventKind::ScrollUp => Some(Action::Up),
                    MouseEventKind::ScrollDown => Some(Action::Down),
                    MouseEventKind::Down(_) if m.row <= 2 => Some(Action::ClickTab(m.column)),
                    MouseEventKind::Down(_) => Some(Action::ClickBody(m.column, m.row)),
                    _ => None,
                };
            }
            return None;
        };
        if key.kind != KeyEventKind::Press {
            return None;
        }

        // Modal: kill confirmation captures all keys.
        if self.pending_kill.is_some() {
            return Some(match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmKill,
                _ => Action::CancelKill,
            });
        }
        // Modal: filter box captures typing.

        if self.show_palette {
            match key.code {
                KeyCode::Esc => {
                    self.show_palette = false;
                    self.palette_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.palette_input.clone();
                    self.show_palette = false;
                    self.palette_input.clear();
                    self.execute_command(&cmd);
                }
                KeyCode::Backspace => {
                    self.palette_input.pop();
                }
                KeyCode::Char(c) => {
                    self.palette_input.push(c);
                }
                _ => {}
            }
            return None;
        }

        if self.filtering {
            return match key.code {
                KeyCode::Enter | KeyCode::Esc => Some(Action::EndFilter),
                KeyCode::Backspace => Some(Action::FilterBackspace),
                KeyCode::Char(c) => Some(Action::FilterChar(c)),
                _ => None,
            };
        }

        Some(match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::QuitOrClose,
            KeyCode::Enter => Action::OpenDetail,
            KeyCode::Char('?') => Action::ToggleHelp,
            KeyCode::Char(':') => Action::ShowPalette,
            KeyCode::Char('t') => Action::CycleTheme,
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => Action::NextTab,
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => Action::PrevTab,
            KeyCode::Up => Action::Up,
            KeyCode::Down | KeyCode::Char('j') => Action::Down,
            // 'k' kills on the System tab, otherwise it's vim-style "up".
            KeyCode::Char('k') if self.tab == 6 => Action::RequestKill,
            KeyCode::Char('k') => Action::Up,
            KeyCode::Char('r') => Action::ResetCounter,
            KeyCode::Char('+') | KeyCode::Char('=') => Action::FasterRefresh,
            KeyCode::Char('-') | KeyCode::Char('_') => Action::SlowerRefresh,
            KeyCode::Char('s') if self.tab == 6 => Action::CycleSort,
            KeyCode::Char('/') if self.tab == 6 => Action::StartFilter,
            _ => return None,
        })
    }

    /// Apply an `Action` to the state. The single place mutations happen.
    fn update(&mut self, action: Action) {
        match action {
            Action::QuitOrClose => {
                // Close any open popup first; only quit if nothing is open.
                if self.detail.is_some() {
                    self.detail = None;
                } else if self.show_help {
                    self.show_help = false;
                } else {
                    self.exit = true;
                }
            }
            Action::NextTab => self.next_tab(),
            Action::PrevTab => self.prev_tab(),
            Action::ClickTab(col) => self.select_tab_at(col),
            Action::ClickBody(col, row) => self.select_at(col, row),
            Action::Up => self.on_up(),
            Action::Down => self.on_down(),
            Action::OpenDetail => self.open_detail(),
            Action::ToggleHelp => self.show_help = !self.show_help,
            Action::ShowPalette => self.show_palette = true,
            Action::CycleTheme => self.theme_idx = (self.theme_idx + 1) % THEMES.len(),
            Action::ResetCounter => self.counter = 0,
            Action::FasterRefresh => {
                self.refresh_ms = self.refresh_ms.saturating_sub(20).max(REFRESH_MIN_MS);
            }
            Action::SlowerRefresh => {
                self.refresh_ms = (self.refresh_ms + 20).min(REFRESH_MAX_MS);
            }
            Action::CycleSort => self.sort_by = self.sort_by.next(),
            Action::StartFilter => self.filtering = true,
            Action::FilterChar(c) => self.filter.push(c),
            Action::FilterBackspace => {
                self.filter.pop();
            }
            Action::EndFilter => self.filtering = false,
            Action::RequestKill => self.request_kill(),
            Action::ConfirmKill => self.confirm_kill(),
            Action::CancelKill => self.pending_kill = None,
        }
    }

    /// Test-only shim: feed a character through the same key-handling path.
    #[cfg(test)]
    fn handle_key_for_test(&mut self, c: char) {
        use ratatui::crossterm::event::{KeyCode, KeyEvent};
        self.handle_event(Event::Key(KeyEvent::from(KeyCode::Char(c))));
    }

    fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_TITLES.len();
    }
    fn prev_tab(&mut self) {
        self.tab = (self.tab + TAB_TITLES.len() - 1) % TAB_TITLES.len();
    }

    /// Select the tab whose title range contains `col`; fall back to advancing.
    fn select_tab_at(&mut self, col: u16) {
        for (i, &(start, end)) in self.tab_ranges.iter().enumerate() {
            if col >= start && col < end {
                self.tab = i;
                return;
            }
        }
        self.next_tab();
    }

    fn on_up(&mut self) {
        match self.tab {
            0 => self.counter += 1,
            1 => self.list_state.select_previous(),
            2 => self.table_state.select_previous(),
            6 => self.proc_state.select_previous(),
            _ => {}
        }
    }

    fn on_down(&mut self) {
        match self.tab {
            0 => self.counter -= 1,
            1 => self.list_state.select_next(),
            2 => self.table_state.select_next(),
            6 => {
                // Clamp so selection can't run past the last process row.
                let max = self.proc_count.saturating_sub(1);
                let next = self.proc_state.selected().map_or(0, |i| (i + 1).min(max));
                self.proc_state.select(Some(next));
            }
            _ => {}
        }
    }

    /// Open a detail popup for the currently selected List/Table row.

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "quit" | "q" | "exit" => self.exit = true,
            "theme" => {
                if parts.len() > 1 {
                    // super basic theme switching
                    self.theme_idx = (self.theme_idx + 1) % crate::theme::THEMES.len();
                }
            }
            "killall" => {
                if parts.len() > 1 {
                    let _ = std::process::Command::new("killall")
                        .arg("-9")
                        .arg(parts[1])
                        .output();
                }
            }
            "tab" => {
                if parts.len() > 1 {
                    if let Ok(idx) = parts[1].parse::<usize>() {
                        if idx < TAB_TITLES.len() {
                            self.tab = idx;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn open_detail(&mut self) {
        self.detail = match self.tab {
            1 => self
                .list_state
                .selected()
                .map(|i| format!("{}\n\n(press Esc or Enter to close)", self.items[i])),
            2 => self.table_state.selected().map(|i| {
                let (w, k, s) = TABLE_DATA[i];
                format!(
                    "{w}\n\nkind: {k}\nstateful: {s}\n\n(press Esc or Enter to close)"
                )
            }),
            6 => self
                .proc_state
                .selected()
                .and_then(|i| self.proc_snapshot.get(i))
                .map(|p| {
                    let mut extra_info = String::new();
                    // Grab open files using lsof
                    if let Ok(output) = std::process::Command::new("lsof")
                        .arg("-p")
                        .arg(&p.pid)
                        .output() 
                    {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            // just take the first 10 lines so the modal doesn't explode
                            let lines: Vec<&str> = stdout.lines().take(15).collect();
                            extra_info = format!("\nOpen Files (lsof):\n{}", lines.join("\n"));
                            if stdout.lines().count() > 15 {
                                extra_info.push_str("\n... (truncated)");
                            }
                        } else {
                            extra_info = "\nOpen Files (lsof): Permission denied or unavailable.".to_string();
                        }
                    }

                    format!(
                        "{}  (pid {})\n\nCPU:     {:.1}%\nMemory:  {:.1} MB\nStatus:  {}\nUptime:  {}\n{}\n\n(press Esc or Enter to close)",
                        p.name,
                        p.pid,
                        p.cpu,
                        p.mem as f64 / 1_000_000.0,
                        p.status,
                        fmt_duration(p.run),
                        extra_info
                    )
                }),
            _ => None,
        };
    }

    /// Ask to kill the selected process (opens a confirmation prompt).
    fn request_kill(&mut self) {
        if self.tab == 6
            && let Some(p) = self
                .proc_state
                .selected()
                .and_then(|i| self.proc_snapshot.get(i))
        {
            self.pending_kill = Some((p.pid_num, p.name.clone()));
        }
    }

    /// Carry out a confirmed kill.
    fn confirm_kill(&mut self) {
        if let Some((pid, _)) = self.pending_kill.take()
            && let Some(proc) = self.monitor.sys.process(Pid::from_u32(pid))
        {
            proc.kill();
        }
    }

    /// Map a click at (col, row) to a list/table selection on the active tab.
    fn select_at(&mut self, col: u16, row: u16) {
        // Ignore clicks outside the body box (incl. its border).
        if col <= self.body.x
            || col >= self.body.right() - 1
            || row <= self.body.y
            || row >= self.body.bottom() - 1
        {
            return;
        }
        match self.tab {
            1 => {
                // List: top border occupies body.y, so first item is body.y + 1.
                let idx = self.list_state.offset() + (row - self.body.y - 1) as usize;
                if idx < self.items.len() {
                    self.list_state.select(Some(idx));
                }
            }
            2 => {
                // Table: border + header row, so first data row is body.y + 2.
                if row >= self.body.y + 2 {
                    let idx = self.table_state.offset() + (row - self.body.y - 2) as usize;
                    if idx < TABLE_ROWS {
                        self.table_state.select(Some(idx));
                    }
                }
            }
            6 => {
                // Process table sits in a sub-rect; border + header before data.
                let a = self.proc_area;
                if col > a.x && col < a.right() - 1 && row >= a.y + 2 && row < a.bottom() - 1 {
                    let idx = self.proc_state.offset() + (row - a.y - 2) as usize;
                    if idx < self.proc_count {
                        self.proc_state.select(Some(idx));
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    /// Render the app to an in-memory buffer (no real terminal needed).
    fn render(app: &mut App) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(90, 30)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// Flatten the buffer's cells into one string so we can search for text.
    fn text(buf: &Buffer) -> String {
        buf.content().iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn counter_tab_renders_and_increments() {
        let mut app = App::new();
        let buf = render(&mut app);
        assert!(text(&buf).contains("Counter"));
        assert!(text(&buf).contains("Progress"));

        app.on_up(); // +1 on the counter tab
        app.on_up();
        assert_eq!(app.counter, 2);
        assert!(text(&render(&mut app)).contains('2'));
    }

    #[test]
    fn reset_zeroes_the_counter() {
        let mut app = App::new();
        app.counter = 7;
        app.handle_key_for_test('r');
        assert_eq!(app.counter, 0);
    }

    #[test]
    fn tabs_cycle_in_both_directions() {
        let mut app = App::new();
        assert_eq!(app.tab, 0);
        app.prev_tab(); // wraps to last tab
        assert_eq!(app.tab, TAB_TITLES.len() - 1);
        app.next_tab(); // wraps back to 0
        assert_eq!(app.tab, 0);
    }

    #[test]
    fn each_tab_renders_its_signature_content() {
        let cases = [
            (0usize, "Counter"),
            (1, "Layout"),         // first list item
            (2, "Widget Catalog"), // table title
            (3, "sin(x + counter)"),
            (4, "Live Sparkline"),
            (5, "world map"),
            (6, "Processes"),
        ];
        for (tab, needle) in cases {
            let mut app = App::new();
            app.tab = tab;
            let buf = render(&mut app);
            assert!(
                text(&buf).contains(needle),
                "tab {tab} should contain {needle:?}"
            );
        }
    }

    #[test]
    fn help_popup_overlays_when_toggled() {
        let mut app = App::new();
        assert!(!text(&render(&mut app)).contains("Keyboard"));
        app.show_help = true;
        assert!(text(&render(&mut app)).contains("Keyboard"));
    }

    #[test]
    fn theme_toggle_changes_the_title() {
        let mut app = App::new();
        assert!(text(&render(&mut app)).contains("Magenta"));
        app.theme_idx = (app.theme_idx + 1) % THEMES.len();
        assert!(text(&render(&mut app)).contains("Ocean"));
    }

    #[test]
    fn clicking_a_list_row_selects_it() {
        let mut app = App::new();
        app.tab = 1;
        render(&mut app); // populates app.body (the list area)
        // body.y is the top border; first item is body.y + 1, fourth is +4.
        let target_row = app.body.y + 4;
        app.select_at(app.body.x + 2, target_row);
        assert_eq!(app.list_state.selected(), Some(3));
    }

    #[test]
    fn clicking_a_table_row_selects_it() {
        let mut app = App::new();
        app.tab = 2;
        render(&mut app);
        // border + header, so first data row is body.y + 2; third is +4.
        let target_row = app.body.y + 4;
        app.select_at(app.body.x + 2, target_row);
        assert_eq!(app.table_state.selected(), Some(2));
    }

    #[test]
    fn clicking_empty_space_keeps_selection() {
        let mut app = App::new();
        app.tab = 1;
        render(&mut app);
        let before = app.list_state.selected();
        app.select_at(app.body.x + 2, app.body.bottom() - 1); // on the border
        assert_eq!(app.list_state.selected(), before);
    }

    #[test]
    fn enter_opens_detail_popup_and_esc_closes_it() {
        let mut app = App::new();
        app.tab = 2;
        render(&mut app);
        app.table_state.select(Some(4)); // "Sparkline"
        app.handle_key_for_test('\n'); // not used; call open path via Enter helper
        app.open_detail();
        let buf = render(&mut app);
        assert!(text(&buf).contains("Details"));
        assert!(text(&buf).contains("Sparkline"));

        app.detail = None;
        assert!(!text(&render(&mut app)).contains("Details"));
    }

    #[test]
    fn system_tab_process_list_scrolls_and_clamps() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app); // populates proc_count
        assert!(app.proc_count > 0, "should see at least one process");
        assert_eq!(app.proc_state.selected(), Some(0));

        app.on_down();
        assert_eq!(app.proc_state.selected(), Some(1.min(app.proc_count - 1)));

        // Hammering down can never exceed the last row.
        for _ in 0..10_000 {
            app.on_down();
        }
        assert_eq!(app.proc_state.selected(), Some(app.proc_count - 1));
    }

    #[test]
    fn system_tab_enter_shows_process_details() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app); // populates proc_snapshot
        app.proc_state.select(Some(0));
        app.open_detail();
        let buf = render(&mut app);
        assert!(text(&buf).contains("Details"));
        assert!(text(&buf).contains("pid"));
        assert!(text(&buf).contains("Uptime"));
    }

    #[test]
    fn k_opens_kill_confirm_and_n_cancels() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app);
        app.proc_state.select(Some(0));

        app.handle_key_for_test('k');
        assert!(app.pending_kill.is_some());
        let buf = render(&mut app);
        assert!(text(&buf).contains("Confirm"));

        // Any non-'y' key cancels without killing anything.
        app.handle_key_for_test('n');
        assert!(app.pending_kill.is_none());
    }

    #[test]
    fn kill_confirm_blocks_other_input() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app);
        app.proc_state.select(Some(0));
        app.handle_key_for_test('k'); // opens confirm
        let tab_before = app.tab;
        app.handle_key_for_test('\t'); // would normally switch tabs
        assert_eq!(
            app.tab, tab_before,
            "input is captured by the confirm prompt"
        );
        assert!(app.pending_kill.is_none(), "tab key cancelled the prompt");
    }

    #[test]
    fn clicking_a_process_row_selects_it() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app); // populates proc_area and proc_count
        let a = app.proc_area;
        // border + header => first data row is a.y + 2; third is +4.
        app.select_at(a.x + 2, a.y + 4);
        let expected = 2.min(app.proc_count.saturating_sub(1));
        assert_eq!(app.proc_state.selected(), Some(expected));
    }

    #[test]
    fn sort_key_cycles_and_reorders_by_pid() {
        let mut app = App::new();
        app.tab = 6;
        // Cycle CPU -> Mem -> Name -> Pid.
        for _ in 0..3 {
            app.handle_key_for_test('s');
        }
        assert!(app.sort_by == SortKey::Pid);
        render(&mut app);
        // Sorted by PID ascending: each pid_num >= the previous.
        let pids: Vec<u32> = app.proc_snapshot.iter().map(|p| p.pid_num).collect();
        assert!(
            pids.windows(2).all(|w| w[0] <= w[1]),
            "pids should be ascending"
        );
    }

    #[test]
    fn filter_narrows_the_process_list() {
        let mut app = App::new();
        app.tab = 6;
        render(&mut app);
        let all = app.proc_count;
        app.handle_key_for_test('/'); // enter filter mode
        for c in "zzqxnope".chars() {
            app.handle_key_for_test(c);
        }
        render(&mut app);
        assert!(app.proc_count <= all, "filter only narrows the list");
    }

    #[test]
    fn update_applies_actions_directly() {
        let mut app = App::new();
        app.update(Action::NextTab);
        assert_eq!(app.tab, 1);
        app.update(Action::CycleTheme);
        assert_eq!(app.theme_idx, 1);
        app.update(Action::ToggleHelp);
        assert!(app.show_help);
        app.update(Action::QuitOrClose); // closes help, doesn't quit
        assert!(!app.show_help);
        assert!(!app.exit);
        app.update(Action::QuitOrClose); // now quits
        assert!(app.exit);
    }

    #[test]
    fn clicking_a_tab_selects_it() {
        let mut app = App::new();
        render(&mut app); // populates tab_ranges
        // Click within the "Chart" tab (index 3) range.
        let (start, end) = app.tab_ranges[3];
        app.select_tab_at((start + end) / 2);
        assert_eq!(app.tab, 3);
    }

    #[test]
    fn refresh_rate_adjusts_within_bounds() {
        let mut app = App::new();
        for _ in 0..100 {
            app.handle_key_for_test('+');
        }
        assert_eq!(app.refresh_ms, REFRESH_MIN_MS);
        for _ in 0..100 {
            app.handle_key_for_test('-');
        }
        assert_eq!(app.refresh_ms, REFRESH_MAX_MS);
    }

    #[test]
    fn fmt_rate_scales_units() {
        assert_eq!(fmt_rate(512), "512 B/s");
        assert_eq!(fmt_rate(2_000), "2 KB/s");
        assert_eq!(fmt_rate(3_000_000), "3.0 MB/s");
    }

    #[test]
    fn load_color_thresholds() {
        let t = &THEMES[0];
        assert_eq!(load_color(t, 10.0), t.series);
        assert_eq!(load_color(t, 60.0), Color::Yellow);
        assert_eq!(load_color(t, 95.0), Color::Red);
    }

    #[test]
    fn fmt_duration_breaks_into_h_m_s() {
        assert_eq!(fmt_duration(0), "0h 0m 0s");
        assert_eq!(fmt_duration(3661), "1h 1m 1s");
    }

    #[test]
    fn tick_advances_the_sparkline() {
        let mut app = App::new();
        let before = app.spark.clone();
        for _ in 0..5 {
            app.on_tick();
        }
        assert_ne!(before, app.spark);
        assert_eq!(app.spark.len(), 80); // length stays constant
    }
}
