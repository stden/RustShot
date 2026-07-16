# Plan: Apply enums and the newtype pattern to primitive-typed APIs

## Metadata

- **ID**: 0002
- **Status**: Draft (implementation blocked on plan 0001 landing — same files)
- **Created**: 2026-07-16
- **Protocol**: SPIR
- **Depends on**: 0001-split-oversized-modules

## Executive Summary

A full-code survey found the codebase already practices type-driven design in
places (`Format`, `MenuAction`, `AppMode` enums; a validated `Fps(u32)`
newtype; `Region`/layout structs) but leaks primitives across module
boundaries: `Fps` is unpacked to a bare `u32` at the settings boundary and
re-validated downstream; physical vs logical pixel coordinates are
distinguished only by parameter names; screen indices, window ids, scale
factors, and rect tuples travel as bare `usize`/`u64`/`f32`/`(i32,i32,u32,u32)`.
This plan introduces newtypes and enums in three phases ordered by
value-to-churn ratio. Explicit non-goal: replacing `Result<_, String>` with
error enums — that convention is documented in `lib.rs` as deliberate
(~181 raw error-construction sites; no correctness gain).

## Success Metrics

- Each converted API can no longer be called with transposed/mixed-up
  primitives (compile-time enforcement).
- No behavior change: full test suite passes; test count changes only where
  signatures force test-call updates (bodies/assertions unchanged).
- Benches (`rustshot::*` paths) still compile; the `benchmark_*` tuple ABI is
  preserved unless a phase explicitly migrates it.

## Phases (Machine Readable)

```yaml
phases:
  - id: A
    name: Low-risk, high-value threading
    items: [fps-threading, monitor-origin-point, appmoderequest-args, format-source-enum]
  - id: B
    name: Domain identifier newtypes
    items: [screen-index, scale-factor, window-id]
  - id: C
    name: Physical/Logical coordinate types (staged, leaf-first)
    items: [physical-logical-newtypes]
```

## Phase A — low risk, high value

1. **Thread `Fps` through the recording path** (survey #1, best ratio — the
   newtype exists at `settings::Fps`). `main.rs` currently calls
   `settings.recording_fps.get()` and `record.rs` re-validates the raw `u32`
   in `validate_recording_start`. Change `RecordingSession::start`,
   `start_in_dir`, `record_loop` (both cfg variants), `wmf_record` to take
   `Fps`; call `.get()` only at the WMF FFI edge. Keep pure helper
   `frame_duration_hns(u32)` as-is to contain churn. ~7 signatures,
   2 production call sites, moderate test updates.
2. **Monitor-origin point**: same five `record.rs` functions take bare
   `screen_x: i32, screen_y: i32` — bundle into a point type (shared with
   Phase C's `Physical` types if C lands; a simple `struct` first).
3. **`should_run_cli_mode` takes `&AppModeRequest`** instead of six positional
   bools (the struct already exists; `decide_app_mode` currently re-explodes
   it positionally). One fn + mirror in `main.rs`.
4. **`FormatSource::{Explicit, Inferred}` enum** replacing the
   `format_explicit: bool` threaded through `effective_cli_format`,
   `app_mode_request`, `decide_app_mode_with_format_explicit`,
   `AppModeRequest.format_explicit`.

## Phase B — domain identifiers

5. **`ScreenIndex(usize)`** in `capture`: fixes the count-vs-index footgun in
   `resolve_screen_index(screen_count, requested)`; ~8 pub signatures + MCP
   JSON boundary (`mcp.rs` parses the index) + test updates.
6. **`ScaleFactor(f32)`** with validating constructor (finite, positive) —
   centralizes the ad-hoc `is_finite() && > 0.0` guard in
   `scaled_physical_dimension`; used by `virtual_desktop_scale_factor`,
   `logical_to_physical`, layout-input tuples.
7. **`WindowId(u64)`** (opaque HWND) through `window.rs` capture fns, `Args`,
   MCP boundary. Low mixup risk (only domain `u64`) — do last in B.

## Phase C — Physical/Logical coordinates (staged)

8. **`Physical(i32)` / `Logical(i32)` (+ size variants)** in `selection.rs`.
   Highest correctness payoff: today the distinction lives only in parameter
   names (`logical_to_physical`, `monitor_scale_ratio(logical_size,
   physical_size)`, 8 interleaved `logical_*`/`physical_*` fields in each
   layout struct — a transposition compiles silently). Very large churn
   (~15 fns, both layout structs, six `benchmark_*` tuple wrappers, ~4.7k
   test lines). Strategy: leaf helpers first (`logical_to_physical`,
   `monitor_scale_ratio`, `scaled_physical_dimension`), structs last; keep the
   `benchmark_*` tuple ABI as an explicit compatibility shim so benches don't
   churn. Each step lands green before the next starts.

## Explicit non-goals ("not worth it" from the survey)

- `Result<_, String>` → error enums (documented project convention).
- `Timestamp` newtype for `generate_timestamp()` (filename-only use).
- MCP JSON wire strings (protocol boundary; already funneled into `Format`
  at the edge). Optional micro-cleanup: derive the `["png","jpg","bmp"]`
  schema literal from `Format::ALL`.
- Collapsing `main::CliFormat` into `capture::Format` would couple `capture`
  to clap — keep the boundary adapter.

## Risks

- Pub-signature changes ripple into `benches/benchmarks.rs`,
  `tests/bench_functions.rs`, `tests/bdd.rs` (~182 references to the hottest
  fns) — mechanical but large; phases land as separate commits, each green.
- MCP JSON boundary (screen index, window id) must keep accepting raw JSON
  numbers — newtypes wrap after parsing, wire format unchanged.

## Rollback Strategy

Each phase is one commit on a green tree; rollback is per-phase `git revert`.
