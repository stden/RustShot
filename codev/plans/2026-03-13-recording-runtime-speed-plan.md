# Recording Runtime Speed Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reduce recording CPU usage and lag by removing unnecessary per-frame allocations and full-screen crop work from the Windows recording loop.

**Architecture:** Keep the Media Foundation writer flow intact, but change the hot loop to capture only the selected region and convert that region directly into the BGRA frame buffer expected by the encoder. Add a small pure helper for padded RGBA-to-BGRA conversion so the performance-sensitive behavior is testable without real screen hardware.

**Tech Stack:** Rust, `screenshots`, `image`, Windows Media Foundation via `windows`

### Task 1: Define the optimized frame conversion contract

**Files:**
- Modify: `src/record.rs`
- Test: `src/record.rs`

**Step 1: Write the failing test**
- Add a test for converting an already-even RGBA frame directly into BGRA bottom-up output.
- Add a test for converting an odd-sized RGBA frame into an even-sized padded BGRA output buffer.

**Step 2: Run test to verify it fails**
- Run targeted `cargo test` commands for the new conversion tests.
- Expected: compile failure or assertion failure because the helper does not exist yet.

**Step 3: Write minimal implementation**
- Add a pure helper that writes a selected RGBA frame into a caller-provided BGRA bottom-up buffer with zero-filled padding.

**Step 4: Run test to verify it passes**
- Re-run the targeted conversion tests.

### Task 2: Remove crop/pad allocations from the recording loop

**Files:**
- Modify: `src/record.rs`
- Test: `src/record.rs`

**Step 1: Write the failing test**
- Add a test for validating the destination frame buffer length expected by the helper.

**Step 2: Run test to verify it fails**
- Run the targeted helper test and confirm it fails for the missing validation behavior.

**Step 3: Write minimal implementation**
- Preallocate the BGRA frame buffer once before the recording loop.
- On Windows, capture only the requested region with `capture_area_ignore_area_check`.
- Reuse the preallocated BGRA buffer for every frame instead of allocating cropped and padded `image::RgbaImage` values.

**Step 4: Run test to verify it passes**
- Re-run the targeted `record` tests.

### Task 3: Verify behavior

**Files:**
- Modify: `benches/benchmarks.rs` only if a small benchmark update is needed.

**Step 1: Run focused tests**
- Run targeted `cargo test` filters for the new `record` helper tests.

**Step 2: Run formatting**
- Run: `cargo fmt --all`

**Step 3: Run lint**
- Run: `cargo clippy --all-targets -- -D warnings`

**Step 4: Run full tests**
- Run: `cargo test`
