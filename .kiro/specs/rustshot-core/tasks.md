# Implementation Plan: RustShot Core

## Overview

This plan covers the complete implementation of RustShot's core feature set across all modules. Since the codebase already exists (~5,700 lines, 433 tests), tasks focus on verifying, hardening, and extending coverage of the existing implementation against the formal requirements. Each task builds incrementally, starting with foundational data models and utilities, then layering capture, selection, recording, tray, and CLI on top.

## Tasks

- [x] 1. Capture engine — format, screen resolution, and file output
  - [x] 1.1 Implement `Format` enum with PNG/JPG/BMP variants, `extension()`, `to_image_format()`, and `Default` (PNG)
    - Ensure `Format::extension()` returns lowercase strings without leading dot
    - Ensure `Format::to_image_format()` maps to distinct `image` crate variants
    - Ensure `Format::default()` returns `Png`
    - _Requirements: 3.1, 3.2, 3.4, 3.5, 3.6_

  - [x] 1.2 Implement `resolve_screen_index`, `default_screen_index`, and screen validation
    - `resolve_screen_index(None, count)` returns 0 when count > 0
    - `resolve_screen_index(Some(i), count)` returns `Ok(i)` when i < count, `Err` otherwise
    - Error message contains the requested index and valid range "0-N"
    - Zero count returns "No screens detected" error
    - `default_screen_index` prefers primary monitor, falls back to index 0
    - _Requirements: 1.4, 1.7, 1.8, 4.1, 4.2, 4.3_

  - [x] 1.3 Implement `resolve_output_path`, `resolve_output_path_in_dir`, and `ensure_parent_directory`
    - Explicit path is returned verbatim without timestamp or extension change
    - Default path uses `screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>` pattern
    - Default directory is `Pictures/RustShot/`
    - `ensure_parent_directory` creates missing parents recursively and is idempotent
    - Generated filenames contain only alphanumeric characters, underscores, and dots
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [x] 1.4 Implement `save_image_optimized` and `bgra_to_rgba`
    - PNG and BMP save RGBA data directly
    - JPG converts RGBA to RGB before encoding
    - `bgra_to_rgba` performs in-place channel swap on 4-byte aligned chunks
    - Partial trailing bytes (< 4) are left untouched
    - _Requirements: 3.1, 3.3_

  - [x] 1.5 Write unit tests for capture engine
    - Test `Format` enum: extension, to_image_format, default, clone, debug
    - Test `resolve_screen_index` boundary conditions: 0 screens, 1 screen, N screens, out-of-range
    - Test `resolve_output_path` with explicit path, default path, all three formats
    - Test `ensure_parent_directory` idempotency and nested creation
    - Test `save_image_optimized` for all formats including 1x1 pixel edge case
    - Test `bgra_to_rgba` for empty, single pixel, multi-pixel, and trailing bytes
    - Test `capture_screenshot_with` closure injection: success, capture error, save error, directory error
    - _Requirements: 1.7, 1.8, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [-] 2. Checkpoint — Ensure all capture engine tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 3. Selection overlay — region selection, multi-monitor, and DPI-aware layout
  - [~] 3.1 Implement `Region` struct and `build_region_from_drag`
    - `Region` supports negative x/y coordinates (multi-monitor) with u32 width/height
    - `build_region_from_drag` normalizes reversed drag coordinates to positive width/height
    - Regions ≤ 5×5 pixels return `None`; regions ≥ 6×6 return `Some(Region)`
    - _Requirements: 5.6, 5.7, 5.9, 5.10_

  - [~] 3.2 Implement `find_screen_index_at_position` and `preferred_screen_index`
    - Hit-tests cursor against monitor bounds with exclusive right/bottom boundaries
    - Returns `None` when cursor is outside all monitors
    - Supports negative coordinate origins for multi-monitor setups
    - `preferred_screen_index` fallback chain: cursor position → primary monitor → first monitor
    - _Requirements: 4.4, 4.5, 4.6, 4.7_

  - [~] 3.3 Implement `build_physical_screen_layouts` and `normalize_physical_screen_layouts`
    - DPI-aware physical coordinate inference from logical screen geometry
    - Normalization shifts all layouts so minimum origin is (0, 0)
    - Handles mixed-DPI multi-monitor configurations
    - _Requirements: 4.7, 5.1, 5.10_

  - [~] 3.4 Implement `resolve_recording_selection` and `virtual_desktop_scale_factor`
    - Validates that recording region fits within a single monitor
    - Returns local physical coordinates relative to the containing monitor
    - `virtual_desktop_scale_factor` returns primary monitor's scale factor
    - _Requirements: 5.1, 5.5_

  - [~] 3.5 Write unit tests for selection overlay
    - Test `Region` struct: copy, clone, debug, negative coords, zero dimensions
    - Test `build_region_from_drag`: normal drag, reversed drag, minimum size boundary (5×5 vs 6×6), zero-size, negative coords
    - Test `find_screen_index_at_position`: inside monitor, outside all, boundary exclusivity, negative origins, gap between screens
    - Test `preferred_screen_index`: cursor hit, primary fallback, first fallback, empty input
    - Test `build_physical_screen_layouts`: single monitor, horizontal, vertical, mixed DPI, empty input
    - Test `normalize_physical_screen_layouts`: origin at (0,0), negative offset shift, empty input
    - Test `resolve_recording_selection`: within single monitor, crossing monitors, exact bounds, empty layouts
    - _Requirements: 4.4, 4.5, 4.6, 4.7, 5.1, 5.6, 5.7, 5.9, 5.10_

- [~] 4. Checkpoint — Ensure all selection overlay tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Recording engine — session management, validation, and pixel conversion
  - [~] 5.1 Implement `RecordingSession` start/stop lifecycle and validation
    - `validate_recording_start` rejects 0 FPS and > 240 FPS
    - `validate_recording_start` rejects non-Windows platforms
    - `RecordingSession::start` creates output path, spawns recording thread
    - `RecordingSession::stop` signals via stop flag file, joins thread, returns output path
    - _Requirements: 8.4, 8.5_

  - [~] 5.2 Implement `validate_capture_region`, `frame_duration_hns`, `round_to_even`, and `rgba_to_bgra_bottom_up`
    - `validate_capture_region` ensures region fits within frame, rejects negative coords and zero dimensions
    - `frame_duration_hns` converts FPS to hundred-nanosecond units (10,000,000 HNS/sec)
    - `round_to_even` rounds up to even values (H.264 encoder requirement)
    - `rgba_to_bgra_bottom_up` performs channel swap and vertical row flip
    - _Requirements: 5.5_

  - [~] 5.3 Implement recording output path generation
    - Recording files saved to `Pictures/RustShot/Recordings/` directory
    - Filenames use `recording_YYYYMMDD_HHMMSS_NNNNNNNNN.mp4` pattern
    - `get_recording_path_in_dir` supports custom directory
    - _Requirements: 2.1, 2.2_

  - [~] 5.4 Write unit tests for recording engine
    - Test `validate_recording_start`: 0 FPS, valid FPS, > 240 FPS, non-Windows rejection
    - Test `validate_capture_region`: exact bounds, overflow, negative coords, zero dimensions
    - Test `frame_duration_hns`: 1 FPS (10M), 30 FPS (333333), 60 FPS (166666), 0 FPS (0), monotonic decrease
    - Test `round_to_even`: even stays, odd rounds up, zero stays, result always even, differs by at most 1
    - Test `rgba_to_bgra_bottom_up`: channel swap, row flip, single pixel, multi-row, empty input, alpha preservation
    - Test `pack_u32_pair`: roundtrip, zero/max values, 1920×1080 encoding
    - Test `RecordingSession`: output path accessor, stop lifecycle, error propagation
    - _Requirements: 5.5, 8.4, 8.5_

- [~] 6. Checkpoint — Ensure all recording engine tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 7. Settings store — JSON parsing, formatting, and normalization
  - [~] 7.1 Implement `Settings` struct with defaults and `parse_settings_json`
    - Default: `default_format = "png"`, `recording_fps = 30`, `auto_start = true`
    - Default directories: `Pictures/RustShot/` and `Pictures/RustShot/Recordings/`
    - Hand-rolled JSON parser extracts string, number, and bool values
    - Missing or invalid fields fall back to defaults
    - _Requirements: 1.9, 3.2_

  - [~] 7.2 Implement `format_settings_json` and escape/unescape utilities
    - Produces valid JSON with all five settings keys
    - `escape_json_string` escapes `\`, `"`, `\n`, `\r`, `\t`
    - `unescape_json_string` reverses escape sequences
    - Roundtrip: `parse(format(settings)) == settings`
    - _Requirements: 3.2_

  - [~] 7.3 Implement `normalize_default_format` and `normalize_recording_fps`
    - `normalize_default_format` validates "png", "jpg", "bmp"; falls back to "png"
    - `normalize_recording_fps` converts 0 to 30; passes through non-zero values
    - _Requirements: 3.2_

  - [~] 7.4 Implement `load_settings`, `save_settings`, and `generate_timestamp`
    - `load_settings` reads from settings file, returns defaults if file missing
    - `save_settings` writes formatted JSON to settings path
    - `generate_timestamp` produces `YYYYMMDD_HHMMSS_NNNNNNNNN` format
    - _Requirements: 2.2_

  - [~] 7.5 Write unit tests for settings store
    - Test `Settings` defaults: format, fps, auto_start, directories
    - Test `parse_settings_json`: empty input, partial keys, garbage input, type mismatches, duplicate keys
    - Test `format_settings_json`: all keys present, valid JSON structure, backslash escaping
    - Test roundtrip: `parse(format(s)) == s` for various configurations
    - Test `escape_json_string` / `unescape_json_string`: all special chars, Windows paths, roundtrip
    - Test `extract_json_string/number/bool`: valid values, missing keys, type mismatches, boundary values
    - Test `normalize_default_format`: valid formats, unknown strings, empty, case sensitivity
    - Test `normalize_recording_fps`: 0 → 30, non-zero passthrough, boundary values
    - Test `generate_timestamp`: length, format, uniqueness across calls
    - _Requirements: 1.9, 2.2, 3.2_

- [~] 8. Checkpoint — Ensure all settings store tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Tray manager — icon generation and menu creation
  - [~] 9.1 Implement `build_default_icon_pixels` and `create_tray`
    - Generate 32×32 RGBA camera glyph with blue body RGB(41, 128, 185) and white lens
    - Every pixel alpha is either 0 (transparent) or 255 (opaque)
    - Icon has horizontal symmetry
    - Context menu with 7 items: Capture, Capture Window, Open Folder, Settings, Start Recording, Stop Recording, Exit
    - All menu item IDs are unique and non-empty
    - _Requirements: 7.1, 7.2, 7.7, 7.8, 7.9_

  - [~] 9.2 Write unit tests for tray manager
    - Test icon dimensions: 32×32, total 1024 pixels, RGBA buffer length = 4096
    - Test icon colors: center is white (lens), body pixels are blue, corners are transparent
    - Test alpha values: every pixel is 0 or 255 (no partial alpha)
    - Test icon symmetry: horizontal opacity symmetry
    - Test menu constants: all 7 IDs are unique, non-empty, contain no whitespace
    - _Requirements: 7.1, 7.2, 7.7, 7.8, 7.9_

- [ ] 10. CLI handler — format resolution and mode detection
  - [~] 10.1 Implement `effective_cli_format`, `cli_format_flag_present_in`, and format resolution chain
    - Priority: explicit `--format` flag > output file extension > settings default
    - `cli_format_flag_present_in` scans argv for `--format`/`-f`, skipping values of `--output`/`--screen`
    - `format_from_extension` maps file extensions case-insensitively (png, jpg, jpeg, bmp)
    - _Requirements: 1.3, 1.9, 1.10_

  - [~] 10.2 Implement `should_run_cli_mode` and `selection_stderr_message`
    - CLI mode triggers when any of: `--cli`, `--list`, `--output`, `--screen`, `--format`, `--window` is set
    - `selection_stderr_message` returns `None` for "Selection cancelled", `Some` for other errors
    - _Requirements: 1.1, 6.2, 6.5, 6.6_

  - [~] 10.3 Implement `format_screen_list_output` and `write_error_line`
    - Screen list formatted with indentation and index numbers
    - `write_error_line` appends newline to error messages
    - _Requirements: 1.5_

  - [~] 10.4 Write unit tests for CLI handler
    - Test `effective_cli_format`: explicit wins, extension fallback, settings fallback, unknown extension
    - Test `cli_format_flag_present_in`: long form, short form, equals form, skipping output/screen values
    - Test `format_from_extension`: png, jpg, jpeg, bmp, case insensitivity, unknown
    - Test `should_run_cli_mode`: each flag individually, combined flags, select-only is false
    - Test `selection_stderr_message`: cancelled returns None, other errors return Some
    - Test `format_screen_list_output`: empty, single, multiple entries, indentation
    - _Requirements: 1.1, 1.3, 1.5, 1.9, 1.10, 6.2, 6.5, 6.6_

- [~] 11. Checkpoint — Ensure all CLI handler tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 12. Application lifecycle — mode routing, clipboard integration, and tray event loop
  - [~] 12.1 Implement `decide_app_mode` and CLI argument routing
    - `--select` / `--select-record` → Selection mode
    - `--cli`, `--list`, `--output`, `--screen`, `--format`, `--window` → CLI mode
    - No flags → Tray mode (default)
    - Selection mode takes priority over CLI mode when both flags are set
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

  - [~] 12.2 Implement `run_cli_with_capture` and `run_cli_with_capture_using_settings`
    - Closure-injected capture function receives output path, screen index, and format
    - `--list` flag prints screen list and exits without capturing
    - `--window` flag routes to active window capture handler
    - Unsupported output extension without explicit `--format` returns error
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10_

  - [~] 12.3 Implement `run_selection_with_clipboard` for clipboard integration
    - Successful selection copies cropped RGBA image to system clipboard
    - Cancelled selection returns "Selection cancelled" error without modifying clipboard
    - Non-cancellation errors propagated with "Selection error: " prefix
    - Clipboard errors propagated directly
    - _Requirements: 6.1, 6.2, 6.3, 6.4_

  - [~] 12.4 Implement `open_screenshots_folder_with_handlers` and `sync_autorun`
    - Creates screenshots directory if missing, then opens in system file manager
    - `sync_autorun` enables/disables Windows registry auto-start based on settings
    - _Requirements: 7.5_

  - [~] 12.5 Write unit tests for application lifecycle
    - Test `decide_app_mode`: all flag combinations, priority rules (select > cli > tray)
    - Test `run_cli_with_capture`: output/format/screen forwarding, list mode, window mode, error propagation
    - Test `run_cli_with_capture_using_settings`: settings format fallback, explicit format override, unsupported extension handling
    - Test `run_selection_with_clipboard`: success copies image, cancelled skips clipboard, error wrapping
    - Test `open_screenshots_folder_with_handlers`: create success opens folder, create failure skips open
    - Test `sync_autorun_with_handlers`: enable/disable mutual exclusivity
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 6.1, 6.2, 6.3, 6.4, 7.5_

- [ ] 13. Tray mode — hotkey registration and child process spawning
  - [~] 13.1 Implement `RustShotApp::new` with hotkey registration and tray event handling
    - Register PrintScreen as global hotkey for screenshot capture
    - Register Ctrl+Shift+R as global hotkey for toggling screen recording
    - `register_hotkey_with` closure-injected for testability
    - _Requirements: 8.1, 8.2_

  - [~] 13.2 Implement tray menu action handlers and child process management
    - "Capture" spawns `--select` child process for region selection
    - "Capture Window" captures active window (Windows only)
    - "Open Folder" opens screenshots directory in file manager
    - "Start/Stop Recording" toggles recording via `--select-record` child process
    - "Exit" shuts down cleanly
    - PrintScreen hotkey triggers same flow as "Capture" menu action
    - Ctrl+Shift+R hotkey triggers same flow as recording toggle
    - _Requirements: 7.3, 7.4, 7.5, 7.6, 8.3, 8.4, 8.5_

  - [~] 13.3 Write unit tests for tray mode
    - Test `register_hotkey_with`: success returns ID, error wraps message
    - Test hotkey constants: PrintScreen and Ctrl+Shift+R are distinct
    - Test `child_process_is_running`: Ok(None) → running, Ok(Some) → exited, Err → not running
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [~] 14. Final checkpoint — Ensure all tests pass
  - Run `cargo test` to verify all tests pass across all modules.
  - Run `cargo clippy` to verify no lint warnings.
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- The codebase already has ~433 tests; tasks focus on verifying coverage against formal requirements and filling gaps
- Unit tests validate specific examples and edge cases using closure-based dependency injection
- All code is Rust; no pseudocode translation needed
