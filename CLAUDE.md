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

## Layout

- `src/main.rs` — App state, event loop, tab routing
- `src/ui.rs` — Widget rendering per tab
