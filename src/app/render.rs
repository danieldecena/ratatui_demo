use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, BorderType, Borders, Cell, Chart, Dataset, Gauge, GraphType, LineGauge, List,
        ListItem, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Sparkline,
        Table, Tabs,
        canvas::{Canvas, Circle, Map, MapResolution},
    },
};

use super::{App, ProcRow, SortKey, TAB_TITLES, TABLE_DATA, fmt_rate, load_color};
use crate::popup;
use crate::theme::Theme;

impl App {
    pub(super) fn draw(&mut self, frame: &mut Frame) {
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
            6 => self.draw_system(frame, chunks[1]),
            _ => self.draw_claude(frame, chunks[1]),
        }
        self.draw_footer(frame, chunks[2]);

        if let Some((pid, name)) = &self.pending_kill {
            let prompt = format!("Kill {name} (pid {pid})?");
            frame.render_widget(
                popup::ConfirmModal::new(self.theme(), &prompt),
                frame.area(),
            );
        } else if let Some(body) = &self.detail {
            frame.render_widget(popup::DetailModal::new(self.theme(), body), frame.area());
        } else if self.show_help {
            frame.render_widget(popup::HelpModal::new(self.theme()), frame.area());
        }
    }

    fn draw_tabs(&mut self, frame: &mut Frame, area: Rect) {
        // Compute each tab's clickable x-range, mirroring how Tabs lays out:
        // [pad_left(1)][title][pad_right(1)][divider(1)] inside the bordered area.
        let inner_x = area.x + 1;
        let inner_right = area.x + area.width.saturating_sub(1);
        let mut x = inner_x;
        let mut ranges = Vec::with_capacity(TAB_TITLES.len());
        for title in TAB_TITLES {
            let w = title.chars().count() as u16;
            let start = x;
            let end = (x + 1 + w + 1).min(inner_right);
            ranges.push((start, end));
            x = end + 1; // skip the divider
        }
        self.tab_ranges = ranges;

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
                Style::default()
                    .fg(theme.value)
                    .add_modifier(Modifier::BOLD),
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
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();
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

    fn draw_table(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};

        let theme = self.theme();

        let header = Row::new(vec!["Protocol", "Local Address", "Remote Address", "State"])
            .style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);

        let rows: Vec<Row> = self
            .tcp_sockets
            .iter()
            .map(|s| {
                let state_color = match s.state.as_str() {
                    "ESTABLISHED" => Color::Green,
                    "LISTEN" => Color::Yellow,
                    "CLOSE_WAIT" => Color::Red,
                    _ => Color::White,
                };
                Row::new(vec![
                    Cell::from(s.proto.clone()),
                    Cell::from(s.local.clone()),
                    Cell::from(s.remote.clone()),
                    Cell::from(s.state.clone()).style(Style::default().fg(state_color)),
                ])
            })
            .collect();

        let widths = [
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(25),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(" Live TCP Sockets ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn draw_chart(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::style::{Color, Style};
        use ratatui::symbols;
        use ratatui::widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType};

        let theme = self.theme();

        let rx_data: Vec<(f64, f64)> = self
            .monitor
            .rx_history
            .iter()
            .enumerate()
            .map(|(i, &val)| (i as f64, val))
            .collect();

        let tx_data: Vec<(f64, f64)> = self
            .monitor
            .tx_history
            .iter()
            .enumerate()
            .map(|(i, &val)| (i as f64, val))
            .collect();

        // Calculate max Y for the axis bounds
        let max_val = self
            .monitor
            .rx_history
            .iter()
            .chain(self.monitor.tx_history.iter())
            .copied()
            .fold(1.0, f64::max)
            .max(1.0); // minimum 1.0 MB/s to prevent zero bounds

        let datasets = vec![
            Dataset::default()
                .name("Download (MB/s)")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&rx_data),
            Dataset::default()
                .name("Upload (MB/s)")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Magenta))
                .data(&tx_data),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(" Live Network Bandwidth ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(theme.axis))
                    .bounds([0.0, 100.0])
                    .labels(["-100t", "-50t", "now"]),
            )
            .y_axis(
                Axis::default()
                    .title("MB/s")
                    .style(Style::default().fg(theme.axis))
                    .bounds([0.0, max_val])
                    .labels(vec![
                        ratatui::text::Line::from("0.0"),
                        ratatui::text::Line::from(format!("{:.1}", max_val / 2.0)),
                        ratatui::text::Line::from(format!("{:.1}", max_val)),
                    ]),
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
                ctx.print(
                    lon,
                    25.0,
                    Span::styled("◉", Style::default().fg(theme.accent)),
                );
            });
        frame.render_widget(canvas, area);
    }

    fn draw_system(&mut self, frame: &mut Frame, area: Rect) {
        let theme = *self.theme();
        let rows = Layout::vertical([
            Constraint::Length(3), // cpu gauge
            Constraint::Length(3), // memory | swap
            Constraint::Length(3), // network | disk I/O
            Constraint::Min(5),    // cores + history | processes
        ])
        .split(area);

        let to_gb = |b: u64| b as f64 / 1_000_000_000.0;

        // CPU gauge (color-coded by load)
        let cpu = self.monitor.sys.global_cpu_usage().clamp(0.0, 100.0) as f64;
        let cpu_gauge = Gauge::default()
            .block(Block::default().title(" CPU ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(load_color(&theme, cpu)))
            .ratio(cpu / 100.0)
            .label(format!("{cpu:.0}%"));
        frame.render_widget(cpu_gauge, rows[0]);

        // Memory | Swap
        let mem_swap = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(rows[1]);

        let mtotal = self.monitor.sys.total_memory().max(1);
        let mused = self.monitor.sys.used_memory();
        let mpct = mused as f64 / mtotal as f64 * 100.0;
        let mem_gauge = Gauge::default()
            .block(Block::default().title(" Memory ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(load_color(&theme, mpct)))
            .ratio(mpct / 100.0)
            .label(format!("{:.1} / {:.1} GB", to_gb(mused), to_gb(mtotal)));
        frame.render_widget(mem_gauge, mem_swap[0]);

        let stotal = self.monitor.sys.total_swap();
        let sused = self.monitor.sys.used_swap();
        let spct = if stotal == 0 {
            0.0
        } else {
            sused as f64 / stotal as f64 * 100.0
        };
        let swap_label = if stotal == 0 {
            "none".to_string()
        } else {
            format!("{:.1} / {:.1} GB", to_gb(sused), to_gb(stotal))
        };
        let swap_gauge = Gauge::default()
            .block(Block::default().title(" Swap ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(load_color(&theme, spct)))
            .ratio(spct / 100.0)
            .label(swap_label);
        frame.render_widget(swap_gauge, mem_swap[1]);

        // Network | Disk I/O
        let net_disk = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[2]);
        let net = Paragraph::new(Line::from(vec![
            "↓ ".fg(theme.series),
            fmt_rate(self.monitor.net_rx).fg(theme.value),
            "   ↑ ".fg(theme.series),
            fmt_rate(self.monitor.net_tx).fg(theme.value),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().title(" Network ").borders(Borders::ALL));
        frame.render_widget(net, net_disk[0]);
        let disk = Paragraph::new(Line::from(vec![
            "R ".fg(theme.series),
            fmt_rate(self.monitor.disk_r).fg(theme.value),
            "   W ".fg(theme.series),
            fmt_rate(self.monitor.disk_w).fg(theme.value),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().title(" Disk I/O ").borders(Borders::ALL));
        frame.render_widget(disk, net_disk[1]);

        // Bottom: left column (per-core bars + history) | processes table
        let bottom = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(rows[3]);
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
            .data(&self.monitor.cpu_history)
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

        let cpus = self.monitor.sys.cpus();
        let rows = inner.height as usize;
        let n = cpus.len().min(rows);
        if n == 0 {
            return;
        }
        let slots = Layout::vertical(vec![Constraint::Length(1); n]).split(inner);
        for (i, cpu) in cpus.iter().take(n).enumerate() {
            let usage = cpu.cpu_usage().clamp(0.0, 100.0) as f64;
            let gauge = LineGauge::default()
                .ratio(usage / 100.0)
                .label(format!("{i:>2}"))
                .filled_style(Style::default().fg(load_color(theme, usage)))
                .unfilled_style(Style::default().fg(theme.axis));
            frame.render_widget(gauge, slots[i]);
        }
    }

    /// Scrollable, selectable table of all processes sorted by CPU.
    fn draw_processes(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.proc_area = area;
        let needle = self.filter.to_lowercase();
        let mut procs: Vec<ProcRow> = self
            .monitor
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
            .filter(|p| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .collect();

        // Sort by the active key, descending for numeric, ascending for text.
        match self.sort_by {
            SortKey::Cpu => procs.sort_by(|a, b| b.cpu.total_cmp(&a.cpu)),
            SortKey::Mem => procs.sort_by_key(|p| std::cmp::Reverse(p.mem)),
            SortKey::Name => procs.sort_by_key(|p| p.name.to_lowercase()),
            SortKey::Pid => procs.sort_by_key(|p| p.pid_num),
        }
        self.proc_count = procs.len();

        let header = Row::new(["Process", "CPU%", "Mem"]).style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        );
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

        let title = if self.filtering || !self.filter.is_empty() {
            format!(
                " Processes · sort {} · /{}{} ",
                self.sort_by.label(),
                self.filter,
                if self.filtering { "_" } else { "" }
            )
        } else {
            format!(
                " Processes · sort {} (s) · filter (/) ",
                self.sort_by.label()
            )
        };

        let table = Table::new(proc_rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .row_highlight_style(theme.highlight())
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(table, area, &mut self.proc_state);

        // Scrollbar reflecting selection position within the full list.
        let pos = self.proc_state.selected().unwrap_or(0);
        let mut sb_state = ScrollbarState::new(self.proc_count.max(1)).position(pos);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.accent))
            .track_style(Style::default().fg(theme.axis));
        frame.render_stateful_widget(scrollbar, area, &mut sb_state);

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
            _ => "↑/↓ select  Enter details  s sort  / filter  k kill",
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

    fn draw_claude(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{
            BarChart, Block, BorderType, Borders, List, ListItem, Paragraph, Wrap,
        };

        // Split into Top (Stats) and Bottom (History)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);

        // Draw Stats Block
        if let Some(stats) = &self.claude_stats {
            let stats_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[0]);

            let summary = format!(
                "🔥 Total Sessions: {}
💬 Total Messages: {}
📅 First Session: {}",
                stats.total_sessions,
                stats.total_messages,
                stats.first_session_date.split('T').next().unwrap_or("")
            );

            let summary_p = Paragraph::new(summary)
                .block(
                    Block::default()
                        .title(" Lifetime Stats ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .style(Style::default().fg(self.theme().accent));

            frame.render_widget(summary_p, stats_chunks[0]);

            // Model usage BarChart
            let mut chart_data: Vec<(&str, u64)> = stats
                .model_usage
                .iter()
                .map(|(model, usage)| {
                    let short_name = if model.contains("opus") {
                        "Opus"
                    } else if model.contains("sonnet") {
                        "Sonnet"
                    } else if model.contains("haiku") {
                        "Haiku"
                    } else {
                        "Other"
                    };
                    let total_tokens =
                        usage.inputTokens + usage.outputTokens + usage.cacheReadInputTokens;
                    (short_name, total_tokens / 1_000_000) // in millions
                })
                .collect();
            chart_data.sort_by(|a, b| b.1.cmp(&a.1));
            chart_data.truncate(5);

            let barchart = BarChart::default()
                .block(
                    Block::default()
                        .title(" Tokens (Millions) ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .data(&chart_data)
                .bar_width(8)
                .bar_gap(2)
                .bar_style(Style::default().fg(self.theme().series))
                .value_style(Style::default().fg(Color::Black).bg(self.theme().series));

            frame.render_widget(barchart, stats_chunks[1]);
        }

        // Draw History List
        let items: Vec<ListItem> = self
            .claude_history
            .iter()
            .map(|row| {
                let content = vec![Line::from(vec![
                    Span::styled(
                        format!("[{}] ", row.project),
                        Style::default().fg(self.theme().accent),
                    ),
                    Span::raw(row.display.replace(
                        "
", " ",
                    )),
                ])];
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Prompt History ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.claude_state);
    }
}
