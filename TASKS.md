# TASKS — ratatui_demo

> Per-project task list. Source of truth for what's done / next.

_Last updated: 2026-06-22_

## 🟡 In Progress

- (none)

## 🔴 Blocked

- (none)

## 🟢 Next Up

- [ ] **Kill-process support** — press `k` (with a confirm prompt) to terminate
      the selected process via `sysinfo` `Process::kill()`.
- [ ] Mouse click-to-select on the System process list (like List/Table tabs).
- [ ] Optional: strip demo tabs to ship a standalone `bottom`-style monitor.
- [ ] Optional: add README + LICENSE and `git init` for a pushable repo.

## ✅ Done

- [x] Scaffold Rust project, add `ratatui` 0.30.2, build & run a TUI.
- [x] Counter tab (state + gauge).
- [x] Tabs navigation (keyboard + click on tab bar).
- [x] List tab (scrollable, selectable; click-to-select).
- [x] Table tab (headers, columns; click-to-select).
- [x] Chart tab (Braille sine wave, counter shifts phase).
- [x] Live tab (auto-updating sparkline, tick loop).
- [x] Canvas tab (world map + animated marker).
- [x] Color theming (`t` cycles 3 themes).
- [x] Help popup (`?`) and Details popup (`Enter` on List/Table).
- [x] Refactor into modules: main / app / theme / popup.
- [x] Test suite via `TestBackend` (12 tests, all passing).
- [x] System tab — live CPU + memory gauges, CPU history sparkline, top processes.
- [x] System tab — per-core CPU `LineGauge` bars + scrollable/selectable process list.
- [x] System tab — Enter opens a process Details popup (PID, CPU%, memory, status, uptime).

## 🧠 Context

- Stack: Rust + `ratatui` 0.30.2 (crossterm backend) + `sysinfo` 0.39.5.
- Layout: 7 tabs — Counter, List, Table, Chart, Live, Canvas, System.
- Structure: `src/main.rs` (terminal setup), `src/app.rs` (state + drawing + tests),
  `src/theme.rs` (palettes), `src/popup.rs` (help/detail modals).
- Run: `cargo run`. Test: `cargo test`.
- Build target is redirected when building in the sandbox; locally it's just `cargo`.
