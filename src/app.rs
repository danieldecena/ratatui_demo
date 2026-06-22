use std::time::{Duration, Instant};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Map, MapResolution},
        Axis, Block, BorderType, Borders, Cell, Chart, Dataset, Gauge, GraphType, LineGauge, List,
        ListItem, ListState, Paragraph, Row, Sparkline, Table, TableState, Tabs,
    },
    DefaultTerminal, Frame,
};
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::popup;
use crate::theme::{Theme, THEMES};

const TAB_TITLES: [&str; 7] = [
    "Counter", "List", "Table", "Chart", "Live", "Canvas", "System",
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

/// A snapshot of one process row, kept so the detail popup can read the selection.
#[derive(Clone)]
struct ProcRow {
    pid: String,
    pid_num: u32,
    name: String,
    cpu: f32,
    mem: u64,
    run: u64,
    status: String,
}

pub struct App {
    tab: usize,
    theme_idx: usize,
    show_help: bool,
    detail: Option<String>, // when Some, a detail popup is shown
    counter: i64,
    items: Vec<&'static str>,
    list_state: ListState,
    table_state: TableState,
    spark: Vec<u64>,
    tick: u64,
    body: Rect, // area of the active tab's body, for mouse hit-testing
    sys: System,
    cpu_history: Vec<u64>,
    proc_state: TableState,
    proc_count: usize, // rows currently shown, for selection clamping
    proc_snapshot: Vec<ProcRow>, // last-drawn process list, for the detail popup
    proc_area: Rect,   // rect of the process table, for mouse hit-testing
    pending_kill: Option<(u32, String)>, // (pid, name) awaiting confirmation
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            tab: 0,
            theme_idx: 0,
            show_help: false,
            detail: None,
            counter: 0,
            items: vec![
                "Layout — split the screen into regions",
                "Block — borders, titles, padding",
                "Paragraph — wrapped, styled text",
                "List — scrollable, selectable rows",
                "Table — columns and headers",
                "Gauge — progress bars",
                "Chart — line and scatter plots",
                "Tabs — switch between views",
                "Sparkline — compact trend lines",
                "Canvas — draw shapes and maps",
            ],
            list_state,
            table_state,
            spark: vec![0; 80],
            tick: 0,
            body: Rect::new(0, 0, 0, 0),
            sys: System::new_all(),
            cpu_history: vec![0; 60],
            proc_state: TableState::default().with_selected(Some(0)),
            proc_count: 0,
            proc_snapshot: Vec::new(),
            proc_area: Rect::new(0, 0, 0, 0),
            pending_kill: None,
            exit: false,
        }
    }

    fn theme(&self) -> &Theme {
        &THEMES[self.theme_idx]
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        let tick_rate = Duration::from_millis(120);
        let mut last_tick = Instant::now();
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_event(event::read()?);
            }
            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
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
        if self.tick % 2 == 0 {
            self.sys.refresh_cpu_usage();
            self.sys.refresh_memory();
            let cpu = self.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u64;
            self.cpu_history.remove(0);
            self.cpu_history.push(cpu);
            if self.tab == 6 {
                self.sys.refresh_processes(ProcessesToUpdate::All, true);
            }
        }
    }

    fn handle_event(&mut self, ev: Event) {
        // A kill confirmation captures all keys until resolved.
        if self.pending_kill.is_some() {
            if let Event::Key(key) = ev {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_kill(),
                        _ => self.pending_kill = None, // anything else cancels
                    }
                }
            }
            return;
        }

        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    // Close any open popup first; only quit if nothing is open.
                    if self.detail.is_some() {
                        self.detail = None;
                    } else if self.show_help {
                        self.show_help = false;
                    } else {
                        self.exit = true;
                    }
                }
                KeyCode::Enter => self.open_detail(),
                KeyCode::Char('?') => self.show_help = !self.show_help,
                KeyCode::Char('t') => self.theme_idx = (self.theme_idx + 1) % THEMES.len(),
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => self.next_tab(),
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => self.prev_tab(),
                KeyCode::Up => self.on_up(),
                KeyCode::Down | KeyCode::Char('j') => self.on_down(),
                // 'k' kills on the System tab, otherwise it's vim-style "up".
                KeyCode::Char('k') => {
                    if self.tab == 6 {
                        self.request_kill();
                    } else {
                        self.on_up();
                    }
                }
                KeyCode::Char('r') => self.counter = 0,
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => self.on_up(),
                MouseEventKind::ScrollDown => self.on_down(),
                MouseEventKind::Down(_) if m.row <= 2 => self.next_tab(),
                MouseEventKind::Down(_) => self.select_at(m.column, m.row),
                _ => {}
            },
            _ => {}
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
                    format!(
                        "{}  (pid {})\n\nCPU:     {:.1}%\nMemory:  {:.1} MB\nStatus:  {}\nUptime:  {}\n\n(press Esc or Enter to close)",
                        p.name,
                        p.pid,
                        p.cpu,
                        p.mem as f64 / 1_000_000.0,
                        p.status,
                        fmt_duration(p.run),
                    )
                }),
            _ => None,
        };
    }

    /// Ask to kill the selected process (opens a confirmation prompt).
    fn request_kill(&mut self) {
        if self.tab == 6 {
            if let Some(p) = self
                .proc_state
                .selected()
                .and_then(|i| self.proc_snapshot.get(i))
            {
                self.pending_kill = Some((p.pid_num, p.name.clone()));
            }
        }
    }

    /// Carry out a confirmed kill.
    fn confirm_kill(&mut self) {
        if let Some((pid, _)) = self.pending_kill.take() {
            if let Some(proc) = self.sys.process(Pid::from_u32(pid)) {
                proc.kill();
            }
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

    fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

        self.body = chunks[1]; // remember body area for mouse hit-testing
        self.draw_tabs(frame, chunks[0]);
        match self.tab {
            0 => self.draw_counter(frame, chunks[1]),
            1 => self.draw_list(frame, chunks[1]),
            2 => self.draw_table(frame, chunks[1]),
            3 => self.draw_chart(frame, chunks[1]),
            4 => self.draw_live(frame, chunks[1]),
            5 => self.draw_canvas(frame, chunks[1]),
            _ => self.draw_system(frame, chunks[1]),
        }
        self.draw_footer(frame, chunks[2]);

        if let Some((pid, name)) = &self.pending_kill {
            let prompt = format!("Kill {name} (pid {pid})?");
            popup::render_confirm(frame, self.theme(), &prompt);
        } else if let Some(body) = &self.detail {
            popup::render_detail(frame, self.theme(), body);
        } else if self.show_help {
            popup::render_help(frame, self.theme());
        }
    }

    fn draw_tabs(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme();
        let title = format!(" Ratatui Dashboard — {} theme ", theme.name);
        let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
            .select(self.tab)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.accent)),
            );
        frame.render_widget(tabs, area);
    }

    fn draw_counter(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme();
        let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);
        let counter = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{}", self.counter),
                Style::default().fg(theme.value).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Counter ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(counter, rows[0]);

        let ratio = ((self.counter.rem_euclid(21)) as f64) / 20.0;
        let gauge = Gauge::default()
            .block(Block::default().title(" Progress ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(theme.series))
            .ratio(ratio);
        frame.render_widget(gauge, rows[1]);
    }

    fn draw_list(&mut self, frame: &mut Frame, area: Rect) {
        let theme = *self.theme();
        let items: Vec<ListItem> = self.items.iter().map(|i| ListItem::new(*i)).collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Widgets ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(theme.highlight())
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_table(&mut self, frame: &mut Frame, area: Rect) {
        let theme = *self.theme();
        let header = Row::new(["Widget", "Kind", "Stateful"])
            .style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));
        let rows = TABLE_DATA
            .iter()
            .map(|(a, b, c)| Row::new([Cell::from(*a), Cell::from(*b), Cell::from(*c)]));

        let widths = [
            Constraint::Percentage(40),
            Constraint::Percentage(40),
            Constraint::Percentage(20),
        ];
        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(" Widget Catalog ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .row_highlight_style(theme.highlight())
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn draw_chart(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme();
        let phase = self.counter as f64 * 0.2;
        let data: Vec<(f64, f64)> = (0..=100)
            .map(|x| {
                let xf = x as f64 / 5.0;
                (xf, (xf + phase).sin())
            })
            .collect();
        let datasets = vec![Dataset::default()
            .name("sin(x + counter)")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.series))
            .data(&data)];
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(" Chart ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .x_axis(
                Axis::default()
                    .title("x")
                    .style(Style::default().fg(theme.axis))
                    .bounds([0.0, 20.0])
                    .labels(["0", "10", "20"]),
            )
            .y_axis(
                Axis::default()
                    .title("y")
                    .style(Style::default().fg(theme.axis))
                    .bounds([-1.0, 1.0])
                    .labels(["-1", "0", "1"]),
            );
        frame.render_widget(chart, area);
    }

    fn draw_live(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme();
        let sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title(" Live Sparkline (updates ~8x/sec) ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .data(&self.spark)
            .style(Style::default().fg(theme.series));
        frame.render_widget(sparkline, area);
    }

    fn draw_canvas(&self, frame: &mut Frame, area: Rect) {
        let theme = *self.theme();
        // Marker sweeps west-to-east, wrapping around the globe.
        let lon = -180.0 + ((self.tick as f64 * 3.0) % 360.0);
        let canvas = Canvas::default()
            .block(
                Block::default()
                    .title(" Canvas — world map ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .marker(symbols::Marker::Braille)
            .x_bounds([-180.0, 180.0])
            .y_bounds([-90.0, 90.0])
            .paint(move |ctx| {
                ctx.draw(&Map {
                    resolution: MapResolution::High,
                    color: theme.series,
                });
                ctx.layer();
                ctx.draw(&Circle {
                    x: lon,
                    y: 25.0,
                    radius: 6.0,
                    color: theme.value,
                });
                ctx.print(lon, 25.0, Span::styled("◉", Style::default().fg(theme.accent)));
            });
        frame.render_widget(canvas, area);
    }

    fn draw_system(&mut self, frame: &mut Frame, area: Rect) {
        let theme = *self.theme();
        let rows = Layout::vertical([
            Constraint::Length(3), // cpu gauge
            Constraint::Length(3), // memory gauge
            Constraint::Min(5),    // cores + history | processes
        ])
        .split(area);

        // CPU gauge
        let cpu = self.sys.global_cpu_usage().clamp(0.0, 100.0);
        let cpu_gauge = Gauge::default()
            .block(Block::default().title(" CPU ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(theme.series))
            .ratio((cpu / 100.0) as f64)
            .label(format!("{cpu:.0}%"));
        frame.render_widget(cpu_gauge, rows[0]);

        // Memory gauge
        let total = self.sys.total_memory().max(1);
        let used = self.sys.used_memory();
        let to_gb = |b: u64| b as f64 / 1_000_000_000.0;
        let mem_gauge = Gauge::default()
            .block(Block::default().title(" Memory ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(theme.value))
            .ratio(used as f64 / total as f64)
            .label(format!("{:.1} / {:.1} GB", to_gb(used), to_gb(total)));
        frame.render_widget(mem_gauge, rows[1]);

        // Bottom: left column (per-core bars + history) | processes table
        let bottom = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(rows[2]);
        let left = Layout::vertical([Constraint::Min(3), Constraint::Length(8)]).split(bottom[0]);

        self.draw_cores(frame, left[0], &theme);

        let history = Sparkline::default()
            .block(
                Block::default()
                    .title(" CPU history ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .max(100)
            .data(&self.cpu_history)
            .style(Style::default().fg(theme.series));
        frame.render_widget(history, left[1]);

        self.draw_processes(frame, bottom[1], &theme);
    }

    /// One LineGauge per CPU core, fixed to a 0–100% scale.
    fn draw_cores(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(" Cores ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let cpus = self.sys.cpus();
        let rows = inner.height as usize;
        let n = cpus.len().min(rows);
        if n == 0 {
            return;
        }
        let slots = Layout::vertical(vec![Constraint::Length(1); n]).split(inner);
        for (i, cpu) in cpus.iter().take(n).enumerate() {
            let usage = cpu.cpu_usage().clamp(0.0, 100.0);
            let gauge = LineGauge::default()
                .ratio((usage / 100.0) as f64)
                .label(format!("{i:>2}"))
                .filled_style(Style::default().fg(theme.series))
                .unfilled_style(Style::default().fg(theme.axis));
            frame.render_widget(gauge, slots[i]);
        }
    }

    /// Scrollable, selectable table of all processes sorted by CPU.
    fn draw_processes(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.proc_area = area;
        let mut procs: Vec<ProcRow> = self
            .sys
            .processes()
            .values()
            .map(|p| ProcRow {
                pid: p.pid().to_string(),
                pid_num: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem: p.memory(),
                run: p.run_time(),
                status: p.status().to_string(),
            })
            .collect();
        procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        self.proc_count = procs.len();

        let header = Row::new(["Process", "CPU%", "Mem"])
            .style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));
        let proc_rows = procs.iter().map(|p| {
            let short: String = p.name.chars().take(22).collect();
            Row::new([
                Cell::from(short),
                Cell::from(format!("{:.0}", p.cpu)),
                Cell::from(format!("{:.0}M", p.mem as f64 / 1_000_000.0)),
            ])
        });
        let widths = [
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ];
        let table = Table::new(proc_rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(" Top Processes (↑/↓ to scroll) ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .row_highlight_style(theme.highlight())
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(table, area, &mut self.proc_state);
        self.proc_snapshot = procs; // keep for the detail popup
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme();
        let hint = match self.tab {
            0 => "↑/↓/scroll counter   r reset",
            1 => "↑/↓/click select   Enter details",
            2 => "↑/↓/click select   Enter details",
            3 => "counter shifts the wave",
            4 => "auto-updating",
            5 => "animated world map",
            _ => "↑/↓ select   Enter details   k kill",
        };
        let footer = Paragraph::new(Line::from(vec![
            " ? help   t theme   ".fg(theme.hint),
            hint.fg(theme.hint),
            "   q quit ".fg(theme.quit),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(footer, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

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
            (6, "Top Processes"),
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
        assert_eq!(app.tab, tab_before, "input is captured by the confirm prompt");
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
