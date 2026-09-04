//! Executable BDD specifications for RustShot.
//!
//! The Gherkin feature files under `tests/features/` are the specification;
//! the step definitions below drive RustShot's public pure-logic API and
//! assert the documented behavior. Run with `cargo test --test bdd`.

use cucumber::gherkin::Step;
use cucumber::{given, then, when, World};
use rustshot::{app, capture, cli, mcp, record, selection, settings, tray};
use serde_json::Value;
use std::path::PathBuf;

#[path = "bdd/requirements.rs"]
mod requirements;

/// Monitor input row for layout building: logical position and size, physical
/// size, primary flag — the tuple shape the `selection::benchmark_*` API takes.
type MonitorInput = (i32, i32, u32, u32, u32, u32, bool);
/// A monitor's physical geometry as (x, y, width, height).
type PhysicalRect = (i32, i32, u32, u32);
/// A normalized layout and its virtual-desktop bounds.
type NormalizedLayout = (Vec<PhysicalRect>, i32, i32, u32, u32);
/// The selected monitor index and monitor-relative recording rectangle.
type RecordingSelection = (usize, PhysicalRect);
/// A monitor as (origin, size) for overlay targeting.
type Monitor = ((i32, i32), (u32, u32));
/// A fully resolved monitor layout with logical and physical geometry.
type ResolvedPhysicalLayout = (i32, i32, u32, u32, i32, i32, u32, u32);

/// Shared scenario state. Every field defaults to a benign value so unrelated
/// scenarios do not interfere; each `When` step records its computed result in
/// the matching `*_result`-style field for the `Then` step to assert.
#[derive(Debug, Default, World)]
struct RustShotWorld {
    // --- application mode routing ---
    select_record: bool,
    select: bool,
    cli_flag: bool,
    list: bool,
    window: bool,
    has_output: bool,
    has_screen: bool,
    format_explicit: bool,
    app_mode: Option<cli::AppMode>,
    executable_process: Option<std::process::Output>,

    // --- first-run startup installation ---
    startup_current_exe: Option<PathBuf>,
    startup_local_app_data: Option<PathBuf>,
    startup_installed_exe: Option<PathBuf>,
    startup_installed_exists: bool,
    startup_events: Vec<String>,
    startup_should_exit: Option<Result<bool, String>>,
    startup_settings_result: Option<Result<(), String>>,

    // --- format resolution ---
    format_from_ext: Option<Option<capture::Format>>,

    // --- settings normalization ---
    normalized_format: Option<String>,
    normalized_fps: Option<u32>,

    // --- recording ---
    validate_result: Option<Result<(), String>>,
    frame_duration: Option<i64>,
    round_result: Option<u32>,
    strict_fps_result: Option<Result<u32, String>>,
    recording_dimensions_result: Option<Result<(u32, u32, usize), String>>,
    frame_timing_result: Option<Result<(i64, i64), String>>,
    bitrate_result: Option<Result<u32, String>>,

    // --- selection drag ---
    drag_start: (i32, i32),
    drag_end: (i32, i32),
    region: Option<Option<selection::Region>>,

    // --- raw argument scanning ---
    argv: Vec<String>,
    format_flag_detected: Option<bool>,

    // --- effective format and output-path resolution ---
    cli_format: Option<capture::Format>,
    settings_format: Option<capture::Format>,
    output_path: Option<PathBuf>,
    unsupported_ext: Option<Option<String>>,
    resolved_path: Option<PathBuf>,

    // --- requirement-level screenshot pipeline ---
    capture_pipeline_result: Option<Result<PathBuf, String>>,
    capture_pipeline_stages: Vec<String>,
    screenshot_parent_result: Option<Result<(), String>>,
    screenshot_parent_root: Option<PathBuf>,
    image_encoder: Option<String>,

    // --- screen index resolution ---
    screen_index_result: Option<Result<usize, String>>,

    // --- checked dimension rounding and pixel conversion ---
    checked_round_result: Option<Result<u32, String>>,
    bgra_conversion_result: Option<Result<(), String>>,
    converted_pixels: Option<Result<Vec<u8>, String>>,
    packed_pair: Option<u64>,

    // --- MCP protocol ---
    // Outer Option: a message was sent; inner Option: the server's reply
    // (None for notifications, which must stay unanswered).
    mcp_reply: Option<Option<String>>,

    // --- multi-monitor physical layout ---
    monitor_inputs: Vec<MonitorInput>,
    layout_result: Option<Result<Vec<PhysicalRect>, String>>,
    layout_sizes_result: Option<Result<Vec<(u32, u32)>, String>>,
    resolved_layout_inputs: Vec<ResolvedPhysicalLayout>,
    normalized_layout_result: Option<Result<NormalizedLayout, String>>,

    // --- selection requirement boundaries ---
    ranges_overlap: Option<bool>,
    monitor_scale_ratio: Option<f32>,
    scaled_gap: Option<i32>,
    physical_coordinate: Option<i32>,
    typed_physical_point: Option<Result<selection::PhysicalPoint, String>>,
    typed_physical_size: Option<Result<selection::PhysicalSize, String>>,
    scale_factor_result: Option<Result<selection::ScaleFactor, String>>,
    physical_monitors: Vec<(i32, i32, u32, u32, bool)>,
    recording_selection_result: Option<Result<RecordingSelection, String>>,
    cursor_monitor: Option<Option<usize>>,
    preferred_monitor: Option<Option<usize>>,

    // --- settings persistence requirements ---
    settings_input: Option<settings::Settings>,
    serialized_settings: Option<String>,
    parsed_settings: Option<settings::Settings>,

    // --- tray domain requirements ---
    tray_action: Option<Option<tray::MenuAction>>,
    tray_pixels: Option<Vec<u8>>,

    // --- active window capture boundary ---
    active_window_dir: Option<PathBuf>,
    active_window_result: Option<Result<PathBuf, String>>,
    active_window_output_result: Option<Result<PathBuf, String>>,
    active_window_capture_calls: usize,
    active_window_expected_pixels: Vec<u8>,
    active_window_clipboard_images: Vec<Vec<u8>>,
    active_window_saved_images: Vec<Vec<u8>>,

    // --- overlay monitor targeting ---
    connected_monitors: Vec<Monitor>,
    overlay_monitor: Option<Option<usize>>,

    // --- recording start boundary ---
    recording_start_error: Option<String>,

    // --- display-gated MCP scenarios ---
    // True when the When step skipped its call because no display server is
    // available; the Then step then asserts nothing (live-capture policy).
    mcp_display_skipped: bool,

    // --- cross-platform live-capture E2E ---
    live_capture_active: bool,
    live_capture_path: Option<PathBuf>,
    live_capture_process: Option<std::process::Output>,
    live_capture_elapsed: Option<std::time::Duration>,
}

fn app_mode_name(mode: cli::AppMode) -> &'static str {
    match mode {
        cli::AppMode::Tray => "Tray",
        cli::AppMode::Cli => "Cli",
        cli::AppMode::Selection => "Selection",
        cli::AppMode::SelectRecord => "SelectRecord",
    }
}

fn format_name(format: capture::Format) -> &'static str {
    match format {
        capture::Format::Png => "Png",
        capture::Format::Jpg => "Jpg",
        capture::Format::Bmp => "Bmp",
    }
}

// ===========================================================================
// Application mode routing
// ===========================================================================

#[given("no CLI flags are set")]
fn no_flags(_world: &mut RustShotWorld) {
    // The World already defaults every flag to false.
}

#[given(regex = r#"^the "([a-z_]+)" flag is set$"#)]
fn set_flag(world: &mut RustShotWorld, flag: String) {
    match flag.as_str() {
        "select_record" => world.select_record = true,
        "select" => world.select = true,
        "cli" => world.cli_flag = true,
        "list" => world.list = true,
        "window" => world.window = true,
        "has_output" => world.has_output = true,
        "has_screen" => world.has_screen = true,
        "format_explicit" => world.format_explicit = true,
        other => panic!("unknown flag in spec: {other}"),
    }
}

#[when("the application mode is decided")]
fn decide_mode(world: &mut RustShotWorld) {
    let request = cli::AppModeRequest {
        select_record: world.select_record,
        select: world.select,
        cli: world.cli_flag,
        list: world.list,
        window: world.window,
        has_output: world.has_output,
        has_screen: world.has_screen,
        format_explicit: world.format_explicit,
    };
    world.app_mode = Some(cli::decide_app_mode(request));
}

#[then(regex = r#"^the application mode is "([A-Za-z]+)"$"#)]
fn assert_mode(world: &mut RustShotWorld, expected: String) {
    let mode = world.app_mode.expect("application mode was not decided");
    assert_eq!(app_mode_name(mode), expected);
}

// ===========================================================================
// Format resolution
// ===========================================================================

#[when(regex = r#"^the format for extension "([^"]*)" is resolved$"#)]
fn resolve_format(world: &mut RustShotWorld, extension: String) {
    world.format_from_ext = Some(cli::format_from_extension(&extension));
}

#[then(regex = r#"^the resolved format is "([A-Za-z]+)"$"#)]
fn assert_format(world: &mut RustShotWorld, expected: String) {
    let resolved = world
        .format_from_ext
        .expect("format resolution did not run")
        .expect("expected a format but none was resolved");
    assert_eq!(format_name(resolved), expected);
}

#[then("no format is resolved")]
fn assert_no_format(world: &mut RustShotWorld) {
    let resolved = world
        .format_from_ext
        .expect("format resolution did not run");
    assert!(resolved.is_none(), "expected no format, got {resolved:?}");
}

// ===========================================================================
// Settings normalization
// ===========================================================================

#[when(regex = r#"^the default format "([^"]*)" is normalized$"#)]
fn normalize_format(world: &mut RustShotWorld, input: String) {
    // Normalization is now the `Format` enum's constructor: parsing a settings
    // string yields a Format, so an unsupported value cannot survive.
    world.normalized_format = Some(
        capture::Format::from_settings_str(&input)
            .extension()
            .to_string(),
    );
}

#[then(regex = r#"^the normalized default format is "([^"]*)"$"#)]
fn assert_normalized_format(world: &mut RustShotWorld, expected: String) {
    let actual = world
        .normalized_format
        .as_deref()
        .expect("format normalization did not run");
    assert_eq!(actual, expected);
}

#[when(regex = r"^the recording FPS (\d+) is normalized$")]
fn normalize_fps(world: &mut RustShotWorld, input: u32) {
    // Normalization is now the `Fps` newtype's forgiving constructor.
    world.normalized_fps = Some(settings::Fps::from_settings(input).get());
}

#[then(regex = r"^the normalized recording FPS is (\d+)$")]
fn assert_normalized_fps(world: &mut RustShotWorld, expected: u32) {
    let actual = world.normalized_fps.expect("fps normalization did not run");
    assert_eq!(actual, expected);
}

// ===========================================================================
// Recording validation and timing
// ===========================================================================

#[when(regex = r"^recording start is validated with (\d+) FPS$")]
fn validate_recording(world: &mut RustShotWorld, fps: u32) {
    world.validate_result = Some(record::validate_recording_start(fps));
}

#[then(regex = r#"^validation fails with "([^"]+)"$"#)]
fn assert_validation_fails(world: &mut RustShotWorld, message: String) {
    let result = world
        .validate_result
        .as_ref()
        .expect("recording validation did not run");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(message.as_str())
    );
}

#[then("validation succeeds")]
fn assert_validation_succeeds(world: &mut RustShotWorld) {
    let result = world
        .validate_result
        .as_ref()
        .expect("recording validation did not run");
    // Recording is a Windows-only feature; on other platforms a valid FPS is
    // rejected with the platform message. The spec text describes the Windows
    // behavior, so assert per platform to keep the suite honest everywhere.
    #[cfg(windows)]
    assert!(result.is_ok(), "expected success, got {result:?}");
    #[cfg(not(windows))]
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some("Recording is only supported on Windows"),
    );
}

#[when(regex = r"^the frame duration for (\d+) FPS is computed$")]
fn compute_frame_duration(world: &mut RustShotWorld, fps: u32) {
    world.frame_duration = Some(record::frame_duration_hns(fps));
}

#[then(regex = r"^the frame duration is (\d+) hundred-nanosecond units$")]
fn assert_frame_duration(world: &mut RustShotWorld, expected: i64) {
    let actual = world
        .frame_duration
        .expect("frame duration was not computed");
    assert_eq!(actual, expected);
}

#[when(regex = r"^(\d+) is rounded to even$")]
fn round_even(world: &mut RustShotWorld, value: u32) {
    world.round_result = Some(record::round_to_even(value));
}

#[then(regex = r"^the rounded value is (\d+)$")]
fn assert_round_even(world: &mut RustShotWorld, expected: u32) {
    let actual = world.round_result.expect("rounding did not run");
    assert_eq!(actual, expected);
}

// ===========================================================================
// Selection region from a drag
// ===========================================================================

#[given(regex = r"^a drag from \((-?\d+), (-?\d+)\) to \((-?\d+), (-?\d+)\)$")]
fn drag_from_to(world: &mut RustShotWorld, x1: i32, y1: i32, x2: i32, y2: i32) {
    world.drag_start = (x1, y1);
    world.drag_end = (x2, y2);
}

#[when("the drag region is built")]
fn build_region(world: &mut RustShotWorld) {
    world.region = Some(selection::build_region_from_drag(
        world.drag_start,
        world.drag_end,
    ));
}

#[then(regex = r"^a region is produced at \((-?\d+), (-?\d+)\) sized (\d+) x (\d+)$")]
fn assert_region(world: &mut RustShotWorld, x: i32, y: i32, width: u32, height: u32) {
    let region = world
        .region
        .expect("region was not built")
        .expect("expected a region but the drag was rejected");
    assert_eq!(region.physical_x, x, "physical_x");
    assert_eq!(region.physical_y, y, "physical_y");
    assert_eq!(region.physical_width, width, "physical_width");
    assert_eq!(region.physical_height, height, "physical_height");
}

#[then("no region is produced")]
fn assert_no_region(world: &mut RustShotWorld) {
    let region = world.region.expect("region was not built");
    assert!(region.is_none(), "expected no region, got {region:?}");
}

// ===========================================================================
// Raw argument scanning for an explicit format flag
// ===========================================================================

#[given(regex = r#"^the command line "([^"]*)"$"#)]
fn command_line(world: &mut RustShotWorld, argv: String) {
    world.argv = argv.split_whitespace().map(str::to_string).collect();
}

#[when("the arguments are scanned for a format flag")]
fn scan_format_flag(world: &mut RustShotWorld) {
    world.format_flag_detected = Some(cli::cli_format_flag_present_in(world.argv.clone()));
}

#[then(regex = r"^a format flag (is|is not) detected$")]
fn assert_format_flag(world: &mut RustShotWorld, verdict: String) {
    let detected = world
        .format_flag_detected
        .expect("the arguments were not scanned");
    assert_eq!(detected, verdict == "is");
}

// ===========================================================================
// Effective CLI format and unsupported output extensions
// ===========================================================================

fn known_format(spec: &str) -> capture::Format {
    cli::format_from_extension(spec).unwrap_or_else(|| panic!("unknown format in spec: {spec:?}"))
}

#[given(regex = r#"^the CLI format "([a-z]+)" was given (explicitly|implicitly)$"#)]
fn cli_format_given(world: &mut RustShotWorld, format: String, how: String) {
    world.cli_format = Some(known_format(&format));
    world.format_explicit = how == "explicitly";
}

#[given(regex = r#"^the settings format is "([a-z]+)"$"#)]
fn settings_format_given(world: &mut RustShotWorld, format: String) {
    world.settings_format = Some(known_format(&format));
}

#[given(regex = r#"^the output path is "([^"]*)"$"#)]
fn output_path_given(world: &mut RustShotWorld, path: String) {
    world.output_path = (!path.is_empty()).then(|| PathBuf::from(path));
}

#[when("the effective CLI format is resolved")]
fn resolve_effective_format(world: &mut RustShotWorld) {
    let resolved = cli::effective_cli_format(
        world.cli_format.expect("the CLI format was not set"),
        world.format_explicit,
        world.output_path.as_deref(),
        world
            .settings_format
            .expect("the settings format was not set"),
    );
    // Stored through the same field as extension resolution so the existing
    // `the resolved format is "..."` assertion is reused verbatim.
    world.format_from_ext = Some(Some(resolved));
}

#[when("the output path is checked for an unsupported extension")]
fn check_unsupported_extension(world: &mut RustShotWorld) {
    world.unsupported_ext = Some(cli::unsupported_output_extension(
        world.output_path.as_deref(),
    ));
}

#[then(regex = r#"^the reported unsupported extension is (nothing|"[^"]*")$"#)]
fn assert_unsupported_extension(world: &mut RustShotWorld, expected: String) {
    let actual = world
        .unsupported_ext
        .as_ref()
        .expect("the output path was not checked");
    if expected == "nothing" {
        assert!(actual.is_none(), "expected no report, got {actual:?}");
    } else {
        assert_eq!(actual.as_deref(), Some(expected.trim_matches('"')));
    }
}

// ===========================================================================
// Output file path resolution
// ===========================================================================

#[when(
    regex = r#"^the output path "([^"]+)" is resolved for format "([a-z]+)" in directory "([^"]+)"$"#
)]
fn resolve_explicit_output(world: &mut RustShotWorld, path: String, format: String, dir: String) {
    world.resolved_path = Some(capture::resolve_output_path_in_dir(
        Some(PathBuf::from(path)),
        known_format(&format),
        std::path::Path::new(&dir),
    ));
}

#[when(regex = r#"^an output path is generated for format "([a-z]+)" in directory "([^"]+)"$"#)]
fn generate_output_path(world: &mut RustShotWorld, format: String, dir: String) {
    world.resolved_path = Some(capture::resolve_output_path_in_dir(
        None,
        known_format(&format),
        std::path::Path::new(&dir),
    ));
}

#[then(regex = r#"^the resolved path is "([^"]+)"$"#)]
fn assert_resolved_path(world: &mut RustShotWorld, expected: String) {
    let resolved = world
        .resolved_path
        .as_ref()
        .expect("the output path was not resolved");
    assert_eq!(resolved, &PathBuf::from(expected));
}

#[then(
    regex = r#"^the generated file is in "([^"]+)" and its name starts with "([^"]+)" and ends with "([^"]+)"$"#
)]
fn assert_generated_name(world: &mut RustShotWorld, dir: String, prefix: String, suffix: String) {
    let resolved = world
        .resolved_path
        .as_ref()
        .expect("the output path was not resolved");
    assert_eq!(resolved.parent(), Some(std::path::Path::new(&dir)));
    let name = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .expect("generated file name is not valid UTF-8");
    // The timestamp between prefix and suffix is deliberately not asserted.
    assert!(name.starts_with(&prefix), "{name:?} vs prefix {prefix:?}");
    assert!(name.ends_with(&suffix), "{name:?} vs suffix {suffix:?}");
}

// ===========================================================================
// Screen index resolution
// ===========================================================================

#[when(regex = r"^screen index (\d+|none) is resolved among (\d+) screens$")]
fn resolve_screen(world: &mut RustShotWorld, index: String, count: usize) {
    let requested = (index != "none").then(|| index.parse::<usize>().expect("index in spec"));
    world.screen_index_result = Some(capture::resolve_screen_index(count, requested));
}

#[then(regex = r"^the selected screen index is (\d+)$")]
fn assert_screen_index(world: &mut RustShotWorld, expected: usize) {
    let result = world
        .screen_index_result
        .as_ref()
        .expect("no screen index was resolved");
    assert_eq!(result.as_ref().ok(), Some(&expected), "got {result:?}");
}

#[then(regex = r#"^screen resolution fails with "([^"]+)"$"#)]
fn assert_screen_error(world: &mut RustShotWorld, message: String) {
    let result = world
        .screen_index_result
        .as_ref()
        .expect("no screen index was resolved");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(message.as_str())
    );
}

// ===========================================================================
// Checked dimension rounding and frame pixel conversion
// ===========================================================================

#[when(regex = r"^the dimension (\d+) is rounded to even with overflow checking$")]
fn checked_round(world: &mut RustShotWorld, value: u32) {
    world.checked_round_result = Some(record::checked_round_to_even(value));
}

#[then(regex = r"^the checked rounding yields (\d+)$")]
fn assert_checked_round(world: &mut RustShotWorld, expected: u32) {
    let result = world
        .checked_round_result
        .as_ref()
        .expect("checked rounding did not run");
    assert_eq!(result.as_ref().ok(), Some(&expected), "got {result:?}");
}

#[then(regex = r#"^the checked rounding fails with "([^"]+)"$"#)]
fn assert_checked_round_fails(world: &mut RustShotWorld, message: String) {
    let result = world
        .checked_round_result
        .as_ref()
        .expect("checked rounding did not run");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(message.as_str())
    );
}

#[when(regex = r"^a (\d+)x(\d+) RGBA frame is converted into a (\d+)-byte BGRA buffer$")]
fn convert_frame_into_buffer(world: &mut RustShotWorld, width: u32, height: u32, out_len: usize) {
    let rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut out = vec![0u8; out_len];
    world.bgra_conversion_result = Some(record::rgba_to_bgra_bottom_up_into(
        &rgba, &mut out, width, height,
    ));
}

#[then("the conversion succeeds")]
fn assert_conversion_succeeds(world: &mut RustShotWorld) {
    let result = world
        .bgra_conversion_result
        .as_ref()
        .expect("no conversion ran");
    assert!(result.is_ok(), "expected success, got {result:?}");
}

#[then(regex = r#"^the conversion fails mentioning "([^"]+)"$"#)]
fn assert_conversion_fails(world: &mut RustShotWorld, needle: String) {
    let result = world
        .bgra_conversion_result
        .as_ref()
        .expect("no conversion ran");
    let error = result
        .as_ref()
        .expect_err("expected the conversion to fail");
    assert!(
        error.contains(&needle),
        "{error:?} does not mention {needle:?}"
    );
}

// ===========================================================================
// MCP server protocol
// ===========================================================================

#[when("the MCP server receives:")]
fn mcp_receive(world: &mut RustShotWorld, step: &Step) {
    let message = step
        .docstring
        .as_ref()
        .expect("this step needs a docstring with the JSON-RPC message")
        .trim();
    world.mcp_reply = Some(mcp::handle_message(message));
}

#[then("the MCP server does not reply")]
fn mcp_no_reply(world: &mut RustShotWorld) {
    let reply = world.mcp_reply.as_ref().expect("no MCP message was sent");
    assert!(reply.is_none(), "expected silence, got {reply:?}");
}

fn mcp_reply_json(world: &RustShotWorld) -> Value {
    let reply = world
        .mcp_reply
        .as_ref()
        .expect("no MCP message was sent")
        .as_ref()
        .expect("the MCP server did not reply");
    serde_json::from_str(reply).expect("MCP reply is not valid JSON")
}

#[then(regex = r#"^the MCP reply at "([^"]+)" is (.+)$"#)]
fn assert_mcp_reply_at(world: &mut RustShotWorld, pointer: String, expected: String) {
    let reply = mcp_reply_json(world);
    let expected: Value =
        serde_json::from_str(&expected).expect("expected value in spec is not valid JSON");
    assert_eq!(
        reply.pointer(&pointer),
        Some(&expected),
        "reply: {reply}, pointer: {pointer}"
    );
}

#[when("the MCP server receives on a display-equipped machine:")]
fn mcp_receive_with_display(world: &mut RustShotWorld, step: &Step) {
    // Same guard as the live-capture suite: enumerating displays on a truly
    // headless unix box can panic inside the capture backend, so the scenario
    // self-skips there instead of crashing the whole run.
    let display_available = cfg!(windows)
        || std::env::var_os("DISPLAY").is_some()
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    if !display_available {
        eprintln!("skipping display-gated MCP scenario: no display server");
        world.mcp_display_skipped = true;
        return;
    }
    let message = step
        .docstring
        .as_ref()
        .expect("this step needs a docstring with the JSON-RPC message")
        .trim();
    world.mcp_reply = Some(mcp::handle_message(message));
}

#[then(regex = r#"^the MCP reply reports a tool error mentioning "([^"]+)"$"#)]
fn assert_mcp_tool_error(world: &mut RustShotWorld, needle: String) {
    if world.mcp_display_skipped {
        return;
    }
    let reply = mcp_reply_json(world);
    assert_eq!(
        reply.pointer("/result/isError"),
        Some(&Value::Bool(true)),
        "reply: {reply}"
    );
    let text = reply
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("no text content in {reply}"));
    assert!(
        text.contains(&needle),
        "{text:?} does not mention {needle:?}"
    );
}

#[then(regex = r#"^the MCP reply text at "([^"]+)" contains "([^"]+)"$"#)]
fn assert_mcp_reply_text(world: &mut RustShotWorld, pointer: String, needle: String) {
    let reply = mcp_reply_json(world);
    let text = reply
        .pointer(&pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("no string at {pointer} in {reply}"));
    assert!(
        text.contains(&needle),
        "{text:?} does not contain {needle:?}"
    );
}

// ===========================================================================
// Multi-monitor physical layout
// ===========================================================================

/// Data-table rows of a step, header row skipped.
fn table_rows(step: &Step) -> Vec<Vec<String>> {
    let table = step.table.as_ref().expect("this step needs a data table");
    table.rows.iter().skip(1).cloned().collect()
}

fn cell<T: std::str::FromStr>(row: &[String], index: usize) -> T
where
    T::Err: std::fmt::Debug,
{
    row[index]
        .parse()
        .unwrap_or_else(|error| panic!("bad table cell {:?}: {error:?}", row[index]))
}

#[given("the monitor layout:")]
fn monitor_layout(world: &mut RustShotWorld, step: &Step) {
    world.monitor_inputs = table_rows(step)
        .iter()
        .map(|row| {
            // 100% scale in these specs: physical size equals logical size.
            let width: u32 = cell(row, 2);
            let height: u32 = cell(row, 3);
            (
                cell(row, 0),
                cell(row, 1),
                width,
                height,
                width,
                height,
                row[4] == "yes",
            )
        })
        .collect();
}

#[when("the physical screen layout is built")]
fn build_layout(world: &mut RustShotWorld) {
    world.layout_result = Some(selection::benchmark_build_physical_screen_layouts(
        &world.monitor_inputs,
    ));
}

#[then("the physical layout is:")]
fn assert_layout(world: &mut RustShotWorld, step: &Step) {
    let expected: Vec<(i32, i32, u32, u32)> = table_rows(step)
        .iter()
        .map(|row| (cell(row, 0), cell(row, 1), cell(row, 2), cell(row, 3)))
        .collect();
    let actual = world
        .layout_result
        .as_ref()
        .expect("the layout was not built")
        .as_ref()
        .unwrap_or_else(|error| panic!("layout building failed: {error}"));
    assert_eq!(actual, &expected);
}

#[when(
    regex = r"^screen layout inputs are built for captured screen (\d+) sized (\d+)x(\d+) from:$"
)]
fn build_layout_inputs(
    world: &mut RustShotWorld,
    step: &Step,
    index: usize,
    width: u32,
    height: u32,
) {
    let screens: Vec<(i32, i32, u32, u32, f32, bool)> = table_rows(step)
        .iter()
        .map(|row| {
            (
                cell(row, 0),
                cell(row, 1),
                cell(row, 2),
                cell(row, 3),
                cell(row, 4),
                row[5] == "yes",
            )
        })
        .collect();
    world.layout_sizes_result = Some(
        selection::benchmark_build_screen_layout_inputs_physical_sizes(
            screens,
            index,
            (width, height),
        ),
    );
}

#[then(regex = r#"^building the layout inputs fails with "([^"]+)"$"#)]
fn assert_layout_inputs_fail(world: &mut RustShotWorld, message: String) {
    let result = world
        .layout_sizes_result
        .as_ref()
        .expect("layout inputs were not built");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(message.as_str())
    );
}

// ===========================================================================
// Default FPS and the active-window capture boundary
// ===========================================================================

#[when("the default FPS is requested")]
fn default_fps(world: &mut RustShotWorld) {
    world.normalized_fps = Some(settings::Fps::default().get());
}

#[when("the active window is captured into a temporary directory")]
fn capture_active_window(world: &mut RustShotWorld) {
    let dir = std::env::temp_dir().join(format!("rustshot_bdd_active_{}", std::process::id()));
    world.active_window_result = Some(capture::capture_active_window_in_dir(
        None,
        capture::Format::Png,
        &dir,
    ));
    world.active_window_dir = Some(dir);
}

#[then("the capture yields a saved image file or a human-readable error")]
fn assert_active_window_boundary(world: &mut RustShotWorld) {
    match world
        .active_window_result
        .as_ref()
        .expect("no active-window capture ran")
    {
        Ok(path) => {
            assert!(path.exists(), "the reported file must exist");
            let _ = std::fs::remove_file(path);
        }
        Err(error) => assert!(!error.is_empty(), "the error message must not be empty"),
    }
    if let Some(dir) = world.active_window_dir.take() {
        let _ = std::fs::remove_dir(dir);
    }
}

// ===========================================================================
// Overlay monitor targeting
// ===========================================================================

#[given("the connected monitors:")]
fn connected_monitors(world: &mut RustShotWorld, step: &Step) {
    world.connected_monitors = table_rows(step)
        .iter()
        .map(|row| ((cell(row, 0), cell(row, 1)), (cell(row, 2), cell(row, 3))))
        .collect();
}

#[when(regex = r"^the overlay targets origin \((-?\d+), (-?\d+)\) sized (\d+) x (\d+)$")]
fn overlay_targets(world: &mut RustShotWorld, x: i32, y: i32, width: u32, height: u32) {
    world.overlay_monitor = Some(selection::benchmark_target_overlay_monitor_index(
        &world.connected_monitors,
        (x, y),
        (width, height),
    ));
}

#[then(regex = r"^overlay monitor (\d+) is selected$")]
fn assert_overlay_monitor(world: &mut RustShotWorld, expected: usize) {
    let selected = world
        .overlay_monitor
        .expect("the overlay target was not resolved");
    assert_eq!(selected, Some(expected));
}

#[then("no overlay monitor is selected")]
fn assert_no_overlay_monitor(world: &mut RustShotWorld) {
    let selected = world
        .overlay_monitor
        .expect("the overlay target was not resolved");
    assert!(selected.is_none(), "expected no monitor, got {selected:?}");
}

// ===========================================================================
// Recording start boundary
// ===========================================================================

#[when("a recording start is requested for a zero-sized region")]
fn recording_start_zero_region(world: &mut RustShotWorld) {
    let region = selection::Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 0,
    };
    let dir = std::env::temp_dir().join("rustshot_bdd_zero_region");
    match record::RecordingSession::start_in_dir(0, 0, region, settings::Fps::new(30).unwrap(), dir)
    {
        Ok(session) => {
            // Should be unreachable; stop the runaway recorder before failing.
            let _ = session.stop();
            world.recording_start_error = None;
        }
        Err(error) => world.recording_start_error = Some(error),
    }
}

#[then("the request is rejected before any recorder is created")]
fn assert_recording_start_rejected(world: &mut RustShotWorld) {
    let error = world
        .recording_start_error
        .as_deref()
        .expect("the recording start was accepted");
    // The zero-size check runs on Windows; elsewhere the platform gate
    // rejects the request even earlier. Both are "no recorder was created".
    #[cfg(windows)]
    assert_eq!(error, "Recording region must have non-zero size");
    #[cfg(not(windows))]
    assert_eq!(error, "Recording is only supported on Windows");
}

#[then("the computed physical sizes are:")]
fn assert_layout_sizes(world: &mut RustShotWorld, step: &Step) {
    let expected: Vec<(u32, u32)> = table_rows(step)
        .iter()
        .map(|row| (cell(row, 0), cell(row, 1)))
        .collect();
    let actual = world
        .layout_sizes_result
        .as_ref()
        .expect("layout inputs were not built")
        .as_ref()
        .unwrap_or_else(|error| panic!("building layout inputs failed: {error}"));
    assert_eq!(actual, &expected);
}

// ===========================================================================
// Cross-platform live-capture E2E
// ===========================================================================

#[given(regex = r#"^the live capture scenario targets \"(linux|macos)\"$"#)]
fn target_live_capture_platform(world: &mut RustShotWorld, platform: String) {
    world.live_capture_active = std::env::consts::OS == platform;

    if world.live_capture_active
        && platform == "linux"
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("RUSTSHOT_LIVE_CAPTURE_STRICT").is_none()
    {
        eprintln!("skipping Linux live-capture scenario: no display server");
        world.live_capture_active = false;
    }
}

#[when("screen 0 is captured as PNG by the RustShot CLI with timing")]
fn capture_screen_zero_with_timing(world: &mut RustShotWorld) {
    if !world.live_capture_active {
        return;
    }

    let path = std::env::temp_dir().join(format!(
        "rustshot_bdd_capture_{}_{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_nanos()
    ));
    let started = std::time::Instant::now();
    let process = std::process::Command::new(env!("CARGO_BIN_EXE_RustShot"))
        .args(["--cli", "--screen", "0", "--format", "png", "--output"])
        .arg(&path)
        .output()
        .expect("failed to launch the RustShot CLI");

    world.live_capture_elapsed = Some(started.elapsed());
    world.live_capture_process = Some(process);
    world.live_capture_path = Some(path);
}

#[then("the captured PNG is a valid non-empty screen image")]
fn assert_live_capture_image(world: &mut RustShotWorld) {
    if !world.live_capture_active {
        return;
    }

    let process = world
        .live_capture_process
        .as_ref()
        .expect("the RustShot CLI was not launched");
    assert!(
        process.status.success(),
        "capture process failed (status {}): stdout={} stderr={}",
        process.status,
        String::from_utf8_lossy(&process.stdout),
        String::from_utf8_lossy(&process.stderr)
    );

    let path = world
        .live_capture_path
        .as_ref()
        .expect("the capture output path was not recorded");
    let metadata = std::fs::metadata(path).expect("the CLI did not create the screenshot file");
    assert!(metadata.len() > 100, "captured PNG is suspiciously small");
    let image = image::open(path).expect("the captured file is not a decodable image");
    assert!(image.width() > 0, "captured image width must be non-zero");
    assert!(image.height() > 0, "captured image height must be non-zero");
}

#[then(regex = r"^the timed capture completes within (\d+) milliseconds$")]
fn assert_live_capture_duration(world: &mut RustShotWorld, maximum_ms: u64) {
    if !world.live_capture_active {
        return;
    }

    let elapsed = world
        .live_capture_elapsed
        .expect("the capture duration was not recorded");
    eprintln!(
        "{} screen capture completed in {:.2} ms",
        std::env::consts::OS,
        elapsed.as_secs_f64() * 1_000.0
    );
    assert!(
        elapsed <= std::time::Duration::from_millis(maximum_ms),
        "capture took {:.2} ms, exceeding the {maximum_ms} ms E2E limit",
        elapsed.as_secs_f64() * 1_000.0
    );

    if let Some(path) = world.live_capture_path.take() {
        let _ = std::fs::remove_file(path);
    }
}

fn main() {
    futures::executor::block_on(
        RustShotWorld::cucumber()
            .fail_on_skipped()
            .run_and_exit("tests/features"),
    );
}
