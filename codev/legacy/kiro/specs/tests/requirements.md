# Requirements Document

## Introduction

Comprehensive unit test coverage for the RustShot project. The goal is to add thorough tests across all modules, focusing on pure logic functions, coordinate math, JSON parsing, CLI argument handling, and recording helpers. Tests target functions that are already extracted and testable without GUI or hardware dependencies.

## Glossary

- **Test_Suite**: The collection of all `#[cfg(test)]` inline unit tests and `tests/` integration tests in the RustShot project
- **Selection_Module**: `src/selection.rs` — multi-monitor DPI-aware coordinate transformations and region selection logic
- **CLI_Module**: `src/cli.rs` — command-line argument parsing helpers and format resolution
- **Record_Module**: `src/record.rs` — screen recording helpers (frame timing, dimension rounding, pixel conversion, region validation)
- **Main_Module**: `src/main.rs` — application mode routing, autorun logic, CLI dispatch
- **Settings_Module**: `src/settings.rs` — hand-rolled JSON settings persistence
- **Capture_Module**: `src/capture.rs` — screenshot capture and file saving helpers
- **Tray_Module**: `src/tray.rs` — system tray icon pixel generation and menu constants
- **PBT**: Property-based testing — generating random inputs to verify invariants hold for all values

## Requirements

### Requirement 1: Selection Module Coordinate Math Tests

**User Story:** As a developer, I want thorough tests for the selection module's pure coordinate math functions, so that multi-monitor DPI-aware layouts are computed correctly.

#### Acceptance Criteria

1. WHEN `ranges_overlap` is called with two non-overlapping ranges, THE Test_Suite SHALL verify the function returns `false`
2. WHEN `ranges_overlap` is called with two overlapping ranges, THE Test_Suite SHALL verify the function returns `true`
3. WHEN `ranges_overlap` is called with adjacent (touching but not overlapping) ranges, THE Test_Suite SHALL verify the function returns `false`
4. WHEN `ranges_overlap` is called with zero-size ranges, THE Test_Suite SHALL verify correct behavior at boundaries
5. WHEN `monitor_scale_ratio` is called with a zero logical size, THE Test_Suite SHALL verify the function returns `1.0`
6. WHEN `monitor_scale_ratio` is called with valid logical and physical sizes, THE Test_Suite SHALL verify the ratio equals `physical / logical`
7. WHEN `scaled_gap` is called with a gap value and monitor dimensions, THE Test_Suite SHALL verify the result equals `round(gap * physical / logical)`
8. WHEN `logical_to_physical` is called with negative coordinates and a fractional scale factor, THE Test_Suite SHALL verify correct rounding behavior
9. WHEN `logical_to_physical` is called with zero, THE Test_Suite SHALL verify the result is zero regardless of scale factor

### Requirement 2: Selection Module Layout Computation Tests

**User Story:** As a developer, I want tests for physical screen layout computation, so that multi-monitor positioning is correct across DPI boundaries.

#### Acceptance Criteria

1. WHEN `build_physical_screen_layouts` is called with a single primary monitor, THE Test_Suite SHALL verify the primary monitor is placed at physical origin (0, 0)
2. WHEN `build_physical_screen_layouts` is called with two horizontally adjacent monitors at different DPI, THE Test_Suite SHALL verify physical positions account for each monitor's pixel dimensions
3. WHEN `build_physical_screen_layouts` is called with two vertically stacked monitors, THE Test_Suite SHALL verify the second monitor's physical Y equals the first monitor's physical height
4. WHEN `build_physical_screen_layouts` is called with an empty input slice, THE Test_Suite SHALL verify the function returns an error
5. WHEN `build_physical_screen_layouts` is called with no primary monitor, THE Test_Suite SHALL verify the first monitor is used as anchor at origin (0, 0)
6. WHEN `normalize_physical_screen_layouts` is called with layouts that have negative physical coordinates, THE Test_Suite SHALL verify all coordinates are shifted so the minimum X and Y become zero
7. WHEN `normalize_physical_screen_layouts` is called, THE Test_Suite SHALL verify the returned total width and height span the full virtual desktop

### Requirement 3: Selection Module Region and Recording Tests

**User Story:** As a developer, I want tests for region building and recording selection resolution, so that drag-to-select and recording region mapping work correctly.

#### Acceptance Criteria

1. WHEN `build_region_from_drag` is called with identical start and end points, THE Test_Suite SHALL verify the function returns `None`
2. WHEN `build_region_from_drag` is called with negative coordinates spanning a valid area, THE Test_Suite SHALL verify a correct `Region` is returned with the minimum coordinates as origin
3. WHEN `resolve_recording_selection` is called with a region fully inside one monitor, THE Test_Suite SHALL verify the correct screen index and local coordinates are returned
4. WHEN `resolve_recording_selection` is called with a region spanning two monitors, THE Test_Suite SHALL verify the function returns an error
5. WHEN `resolve_recording_selection` is called with a region at the exact boundary of a monitor, THE Test_Suite SHALL verify the region is accepted as belonging to that monitor
6. WHEN `find_screen_index_at_position` is called with a cursor position inside a monitor, THE Test_Suite SHALL verify the correct monitor index is returned
7. WHEN `find_screen_index_at_position` is called with a cursor position outside all monitors, THE Test_Suite SHALL verify `None` is returned
8. WHEN `preferred_screen_index` is called with `None` cursor position, THE Test_Suite SHALL verify the primary monitor index is returned

### Requirement 4: CLI Module Format Detection Tests

**User Story:** As a developer, I want thorough tests for CLI format flag detection, so that explicit vs implicit format resolution is always correct.

#### Acceptance Criteria

1. WHEN `cli_format_flag_present_in` is called with `--format png` arguments, THE Test_Suite SHALL verify the function returns `true`
2. WHEN `cli_format_flag_present_in` is called with `-f` short flag, THE Test_Suite SHALL verify the function returns `true`
3. WHEN `cli_format_flag_present_in` is called with `--format=png` equals form, THE Test_Suite SHALL verify the function returns `true`
4. WHEN `cli_format_flag_present_in` is called with `-fpng` attached short form, THE Test_Suite SHALL verify the function returns `true`
5. WHEN `cli_format_flag_present_in` is called with no format-related arguments, THE Test_Suite SHALL verify the function returns `false`
6. WHEN `cli_format_flag_present_in` is called with `-o --format` where `--format` is the value of `-o`, THE Test_Suite SHALL verify the function correctly skips the output value and does not false-positive
7. WHEN `cli_format_flag_present_in` is called with `-s 0 --format png`, THE Test_Suite SHALL verify the function correctly skips the screen value and detects the format flag

### Requirement 5: CLI Module Format Resolution Precedence Tests

**User Story:** As a developer, I want tests for the effective format resolution chain, so that the precedence of explicit flag > output extension > settings is always respected.

#### Acceptance Criteria

1. WHEN `effective_cli_format` is called with `format_explicit=true`, THE Test_Suite SHALL verify the CLI format argument wins regardless of output path or settings
2. WHEN `effective_cli_format` is called with `format_explicit=false` and an output path with a known extension, THE Test_Suite SHALL verify the output extension wins over settings
3. WHEN `effective_cli_format` is called with `format_explicit=false` and no output path, THE Test_Suite SHALL verify the settings format is used as fallback
4. WHEN `effective_cli_format` is called with `format_explicit=false` and an output path with an unknown extension, THE Test_Suite SHALL verify the settings format is used as fallback
5. WHEN `output_extension` is called with a path that has no extension, THE Test_Suite SHALL verify `None` is returned
6. WHEN `format_from_extension` is called with case-insensitive variants like "JPEG" and "PNG", THE Test_Suite SHALL verify the correct format is returned
7. WHEN `unsupported_output_extension` is called with a known extension, THE Test_Suite SHALL verify `None` is returned

### Requirement 6: CLI Module Mode Detection Tests

**User Story:** As a developer, I want tests for all CLI mode detection flag combinations, so that the app correctly routes to CLI, selection, or tray mode.

#### Acceptance Criteria

1. WHEN `should_run_cli_mode` is called with all flags set to `false`, THE Test_Suite SHALL verify the function returns `false` (tray mode)
2. WHEN `should_run_cli_mode` is called with `cli=true`, THE Test_Suite SHALL verify the function returns `true`
3. WHEN `should_run_cli_mode` is called with `list=true`, THE Test_Suite SHALL verify the function returns `true`
4. WHEN `should_run_cli_mode` is called with `window=true`, THE Test_Suite SHALL verify the function returns `true`
5. WHEN `should_run_cli_mode` is called with `has_output=true`, THE Test_Suite SHALL verify the function returns `true`
6. WHEN `should_run_cli_mode` is called with `has_screen=true`, THE Test_Suite SHALL verify the function returns `true`
7. WHEN `should_run_cli_mode` is called with `format_explicit=true`, THE Test_Suite SHALL verify the function returns `true`
8. WHEN `format_screen_list_output` is called with an empty screen list, THE Test_Suite SHALL verify the output contains only the header line
9. WHEN `selection_stderr_message` is called with "Selection cancelled", THE Test_Suite SHALL verify `None` is returned (suppressed)
10. WHEN `selection_stderr_message` is called with any other error string, THE Test_Suite SHALL verify `Some` is returned with the error message


### Requirement 7: Record Module Validation and Timing Tests

**User Story:** As a developer, I want thorough tests for recording validation and frame timing, so that edge cases in FPS handling and dimension rounding are covered.

#### Acceptance Criteria

1. WHEN `validate_recording_start` is called with FPS of 0, THE Test_Suite SHALL verify the function returns an error mentioning "greater than 0"
2. WHEN `validate_recording_start` is called with FPS exceeding `MAX_RECORDING_FPS` (240), THE Test_Suite SHALL verify the function returns an error mentioning the limit
3. WHEN `validate_recording_start` is called with FPS of 1, THE Test_Suite SHALL verify the function accepts the value on Windows and rejects on non-Windows
4. WHEN `validate_recording_start` is called with FPS of exactly 240, THE Test_Suite SHALL verify the function accepts the value on Windows
5. WHEN `frame_duration_hns` is called with FPS of 0, THE Test_Suite SHALL verify the function returns 0 without division by zero
6. WHEN `frame_duration_hns` is called with standard FPS values (24, 30, 60, 120, 240), THE Test_Suite SHALL verify the result equals `10_000_000 / fps` using integer division
7. THE Test_Suite SHALL verify that `frame_duration_hns` returns monotonically decreasing values as FPS increases
8. WHEN `round_to_even` is called with an even value, THE Test_Suite SHALL verify the value is returned unchanged
9. WHEN `round_to_even` is called with an odd value, THE Test_Suite SHALL verify the result is the next even number (value + 1)
10. WHEN `round_to_even` is called with 0, THE Test_Suite SHALL verify the result is 0
11. THE Test_Suite SHALL verify that `round_to_even` always returns an even number for all inputs in the range 0..=100

### Requirement 8: Record Module Region Validation Tests

**User Story:** As a developer, I want tests for capture region validation, so that boundary conditions and invalid regions are properly rejected.

#### Acceptance Criteria

1. WHEN `validate_capture_region` is called with a region that has negative X or Y, THE Test_Suite SHALL verify the function returns an error
2. WHEN `validate_capture_region` is called with a region that has zero width or height, THE Test_Suite SHALL verify the function returns an error about non-zero size
3. WHEN `validate_capture_region` is called with a region whose right edge exceeds the frame width, THE Test_Suite SHALL verify the function returns an error
4. WHEN `validate_capture_region` is called with a region whose bottom edge exceeds the frame height, THE Test_Suite SHALL verify the function returns an error
5. WHEN `validate_capture_region` is called with a region that exactly fills the frame, THE Test_Suite SHALL verify the function accepts the region
6. WHEN `validate_capture_region` is called with a valid region strictly inside the frame, THE Test_Suite SHALL verify the function accepts the region

### Requirement 9: Record Module Pixel Conversion Tests

**User Story:** As a developer, I want tests for RGBA-to-BGRA bottom-up conversion, so that pixel data is correctly transformed for the Media Foundation encoder.

#### Acceptance Criteria

1. WHEN `rgba_to_bgra_bottom_up` is called with a single-row image, THE Test_Suite SHALL verify only the channel swap occurs (no row flip)
2. WHEN `rgba_to_bgra_bottom_up` is called with a multi-row image, THE Test_Suite SHALL verify rows are reversed and channels are swapped
3. THE Test_Suite SHALL verify that applying `rgba_to_bgra_bottom_up` twice produces the original input (round-trip identity)
4. WHEN `rgba_to_bgra_bottom_up` is called, THE Test_Suite SHALL verify the output length equals the input length
5. WHEN `rgba_to_bgra_bottom_up` is called, THE Test_Suite SHALL verify the alpha channel is preserved for every pixel
6. WHEN `pack_u32_pair` is called with (hi, lo), THE Test_Suite SHALL verify the upper 32 bits equal `hi` and the lower 32 bits equal `lo`
7. WHEN `pack_u32_pair` is called with (0, 0), THE Test_Suite SHALL verify the result is 0
8. WHEN `pack_u32_pair` is called with (u32::MAX, u32::MAX), THE Test_Suite SHALL verify the result is u64::MAX

### Requirement 10: Main Module App Mode Routing Tests

**User Story:** As a developer, I want tests for application mode routing, so that the correct mode (tray, CLI, selection) is chosen based on arguments.

#### Acceptance Criteria

1. WHEN `decide_app_mode` is called with `select=true`, THE Test_Suite SHALL verify the result is `Selection` mode
2. WHEN `decide_app_mode` is called with `select_record=true`, THE Test_Suite SHALL verify the result is `Selection` mode
3. WHEN `decide_app_mode` is called with CLI-triggering flags, THE Test_Suite SHALL verify the result is `Cli` mode
4. WHEN `decide_app_mode` is called with no flags, THE Test_Suite SHALL verify the result is `Tray` mode
5. WHEN `run_cli_with_capture` is called with `list=true`, THE Test_Suite SHALL verify the output starts with "Available screens:"
6. WHEN `run_cli_with_capture` is called with `window=true`, THE Test_Suite SHALL verify the window capture function is invoked
7. WHEN `run_selection_with_clipboard` is called and the selection function returns "Selection cancelled", THE Test_Suite SHALL verify the error is propagated without wrapping

### Requirement 11: Main Module Autorun and Helper Tests

**User Story:** As a developer, I want tests for autorun synchronization and helper functions, so that registry logic and error formatting are correct.

#### Acceptance Criteria

1. WHEN `sync_autorun_with_handlers` is called with `enabled=true`, THE Test_Suite SHALL verify the enable handler is called and the disable handler is not called
2. WHEN `sync_autorun_with_handlers` is called with `enabled=false`, THE Test_Suite SHALL verify the disable handler is called and the enable handler is not called
3. WHEN `open_screenshots_folder_at_with_handlers` is called and directory creation fails, THE Test_Suite SHALL verify the open handler is not called
4. WHEN `open_screenshots_folder_at_with_handlers` is called and directory creation succeeds, THE Test_Suite SHALL verify the open handler is called with the correct path
5. WHEN `selection_stderr_message` is called with "Selection cancelled", THE Test_Suite SHALL verify `None` is returned
6. WHEN `write_error_line` is called with a message, THE Test_Suite SHALL verify the output contains the message followed by a newline
7. WHEN `quoted_autorun_command` is called with a path (Windows), THE Test_Suite SHALL verify the result is the path wrapped in double quotes

### Requirement 12: Settings Module JSON Roundtrip Tests

**User Story:** As a developer, I want roundtrip tests for the hand-rolled JSON parser and formatter, so that settings are never silently corrupted.

#### Acceptance Criteria

1. FOR ALL valid `Settings` objects, formatting then parsing SHALL produce an equivalent `Settings` object (roundtrip property)
2. WHEN `parse_settings_json` is called with an empty string, THE Test_Suite SHALL verify default settings are returned
3. WHEN `parse_settings_json` is called with garbage input, THE Test_Suite SHALL verify default settings are returned without panic
4. WHEN `parse_settings_json` is called with a JSON object containing only some keys, THE Test_Suite SHALL verify missing fields use defaults and present fields are parsed
5. WHEN `parse_settings_json` is called with duplicate keys, THE Test_Suite SHALL verify the first occurrence wins
6. WHEN `parse_settings_json` is called with wrong-type values (number for string field, string for number field), THE Test_Suite SHALL verify the field falls back to its default
7. WHEN `format_settings_json` is called, THE Test_Suite SHALL verify the output contains exactly one occurrence of each settings key
8. WHEN settings contain Windows backslash paths, THE Test_Suite SHALL verify format-then-parse preserves the paths exactly
9. WHEN settings contain paths with embedded quote characters, THE Test_Suite SHALL verify format-then-parse preserves the paths exactly

### Requirement 13: Settings Module Normalization Tests

**User Story:** As a developer, I want tests for format and FPS normalization, so that invalid user-edited values are safely corrected.

#### Acceptance Criteria

1. WHEN `normalize_default_format` is called with "png", "jpg", or "bmp", THE Test_Suite SHALL verify the value is returned unchanged
2. WHEN `normalize_default_format` is called with an unsupported format string, THE Test_Suite SHALL verify "png" is returned as default
3. WHEN `normalize_default_format` is called with a mixed-case variant like "PNG", THE Test_Suite SHALL verify "png" is returned (case-sensitive rejection)
4. WHEN `normalize_recording_fps` is called with 0, THE Test_Suite SHALL verify 30 is returned as default
5. WHEN `normalize_recording_fps` is called with a positive value, THE Test_Suite SHALL verify the value is returned unchanged
6. WHEN `extract_json_string` is called with a key that does not exist, THE Test_Suite SHALL verify `None` is returned
7. WHEN `extract_json_number` is called with a floating-point value, THE Test_Suite SHALL verify `None` is returned (integers only)
8. WHEN `extract_json_bool` is called with a value like "trueish", THE Test_Suite SHALL verify `None` is returned (strict token boundary)

### Requirement 14: Capture Module BGRA/RGBA Swap Tests

**User Story:** As a developer, I want tests for the BGRA-to-RGBA channel swap, so that pixel data integrity is guaranteed.

#### Acceptance Criteria

1. THE Test_Suite SHALL verify that applying `bgra_to_rgba` twice restores the original pixel data (double-swap identity)
2. WHEN `bgra_to_rgba` is called, THE Test_Suite SHALL verify the green channel (index 1) and alpha channel (index 3) are unchanged
3. WHEN `bgra_to_rgba` is called with an empty buffer, THE Test_Suite SHALL verify no panic occurs and the buffer remains empty
4. WHEN `bgra_to_rgba` is called with trailing bytes not aligned to 4, THE Test_Suite SHALL verify the trailing bytes are untouched
5. WHEN `bgra_to_rgba` is called with a large buffer, THE Test_Suite SHALL verify the buffer length is preserved

### Requirement 15: Capture Module Image Save and Path Tests

**User Story:** As a developer, I want tests for image saving and path resolution, so that file I/O edge cases are handled correctly.

#### Acceptance Criteria

1. WHEN `save_image_optimized` is called with a valid image and PNG format, THE Test_Suite SHALL verify the file is created and is a valid PNG
2. WHEN `save_image_optimized` is called with a valid image and JPG format, THE Test_Suite SHALL verify the file is created (RGBA-to-RGB conversion succeeds)
3. WHEN `save_image_optimized` is called with a valid image and BMP format, THE Test_Suite SHALL verify the file is created
4. WHEN `ensure_parent_directory` is called twice on the same path, THE Test_Suite SHALL verify both calls succeed (idempotence)
5. WHEN `ensure_parent_directory` is called with a deeply nested path, THE Test_Suite SHALL verify all intermediate directories are created
6. WHEN `resolve_output_path_in_dir` is called with an explicit path, THE Test_Suite SHALL verify the provided directory is ignored and the explicit path is returned verbatim

### Requirement 16: Tray Module Icon Pixel Invariant Tests

**User Story:** As a developer, I want tests for tray icon pixel generation invariants, so that the icon rendering is deterministic and correct.

#### Acceptance Criteria

1. THE Test_Suite SHALL verify every pixel in the icon has alpha of either 0 (transparent) or 255 (opaque) — no partial transparency
2. THE Test_Suite SHALL verify every opaque non-white pixel has exactly RGB(41, 128, 185) (camera body blue)
3. THE Test_Suite SHALL verify every white pixel (lens) is also fully opaque
4. THE Test_Suite SHALL verify the icon has horizontal symmetry (left half mirrors right half in terms of opaque/transparent pattern)
5. THE Test_Suite SHALL verify the total pixel count (opaque + transparent) equals 1024 (32×32)
6. THE Test_Suite SHALL verify the lens (white) pixel count is less than the body (blue) pixel count

### Requirement 17: Property-Based Test for Coordinate Transformations

**User Story:** As a developer, I want property-based tests for coordinate math, so that invariants hold across a wide range of inputs.

#### Acceptance Criteria

1. FOR ALL `i32` values and positive `f32` scale factors, `logical_to_physical(value, scale)` SHALL produce a result within 1 unit of `value * scale` (rounding tolerance)
2. FOR ALL non-overlapping range pairs, `ranges_overlap` SHALL return `false`; FOR ALL overlapping range pairs, `ranges_overlap` SHALL return `true`
3. FOR ALL even `u32` values, `round_to_even(value)` SHALL return the value unchanged; FOR ALL odd values, `round_to_even(value)` SHALL return `value + 1`
4. FOR ALL valid RGBA pixel buffers of dimensions (w, h), applying `rgba_to_bgra_bottom_up` twice SHALL produce the original buffer (round-trip identity)
5. FOR ALL valid `Settings` objects with supported format strings and positive FPS, `parse_settings_json(format_settings_json(settings))` SHALL produce an equivalent `Settings` object (JSON roundtrip)
