# RustShot Documentation Map

The guiding rule: **the "why" lives next to the code.** Architecture and design
rationale are rustdoc comments, behaviour is an executable spec, and anything
that isn't RustShot-specific lives in the central `iopm` repo. There is no
separate architecture document to drift out of date.

## 1. Architecture & design rationale → the code

Start at [`src/lib.rs`](../src/lib.rs): it carries the data-flow overview and the
key design decisions (no `serde`, `Result<_, String>` everywhere, `softbuffer`
for the overlay, child-process isolation for the selection window).

The hard-won, non-obvious constraints are documented at the code they constrain:

| Where | What it explains |
|---|---|
| `selection.rs::build_physical_screen_layouts` | Why there is **no global scale factor**; primary anchored at physical (0,0), neighbours inferred iteratively. |
| `selection.rs::resolve_recording_selection` | Why a recording region must fit inside **one** monitor (no compositing path). |
| `selection.rs::capture_screen_at_cursor` | Why the overlay covers only the cursor's monitor, and why the spanning-desktop overlay was abandoned. |
| `capture.rs` (capture → `from_raw`) | The forced `image` 0.24/0.25 bridge via `screenshots` 0.8, and its per-capture copy cost. |
| `capture.rs::capture_active_window_platform` | Why `GetDC(None)` is deliberate (null HWND ≠ `Some(HWND::default())`). |
| `record.rs` (recording hot loop) | Why the BGRA buffer is preallocated and capture is region-scoped (per-frame allocation was the lag source). |
| `main.rs` (stop flag) | The `%TEMP%/rustshot_stop_<pid>.flag` IPC contract between parent and child. |
| `main.rs::sync_autorun` | Two-way autostart sync; `HKCU` only (keeps RustShot elevation-free); Tray mode only. |
| `tray.rs` (menu IDs) | Why recording is one toggle entry, not Start + Stop. |

## 2. Behaviour → executable specifications

Gherkin features in [`tests/features/`](../tests/features/) **are** the
behavioural spec; [`tests/bdd.rs`](../tests/bdd.rs) runs them against the public
API via Cucumber.

```
cargo test --test bdd
```

Covers app-mode routing, format resolution, settings normalization, recording
validation/timing, and drag→region. A spec that drifts from the code fails CI.

## 3. Process → `codev/` (SPIR)

| Path | Contents |
|------|----------|
| [`codev/protocols/spir/`](../codev/protocols/spir/) | The active protocol + spec/plan/review templates. |
| `codev/specs/`, `codev/plans/`, `codev/reviews/` | Where new SPIR artifacts go (currently empty). |

## 4. Project-specific technical docs → `docs/`

| File | Topic |
|------|-------|
| [`INSTALLERS.md`](INSTALLERS.md) / [`.ru`](INSTALLERS.ru.md) | Installer overview. |
| [`INNO_SETUP_PIPELINE.md`](INNO_SETUP_PIPELINE.md) / [`.ru`](INNO_SETUP_PIPELINE.ru.md) | Inno Setup packaging pipeline. |
| [`github-pipeline.md`](github-pipeline.md) | CI pipeline notes. |

## 5. Narrative → `articles/` and `wiki/`

[`articles/`](../articles/) holds four long-form RU essays on the engineering
themes behind RustShot (indexed by [`ARTICLES.ru.md`](../ARTICLES.ru.md)).
[`wiki/`](../wiki/) is a synthesized knowledge base over them.

## 6. Not here — the central `iopm` repo

General, non-RustShot guidance was moved out to `iopm/docs/` so it isn't
duplicated per project:

- `rust-best-practices.md`
- `rust-installers.md`
- `TESTING_TDD_GUIDE.md`
- `TESTING_CHECKLIST_RUST.md`

## Onboarding files (repo root)

[`README.md`](../README.md) (+ `.ru`), [`CLAUDE.md`](../CLAUDE.md) (+ `.ru`),
[`AGENTS.md`](../AGENTS.md), [`GEMINI.md`](../GEMINI.md). Agent tooling lives in
`.claude/` and `.codex/`.
