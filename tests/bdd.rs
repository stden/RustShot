//! Executable BDD specifications for RustShot.
//!
//! The Gherkin feature files under `tests/features/` are the specification;
//! the step definitions below drive RustShot's public pure-logic API and
//! assert the documented behavior. Run with `cargo test --test bdd`.

use cucumber::{given, then, when, World};
use rustshot::{capture, cli, record, selection, settings};

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

    // --- format resolution ---
    format_from_ext: Option<Option<capture::Format>>,

    // --- settings normalization ---
    normalized_format: Option<String>,
    normalized_fps: Option<u32>,

    // --- recording ---
    validate_result: Option<Result<(), String>>,
    frame_duration: Option<i64>,
    round_result: Option<u32>,

    // --- selection drag ---
    drag_start: (i32, i32),
    drag_end: (i32, i32),
    region: Option<Option<selection::Region>>,
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
    world.normalized_format = Some(settings::normalize_default_format(&input).to_string());
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
    world.normalized_fps = Some(settings::normalize_recording_fps(input));
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

fn main() {
    futures::executor::block_on(
        RustShotWorld::cucumber()
            .fail_on_skipped()
            .run_and_exit("tests/features"),
    );
}
