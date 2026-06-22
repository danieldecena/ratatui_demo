# Improvement Plan — ratatui_demo

> A phased roadmap from "working demo + monitor" to a solid, shippable terminal app.
> Each phase is independently valuable and lands with passing tests + zero warnings.

_Created: 2026-06-22 · Stack: Rust, ratatui 0.30.2, sysinfo 0.39.5_

---

## Guiding principles

- Keep `cargo test` green and `cargo clippy`/`cargo fmt` clean after every phase.
- Every user-visible behavior gets a `TestBackend` test.
- No regressions to the existing 7 tabs; the demo stays runnable throughout.
- Small, reviewable commits — one logical change each.

---

## Phase 1 — Robustness (do first)

Goal: the app never leaves the terminal broken, and the repo stays healthy.

1. **Panic-safe terminal restore**
   - Add `color-eyre`. Install a panic + error hook that calls `ratatui::restore()`
     before printing, so a crash returns a usable terminal.
   - `main` returns `color_eyre::Result<()>`; wrap setup/teardown so mouse capture is
     always disabled even on the error path.
   - Files: `Cargo.toml`, `src/main.rs`.
   - Accept: forcing a `panic!` mid-run leaves the terminal usable (manual check);
     `cargo run` still works.

2. **Lint + format clean**
   - `cargo fmt`; resolve all `cargo clippy -- -D warnings`.
   - Files: workspace-wide.
   - Accept: `cargo clippy -- -D warnings` exits 0.

3. **CI workflow**
   - `.github/workflows/ci.yml`: on push/PR run `fmt --check`, `clippy -D warnings`,
     `test` on stable (Linux + macOS).
   - Accept: workflow file present and valid; jobs defined for both OSes.

_Effort: small. Highest value-to-cost._

---

## Phase 2 — System monitor depth

Goal: turn the System tab into a genuinely useful `bottom`/`htop`-style view.

1. **Sortable process columns**
   - Cycle sort key with `s` (CPU → Memory → Name → PID); show the active key in the
     block title with a ▲/▼ marker.
   - State: `sort_by: SortKey`, `sort_desc: bool`.
   - Accept: test that toggling sort reorders `proc_snapshot` deterministically.

2. **Filter / search**
   - `/` opens an input line (using a simple text buffer); typed text filters the
     process list by name (case-insensitive); `Esc` clears.
   - State: `filter: Option<String>`.
   - Accept: test that a filter narrows the snapshot and clears cleanly.

3. **Color-coded load**
   - CPU gauge and per-core `LineGauge`s shift green → yellow → red by threshold
     (e.g. <50 / <80 / ≥80). Same for the memory gauge.
   - Helper `fn load_color(pct: f64) -> Color`.
   - Accept: unit test on the threshold helper.

4. **Scrollbar on the process list**
   - Add a `Scrollbar` tied to `proc_state` so scroll position is visible.
   - Accept: renders without panic at various list lengths (TestBackend).

5. **More stats**
   - Swap usage (second memory gauge or combined), plus network and disk I/O via
     `sysinfo` `Networks`/`Disks` (totals or rates). Lay out in a compact strip.
   - Accept: System tab renders the new rows; refresh logic updates them on tick.

_Effort: medium. Biggest functional payoff._

---

## Phase 3 — Polish / UX

1. **Persist config** — save theme + last tab + sort key to a small TOML in the
   platform config dir (`directories` crate); load on startup.
   Accept: round-trip test of save/load; missing file falls back to defaults.

2. **Precise tab clicking** — map a top-bar click to the exact tab under the cursor
   (compute each tab's x-range) instead of always advancing.
   Accept: test that clicking within a tab's x-range selects that tab.

3. **Configurable refresh rate + FPS** — `+`/`-` adjust tick rate; optional FPS
   readout in a corner.
   Accept: tick-rate state changes; render unaffected.

---

## Phase 4 — Architecture / maintainability

1. **Split `app.rs`** — move each tab's draw code into `src/tabs/{counter,list,table,
   chart,live,canvas,system}.rs`; keep `App` state central.
2. **Elm-style `Action` enum** — translate key/mouse events into `Action` values, then
   apply them in one `update(action)` method (per the Ratatui docs' TEA pattern). Makes
   event handling testable without synthesizing key events.
   Accept: behavior unchanged; tests target `update(Action::…)` directly.

_Effort: medium; do after features settle so you're not refactoring twice._

---

## Phase 5 — Distribution

1. **Release profile** in `Cargo.toml` (`opt-level`, `lto`, `strip`).
2. **`cargo install` readiness** — metadata (`description`, `repository`, `keywords`,
   `categories`) so it could be published.
3. **GitHub release workflow** — build binaries for macOS (arm64/x86_64) + Linux on tag.
4. Optional: rename/extract a standalone `sysmon` binary (the held TASKS.md item) as a
   second `[[bin]]` or sibling crate, sharing the System-tab code.

---

## Suggested order & rough effort

| Phase | Theme            | Effort | Priority |
|-------|------------------|--------|----------|
| 1     | Robustness       | S      | ★★★ do now |
| 2     | Monitor depth    | M      | ★★★ |
| 3     | Polish / UX      | M      | ★★ |
| 4     | Architecture     | M      | ★★ |
| 5     | Distribution     | S–M    | ★ |

## Definition of done (per phase)

- `cargo test` green, `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean.
- New behavior covered by `TestBackend` tests.
- `README.md` controls table and `TASKS.md` updated.
- One commit per logical change with a clear message.
