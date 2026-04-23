# Review: RustShot Screenshot and Recording Utility -- Uncommitted Changes

## Summary

This review covers all uncommitted changes in the RustShot repository relative
to the single existing commit (`14d4d8b`). The changes touch 8 tracked files
(+302/-47 lines), add 3 new files (438 lines), and introduce 12 new Codev
planning artifacts (specs, plans, roadmap). The code changes primarily address
Phase 1 of plan 0001 (core capture and recording hardening), introduce the
initial `settings.rs` module (Phase 2 of plan 0003), restructure CI workflows
(related to plan 0005), and add MCP scaffolding documentation (related to
plan 0004). No implementation phases have been formally committed with `[Phase`
tags; all work is staged as uncommitted modifications.

## Spec Compliance

### Spec 0001: RustShot Screenshot and Recording Utility

- [x] Screenshot capture works via PrintScreen hotkey (unchanged, already passing)
- [x] Region selection with visual feedback -- crosshair + border (unchanged)
- [x] Screenshots copied to clipboard (unchanged)
- [x] CLI mode with format/screen/output options (unchanged, routing tests present)
- [x] Multi-monitor support with virtual desktop compositing (improved: `virtual_desktop_scale_factor` now prefers primary monitor, with fallback)
- [x] Screen recording via Ctrl+Shift+R (unchanged core, improved image conversion layer)
- [x] System tray with context menu (modified: Settings item added, recording label consolidated to single toggle entry)
- [x] Tests passing: 433 passing (up from 400 baseline), 33 new tests added
- [x] CI pipeline on Windows and Linux (restructured into 4 jobs: fmt, clippy, test-windows, test-linux)
- [ ] All tests pass with >90% coverage on testable logic -- Not measured (no coverage tooling configured)
- [ ] Performance benchmarks: capture <100ms, overlay render <16ms -- Not measured

### Spec 0003: Settings and Runtime Configuration

- [x] RustShot loads settings on startup and falls back to safe defaults when file is missing or malformed (`load_settings()` returns `Settings::default()` on any error)
- [ ] Persisted settings apply to screenshot and recording defaults when explicit CLI options are absent -- Not met. `load_settings()` exists but is not yet called from `main.rs` startup or wired into capture/record default resolution.
- [ ] CLI arguments override persisted settings deterministically -- Not met. No effective-settings resolver exists yet.
- [ ] Windows auto-start can be both enabled and disabled through the persisted settings model -- Not met. `auto_start` field exists in `Settings` but `setup_autorun()` in `main.rs` still unconditionally registers on startup without consulting settings.
- [x] Settings round-trip tests cover complete, partial, and malformed JSON input (tests present: `parse_settings_json_roundtrip`, `parse_settings_json_partial_keys`, `parse_settings_json_empty_returns_defaults`)
- [ ] Documentation explains where settings live and which values are platform-specific -- Not met. No user-facing documentation about settings exists yet.

### Spec 0004: MCP Server Integration

- [ ] `rustshot --mcp` starts a server mode without tray or hotkey initialization -- Not met. No `--mcp` flag exists in the binary.
- [ ] MCP clients can initialize, list tools, and execute supported tools -- Not met. No `src/mcp.rs` module exists.
- [x] Documentation and `.mcp.json` examples describe the intended interface (`mcp-server/README.md` and `.mcp.json` present, though README correctly states "Planned -- not yet implemented")
- [ ] Tests cover request parsing, dispatch, and error mapping -- Not met.

### Spec 0005: Release and Distribution Hardening

- [x] Workflow artifact paths are updated: portable exe uploaded as `rustshot-windows-x64-portable`, installer uploaded as `rustshot-windows-x64-installer`
- [x] Workflow includes `Build installer` step via `iscc installer.iss`
- [x] GitHub Release step attaches both `rustshot.exe` and `RustShotSetup.exe`
- [ ] The installer launches and installs a working RustShot build on Windows -- Not validated (requires manual testing)
- [ ] Release docs match actual artifact names and installation flow -- Not validated
- [ ] A release smoke checklist exists -- Not created

## Deviations from Plan

- **Plan 0001, Phase 1 (Core Hardening)**: Partially complete. Default monitor selection (`default_screen_index`), scale factor resolution (`virtual_desktop_scale_factor`), and tray recording label consolidation are implemented. Documentation and validation closure (Phase 4) has not been started.
- **Plan 0001, Phase 3 (Recording)**: Image conversion was changed to go through raw bytes (`from_raw`) to bridge the `screenshots` crate's image version with the project's `image 0.25` upgrade. This was a practical necessity from the dependency bump, not planned explicitly.
- **Plan 0003 (Settings)**: Only Phase 1 (contract and validation) is partially done. The `Settings` struct, JSON parser, and tray menu wiring exist, but runtime default resolution (Phase 2) and auto-start application (Phase 3) are not implemented.
- **Plan 0004 (MCP)**: Only documentation scaffolding exists. No implementation phases have started.
- **Plan 0005 (Release Hardening)**: CI workflow restructured and release workflow updated, but no tagged validation has been performed.
- **Dependency bump**: `image` crate upgraded from `0.24` to `0.25`. This is not mentioned in any plan but affects `capture.rs` and `record.rs` (image conversion via `from_raw` instead of direct use of `screenshots` return type).

## Key Metrics

- **Commits**: 1 existing (`14d4d8b`), 0 new commits for this work (all changes are uncommitted)
- **Tests**: 433 passing (400 existing + 33 new)
- **Files created**:
  - `src/settings.rs` (376 lines, 25 tests)
  - `.mcp.json` (9 lines)
  - `mcp-server/README.md` (53 lines)
  - `codev/specs/0003-settings-and-runtime-configuration.md` (223 lines)
  - `codev/specs/0004-mcp-server-integration.md` (219 lines)
  - `codev/specs/0005-release-and-distribution-hardening.md` (207 lines)
  - `codev/plans/0001-rustshot-screenshot-utility.md` (332 lines)
  - `codev/plans/0003-settings-and-runtime-configuration.md` (296 lines)
  - `codev/plans/0004-mcp-server-integration.md` (303 lines)
  - `codev/plans/0005-release-and-distribution-hardening.md` (304 lines)
  - `codev/plans/001-feature-roadmap.md` (456 lines)
- **Files deleted**: None
- **Net LOC impact**: +302/-47 in tracked source files, +438 in new source/config files, +2340 in Codev planning artifacts

## Timelog

Not available. All changes are uncommitted against a single initial commit, so no
commit timeline exists. The initial commit (`14d4d8b`) predates these changes.

### Autonomous Operation

Not applicable for this review. The changes appear to be the result of a single
planning and implementation session without formal phase gates.

## Consultation Iteration Summary

No formal consultation rounds are recorded. The specs reference GPT-5 Codex
consultation, but no consultation artifacts (files, logs) were produced.

## Consultation Feedback

### Spec Phase

#### GPT-5 Codex (referenced in specs 0003, 0004, 0005)
- **Concern**: Specs were reviewed for desired state clarity, solution approach selection, and security constraints.
  - **Addressed**: Feedback incorporated directly into spec sections per the notes in each spec.

No separate consultation artifacts exist to evaluate independently.

## Lessons Learned

### What Went Well
- The `default_screen_index` and `virtual_desktop_scale_factor` helpers follow the project's established pattern of extracting pure testable functions from GUI-dependent code. Each has 3 focused unit tests.
- The `image` crate bump from 0.24 to 0.25 was handled cleanly with a `from_raw` bridge conversion, avoiding API breakage throughout the codebase.
- The settings module follows the project's `Result<_, String>` convention and provides good test coverage for its JSON parsing helpers (25 tests).
- CI was restructured into cleaner, more focused jobs (separate clippy, separate Windows/Linux test jobs).

### Challenges Encountered
- **Clippy cannot run in the current development environment** due to a toolchain target mismatch (host is `x86_64-pc-windows-gnu` but cached artifacts are for `x86_64-pc-windows-msvc`). This means local clippy validation is not possible without a clean rebuild targeting the correct triple. CI should catch clippy issues, but local development lacks this safety net.
- **image crate version mismatch**: The `screenshots` crate internally uses `image 0.24`, but the project bumped to `image 0.25`. This required converting captured images through raw bytes (`from_raw`) instead of using them directly. The conversion is correct but adds an allocation and copy per capture.

### What Would Be Done Differently
- Commit changes in logical phases rather than accumulating all changes as uncommitted modifications. The SPIR protocol expects phase-tagged commits for traceability.
- Run `cargo clippy` before accumulating this much change to catch issues early.
- Wire settings into runtime behavior before adding the tray menu item for settings, so the "Settings" menu action has observable user-facing effect beyond opening a file.

## Architecture Updates

The following changes are architecturally relevant but `codev/resources/arch.md`
does not exist yet:

- **New module**: `src/settings.rs` -- application configuration persistence (JSON file in OS config directory). Defines `Settings` struct, load/save helpers, manual JSON parser, and tray-triggered editor opening.
- **New module declaration**: `mod settings;` added to `src/main.rs`.
- **Tray menu change**: `MENU_SETTINGS` constant added to `src/tray.rs`. The separate `stop_record` menu item was removed; recording is now a single toggle-labeled entry (`"Start or Stop Recording (Ctrl+Shift+R)"`).
- **Dependency change**: `image` crate bumped from `0.24` to `0.25`. Capture and record paths now convert images via `from_raw` to bridge the version gap with the `screenshots` crate.
- **CI restructure**: 4 parallel jobs (fmt on Ubuntu, clippy on Windows, test on Windows, test on Linux) replace the previous 3-job layout. `actions/checkout` downgraded from `v5` to `v4`. `Swatinem/rust-cache` removed. `--all-features` flags removed from test commands.
- **Release workflow**: Adds installer build step (`iscc installer.iss`), uploads both portable and installer artifacts, and attaches both to GitHub Releases.
- **Win32 API fix**: `GetDC`/`ReleaseDC` calls in `capture_active_window_platform` changed from `Some(HWND::default())` to `None` for the desktop DC handle, which is the correct Windows API usage for obtaining the screen DC.
- **Primary monitor selection**: `default_screen_index` and `virtual_desktop_scale_factor` helpers added to prefer the primary monitor rather than always using index 0.
- **Coordinate conversion**: `logical_to_physical` helper added to `selection.rs` for proper DPI-aware coordinate mapping in the selection overlay.

**Recommendation**: Create `codev/resources/arch.md` documenting the current module map, application modes, and data flow. The spec 0001 already contains this information but it should be extracted into a standalone architecture resource.

## Lessons Learned Updates

`codev/resources/lessons-learned.md` does not exist yet. Recommendations for initial entries:

- **Dependency version gaps**: When a transitive dependency (like `screenshots`) pins a different major version of a shared crate (like `image`), bridge conversions through raw bytes are necessary. Document this pattern for future dependency bumps.
- **Phase commits**: Accumulating all changes as uncommitted modifications makes review harder and loses the phase-by-phase validation trail that SPIR is designed to provide.
- **Local toolchain validation**: Ensure `cargo clippy` runs locally before accumulating significant changes. Toolchain target mismatches can silently disable this check.

## Technical Debt

1. **Settings not wired to runtime behavior**: The `settings.rs` module exists and the tray has a "Settings" menu item, but settings are not loaded at startup or used to determine screenshot/recording defaults. The `open_settings_dialog()` function is connected to the tray menu, but editing the settings file has no observable effect on application behavior. This creates a misleading user experience.

2. **Auto-start ignores settings**: `setup_autorun()` in `main.rs` unconditionally registers the application at Windows startup without checking `settings.auto_start`. There is no `remove_autorun()` function to disable auto-start.

3. **Manual JSON parser fragility**: The `parse_settings_json` function uses string searching (`json.find`) rather than proper JSON parsing. Known issues:
   - A key like `"screenshots_dir_backup"` would match when searching for `"screenshots_dir"` due to prefix matching.
   - Escaped quotes inside string values would cause incorrect extraction.
   - The parser always returns `Some(settings)` (never `None`), making the `Option` return type misleading.

4. **Image version bridge allocation**: Every capture in both `capture_screenshot` and `wmf_record` converts the image through `from_raw`, which copies the entire pixel buffer. This is functionally correct but adds memory pressure proportional to screen resolution.

5. **CI downgrade without explanation**: `actions/checkout` was downgraded from `v5` to `v4` and `Swatinem/rust-cache` was removed without documented rationale. The cache removal may increase CI build times.

6. **`actions/checkout@v5` to `v4` downgrade**: This may have been intentional (v5 might not exist yet or caused issues), but the reason is not documented.

7. **`CARGO_INCREMENTAL` and `RUSTFLAGS` removed from CI**: The CI workflow previously set `CARGO_INCREMENTAL: 0` and `RUSTFLAGS: -D warnings`. Both were removed. The `-D warnings` enforcement is now only on clippy, not on the main build. Warnings in non-clippy builds will no longer fail CI.

8. **`--all-features` removed from test commands**: Tests now run without `--all-features`. If feature-gated code exists or is added later, those paths will not be tested in CI.

## Follow-up Items

1. **Wire settings into application startup** (Plan 0003, Phase 2): Load settings at startup and use them as defaults for screenshot directory, recording directory, default format, and recording FPS when CLI arguments are absent.

2. **Implement auto-start toggle** (Plan 0003, Phase 3): Add `remove_autorun()` and check `settings.auto_start` during startup instead of unconditionally registering.

3. **Implement MCP server** (Plan 0004, all phases): Create `src/mcp.rs` with `--mcp` CLI flag, JSON-RPC protocol handling, and tool dispatch. The scaffolding docs are in place.

4. **Tagged release validation** (Plan 0005, Phase 4): Perform an end-to-end release workflow test with a tag to verify both artifacts are produced and the installer works.

5. **Create architecture document**: Extract the module map and data flow from spec 0001 into `codev/resources/arch.md`.

6. **Create lessons-learned document**: Initialize `codev/resources/lessons-learned.md` with findings from this review.

7. **Consider replacing manual JSON parser with `serde`**: The current string-searching parser has known edge cases. If settings grow beyond the current 5 fields, `serde_json` would be more robust and maintainable.

8. **Commit changes in phase-tagged commits**: The current accumulated uncommitted state should be broken into logical commits following the SPIR phase structure before further work proceeds.

9. **Restore CI warning enforcement**: Consider re-adding `RUSTFLAGS: -D warnings` to the CI environment or adding `-D warnings` to the test build commands, not just clippy.

10. **Validate `image 0.25` compatibility**: Confirm that the `from_raw` bridge does not silently change pixel format semantics compared to direct `screenshots` crate usage with `image 0.24`.
