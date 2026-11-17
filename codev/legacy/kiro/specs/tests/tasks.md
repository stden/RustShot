# Implementation Plan: Comprehensive Test Suite for RustShot

## Overview

Add `proptest` as a dev-dependency and extend inline `#[cfg(test)]` modules across 7 source files with additional unit tests and property-based tests. All tests are added to existing test modules — no new files are created. Tasks are ordered: infrastructure first, then module-by-module test additions, with property-based tests placed close to related unit tests.

## Tasks

- [x] 1. Add proptest dev-dependency to Cargo.toml
  - Add `proptest = "1"` to the `[dev-dependencies]` section in `Cargo.toml`
  - Verify the project compiles with `cargo check`
  - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5_

- [x] 2. Selection module coordinate math and layout tests
  - [x] 2.1 Add unit tests for `ranges_overlap`, `monitor_scale_ratio`, `scaled_gap`, `logical_to_physical` in `src/selection.rs`
    - Add tests for non-overlapping, overlapping, adjacent (touching), and zero-size ranges in `ranges_overlap`
    - Add tests for zero logical size and valid ratio in `monitor_scale_ratio`
    - Add test for `scaled_gap` verifying `round(gap * physical / logical)`
    - Add tests for `logical_to_physical` with negative coords + fractional scale, and zero input
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9_

  - [x] 2.2 Add unit tests for `build_physical_screen_layouts` and `normalize_physical_screen_layouts` in `src/selection.rs`
    - Test single primary monitor placed at origin
    - Test two horizontally adjacent monitors at different DPI
    - Test two vertically stacked monitors
    - Test empty input returns error
    - Test no primary monitor uses first as anchor
    - Test negative coordinate normalization shifts min X/Y to zero
    - Test total width/height spans full virtual desktop
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

  - [x] 2.3 Add unit tests for `build_region_from_drag`, `resolve_recording_selection`, `find_screen_index_at_position`, `preferred_screen_index` in `src/selection.rs`
    - Test identical start/end returns `None`
    - Test negative coordinates spanning valid area
    - Test region fully inside one monitor
    - Test region spanning two monitors returns error
    - Test exact boundary acceptance
    - Test cursor inside/outside monitors for `find_screen_index_at_position`
    - Test `preferred_screen_index` with `None` cursor falls back to primary
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8_

  - [x] 2.4 Write property test for `logical_to_physical` rounding tolerance in `src/selection.rs`
    - **Requirement 17 Property 1: logical_to_physical rounding tolerance**
    - For all `i32` values and positive `f32` scale factors, result is within 1 unit of `value * scale`
    - Use `proptest!` macro with `any::<i32>()` and positive `f32` strategy
    - **Validates: Requirements 17.1, 1.8, 1.9**

  - [x] 2.5 Write property test for `ranges_overlap` correctness in `src/selection.rs`
    - **Requirement 17 Property 2: ranges_overlap correctness**
    - For all non-overlapping range pairs, returns `false`; for all overlapping pairs, returns `true`
    - Use `proptest!` macro generating random `(i32, u32, i32, u32)` tuples
    - **Validates: Requirements 17.2, 1.1, 1.2, 1.3**

- [x] 3. CLI module tests
  - [x] 3.1 Add unit tests for `cli_format_flag_present_in` in `src/cli.rs`
    - Test `--format png`, `-f`, `--format=png`, `-fpng` all return `true`
    - Test no format args returns `false`
    - Test `-o --format` skips output value (no false positive)
    - Test `-s 0 --format png` skips screen value and detects format
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

  - [x] 3.2 Add unit tests for `effective_cli_format`, `output_extension`, `format_from_extension`, `unsupported_output_extension` in `src/cli.rs`
    - Test explicit flag wins over output path and settings
    - Test output extension wins over settings when not explicit
    - Test settings fallback when no explicit flag and no output path
    - Test unknown extension falls back to settings
    - Test `output_extension` with no extension returns `None`
    - Test case-insensitive `format_from_extension` for "JPEG", "PNG"
    - Test `unsupported_output_extension` returns `None` for known extensions
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_

  - [x] 3.3 Add unit tests for `should_run_cli_mode`, `format_screen_list_output`, `selection_stderr_message`, `write_error_line` in `src/cli.rs`
    - Test all-false returns `false` (tray mode)
    - Test each individual flag (`cli`, `list`, `window`, `has_output`, `has_screen`, `format_explicit`) returns `true`
    - Test empty screen list output contains only header
    - Test "Selection cancelled" returns `None`
    - Test other error strings return `Some`
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 6.10_

- [x] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Record module tests
  - [x] 5.1 Add unit tests for `validate_recording_start`, `frame_duration_hns`, `round_to_even` in `src/record.rs`
    - Test FPS 0 returns error mentioning "greater than 0"
    - Test FPS > 240 returns error mentioning limit
    - Test FPS 1 accepted on Windows, rejected on non-Windows (platform-conditional)
    - Test FPS 240 accepted on Windows
    - Test `frame_duration_hns(0)` returns 0 without panic
    - Test standard FPS values (24, 30, 60, 120, 240) equal `10_000_000 / fps`
    - Test monotonically decreasing durations as FPS increases
    - Test `round_to_even` with even (unchanged), odd (value+1), and zero
    - Test `round_to_even` always returns even for 0..=100
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 7.10, 7.11_

  - [x] 5.2 Add unit tests for `validate_capture_region` in `src/record.rs`
    - Test negative X/Y returns error
    - Test zero width/height returns error
    - Test right/bottom edge exceeding frame returns error
    - Test exact frame fill is accepted
    - Test valid region strictly inside frame is accepted
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

  - [x] 5.3 Add unit tests for `rgba_to_bgra_bottom_up` and `pack_u32_pair` in `src/record.rs`
    - Test single-row: only channel swap, no row flip
    - Test multi-row: rows reversed and channels swapped
    - Test output length equals input length
    - Test alpha channel preserved for every pixel
    - Test `pack_u32_pair(hi, lo)` upper/lower 32 bits
    - Test `pack_u32_pair(0, 0)` is 0 and `(u32::MAX, u32::MAX)` is `u64::MAX`
    - _Requirements: 9.1, 9.2, 9.4, 9.5, 9.6, 9.7, 9.8_

  - [x] 5.4 Write property test for `round_to_even` in `src/record.rs`
    - **Requirement 17 Property 3: round_to_even correctness**
    - For all even `u32`, returns unchanged; for all odd, returns `value + 1`
    - Use `proptest!` macro with `any::<u32>()` (excluding `u32::MAX` for odd case)
    - **Validates: Requirements 17.3, 7.8, 7.9, 7.11**

  - [x] 5.5 Write property test for `rgba_to_bgra_bottom_up` roundtrip in `src/record.rs`
    - **Requirement 17 Property 4: rgba_to_bgra_bottom_up round-trip identity**
    - For all valid RGBA buffers of dimensions (w, h), applying twice produces original
    - Use `proptest!` macro generating small random dimensions and pixel data
    - **Validates: Requirements 17.4, 9.3**

- [x] 6. Main module tests
  - [x] 6.1 Add unit tests for `decide_app_mode` in `src/main.rs`
    - Test `select=true` → Selection mode
    - Test `select_record=true` → Selection mode
    - Test CLI-triggering flags → Cli mode
    - Test no flags → Tray mode
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [x] 6.2 Add unit tests for `run_cli_with_capture` and `run_selection_with_clipboard` in `src/main.rs`
    - Test `list=true` output starts with "Available screens:"
    - Test `window=true` invokes window capture function
    - Test "Selection cancelled" error is propagated without wrapping
    - _Requirements: 10.5, 10.6, 10.7_

  - [x] 6.3 Add unit tests for `sync_autorun_with_handlers`, `open_screenshots_folder_at_with_handlers`, `quoted_autorun_command` in `src/main.rs`
    - Test `enabled=true` calls enable handler, not disable
    - Test `enabled=false` calls disable handler, not enable
    - Test directory creation failure skips open handler
    - Test directory creation success calls open with correct path
    - Test `quoted_autorun_command` wraps path in double quotes (Windows)
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.7_

  - [x] 6.4 Add unit tests for `selection_stderr_message` and `write_error_line` in `src/main.rs`
    - Test "Selection cancelled" returns `None`
    - Test `write_error_line` output contains message + newline
    - _Requirements: 11.5, 11.6_

- [x] 7. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Settings module tests
  - [x] 8.1 Add unit tests for `format_settings_json` and `parse_settings_json` roundtrip in `src/settings.rs`
    - Test empty string returns defaults
    - Test garbage input returns defaults without panic
    - Test partial keys: missing fields use defaults, present fields parsed
    - Test duplicate keys: first occurrence wins
    - Test wrong-type values fall back to defaults
    - Test output contains exactly one occurrence of each key
    - Test Windows backslash paths preserved through format-then-parse
    - Test paths with embedded quotes preserved through format-then-parse
    - _Requirements: 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8, 12.9_

  - [x] 8.2 Add unit tests for `normalize_default_format`, `normalize_recording_fps`, `extract_json_string`, `extract_json_number`, `extract_json_bool` in `src/settings.rs`
    - Test "png", "jpg", "bmp" returned unchanged
    - Test unsupported format returns "png"
    - Test "PNG" (mixed case) returns "png" (case-sensitive rejection)
    - Test FPS 0 returns 30, positive value returned unchanged
    - Test missing key returns `None` for `extract_json_string`
    - Test floating-point value returns `None` for `extract_json_number`
    - Test "trueish" returns `None` for `extract_json_bool`
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 13.7, 13.8_

  - [x] 8.3 Write property test for Settings JSON roundtrip in `src/settings.rs`
    - **Requirement 17 Property 5: Settings JSON roundtrip**
    - For all valid `Settings` with supported format strings and positive FPS, `parse_settings_json(format_settings_json(s))` produces equivalent `Settings`
    - Use `proptest!` macro generating random `Settings` with format in {"png","jpg","bmp"} and FPS > 0
    - **Validates: Requirements 17.5, 12.1**

- [x] 9. Capture module tests
  - [x] 9.1 Add unit tests for `bgra_to_rgba` in `src/capture.rs`
    - Test double-swap restores original (identity)
    - Test green (index 1) and alpha (index 3) unchanged
    - Test empty buffer: no panic, buffer remains empty
    - Test trailing bytes not aligned to 4 are untouched
    - Test large buffer length preserved
    - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5_

  - [x] 9.2 Add unit tests for `save_image_optimized`, `ensure_parent_directory`, `resolve_output_path_in_dir` in `src/capture.rs`
    - Test PNG format creates valid PNG file
    - Test JPG format creates file (RGBA-to-RGB conversion)
    - Test BMP format creates file
    - Test `ensure_parent_directory` idempotent (two calls succeed)
    - Test deeply nested path creates all intermediates
    - Test `resolve_output_path_in_dir` with explicit path ignores directory
    - _Requirements: 15.1, 15.2, 15.3, 15.4, 15.5, 15.6_

- [x] 10. Tray module icon pixel invariant tests
  - [x] 10.1 Add unit tests for `build_default_icon_pixels` invariants in `src/tray.rs`
    - Test every pixel alpha is 0 or 255 (no partial transparency)
    - Test every opaque non-white pixel is RGB(41, 128, 185)
    - Test every white pixel is fully opaque
    - Test horizontal symmetry of opaque/transparent pattern
    - Test total pixel count equals 1024 (32×32)
    - Test lens (white) pixel count < body (blue) pixel count
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5, 16.6_

- [x] 11. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- All tests are inline `#[cfg(test)]` modules in existing source files — no new test files
- Property-based tests use `proptest!` macro blocks alongside unit tests
- Platform-conditional tests use `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]`
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
