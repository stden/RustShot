# Plan: Split all src/*.rs files over 500 lines into submodules

## Metadata

- **ID**: 0001
- **Status**: In progress (implementation running)
- **Created**: 2026-07-16
- **Protocol**: SPIR
- **Spec**: none (mechanical refactoring task, user-directed; spec phase skipped per "Skip SPIR for" guidance)

## Executive Summary

Eight files in `src/` exceed 500 lines (23.8k lines total, worst: `selection.rs`
at 6229). In every one of them the trailing `#[cfg(test)] mod tests { … }` block
is 60–80% of the line count. The split therefore happens in two passes: extract
test modules into sibling `<module>/tests/` files (zero semantic change), then
split only the logic that still exceeds 500 lines into cohesive submodules whose
items are re-exported from the module root so **every existing path keeps
resolving**. No behavior change, no public-API change, no renames.

## Success Metrics

- Every `.rs` file under `src/` is ≤ 500 lines (`wc -l` verified).
- `cargo check`, `cargo fmt --check`, `cargo clippy`, and the full test suite
  pass with the same test counts as before the refactor.
- `benches/benchmarks.rs` compiles unchanged (it consumes
  `rustshot::<module>::…` paths, including `benchmark_*` shims).
- Linux CI (`test-linux` with xvfb) stays green — all `#[cfg(windows)]` gates
  preserved exactly.

## Design rules (apply to every phase)

1. Each oversized `foo.rs` stays as the module root; new files go in a sibling
   `src/foo/` directory (Rust 2018 layout, no `mod.rs` conversion). `lib.rs`
   and the `mod` declarations in `main.rs` are untouched (except the new
   bin-local modules in Phase 2).
2. Tests: inline block → `#[cfg(test)] mod tests;` → `foo/tests.rs` holds
   shared helpers/fixtures + `mod <chunk>;` declarations; themed chunks in
   `foo/tests/<theme>.rs`. Test fn names and bodies verbatim. Shared statics
   (e.g. test-serialization mutexes) stay in `tests.rs` so all chunks use the
   same instance.
3. Logic moves verbatim; only imports, visibility, and module plumbing change.
   Previously-private items moved into a submodule become `pub(super)`;
   `pub` stays `pub`; the root does `mod part; pub use part::*;`.
4. The crate has a dual compile tree (`lib.rs` and `main.rs` both mount the
   shared modules); `settings_dialog` is bin-only behind `#[cfg(windows)]` and
   must stay that way.

## Phases (Machine Readable)

```yaml
phases:
  - id: 1
    name: Split shared library modules
    files: [selection.rs, capture.rs, record.rs, settings.rs, tray.rs, cli.rs]
  - id: 2
    name: Split bin-only modules
    files: [main.rs, settings_dialog.rs]
  - id: 3
    name: Central verification and commit
    files: [all]
```

## Phase Breakdown

### Phase 1: Shared library modules

| File (lines = logic + tests) | Logic split | Test split |
|---|---|---|
| `selection.rs` (6229 = 1469 + 4760) | root + `selection/geometry.rs` (layout/coordinate math) + `selection/overlay.rs` (overlay window, message loop, drag state) | `selection/tests.rs` + ~10 themed chunks |
| `capture.rs` (3425 = 605 + 2819) | root + `capture/bench_support.rs` (`benchmark_*` shims) | `capture/tests.rs` + ~6 chunks |
| `record.rs` (3245 = 636 + 2608) | root + `record/timing.rs` (pure fps/duration/validation math; Windows encoder plumbing stays in root) | `record/tests.rs` + ~6 chunks |
| `settings.rs` (2606 = 567 + 2038) | root + `settings/json.rs` (hand-rolled JSON parse/format) | `settings/tests.rs` + ~5 chunks |
| `tray.rs` (1098 = 191 + 906) | unchanged (logic already ≤ 500) | `tray/tests.rs` + 2–3 chunks |
| `cli.rs` (695 = 182 + 512) | unchanged | `cli/tests.rs` + 2 chunks (args vs app-mode policy) |

Exact seam placement may shift to the code's natural boundaries; the table is
the intent, the 500-line cap is the constraint.

### Phase 2: Bin-only modules

| File | Split |
|---|---|
| `main.rs` (4963 = ~911 + ~4051) | thin crate root (mod decls + `fn main`) + new `app.rs` root with `app/autorun.rs` (Windows autorun/registry), `app/hotkeys.rs` (hotkey + tray wiring), `app/routing.rs` (CLI-mode & format policy). Tests → `#[cfg(test)] mod main_tests;` → `main_tests.rs` + ~9 chunks (path changes from `tests::…` to `main_tests::…`, fn names unchanged). Test-only helper fns gated `#[cfg(test)]` move into the test tree; `#[cfg(any(test, windows))]` items keep their gates in `app/`. |
| `settings_dialog.rs` (857 = 568 + 288) | root + `settings_dialog/layout.rs` (one cohesive chunk, e.g. layout/geometry or dialog-proc handling); tests fit in a single `settings_dialog/tests.rs`. Whole subtree remains bin-only behind `cfg(windows)`. |

Untouched: `mcp.rs` (407), `window.rs` (239), `lib.rs` (57) — already under cap.

### Phase 3: Central verification

1. `cargo check` (bin + lib + test targets); fix any import/visibility fallout.
2. `cargo fmt`; `cargo clippy` clean.
3. Full test suite; test counts must match pre-refactor totals.
4. `wc -l` sweep proving every `src/**/*.rs` ≤ 500.
5. Single commit `[!] refactor: split oversized modules (all src files ≤ 500 lines)`, push.

## Risks

- **Import/visibility fallout** from moving code between module levels —
  mitigated by verbatim moves + central `cargo check` fix-up loop.
- **cfg(windows) mistakes breaking Linux CI** — gates preserved exactly;
  Linux leg of CI is the backstop.
- **Disk pressure** (~2.4 GB free on this machine): parallel edits run without
  building; a single central build with a low-disk watchdog does verification.

## Rollback Strategy

Single-commit refactor on top of a clean pushed state (`eedaa79`); rollback is
`git revert` (or reset before push). No data or config migration involved.
