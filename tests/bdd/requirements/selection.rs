use crate::{cell, selection, table_rows, PhysicalRect, RustShotWorld};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};

#[when(regex = r"^range (-?\d+) sized (\d+) is compared with range (-?\d+) sized (\d+)$")]
fn compare_ranges(world: &mut RustShotWorld, start_a: i32, size_a: u32, start_b: i32, size_b: u32) {
    world.ranges_overlap = Some(selection::benchmark_ranges_overlap(
        start_a, size_a, start_b, size_b,
    ));
}

#[then(regex = r"^the ranges (do|do not) overlap$")]
fn assert_ranges_overlap(world: &mut RustShotWorld, verdict: String) {
    assert_eq!(
        world.ranges_overlap.expect("ranges were not compared"),
        verdict == "do"
    );
}

#[when(
    regex = r"^the monitor scale ratio for (\d+) logical and (\d+) physical pixels is computed$"
)]
fn compute_monitor_scale(world: &mut RustShotWorld, logical: u32, physical: u32) {
    world.monitor_scale_ratio = Some(selection::benchmark_monitor_scale_ratio(logical, physical));
}

#[then(regex = r"^the monitor scale ratio is (-?\d+(?:\.\d+)?)$")]
fn assert_monitor_scale(world: &mut RustShotWorld, expected: f32) {
    let actual = world
        .monitor_scale_ratio
        .expect("monitor scale was not computed");
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "{actual} != {expected}"
    );
}

#[when(regex = r"^logical gap (-?\d+) is scaled from (\d+) to (\d+) pixels$")]
fn scale_gap(world: &mut RustShotWorld, gap: i32, logical: u32, physical: u32) {
    world.scaled_gap = Some(selection::benchmark_scaled_gap(gap, logical, physical));
}

#[then(regex = r"^the scaled gap is (-?\d+)$")]
fn assert_scaled_gap(world: &mut RustShotWorld, expected: i32) {
    assert_eq!(world.scaled_gap.expect("gap was not scaled"), expected);
}

#[when(regex = r"^logical coordinate (-?\d+) is converted at scale (-?\d+(?:\.\d+)?)$")]
fn convert_logical_coordinate(world: &mut RustShotWorld, value: i32, scale: f32) {
    world.physical_coordinate = Some(selection::logical_to_physical(value, scale));
}

#[then(regex = r"^the physical coordinate is (-?\d+)$")]
fn assert_physical_coordinate(world: &mut RustShotWorld, expected: i32) {
    assert_eq!(
        world
            .physical_coordinate
            .expect("logical coordinate was not converted"),
        expected
    );
}

#[when(
    regex = r"^logical point \((-?\d+), (-?\d+)\) is converted through scale factor (-?\d+(?:\.\d+)?)$"
)]
fn convert_logical_point(world: &mut RustShotWorld, x: i32, y: i32, scale: f32) {
    world.typed_physical_point = Some(
        selection::ScaleFactor::try_from(scale)
            .map(|factor| selection::LogicalPoint::new(x, y).to_physical(factor)),
    );
}

#[then(regex = r"^the typed physical point is \((-?\d+), (-?\d+)\)$")]
fn assert_typed_physical_point(world: &mut RustShotWorld, x: i32, y: i32) {
    let point = world
        .typed_physical_point
        .as_ref()
        .expect("logical point was not converted")
        .as_ref()
        .unwrap_or_else(|error| panic!("logical point conversion failed: {error}"));
    assert_eq!((point.x(), point.y()), (x, y));
}

#[when(regex = r"^logical size (\d+) x (\d+) is converted through scale factor (-?\d+(?:\.\d+)?)$")]
fn convert_logical_size(world: &mut RustShotWorld, width: u32, height: u32, scale: f32) {
    world.typed_physical_size = Some(
        selection::ScaleFactor::try_from(scale)
            .and_then(|factor| selection::LogicalSize::new(width, height).to_physical(factor)),
    );
}

#[then(regex = r"^the typed physical size is (\d+) x (\d+)$")]
fn assert_typed_physical_size(world: &mut RustShotWorld, width: u32, height: u32) {
    let size = world
        .typed_physical_size
        .as_ref()
        .expect("logical size was not converted")
        .as_ref()
        .unwrap_or_else(|error| panic!("logical size conversion failed: {error}"));
    assert_eq!((size.width(), size.height()), (width, height));
}

fn parse_scale_factor(value: &str) -> f32 {
    match value {
        "NaN" => f32::NAN,
        "+infinity" => f32::INFINITY,
        "-infinity" => f32::NEG_INFINITY,
        _ => value
            .parse()
            .unwrap_or_else(|error| panic!("invalid scale factor {value:?}: {error}")),
    }
}

#[when(regex = r#"^scale factor \"([^\"]+)\" is validated$"#)]
fn validate_scale_factor(world: &mut RustShotWorld, value: String) {
    world.scale_factor_result = Some(selection::ScaleFactor::try_from(parse_scale_factor(&value)));
}

#[then(regex = r#"^scale factor validation fails with \"([^\"]+)\"$"#)]
fn assert_scale_factor_error(world: &mut RustShotWorld, expected: String) {
    let error = world
        .scale_factor_result
        .as_ref()
        .expect("scale factor was not validated")
        .as_ref()
        .expect_err("scale factor unexpectedly passed validation");
    assert_eq!(error, &expected);
}

#[given("the mixed-DPI monitor layout:")]
fn mixed_dpi_layout(world: &mut RustShotWorld, step: &Step) {
    world.monitor_inputs = table_rows(step)
        .iter()
        .map(|row| {
            (
                cell(row, 0),
                cell(row, 1),
                cell(row, 2),
                cell(row, 3),
                cell(row, 4),
                cell(row, 5),
                row[6] == "yes",
            )
        })
        .collect();
}

#[given("no monitor layout entries")]
fn no_monitor_layout(world: &mut RustShotWorld) {
    world.monitor_inputs.clear();
}

#[then(regex = r#"^building the physical layout fails mentioning \"([^\"]+)\"$"#)]
fn assert_layout_failure(world: &mut RustShotWorld, expected: String) {
    let error = world
        .layout_result
        .as_ref()
        .expect("layout building did not run")
        .as_ref()
        .expect_err("layout building unexpectedly succeeded");
    assert!(error.contains(&expected), "{error:?} lacks {expected:?}");
}

#[given("the resolved physical monitor layout:")]
fn resolved_physical_layout(world: &mut RustShotWorld, step: &Step) {
    world.resolved_layout_inputs = table_rows(step)
        .iter()
        .map(|row| {
            (
                cell(row, 0),
                cell(row, 1),
                cell(row, 2),
                cell(row, 3),
                cell(row, 4),
                cell(row, 5),
                cell(row, 6),
                cell(row, 7),
            )
        })
        .collect();
}

#[when("the resolved physical layout is normalized")]
fn normalize_resolved_layout(world: &mut RustShotWorld) {
    world.normalized_layout_result = Some(
        selection::benchmark_normalize_resolved_physical_layouts(&world.resolved_layout_inputs),
    );
}

fn normalized_layout(world: &RustShotWorld) -> &(Vec<PhysicalRect>, i32, i32, u32, u32) {
    world
        .normalized_layout_result
        .as_ref()
        .expect("layout normalization did not run")
        .as_ref()
        .unwrap_or_else(|error| panic!("layout normalization failed: {error}"))
}

#[then(regex = r"^its original physical origin is \((-?\d+), (-?\d+)\)$")]
fn assert_original_origin(world: &mut RustShotWorld, x: i32, y: i32) {
    let (_, actual_x, actual_y, _, _) = normalized_layout(world);
    assert_eq!((*actual_x, *actual_y), (x, y));
}

#[then(regex = r"^its normalized desktop size is (\d+) x (\d+)$")]
fn assert_normalized_size(world: &mut RustShotWorld, width: u32, height: u32) {
    let (_, _, _, actual_width, actual_height) = normalized_layout(world);
    assert_eq!((*actual_width, *actual_height), (width, height));
}

#[then("the normalized physical layout is:")]
fn assert_normalized_layout(world: &mut RustShotWorld, step: &Step) {
    let expected: Vec<PhysicalRect> = table_rows(step)
        .iter()
        .map(|row| (cell(row, 0), cell(row, 1), cell(row, 2), cell(row, 3)))
        .collect();
    assert_eq!(normalized_layout(world).0, expected);
}

#[given("the physical monitors:")]
fn physical_monitors(world: &mut RustShotWorld, step: &Step) {
    world.physical_monitors = table_rows(step)
        .iter()
        .map(|row| {
            (
                cell(row, 0),
                cell(row, 1),
                cell(row, 2),
                cell(row, 3),
                row[4] == "yes",
            )
        })
        .collect();
}

#[when(regex = r"^recording region \((-?\d+), (-?\d+)\) sized (\d+) x (\d+) is resolved$")]
fn resolve_recording_region(world: &mut RustShotWorld, x: i32, y: i32, width: u32, height: u32) {
    let layouts: Vec<_> = world
        .physical_monitors
        .iter()
        .map(|&(x, y, width, height, _)| (x, y, width, height, x, y, width, height))
        .collect();
    world.recording_selection_result = Some(selection::benchmark_resolve_recording_selection(
        &layouts,
        (x, y, width, height),
    ));
}

#[then(regex = r#"^recording selection fails mentioning \"([^\"]+)\"$"#)]
fn assert_recording_selection_failure(world: &mut RustShotWorld, expected: String) {
    let error = world
        .recording_selection_result
        .as_ref()
        .expect("recording selection was not resolved")
        .as_ref()
        .expect_err("recording selection unexpectedly succeeded");
    assert!(error.contains(&expected), "{error:?} lacks {expected:?}");
}

#[then(
    regex = r"^monitor (\d+) is selected with local region \((-?\d+), (-?\d+)\) sized (\d+) x (\d+)$"
)]
fn assert_local_recording_region(
    world: &mut RustShotWorld,
    monitor: usize,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    let actual = world
        .recording_selection_result
        .as_ref()
        .expect("recording selection was not resolved")
        .as_ref()
        .unwrap_or_else(|error| panic!("recording selection failed: {error}"));
    assert_eq!(*actual, (monitor, (x, y, width, height)));
}

#[when(
    regex = r"^the cursor position \((-?\d+), (-?\d+)\) is resolved against the physical monitors$"
)]
fn resolve_cursor_monitor(world: &mut RustShotWorld, x: i32, y: i32) {
    world.cursor_monitor = Some(selection::find_screen_index_at_position(
        world
            .physical_monitors
            .iter()
            .map(|&(x, y, width, height, _)| (x, y, width, height)),
        (x, y),
    ));
}

#[then("no cursor monitor is selected")]
fn assert_no_cursor_monitor(world: &mut RustShotWorld) {
    assert_eq!(world.cursor_monitor, Some(None));
}

#[when("the preferred monitor is resolved without a cursor")]
fn preferred_without_cursor(world: &mut RustShotWorld) {
    world.preferred_monitor = Some(selection::preferred_screen_index(
        world.physical_monitors.iter().copied(),
        None,
    ));
}

#[when(regex = r"^the preferred monitor is resolved at cursor position \((-?\d+), (-?\d+)\)$")]
fn preferred_at_cursor(world: &mut RustShotWorld, x: i32, y: i32) {
    world.preferred_monitor = Some(selection::preferred_screen_index(
        world.physical_monitors.iter().copied(),
        Some((x, y)),
    ));
}

#[then(regex = r"^preferred monitor (\d+) is selected$")]
fn assert_preferred_monitor(world: &mut RustShotWorld, expected: usize) {
    assert_eq!(world.preferred_monitor, Some(Some(expected)));
}
