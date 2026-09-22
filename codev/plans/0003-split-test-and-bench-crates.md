# Plan: Split oversized test/bench crates (directory-style targets)

## Metadata

- **ID**: 0003
- **Status**: In progress
- **Created**: 2026-07-16
- **Protocol**: SPIR
- **Depends on**: 0001-split-oversized-modules (same discipline, applied outside src/)

## Executive Summary

Plan 0001 capped every `src/**/*.rs` at 500 lines; two files outside `src/`
remain oversized: `tests/bench_functions.rs` (2619) and `benches/benchmarks.rs`
(1963). Both are single-file cargo targets, so the split uses cargo's
directory-style target form (`<name>/main.rs` + submodules) — one crate, one
binary, one fat-LTO link — instead of files-per-target, which would multiply
link cost by the number of chunks.

## Design rules

1. `tests/bench_functions.rs` → `tests/bench_functions/main.rs` (shared
   helpers + `mod` decls) + one submodule per exercised area (capture,
   selection, record, settings, cli/tray), each ≤500 lines. The single-file
   form is deleted in the same change (cargo would otherwise see an ambiguous
   duplicate target).
2. `benches/benchmarks.rs` → `benches/benchmarks/main.rs`
   (`criterion_group!` / `criterion_main!` + shared helpers) + thematic
   submodules; bench fns become `pub(crate)`; group names unchanged so
   criterion history and `^capture_` filters keep working.
3. Verbatim-move discipline from plan 0001: code moves untouched, only mod
   plumbing/visibility/imports change; test- and bench-fn multisets verified
   identical before/after.
4. Non-goal: `codev/protocols/spir/protocol.md` (655 lines of prose) stays.

## Phases

1. Split `tests/bench_functions.rs` (independent of any bench run).
2. Split `benches/benchmarks.rs` (sequenced after in-flight bench runs
   finish — the file is their compile input).
3. Central verification: cargo check --all-targets, full test suite,
   `cargo bench -- <smoke filter>` compile check, wc -l sweep, commit.

## Rollback

Each phase is one commit on a green tree; `git revert` per phase.
