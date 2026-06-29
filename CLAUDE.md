# ratatui_demo

Rust + Ratatui TUI demo — widget reference + live system monitor (`sysinfo`, `tokio`).

## Stack

Rust 2024 edition · ratatui 0.30 · crossterm · tokio (async) · sysinfo

## Run

```
cargo run       # TUI
cargo test
```

## Tabs

| Tab | Purpose |
|---|---|
| Counter | Gauge widget, state demo |
| List | Scrollable selectable list |
| Table | Tabular widget reference (stateful rows) |
| Chart | Line chart + sparkline plots |
| Live | Live system monitor (`sysinfo`, async `tokio`) |
| Canvas | Canvas drawing widget |
| System | Top processes / system stats |
| Claude | Claude Code usage stats dashboard |

## Layout

- `src/main.rs` — entry point + event loop
- `src/app.rs` — `App` state, tab routing, `TAB_TITLES`
- `src/app/render.rs` — widget rendering per tab
- `src/claude.rs` · `src/data.rs` · `src/popup.rs` · `src/theme.rs` — Claude usage, data sources, detail popup, theming
