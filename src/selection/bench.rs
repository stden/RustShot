//! `#[doc(hidden)]` wrappers exposing private selection helpers to benchmarks.

use super::*;

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_scaled_physical_dimension(
    logical_size: u32,
    scale_factor: f32,
) -> Result<u32, String> {
    scaled_physical_dimension(logical_size, scale_factor)
}

/// Expose `build_physical_screen_layouts` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_build_physical_screen_layouts(
    inputs: &[(i32, i32, u32, u32, u32, u32, bool)],
) -> Result<Vec<(i32, i32, u32, u32)>, String> {
    let screen_inputs: Vec<ScreenLayoutInput> = inputs
        .iter()
        .map(
            |&(
                logical_x,
                logical_y,
                logical_width,
                logical_height,
                physical_width,
                physical_height,
                is_primary,
            )| {
                ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width,
                    physical_height,
                    is_primary,
                }
            },
        )
        .collect();
    build_physical_screen_layouts(&screen_inputs).map(|layouts| {
        layouts
            .into_iter()
            .map(|l| {
                (
                    l.physical_x,
                    l.physical_y,
                    l.physical_width,
                    l.physical_height,
                )
            })
            .collect()
    })
}

type BenchmarkNormalizeLayoutInput = (i32, i32, u32, u32, u32, u32, bool);
type BenchmarkNormalizedLayout = (i32, i32, u32, u32);
type BenchmarkResolveLayout = (i32, i32, u32, u32, i32, i32, u32, u32);
type BenchmarkRegion = (i32, i32, u32, u32);
type BenchmarkResolvedSelection = (usize, BenchmarkRegion);

/// Expose `normalize_physical_screen_layouts` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_physical_screen_layouts(
    inputs: &[BenchmarkNormalizeLayoutInput],
) -> Result<(Vec<BenchmarkNormalizedLayout>, i32, i32, u32, u32), String> {
    let screen_inputs: Vec<ScreenLayoutInput> = inputs
        .iter()
        .map(
            |&(
                logical_x,
                logical_y,
                logical_width,
                logical_height,
                physical_width,
                physical_height,
                is_primary,
            )| {
                ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width,
                    physical_height,
                    is_primary,
                }
            },
        )
        .collect();
    let layouts = build_physical_screen_layouts(&screen_inputs)?;
    let (normalized, origin_x, origin_y, width, height) =
        normalize_physical_screen_layouts(&layouts)?;
    Ok((
        normalized
            .into_iter()
            .map(|l| {
                (
                    l.physical_x,
                    l.physical_y,
                    l.physical_width,
                    l.physical_height,
                )
            })
            .collect(),
        origin_x,
        origin_y,
        width,
        height,
    ))
}

/// Expose `resolve_recording_selection` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_resolve_recording_selection(
    layouts: &[BenchmarkResolveLayout],
    region: BenchmarkRegion,
) -> Result<BenchmarkResolvedSelection, String> {
    let physical_layouts: Vec<PhysicalScreenLayout> = layouts
        .iter()
        .map(
            |&(
                logical_x,
                logical_y,
                logical_width,
                logical_height,
                physical_x,
                physical_y,
                physical_width,
                physical_height,
            )| {
                PhysicalScreenLayout {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_x,
                    physical_y,
                    physical_width,
                    physical_height,
                }
            },
        )
        .collect();
    let r = Region {
        physical_x: region.0,
        physical_y: region.1,
        physical_width: region.2,
        physical_height: region.3,
    };
    resolve_recording_selection(&physical_layouts, r).map(|(idx, local)| {
        (
            idx,
            (
                local.physical_x,
                local.physical_y,
                local.physical_width,
                local.physical_height,
            ),
        )
    })
}

/// Expose `ranges_overlap` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_ranges_overlap(start_a: i32, size_a: u32, start_b: i32, size_b: u32) -> bool {
    ranges_overlap(start_a, size_a, start_b, size_b)
}

/// Expose `monitor_scale_ratio` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_monitor_scale_ratio(logical_size: u32, physical_size: u32) -> f32 {
    monitor_scale_ratio(logical_size, physical_size)
}

/// Expose `scaled_gap` for benchmarks.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_scaled_gap(gap: i32, logical_size: u32, physical_size: u32) -> i32 {
    scaled_gap(gap, logical_size, physical_size)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_build_screen_layout_inputs_physical_sizes<I>(
    screens: I,
    captured_screen_index: usize,
    captured_size: (u32, u32),
) -> Result<Vec<(u32, u32)>, String>
where
    I: IntoIterator<Item = (i32, i32, u32, u32, f32, bool)>,
{
    build_screen_layout_inputs(screens, captured_screen_index, captured_size).map(|inputs| {
        inputs
            .into_iter()
            .map(|input| (input.physical_width, input.physical_height))
            .collect()
    })
}
