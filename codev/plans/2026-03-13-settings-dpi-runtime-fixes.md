# Settings, DPI, and Recording Runtime Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make persisted settings authoritative for runtime defaults, fix Windows auto-start synchronization, correct mixed-DPI screen layout mapping, and reject invalid multi-monitor recording selections.

**Architecture:** Keep the existing lightweight settings file flow, but move runtime default resolution into small helpers in `main`, `capture`, `record`, and `selection`. For screen selection, replace the single global scale assumption with per-screen physical bounds and derive recording regions from those physical rectangles so the recorder only accepts a single-monitor region.

**Tech Stack:** Rust, `screenshots`, `display-info`, `winit`, `softbuffer`, Win32 registry and monitor APIs via `windows`.

### Task 1: Runtime settings contract

**Files:**
- Modify: `src/settings.rs`
- Test: `src/settings.rs`

**Step 1: Write failing tests**
- Add tests for invalid `default_format` normalization to `png`.
- Add tests for zero `recording_fps` normalization to `30`.

**Step 2: Run tests to verify they fail**
- Run targeted `cargo test` commands for the new settings tests.

**Step 3: Write minimal implementation**
- Add settings normalization helpers.
- Ensure `load_settings()` returns normalized settings.

**Step 4: Run tests to verify they pass**
- Re-run the targeted settings tests.

### Task 2: Wire settings into capture, CLI, recording, and autorun

**Files:**
- Modify: `src/main.rs`
- Modify: `src/capture.rs`
- Modify: `src/record.rs`
- Test: `src/main.rs`
- Test: `src/capture.rs`
- Test: `src/record.rs`

**Step 1: Write failing tests**
- Add tests for CLI format precedence: explicit CLI format overrides settings, absent CLI format uses settings.
- Add tests for screenshot and recording output path helpers using configured directories.
- Add tests for autorun synchronization helper choosing set vs delete behavior.

**Step 2: Run tests to verify they fail**
- Run the targeted tests in `main`, `capture`, and `record`.

**Step 3: Write minimal implementation**
- Introduce configured-path helpers in `capture` and `record`.
- Resolve effective CLI and tray defaults from loaded settings.
- Replace hardcoded `30` FPS with settings-backed FPS.
- Replace one-way autorun setup with sync logic that can remove the registry value.

**Step 4: Run tests to verify they pass**
- Re-run the targeted module tests.

### Task 3: Mixed-DPI layout and recording-region validation

**Files:**
- Modify: `src/selection.rs`
- Modify: `src/record.rs`
- Test: `src/selection.rs`
- Test: `src/record.rs`

**Step 1: Write failing tests**
- Add tests for physical screen layout placement without a single shared scale factor.
- Add tests that recording selection must be fully contained inside one screen.
- Add tests for record-side validation of negative or out-of-bounds regions.

**Step 2: Run tests to verify they fail**
- Run the targeted selection and record tests.

**Step 3: Write minimal implementation**
- Build virtual desktop composition from per-screen physical bounds.
- Use physical screen rectangles to choose the recording target and compute screen-local coordinates.
- Reject cross-monitor recording regions and validate crop bounds before recording.

**Step 4: Run tests to verify they pass**
- Re-run the targeted selection and record tests.

### Task 4: Full verification

**Files:**
- Modify as needed from previous tasks only.

**Step 1: Run formatting**
- Run: `cargo fmt`

**Step 2: Run lint**
- Run: `cargo clippy --all-targets -- -D warnings`

**Step 3: Run full tests**
- Run: `cargo test`

**Step 4: Confirm behavior**
- Verify all commands exit successfully and no newly introduced warnings fail CI.
