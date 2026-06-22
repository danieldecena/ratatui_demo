# ratatui_demo

A small terminal-UI app built with [Ratatui](https://ratatui.rs) that doubles as a
widget reference and a live system monitor. Built incrementally as a hands-on tour
of Ratatui's widgets, then pointed toward a real `bottom`-style monitor.

## Run

```sh
cargo run
```

Requires a recent stable Rust toolchain (install via <https://rustup.rs>).

## Tabs

| Tab     | Shows                                                                 |
|---------|----------------------------------------------------------------------|
| Counter | State + a `Gauge` — `↑/↓` change the value, `r` resets               |
| List    | Scrollable, selectable `List`; click or `Enter` for details          |
| Table   | `Table` with header/columns; click or `Enter` for details            |
| Chart   | Braille `Chart` sine wave; the counter shifts its phase              |
| Live    | Auto-updating `Sparkline` driven by a tick loop                      |
| Canvas  | `Canvas` world map with an animated marker                          |
| System  | Live CPU + memory gauges, per-core bars, CPU history, process table  |

## Controls

| Key / action        | Effect                                              |
|---------------------|-----------------------------------------------------|
| `Tab` / `← →`       | Switch tabs (or click the tab bar)                  |
| `↑ ↓` / scroll      | Adjust the active tab                               |
| click a row         | Select it (List, Table, System)                     |
| `Enter`             | Open a details popup (List / Table / System)        |
| `s`                 | Cycle process sort: CPU → Mem → Name → PID (System) |
| `/`                 | Filter processes by name (System); `Esc` to finish  |
| `k`                 | Kill the selected process (System tab; confirms)    |
| `+` / `-`           | Faster / slower refresh rate                         |
| `t`                 | Cycle color theme                                   |
| `?`                 | Toggle help                                         |
| `r`                 | Reset the counter                                   |
| `q` / `Esc`         | Close a popup, or quit                              |

Your theme, last tab, sort key, and refresh rate are **persisted** to a small
config file and restored next launch.

## System monitor

The **System** tab reads live stats via [`sysinfo`](https://crates.io/crates/sysinfo):
a color-coded CPU gauge (green → yellow → red), memory and swap gauges, live
network (↓/↑) and disk (R/W) I/O rates, one `LineGauge` per core, a rolling
CPU-history sparkline, and a scrollable, sortable, filterable table of every
process with a scrollbar. Select a process and press
`Enter` for its details (PID, CPU%, memory, status, uptime) or `k` to kill it after
a confirmation prompt.

## Project layout

```
src/
  main.rs        terminal setup + teardown, panic-safe restore
  app.rs         application state, input → Action → update, config, tests
  app/render.rs  all tab rendering (draw_* methods)
  theme.rs       color palettes
  popup.rs       help / detail / confirm modals
```

## Tests

Rendering and behavior are tested headlessly via Ratatui's `TestBackend`:

```sh
cargo test
```

## Built with

- [ratatui](https://crates.io/crates/ratatui) — terminal UI
- [sysinfo](https://crates.io/crates/sysinfo) — system stats

## License

MIT — see [LICENSE](LICENSE).
