# RustShot Documentation Map

The guiding rule: **the "why" lives next to the code.** Architecture and design
rationale are rustdoc comments, behaviour is an executable spec, and anything
that isn't RustShot-specific lives in the central `iopm` repo. There is no
separate architecture document to drift out of date.

## 1. Architecture & design rationale → the code

Start at [`src/lib.rs`](../src/lib.rs): it documents the dependency layers,
interactive capture sequence, and code/test boundaries with Mermaid diagrams.
[`src/domain.rs`](../src/domain.rs) describes shared values and geometry;
[`src/application.rs`](../src/application.rs) describes shared workflows.
Native adapters live under `src/platform/` and reusable pixel transforms under
`src/pixels/`. Feature APIs retain compatibility re-exports.

Plain `cargo doc` preserves Mermaid source. To render diagrams in the generated
library documentation:

```text
cargo rustdoc --lib --all-features -- --html-in-header docs/rustdoc-mermaid.html
```

Open `target/doc/rustshot/index.html` after the command finishes. The optional
renderer loads a pinned Mermaid version from a CDN in the browser; it adds no
production or build dependency. Every diagram keeps an expandable source block,
and the full source remains visible if the renderer cannot load while offline.

> `cargo doc --all-features --no-deps --open` renders all of it. The bin target
> uses `doc = false` so its output does not collide with the library on
> case-insensitive filesystems.

The hard-won, non-obvious constraints are documented at the code they constrain:

| Where | What it explains |
|---|---|
| `domain/layout.rs::build_physical_screen_layouts` | Why there is **no global scale factor**; primary anchored at physical (0,0), neighbours inferred iteratively. |
| `domain/layout.rs::resolve_recording_selection` | Why a recording region must fit inside **one** monitor (no compositing path). |
| `selection/desktop.rs::capture_screen_at_cursor` | Why the overlay covers only the cursor's monitor, and why the spanning-desktop overlay was abandoned. |
| `selection/overlay.rs::OverlayPixelMapCache` | Why an immutable converted screenshot is cached for cursor redraws, when the cache is invalidated, and the one-frame memory tradeoff. |
| `capture.rs` (capture → `from_raw`) | The forced `image` 0.24/0.25 bridge via `screenshots` 0.8, and its per-capture copy cost. |
| `platform/window.rs::capture_active_window_platform` | Why `GetDC(None)` is deliberate (null HWND ≠ `Some(HWND::default())`). |
| `record.rs` (recording hot loop) | Why the BGRA buffer is preallocated and capture is region-scoped (per-frame allocation was the lag source). |
| `app/runtime.rs::stop_flag_path` | The `%TEMP%/rustshot_stop_<pid>.flag` IPC contract; one helper so the parent and child paths cannot drift. |
| `application/autorun.rs::sync_autorun` | Per-user self-install and two-way autostart sync; stable `%LOCALAPPDATA%` executable; `HKCU` and Tray mode only. |
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
capture, first-run startup installation, pixel conversion, the tray, and the MCP
JSON-RPC contract. A spec that drifts from the code fails CI.

Unit test bodies and shared fixtures live under [`tests/unit/`](../tests/unit/).
They are compiled as private child modules of the library, not as extra Cargo
integration targets. Small `cfg(test)` hooks at native/settings boundaries are
excluded from production builds. `cargo test` discovers the suites as before.

[`tests/architecture.rs`](../tests/architecture.rs) enforces the shared layers'
dependency direction by parsing Rust imports and keeps test bodies out of `src/`.
Other test targets guard structure and regressions:
`tests/lint_policy.rs` (the curated `[lints]` table in `Cargo.toml` and `msrv` in
`clippy.toml`) and `tests/e2e_regressions.rs`.

## 3. Process → `codev/`

| Path | Contents |
|------|----------|
| [`codev/protocols/spir/`](../codev/protocols/spir/) | **SPIR** — the active protocol. Specify → Plan → Implement → Review, human-driven with models as reviewers. |
| [`codev/protocols/aidlc/`](../codev/protocols/aidlc/) | **AIDLC** — AI Development Life Cycle. Inception → Construction → Operations, worked in bolts; AI proposes and the team validates. |
| [`codev/plans/`](../codev/plans/) | Implementation plans produced by the selected protocol. |
| [`codev/specs/`](../codev/specs/) | Specifications produced by the selected protocol. |
| [`codev/reviews/`](../codev/reviews/) | Reviews produced by the selected protocol. |

Protocol definitions are validated structurally by `tests/codev_protocols.rs`: phase
transitions must resolve, gates may only require checks their phase declares, and a
check that greps an artifact for a heading must match a heading the template actually
has. Pick a protocol per project, not per commit — see
[`aidlc/protocol.md`](../codev/protocols/aidlc/protocol.md) for when each fits.

## 4. Project-specific technical docs → `docs/`

| File | Topic |
|------|-------|
| [`TUTORIAL.md`](TUTORIAL.md) | **Tutorial, part 2.** Part 1 is in [`README.md`](../README.md) (clone → build → visible tray icon → first capture); this continues with recording, the settings dialog, stable per-user self-installation and its `HKCU\...\Run` entry, the MCP server, per-platform limits, and troubleshooting keyed on RustShot's literal error strings. |
| [`INSTALLERS.md`](INSTALLERS.md) | Installer overview. |
| [`INNO_SETUP_PIPELINE.md`](INNO_SETUP_PIPELINE.md) | Inno Setup packaging pipeline. |
| [`github-pipeline.md`](github-pipeline.md) | CI pipeline notes. |

## 5. Narrative → `articles/` and `wiki/`

[`articles/`](../articles/) holds four long-form essays on the engineering
themes behind RustShot (indexed by [`ARTICLES.md`](../ARTICLES.md)).
[`wiki/`](../wiki/) is the retrieval layer over those essays, project docs,
executable specifications, and code. Each page has stable metadata, tags,
canonical source paths, and related-page IDs. `python wiki/lint.py` rejects
missing sources, broken local links, duplicate IDs, and pages absent from the
retrieval index; CI runs the same validation.

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
