# Test And Benchmark Coverage Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand deterministic test and benchmark coverage to the remaining pure helper logic without changing RustShot behavior.

**Architecture:** Keep the existing binary behavior intact and avoid broad refactors. Extract only the pure CLI-policy helpers from `src/main.rs` into a small shared library module so both unit/integration tests and Criterion benches can exercise them. Reuse the existing benchmark helper exports already present in `capture.rs`, and add a small tray benchmark hook for icon pixel generation.

**Tech Stack:** Rust 2021, Criterion, existing unit/integration tests, `cargo test`, `cargo bench --no-run`, `cargo clippy`

### Task 1: Lock The Target Surface

**Files:**
- Modify: `/srv/RustShot/docs/plans/2026-03-15-test-benchmark-coverage.md`
- Inspect: `/srv/RustShot/src/main.rs`
- Inspect: `/srv/RustShot/src/capture.rs`
- Inspect: `/srv/RustShot/src/tray.rs`
- Inspect: `/srv/RustShot/benches/benchmarks.rs`
- Inspect: `/srv/RustShot/tests/bench_functions.rs`

**Step 1: Confirm the deterministic gaps**

Check the current public/helper surfaces and benchmark harness coverage. Focus on:
- pure CLI helper logic in `src/main.rs`
- existing but unused benchmark helper exports in `src/capture.rs`
- deterministic tray icon generation in `src/tray.rs`

**Step 2: Keep scope bounded**

Do not attempt to benchmark GUI event loops, tray creation, hotkey registration, or platform-specific capture/recording backends that require OS services.

### Task 2: Extract Shared CLI Logic

**Files:**
- Create: `/srv/RustShot/src/cli.rs`
- Modify: `/srv/RustShot/src/lib.rs`
- Modify: `/srv/RustShot/src/main.rs`

**Step 1: Move pure helper logic into `src/cli.rs`**

Extract the pure, deterministic functions from `src/main.rs` into a shared module:
- screen-list formatting
- explicit format flag detection
- output extension parsing and format resolution
- CLI mode decision logic
- selection stderr formatting
- child-process running predicate

Keep signatures simple and benchmark-friendly.

**Step 2: Reuse the shared helpers from `src/main.rs`**

Update `src/main.rs` to call the shared implementations instead of maintaining duplicate local logic.

**Step 3: Preserve behavior**

Do not change user-visible strings, precedence rules, or flag semantics.

### Task 3: Add Tests For Shared Helpers And Benchmark Hooks

**Files:**
- Modify: `/srv/RustShot/tests/bench_functions.rs`
- Modify: `/srv/RustShot/src/tray.rs`

**Step 1: Add integration tests for the extracted CLI helpers**

Add focused tests for:
- format flag detection edge cases
- output extension parsing and unsupported extension reporting
- effective format precedence
- CLI mode decision rules
- formatted screen-list output
- selection stderr suppression rules
- child process running predicate

**Step 2: Add tests for any new tray benchmark hook**

If a tray icon pixel-generation wrapper is added for benchmarking, add a small regression test to verify it preserves the expected buffer shape.

### Task 4: Expand Criterion Coverage

**Files:**
- Modify: `/srv/RustShot/benches/benchmarks.rs`
- Modify: `/srv/RustShot/src/tray.rs` (only if needed for access)

**Step 1: Benchmark the newly shared CLI helpers**

Add Criterion groups for the pure CLI logic:
- format-flag detection
- output extension / format resolution
- CLI mode decision
- screen-list formatting
- selection stderr formatting
- child-process running predicate

**Step 2: Benchmark existing unused capture helpers**

Add Criterion coverage for:
- `benchmark_save_image_optimized`
- `benchmark_ensure_parent_directory`

Use temp directories and synthetic images so the benches stay deterministic.

**Step 3: Benchmark tray icon pixel generation**

Add a small Criterion bench for deterministic icon buffer generation.

### Task 5: Verify End-To-End

**Files:**
- Modify: `/srv/RustShot/src/cli.rs`
- Modify: `/srv/RustShot/src/lib.rs`
- Modify: `/srv/RustShot/src/main.rs`
- Modify: `/srv/RustShot/src/tray.rs`
- Modify: `/srv/RustShot/tests/bench_functions.rs`
- Modify: `/srv/RustShot/benches/benchmarks.rs`

**Step 1: Run formatting**

Run: `cargo fmt`

**Step 2: Run focused tests while iterating**

Use targeted `cargo test <pattern>` runs for the new CLI helper and benchmark-surface tests.

**Step 3: Run full verification**

Run:
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo bench --no-run`

**Step 4: Review the diff**

Confirm the change stays behavior-preserving and only expands deterministic coverage and benchmark surface.
