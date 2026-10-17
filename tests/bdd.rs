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

    // --- capture module helpers ---
    pixel_buffer: Vec<u8>,
    screen_count: usize,
    requested_screen: Option<usize>,
    screen_index_result: Option<Result<usize, String>>,
    rgb_result: Option<Vec<u8>>,
    primary_flags: Vec<bool>,
    default_screen_result: Option<Option<usize>>,

    // --- selection helpers ---
    pack_rgba_result: Option<Vec<u32>>,
    scale_screens: Vec<(bool, f32)>,
    scale_factor_result: Option<Option<f32>>,
    logical_value: i32,
    scale_value: f32,
    physical_result: Option<i32>,
    current_surface_size: Option<(u32, u32)>,
    requested_surface: (u32, u32),
    next_size_result: Option<Option<(u32, u32)>>,

    // --- record helpers ---
    bgra_rgba: Vec<u8>,
    bgra_w: u32,
    bgra_h: u32,
    bgra_result: Option<Vec<u8>>,
    pack_hi: u32,
    pack_lo: u32,
    pack_u64_result: Option<u64>,
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

// ===========================================================================
// Capture module helpers
// ===========================================================================

#[given(regex = r"^a BGRA pixel buffer with bytes \((\d+), (\d+), (\d+), (\d+)\)$")]
fn given_bgra_buffer(world: &mut RustShotWorld, b: u8, g: u8, r: u8, a: u8) {
    world.pixel_buffer = vec![b, g, r, a];
}

#[given("an empty BGRA pixel buffer")]
fn given_empty_bgra_buffer(world: &mut RustShotWorld) {
    world.pixel_buffer = vec![];
}

#[given(regex = r"^a 1-pixel RGBA buffer with bytes \((\d+), (\d+), (\d+), (\d+)\)$")]
fn given_rgba_1px(world: &mut RustShotWorld, r: u8, g: u8, b: u8, a: u8) {
    world.pixel_buffer = vec![r, g, b, a];
}

#[given(regex = r"^an RGBA buffer of (\d+) pixels all set to \((\d+), (\d+), (\d+), (\d+)\)$")]
fn given_rgba_n_pixels(world: &mut RustShotWorld, n: usize, r: u8, g: u8, b: u8, a: u8) {
    world.pixel_buffer = vec![r, g, b, a].into_iter().cycle().take(n * 4).collect();
}

#[given("an empty RGBA pixel buffer")]
fn given_empty_rgba(world: &mut RustShotWorld) {
    world.pixel_buffer = vec![];
}

#[given("a 5-byte RGBA buffer")]
fn given_5byte_rgba(world: &mut RustShotWorld) {
    world.pixel_buffer = vec![10, 20, 30, 255, 99];
}

#[when("the buffer is converted from BGRA to RGBA")]
fn when_bgra_to_rgba(world: &mut RustShotWorld) {
    capture::bgra_to_rgba(&mut world.pixel_buffer);
}

#[when("the RGBA buffer is converted to RGB")]
fn when_rgba_to_rgb(world: &mut RustShotWorld) {
    world.rgb_result = Some(capture::rgba_to_rgb_bytes(&world.pixel_buffer));
}

#[then(regex = r"^the pixel buffer contains bytes \((\d+), (\d+), (\d+), (\d+)\)$")]
fn then_pixel_buffer_bytes(world: &mut RustShotWorld, b0: u8, b1: u8, b2: u8, b3: u8) {
    assert_eq!(world.pixel_buffer, vec![b0, b1, b2, b3]);
}

#[then("the pixel buffer is empty")]
fn then_pixel_buffer_empty(world: &mut RustShotWorld) {
    assert!(world.pixel_buffer.is_empty());
}

#[then(regex = r"^the RGB output contains bytes \((\d+), (\d+), (\d+)\)$")]
fn then_rgb_bytes(world: &mut RustShotWorld, r: u8, g: u8, b: u8) {
    let out = world
        .rgb_result
        .as_ref()
        .expect("RGB conversion did not run");
    assert_eq!(out.as_slice(), &[r, g, b]);
}

#[then(regex = r"^the RGB output has (\d+) bytes$")]
fn then_rgb_len(world: &mut RustShotWorld, expected: usize) {
    let out = world
        .rgb_result
        .as_ref()
        .expect("RGB conversion did not run");
    assert_eq!(out.len(), expected);
}

// --- resolve_screen_index ---

#[given(regex = r"^there are (\d+) screens available$")]
fn given_screen_count(world: &mut RustShotWorld, n: usize) {
    world.screen_count = n;
}

#[when("no screen index is requested")]
fn when_no_screen_index(world: &mut RustShotWorld) {
    world.screen_index_result = Some(capture::resolve_screen_index(world.screen_count, None));
}

#[when(regex = r"^screen index (\d+) is requested$")]
fn when_screen_index_n(world: &mut RustShotWorld, index: usize) {
    world.requested_screen = Some(index);
    world.screen_index_result = Some(capture::resolve_screen_index(
        world.screen_count,
        Some(index),
    ));
}

#[then(regex = r"^screen index (\d+) is resolved$")]
fn then_screen_index_is(world: &mut RustShotWorld, expected: usize) {
    let result = world
        .screen_index_result
        .as_ref()
        .expect("screen index was not resolved");
    assert_eq!(result.as_ref().copied().unwrap(), expected);
}

#[then("screen index resolution fails")]
fn then_screen_index_fails(world: &mut RustShotWorld) {
    let result = world
        .screen_index_result
        .as_ref()
        .expect("screen index was not resolved");
    assert!(result.is_err(), "expected error but got {:?}", result);
}

// --- default_screen_index ---

#[given(regex = r"^the screen primary flags are \[([^\]]*)\]$")]
fn given_primary_flags(world: &mut RustShotWorld, flags_str: String) {
    world.primary_flags = flags_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s == "true")
        .collect();
}

#[when("the default screen index is computed")]
fn when_default_screen_index(world: &mut RustShotWorld) {
    world.default_screen_result = Some(capture::default_screen_index(world.primary_flags.clone()));
}

#[then(regex = r"^the default screen index is (\d+)$")]
fn then_default_screen_is(world: &mut RustShotWorld, expected: usize) {
    let result = world
        .default_screen_result
        .expect("default screen index was not computed");
    assert_eq!(result, Some(expected));
}

#[then("no default screen index exists")]
fn then_no_default_screen(world: &mut RustShotWorld) {
    let result = world
        .default_screen_result
        .expect("default screen index was not computed");
    assert!(result.is_none());
}

// ===========================================================================
// Selection helpers
// ===========================================================================

// --- pack_rgba_to_softbuffer ---

#[when("the RGBA buffer is packed to softbuffer format")]
fn when_pack_rgba(world: &mut RustShotWorld) {
    world.pack_rgba_result = Some(selection::pack_rgba_to_softbuffer(&world.pixel_buffer));
}

#[then(regex = r"^the packed buffer has (\d+) entr(?:y|ies)$")]
fn then_packed_len(world: &mut RustShotWorld, expected: usize) {
    let packed = world
        .pack_rgba_result
        .as_ref()
        .expect("RGBA packing did not run");
    assert_eq!(packed.len(), expected);
}

#[then(regex = r"^the first packed entry is (0x[0-9A-Fa-f]+)$")]
fn then_first_packed_entry(world: &mut RustShotWorld, hex_str: String) {
    let packed = world
        .pack_rgba_result
        .as_ref()
        .expect("RGBA packing did not run");
    let expected = u32::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap();
    assert_eq!(packed[0], expected, "packed[0] = 0x{:08X}", packed[0]);
}

// --- virtual_desktop_scale_factor ---

#[given(regex = r"^the virtual desktop monitors are \[([^\]]*)\]$")]
fn given_scale_screens(world: &mut RustShotWorld, list_str: String) {
    world.scale_screens = list_str
        .split("), (")
        .map(|s| s.trim_matches(|c| c == '(' || c == ')' || c == ' '))
        .filter(|s| !s.is_empty())
        .map(|entry| {
            let parts: Vec<&str> = entry.splitn(2, ',').collect();
            let is_primary = parts[0].trim() == "true";
            let scale: f32 = parts[1].trim().parse().unwrap();
            (is_primary, scale)
        })
        .collect();
}

#[when("the virtual desktop scale factor is computed")]
fn when_scale_factor(world: &mut RustShotWorld) {
    world.scale_factor_result = Some(selection::virtual_desktop_scale_factor(
        world.scale_screens.clone(),
    ));
}

#[then(regex = r"^the scale factor is (\d+(?:\.\d+)?)$")]
fn then_scale_factor(world: &mut RustShotWorld, expected: f32) {
    let result = world
        .scale_factor_result
        .expect("scale factor was not computed");
    assert_eq!(result, Some(expected));
}

#[then("no scale factor exists")]
fn then_no_scale_factor(world: &mut RustShotWorld) {
    let result = world
        .scale_factor_result
        .expect("scale factor was not computed");
    assert!(result.is_none());
}

// --- logical_to_physical ---

#[given(regex = r"^the logical value is (-?\d+) at scale factor (\d+(?:\.\d+)?)$")]
fn given_logical_value(world: &mut RustShotWorld, value: i32, scale: f32) {
    world.logical_value = value;
    world.scale_value = scale;
}

#[when("the logical value is converted to physical pixels")]
fn when_logical_to_physical(world: &mut RustShotWorld) {
    world.physical_result = Some(selection::logical_to_physical(
        world.logical_value,
        world.scale_value,
    ));
}

#[then(regex = r"^the physical pixel value is (-?\d+)$")]
fn then_physical_value(world: &mut RustShotWorld, expected: i32) {
    let result = world
        .physical_result
        .expect("logical_to_physical did not run");
    assert_eq!(result, expected);
}

// --- next_surface_size ---

#[given("the current surface size is not set")]
fn given_no_surface_size(world: &mut RustShotWorld) {
    world.current_surface_size = None;
}

#[given(regex = r"^the current surface size is (\d+) x (\d+)$")]
fn given_surface_size(world: &mut RustShotWorld, w: u32, h: u32) {
    world.current_surface_size = Some((w, h));
}

#[when(regex = r"^the next surface size of (\d+) x (\d+) is requested$")]
fn when_next_surface(world: &mut RustShotWorld, w: u32, h: u32) {
    world.requested_surface = (w, h);
    let result = selection::next_surface_size(world.current_surface_size, w, h);
    world.next_size_result = Some(result.map(|(nw, nh)| (nw.get(), nh.get())));
}

#[then(regex = r"^a surface resize to (\d+) x (\d+) is needed$")]
fn then_resize_needed(world: &mut RustShotWorld, w: u32, h: u32) {
    let result = world
        .next_size_result
        .expect("next_surface_size did not run");
    assert_eq!(result, Some((w, h)), "expected resize to {w}x{h}");
}

#[then("no surface resize is needed")]
fn then_no_resize(world: &mut RustShotWorld) {
    let result = world
        .next_size_result
        .expect("next_surface_size did not run");
    assert!(result.is_none(), "expected no resize but got {result:?}");
}

// ===========================================================================
// Record helpers
// ===========================================================================

// --- rgba_to_bgra_bottom_up ---

#[given(regex = r"^a 1x1 RGBA frame with pixel \((\d+), (\d+), (\d+), (\d+)\)$")]
fn given_1x1_rgba(world: &mut RustShotWorld, r: u8, g: u8, b: u8, a: u8) {
    world.bgra_rgba = vec![r, g, b, a];
    world.bgra_w = 1;
    world.bgra_h = 1;
}

#[given(
    regex = r"^a 1x2 RGBA frame with top pixel \((\d+), (\d+), (\d+), (\d+)\) and bottom pixel \((\d+), (\d+), (\d+), (\d+)\)$"
)]
#[allow(clippy::too_many_arguments)]
fn given_1x2_rgba(
    world: &mut RustShotWorld,
    r0: u8,
    g0: u8,
    b0: u8,
    a0: u8,
    r1: u8,
    g1: u8,
    b1: u8,
    a1: u8,
) {
    world.bgra_rgba = vec![r0, g0, b0, a0, r1, g1, b1, a1];
    world.bgra_w = 1;
    world.bgra_h = 2;
}

#[given("a 2x2 RGBA frame with all zero pixels")]
fn given_2x2_zero_rgba(world: &mut RustShotWorld) {
    world.bgra_rgba = vec![0u8; 16];
    world.bgra_w = 2;
    world.bgra_h = 2;
}

#[when("the RGBA frame is converted to bottom-up BGRA")]
fn when_rgba_to_bgra_bottom_up(world: &mut RustShotWorld) {
    world.bgra_result = Some(record::rgba_to_bgra_bottom_up(
        &world.bgra_rgba,
        world.bgra_w,
        world.bgra_h,
    ));
}

#[when(regex = r"^the BGRA frame of size (\d+)x(\d+) is converted to bottom-up BGRA again$")]
fn when_bgra_to_bgra_again(world: &mut RustShotWorld, w: u32, h: u32) {
    let src = world
        .bgra_result
        .take()
        .expect("no BGRA result to re-convert");
    world.bgra_result = Some(record::rgba_to_bgra_bottom_up(&src, w, h));
}

#[then(regex = r"^the BGRA output pixel is \((\d+), (\d+), (\d+), (\d+)\)$")]
fn then_bgra_pixel(world: &mut RustShotWorld, b: u8, g: u8, r: u8, a: u8) {
    let out = world
        .bgra_result
        .as_ref()
        .expect("BGRA conversion did not run");
    assert_eq!(&out[..4], &[b, g, r, a]);
}

#[then(regex = r"^the bottom-up BGRA first pixel is \((\d+), (\d+), (\d+), (\d+)\)$")]
fn then_bgra_first_pixel(world: &mut RustShotWorld, b: u8, g: u8, r: u8, a: u8) {
    let out = world
        .bgra_result
        .as_ref()
        .expect("BGRA conversion did not run");
    assert_eq!(&out[..4], &[b, g, r, a], "first pixel");
}

#[then(regex = r"^the bottom-up BGRA second pixel is \((\d+), (\d+), (\d+), (\d+)\)$")]
fn then_bgra_second_pixel(world: &mut RustShotWorld, b: u8, g: u8, r: u8, a: u8) {
    let out = world
        .bgra_result
        .as_ref()
        .expect("BGRA conversion did not run");
    assert_eq!(&out[4..8], &[b, g, r, a], "second pixel");
}

#[then("the double-converted output matches the original RGBA frame")]
fn then_double_converted_matches(world: &mut RustShotWorld) {
    let out = world
        .bgra_result
        .as_ref()
        .expect("BGRA conversion did not run");
    assert_eq!(out, &world.bgra_rgba);
}

#[then(regex = r"^the BGRA output has (\d+) bytes$")]
fn then_bgra_len(world: &mut RustShotWorld, expected: usize) {
    let out = world
        .bgra_result
        .as_ref()
        .expect("BGRA conversion did not run");
    assert_eq!(out.len(), expected);
}

// --- pack_u32_pair ---

#[given(regex = r"^the u32 pair \((\d+), (\d+)\)$")]
fn given_u32_pair(world: &mut RustShotWorld, hi: u32, lo: u32) {
    world.pack_hi = hi;
    world.pack_lo = lo;
}

#[when("the pair is packed into a u64")]
fn when_pack_u32(world: &mut RustShotWorld) {
    world.pack_u64_result = Some(record::pack_u32_pair(world.pack_hi, world.pack_lo));
}

#[then(regex = r"^the high word of the packed u64 is (\d+)$")]
fn then_packed_hi(world: &mut RustShotWorld, expected: u32) {
    let packed = world.pack_u64_result.expect("pack_u32_pair did not run");
    assert_eq!((packed >> 32) as u32, expected);
}

#[then(regex = r"^the low word of the packed u64 is (\d+)$")]
fn then_packed_lo(world: &mut RustShotWorld, expected: u32) {
    let packed = world.pack_u64_result.expect("pack_u32_pair did not run");
    assert_eq!(packed as u32, expected);
}

#[then(regex = r"^the packed u64 is (\d+)$")]
fn then_packed_u64(world: &mut RustShotWorld, expected: u64) {
    let packed = world.pack_u64_result.expect("pack_u32_pair did not run");
    assert_eq!(packed, expected);
}

fn main() {
    futures::executor::block_on(
        RustShotWorld::cucumber()
            .fail_on_skipped()
            .run_and_exit("tests/features"),
    );
}
