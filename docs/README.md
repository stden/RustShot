# RustShot Documentation Map

The guiding rule: **the "why" lives next to the code.** Architecture and design
rationale are rustdoc comments, behaviour is an executable spec, and anything
that isn't RustShot-specific lives in the central `iopm` repo. There is no
separate architecture document to drift out of date.

## 1. Architecture & design rationale → the code

Start at [`src/lib.rs`](../src/lib.rs): it carries the data-flow overview and the
key design decisions (no `serde`, `Result<_, String>` everywhere, `softbuffer`
for the overlay, child-process isolation for the selection window).

> `cargo doc --all-features --no-deps --open` renders all of it. This used to
> produce nothing on Windows: the bin target `RustShot` and the lib crate
> `rustshot` resolve to the same `target/doc/` directory on a case-insensitive
> filesystem, and rustdoc clears that directory before writing, so the bin —
> documented second — deleted the whole library tree. `doc = false` on `[[bin]]`
> in `Cargo.toml` is what keeps it from happening again.

The hard-won, non-obvious constraints are documented at the code they constrain:

| Where | What it explains |
|---|---|
| `selection/layout.rs::build_physical_screen_layouts` | Why there is **no global scale factor**; primary anchored at physical (0,0), neighbours inferred iteratively. |
| `selection/layout.rs::resolve_recording_selection` | Why a recording region must fit inside **one** monitor (no compositing path). |
| `selection/desktop.rs::capture_screen_at_cursor` | Why the overlay covers only the cursor's monitor, and why the spanning-desktop overlay was abandoned. |
| `selection/overlay.rs::OverlayPixelMapCache` | Why an immutable converted screenshot is cached for cursor redraws, when the cache is invalidated, and the one-frame memory tradeoff. |
| `capture.rs` (capture → `from_raw`) | The forced `image` 0.24/0.25 bridge via `screenshots` 0.8, and its per-capture copy cost. |
| `capture/platform.rs::capture_active_window_platform` | Why `GetDC(None)` is deliberate (null HWND ≠ `Some(HWND::default())`). |
| `record.rs` (recording hot loop) | Why the BGRA buffer is preallocated and capture is region-scoped (per-frame allocation was the lag source). |
| `app/runtime.rs::stop_flag_path` | The `%TEMP%/rustshot_stop_<pid>.flag` IPC contract; one helper so the parent and child paths cannot drift. |
| `app/autorun.rs::sync_autorun` | Two-way autostart sync; `HKCU` only (keeps RustShot elevation-free); Tray mode only. |
| `tray.rs` (menu IDs) | Why recording is one toggle entry, not Start + Stop. |

## 2. Behaviour → executable specifications

Gherkin features in [`tests/features/`](../tests/features/) **are** the
behavioural spec; [`tests/bdd.rs`](../tests/bdd.rs) runs them against the public
API via Cucumber.

```
cargo test --test bdd
```

Covers app-mode routing, CLI arguments, format resolution and output paths,
settings normalization and persistence, multi-monitor layout and overlay
targeting, drag→region, recording validation/timing, active-window and live
capture, pixel conversion, the tray, and the MCP JSON-RPC contract — 20 feature
files. A spec that drifts from the code fails CI.

Two further test targets guard structure rather than behaviour:
`tests/lint_policy.rs` (the curated `[lints]` table in `Cargo.toml` and `msrv` in
`clippy.toml`) and `tests/e2e_regressions.rs`.

## 3. Process → `codev/`

| Path | Contents |
|------|----------|
| [`codev/protocols/spir/`](../codev/protocols/spir/) | **SPIR** — the active protocol. Specify → Plan → Implement → Review, human-driven with models as reviewers. |
| [`codev/protocols/aidlc/`](../codev/protocols/aidlc/) | **AIDLC** — AI Development Life Cycle. Inception → Construction → Operations, worked in bolts; AI proposes and the team validates. |
| [`codev/plans/`](../codev/plans/) | Three landed plans: `0001-split-oversized-modules`, `0002-enums-and-newtypes`, `0003-split-test-and-bench-crates`. |
| `codev/specs/`, `codev/reviews/` | Currently empty. Both protocols share these paths with `plans/` but require different sections. |

Protocol definitions are validated structurally by `tests/codev_protocols.rs`: phase
transitions must resolve, gates may only require checks their phase declares, and a
check that greps an artifact for a heading must match a heading the template actually
has. Pick a protocol per project, not per commit — see
[`aidlc/protocol.md`](../codev/protocols/aidlc/protocol.md) for when each fits.

## 4. Project-specific technical docs → `docs/`

| File | Topic |
|------|-------|
| [`TUTORIAL.md`](TUTORIAL.md) | **Tutorial, part 2.** Part 1 is in [`README.md`](../README.md) (clone → build → visible tray icon → first capture); this continues with recording, the settings dialog, the `HKCU\...\Run` auto-start entry and how to switch it off, the MCP server, per-platform limits, and troubleshooting keyed on RustShot's literal error strings. |
| [`INSTALLERS.md`](INSTALLERS.md) | Installer overview. |
| [`INNO_SETUP_PIPELINE.md`](INNO_SETUP_PIPELINE.md) | Inno Setup packaging pipeline. |
| [`github-pipeline.md`](github-pipeline.md) | CI pipeline notes. |

## 5. Narrative → `articles/` and `wiki/`

[`articles/`](../articles/) holds four long-form essays on the engineering
themes behind RustShot (indexed by [`ARTICLES.md`](../ARTICLES.md)).
[`wiki/`](../wiki/) is a synthesized knowledge base over them.

## 6. Not here — the central `iopm` repo

General, non-RustShot guidance was moved out to `iopm/docs/` so it isn't
duplicated per project:

- `rust-best-practices.md`
- `rust-installers.md`
- `TESTING_TDD_GUIDE.md`
- `TESTING_CHECKLIST_RUST.md`

## Onboarding files (repo root)

[`README.md`](../README.md), [`CLAUDE.md`](../CLAUDE.md),
[`AGENTS.md`](../AGENTS.md), [`GEMINI.md`](../GEMINI.md). Agent tooling lives in
`.claude/` and `.codex/`.
