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
    reparsed_settings: Option<settings::Settings>,

    // --- recording ---
    validate_result: Option<Result<(), String>>,
    frame_duration: Option<i64>,
    round_result: Option<u32>,
    encoded_dim_result: Option<u32>,
    padded_bgra_frame: Option<Vec<u8>>,

    // --- selection drag & overlay ---
    drag_start: (i32, i32),
    drag_end: (i32, i32),
    region: Option<Option<selection::Region>>,
    packed_dims: (u32, u32),
    packed_buffer: Vec<u32>,
    crop_result: Option<Result<image::RgbaImage, String>>,
    damage_rect_count: Option<usize>,

    // --- overlay frame rendering ---
    render_buffer: Option<Vec<u32>>,
    render_buf_w: usize,
    render_buf_h: usize,
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

#[when(
    regex = r#"^settings JSON with screenshots_dir "([^"]+)" and recording_fps (\d+) is parsed and formatted$"#
)]
fn parse_and_format_settings_json(world: &mut RustShotWorld, dir: String, fps: u32) {
    let escaped = dir.replace('\\', "\\\\");
    let raw_json = format!(
        r#"{{"screenshots_dir": "{escaped}", "recording_fps": {fps}, "default_format": "png", "auto_start": false}}"#
    );
    let parsed = settings::parse_settings_json(&raw_json);
    let formatted = settings::format_settings_json(&parsed);
    world.reparsed_settings = Some(settings::parse_settings_json(&formatted));
}

#[then(regex = r#"^the reparsed settings have screenshots_dir "([^"]+)" and recording_fps (\d+)$"#)]
fn assert_reparsed_settings(world: &mut RustShotWorld, expected_dir: String, expected_fps: u32) {
    let s = world
        .reparsed_settings
        .as_ref()
        .expect("settings JSON was not parsed");
    assert_eq!(s.screenshots_dir.to_string_lossy(), expected_dir);
    assert_eq!(s.recording_fps, expected_fps);
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

#[when(regex = r"^(\d+) is converted to an encoded dimension$")]
fn compute_encoded_dimension(world: &mut RustShotWorld, value: u32) {
    world.encoded_dim_result = Some(record::encoded_dimension(value));
}

#[then(regex = r"^the encoded dimension is (\d+)$")]
fn assert_encoded_dimension(world: &mut RustShotWorld, expected: u32) {
    let actual = world
        .encoded_dim_result
        .expect("encoded dimension did not run");
    assert_eq!(actual, expected);
}

#[when(
    regex = r"^a 1 x 1 RGBA pixel \((\d+), (\d+), (\d+), (\d+)\) is converted into a 2 x 2 bottom-up BGRA frame$"
)]
fn convert_padded_rgba_region(world: &mut RustShotWorld, r: u8, g: u8, b: u8, a: u8) {
    let rgba = [r, g, b, a];
    let mut out = vec![0u8; 2 * 2 * 4];
    record::rgba_region_to_bgra_bottom_up_into(&rgba, 1, 1, &mut out, 2, 2);
    world.padded_bgra_frame = Some(out);
}

#[then(
    regex = r"^the bottom-up BGRA top-left pixel is \((\d+), (\d+), (\d+), (\d+)\) and padding pixel is \((\d+), (\d+), (\d+), (\d+)\)$"
)]
#[allow(clippy::too_many_arguments)]
fn assert_padded_bgra_frame(
    world: &mut RustShotWorld,
    b0: u8,
    g0: u8,
    r0: u8,
    a0: u8,
    bp: u8,
    gp: u8,
    rp: u8,
    ap: u8,
) {
    let frame = world
        .padded_bgra_frame
        .as_ref()
        .expect("padded BGRA frame was not computed");
    // In a 2x2 bottom-up buffer, visual top-left (y=0, x=0) is stored at row 1, col 0 (offset 8..12).
    assert_eq!(&frame[8..12], &[b0, g0, r0, a0]);
    // Visual top-right padding pixel (y=0, x=1) is at offset 12..16, and bottom row (y=1) is at 0..8.
    assert_eq!(&frame[12..16], &[bp, gp, rp, ap]);
    assert_eq!(&frame[0..4], &[bp, gp, rp, ap]);
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

#[given(regex = r"^a (\d+) x (\d+) packed background buffer$")]
fn given_packed_buffer(world: &mut RustShotWorld, width: u32, height: u32) {
    world.packed_dims = (width, height);
    // Pixel (0,0) = 0x00102030, Pixel (1,0) = 0x00204060 (R=32, G=64, B=96), etc.
    world.packed_buffer = vec![0x0010_2030, 0x0020_4060, 0x0030_6090, 0x0040_80A0];
}

#[when(
    regex = r"^a region at \((-?\d+), (-?\d+)\) sized (\d+) x (\d+) is cropped from the packed buffer$"
)]
fn when_crop_packed(world: &mut RustShotWorld, x: i32, y: i32, width: u32, height: u32) {
    let region = selection::Region {
        physical_x: x,
        physical_y: y,
        physical_width: width,
        physical_height: height,
    };
    world.crop_result = Some(selection::crop_packed_region(
        world.packed_dims.0,
        world.packed_dims.1,
        &world.packed_buffer,
        region,
    ));
}

#[then(
    regex = r"^the cropped image is (\d+) x (\d+) with top-left pixel \((\d+), (\d+), (\d+), (\d+)\)$"
)]
fn then_cropped_image(
    world: &mut RustShotWorld,
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let img = world
        .crop_result
        .as_ref()
        .expect("crop did not run")
        .as_ref()
        .expect("expected crop to succeed");
    assert_eq!(img.dimensions(), (width, height));
    assert_eq!(img.get_pixel(0, 0).0, [r, g, b, a]);
}

#[then(regex = r#"^cropping fails with "([^"]+)"$"#)]
fn then_crop_fails(world: &mut RustShotWorld, expected: String) {
    let err = world
        .crop_result
        .as_ref()
        .expect("crop did not run")
        .as_ref()
        .expect_err("expected crop to fail");
    assert_eq!(err, &expected);
}

#[when(
    regex = r"^overlay damage rects are computed for a new (\d+) x (\d+) surface at cursor \((\d+), (\d+)\)$"
)]
fn when_damage_rects_new(world: &mut RustShotWorld, w: usize, h: usize, cx: i32, cy: i32) {
    let rects = selection::overlay_present_damage_rects(
        selection::OverlayDamage::default(),
        w,
        h,
        None,
        Some((cx, cy)),
    );
    world.damage_rect_count = Some(rects.len());
}

#[when(
    regex = r"^overlay damage rects are computed for an unchanged (\d+) x (\d+) surface at cursor \((\d+), (\d+)\)$"
)]
fn when_damage_rects_unchanged(world: &mut RustShotWorld, w: usize, h: usize, cx: i32, cy: i32) {
    let prev = selection::OverlayDamage {
        surface_size: Some((w, h)),
        start_pos: None,
        current_pos: Some((cx, cy)),
    };
    let rects = selection::overlay_present_damage_rects(prev, w, h, None, Some((cx, cy)));
    world.damage_rect_count = Some(rects.len());
}

#[then(regex = r"^(\d+) overlay damage rects? (?:is|are) produced$")]
fn then_damage_rect_count(world: &mut RustShotWorld, expected: usize) {
    let count = world
        .damage_rect_count
        .expect("overlay damage rects were not computed");
    assert_eq!(count, expected);
}

// ===========================================================================
// Overlay frame rendering (border clamping)
// ===========================================================================

/// Render a selection overlay into a zero-filled buffer and store it for
/// subsequent `Then` steps to inspect.
#[when(
    regex = r"^a (\d+) x (\d+) selection overlay is rendered with drag from \((-?\d+), (-?\d+)\) to \((-?\d+), (-?\d+)\)$"
)]
fn when_render_overlay_drag(
    world: &mut RustShotWorld,
    buf_w: usize,
    buf_h: usize,
    sx: i32,
    sy: i32,
    ex: i32,
    ey: i32,
) {
    let bg = vec![0u32; buf_w * buf_h];
    let mut buffer = bg.clone();
    selection::render_selection_overlay_frame(
        &mut buffer,
        buf_w,
        buf_h,
        &bg,
        buf_w,
        Some((sx, sy)),
        Some((ex, ey)),
    );
    world.render_buffer = Some(buffer);
    world.render_buf_w = buf_w;
    world.render_buf_h = buf_h;
}

#[then("the last row of the rendered buffer has at least one non-zero pixel")]
fn then_last_row_has_border(world: &mut RustShotWorld) {
    let buf = world
        .render_buffer
        .as_ref()
        .expect("overlay was not rendered");
    let w = world.render_buf_w;
    let h = world.render_buf_h;
    let start = (h - 1) * w;
    assert!(
        buf[start..start + w].iter().any(|&p| p != 0),
        "expected a border pixel in the last row but all pixels are zero"
    );
}

#[then("the last column of the rendered buffer has at least one non-zero pixel")]
fn then_last_col_has_border(world: &mut RustShotWorld) {
    let buf = world
        .render_buffer
        .as_ref()
        .expect("overlay was not rendered");
    let w = world.render_buf_w;
    let h = world.render_buf_h;
    assert!(
        (0..h).any(|y| buf[y * w + (w - 1)] != 0),
        "expected a border pixel in the last column but all pixels are zero"
    );
}

// ===========================================================================
// Overlay damage rects — cursor movement
// ===========================================================================

#[when(
    regex = r"^overlay damage rects are computed for a (\d+) x (\d+) surface with cursor moving from \((\d+), (\d+)\) to \((\d+), (\d+)\)$"
)]
fn when_damage_rects_cursor_moved(
    world: &mut RustShotWorld,
    w: usize,
    h: usize,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) {
    let prev = selection::OverlayDamage {
        surface_size: Some((w, h)),
        start_pos: None,
        current_pos: Some((x1, y1)),
    };
    let rects = selection::overlay_present_damage_rects(prev, w, h, None, Some((x2, y2)));
    world.damage_rect_count = Some(rects.len());
}

fn main() {
    futures::executor::block_on(
        RustShotWorld::cucumber()
            .fail_on_skipped()
            .run_and_exit("tests/features"),
    );
}
