use crate::{capture, record, settings, RustShotWorld};
use cucumber::{then, when};

fn parse_bytes(spec: &str) -> Vec<u8> {
    if spec.is_empty() {
        return Vec::new();
    }
    spec.split(',')
        .map(|value| value.parse::<u8>().expect("invalid byte in feature"))
        .collect()
}

#[when(regex = r"^an FPS value is strictly constructed from (\d+)$")]
fn construct_fps(world: &mut RustShotWorld, value: u32) {
    world.strict_fps_result = Some(settings::Fps::try_from(value).map(settings::Fps::get));
}

#[then(regex = r"^the strict FPS value is (\d+)$")]
fn assert_strict_fps(world: &mut RustShotWorld, expected: u32) {
    let result = world
        .strict_fps_result
        .as_ref()
        .expect("strict FPS construction did not run");
    assert_eq!(result.as_ref().ok(), Some(&expected));
}

#[then(regex = r#"^strict FPS construction fails with \"([^\"]+)\"$"#)]
fn assert_strict_fps_error(world: &mut RustShotWorld, expected: String) {
    let result = world
        .strict_fps_result
        .as_ref()
        .expect("strict FPS construction did not run");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(expected.as_str())
    );
}

#[when(regex = r"^recording dimensions (\d+) x (\d+) are validated$")]
fn validate_dimensions(world: &mut RustShotWorld, width: u32, height: u32) {
    world.recording_dimensions_result = Some(record::benchmark_recording_dimensions(width, height));
}

#[then(regex = r"^output dimensions are (\d+) x (\d+) with (\d+) RGBA bytes$")]
fn assert_dimensions(world: &mut RustShotWorld, width: u32, height: u32, rgba_bytes: usize) {
    let result = world
        .recording_dimensions_result
        .as_ref()
        .expect("recording dimensions were not validated");
    assert_eq!(result.as_ref().ok(), Some(&(width, height, rgba_bytes)));
}

#[then(regex = r#"^recording dimensions fail with \"([^\"]+)\"$"#)]
fn assert_dimension_error(world: &mut RustShotWorld, expected: String) {
    let result = world
        .recording_dimensions_result
        .as_ref()
        .expect("recording dimensions were not validated");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(expected.as_str())
    );
}

#[when(regex = r"^timing for frame (\d+) at (\d+) FPS is computed$")]
fn compute_exact_timing(world: &mut RustShotWorld, frame: u64, fps: u32) {
    world.frame_timing_result = Some(record::benchmark_frame_timing_hns(frame, fps));
}

#[then(regex = r"^the frame timestamp is (\d+) and duration is (\d+) hundred-nanosecond units$")]
fn assert_exact_timing(world: &mut RustShotWorld, timestamp: i64, duration: i64) {
    let result = world
        .frame_timing_result
        .as_ref()
        .expect("frame timing was not computed");
    assert_eq!(result.as_ref().ok(), Some(&(timestamp, duration)));
}

#[when(regex = r"^bitrate for (\d+) x (\d+) at (\d+) FPS is computed$")]
fn compute_bitrate(world: &mut RustShotWorld, width: u32, height: u32, fps: u32) {
    world.bitrate_result = Some(record::benchmark_recording_bitrate(width, height, fps));
}

#[then(regex = r"^the recording bitrate is (\d+) bits per second$")]
fn assert_bitrate(world: &mut RustShotWorld, expected: u32) {
    let result = world
        .bitrate_result
        .as_ref()
        .expect("recording bitrate was not computed");
    assert_eq!(result.as_ref().ok(), Some(&expected));
}

#[when(
    regex = r#"^RGBA pixels \"([0-9,]*)\" sized (\d+) x (\d+) are converted to bottom-up BGRA$"#
)]
fn convert_rgba_pixels(world: &mut RustShotWorld, pixels: String, width: u32, height: u32) {
    world.converted_pixels = Some(record::rgba_to_bgra_bottom_up(
        &parse_bytes(&pixels),
        width,
        height,
    ));
}

#[when(regex = r#"^RGBA pixels \"([0-9,]*)\" sized (\d+) x (\d+) are converted twice$"#)]
fn convert_rgba_twice(world: &mut RustShotWorld, pixels: String, width: u32, height: u32) {
    world.converted_pixels = Some(
        record::rgba_to_bgra_bottom_up(&parse_bytes(&pixels), width, height)
            .and_then(|once| record::rgba_to_bgra_bottom_up(&once, width, height)),
    );
}

#[when(regex = r"^empty RGBA pixels sized (\d+) x (\d+) are converted to bottom-up BGRA$")]
fn convert_empty_rgba(world: &mut RustShotWorld, width: u32, height: u32) {
    world.converted_pixels = Some(record::rgba_to_bgra_bottom_up(&[], width, height));
}

#[when(regex = r#"^screenshot BGRA pixels \"([0-9,]+)\" are converted to RGBA$"#)]
fn convert_screenshot_bgra(world: &mut RustShotWorld, pixels: String) {
    let mut pixels = parse_bytes(&pixels);
    capture::bgra_to_rgba(&mut pixels);
    world.converted_pixels = Some(Ok(pixels));
}

#[then(regex = r#"^the converted pixels are \"([0-9,]*)\"$"#)]
fn assert_converted_pixels(world: &mut RustShotWorld, expected: String) {
    let actual = world
        .converted_pixels
        .as_ref()
        .expect("pixel conversion did not run")
        .as_ref()
        .unwrap_or_else(|error| panic!("pixel conversion failed: {error}"));
    assert_eq!(actual, &parse_bytes(&expected));
}

#[then(regex = r#"^the converted alpha values are \"([0-9,]+)\"$"#)]
fn assert_converted_alphas(world: &mut RustShotWorld, expected: String) {
    let actual = world
        .converted_pixels
        .as_ref()
        .expect("pixel conversion did not run")
        .as_ref()
        .unwrap_or_else(|error| panic!("pixel conversion failed: {error}"));
    let alphas: Vec<u8> = actual.chunks_exact(4).map(|pixel| pixel[3]).collect();
    assert_eq!(alphas, parse_bytes(&expected));
}

#[then("the converted pixel buffer is empty")]
fn assert_empty_conversion(world: &mut RustShotWorld) {
    let actual = world
        .converted_pixels
        .as_ref()
        .expect("pixel conversion did not run")
        .as_ref()
        .unwrap_or_else(|error| panic!("pixel conversion failed: {error}"));
    assert!(actual.is_empty());
}

#[when(regex = r"^values (\d+) and (\d+) are packed into one u64$")]
fn pack_values(world: &mut RustShotWorld, high: u32, low: u32) {
    world.packed_pair = Some(record::pack_u32_pair(high, low));
}

#[then(regex = r"^unpacking yields (\d+) and (\d+)$")]
fn assert_unpacked_values(world: &mut RustShotWorld, high: u32, low: u32) {
    let packed = world.packed_pair.expect("values were not packed");
    assert_eq!((packed >> 32) as u32, high);
    assert_eq!(packed as u32, low);
}
