# TASKS — ratatui_demo

> Per-project task list. Source of truth for what's done / next.

_Last updated: 2026-06-22_

## 🟡 In Progress

- (none)

## 🔴 Blocked

- (none)

## 🟢 Next Up

- See `PLAN.md` for the roadmap. **Phases 1–5 are done, plus net/disk I/O and the
  render-module split.**
- [ ] Optional: strip demo tabs to ship a standalone `bottom`-style monitor.
      (Held — mutually exclusive with the multi-tab demo. Do as a separate copy.)

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
- [x] System tab — kill the selected process with `k` (confirmation prompt).
- [x] System tab — mouse click-to-select on the process list.
- [x] README + LICENSE (MIT) + `git init` with first commit.
- [x] Test suite expanded to 17 tests (all passing), zero warnings.
- [x] Phase 1 — panic-safe terminal restore (`color-eyre` + panic hook).
- [x] Phase 1 — clippy `-D warnings` clean + `cargo fmt`.
- [x] Phase 1 — GitHub Actions CI (fmt/clippy/test on Linux + macOS).
- [x] Phase 2 — sortable process columns (`s`), name filter (`/`), color-coded
      CPU/mem/swap/core load, process scrollbar, swap gauge.
- [x] Phase 3 — persist theme/tab/sort/refresh to config, precise tab clicking,
      adjustable refresh rate (`+`/`-`).
- [x] Phase 4 — Elm-style `Action` enum + `update()` (input decoupled from logic).
- [x] Phase 5 — release profile (LTO/strip), crate metadata, release workflow.
- [x] Network & disk I/O rates on the System tab.
- [x] Split rendering into `src/app/render.rs` (state/logic stay in `app.rs`).
- [x] Test suite expanded to 24 tests, clippy clean, zero warnings.

## 🧠 Context

- Stack: Rust + `ratatui` 0.30.2 (crossterm backend) + `sysinfo` 0.39.5.
- Layout: 7 tabs — Counter, List, Table, Chart, Live, Canvas, System.
- Structure: `src/main.rs` (terminal setup), `src/app.rs` (state + drawing + tests),
  `src/theme.rs` (palettes), `src/popup.rs` (help / detail / confirm modals).
- Run: `cargo run`. Test: `cargo test`.
- Build target is redirected when building in the sandbox; locally it's just `cargo`.
