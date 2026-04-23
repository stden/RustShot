# Review Correctness Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix correctness bugs found during code review in small pure helper functions without changing broader RustShot behavior.

**Architecture:** Keep the fixes local to the helpers in `settings`, `selection`, and `record`. Add focused regression tests first, then implement the smallest code changes needed to make those tests pass. Avoid touching platform integration paths or refactoring unrelated logic.

**Tech Stack:** Rust 2021, existing unit tests, `cargo test`, `cargo fmt`, `cargo clippy`

### Task 1: Reject malformed numeric suffixes in settings parsing

**Files:**
- Modify: `/srv/RustShot/src/settings.rs`
- Test: `/srv/RustShot/src/settings.rs`

**Step 1: Write the failing test**

Add tests proving `extract_json_number` rejects malformed suffixes such as `30fps` or `1e2` instead of silently parsing a digit prefix.

**Step 2: Run test to verify it fails**

Run: `cargo test extract_json_number_rejects_trailing_identifier_suffix extract_json_number_rejects_exponent_like_suffix`

Expected: FAIL because the current parser accepts the digit prefix.

**Step 3: Write minimal implementation**

Enforce a JSON token boundary after the numeric literal, matching the boolean extractor’s boundary rules.

**Step 4: Run test to verify it passes**

Run the same targeted `cargo test` command and confirm both tests pass.

### Task 2: Remove `round_to_even` overflow panic

**Files:**
- Modify: `/srv/RustShot/src/record.rs`
- Test: `/srv/RustShot/src/record.rs`
- Test: `/srv/RustShot/tests/bench_functions.rs`

**Step 1: Write the failing test**

Replace the current debug-panic expectation with a regression test proving `round_to_even(u32::MAX)` does not panic and returns a stable even value.

**Step 2: Run test to verify it fails**

Run: `cargo test round_to_even_u32_max_returns_u32_max`

Expected: FAIL because the current implementation overflows in debug builds.

**Step 3: Write minimal implementation**

Use saturating arithmetic so the helper stays branch-light but cannot overflow on maximum input.

**Step 4: Run test to verify it passes**

Run the same targeted `cargo test` command and confirm the regression is fixed.

### Task 3: Make screen hit-testing overflow-safe

**Files:**
- Modify: `/srv/RustShot/src/selection.rs`
- Test: `/srv/RustShot/src/selection.rs`

**Step 1: Write the failing test**

Add a test that calls `find_screen_index_at_position` with an `i32::MAX` monitor origin and verifies the right/bottom boundary logic still works without overflowing intermediate arithmetic.

**Step 2: Run test to verify it fails**

Run: `cargo test find_screen_index_at_position_large_origin_no_overflow`

Expected: FAIL because the current `i32` boundary math wraps.

**Step 3: Write minimal implementation**

Switch the boundary comparisons to the module’s existing `i64` helper arithmetic.

**Step 4: Run test to verify it passes**

Run the same targeted `cargo test` command and confirm the overflow case is handled.

### Task 4: Verify the full repository state

**Files:**
- Modify only the files above.

**Step 1: Run formatting**

Run: `cargo fmt`

**Step 2: Run targeted verification while iterating**

Run the new targeted tests after each fix.

**Step 3: Run full verification**

Run:
- `cargo test`
- `cargo clippy --all-targets`

**Step 4: Review the diff**

Confirm the patch is limited to the reviewed helper functions and their regression tests.
