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

/// Normalize already-resolved physical monitor rectangles for executable specs.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_normalize_resolved_physical_layouts(
    inputs: &[BenchmarkResolveLayout],
) -> Result<(Vec<BenchmarkNormalizedLayout>, i32, i32, u32, u32), String> {
    let layouts: Vec<PhysicalScreenLayout> = inputs
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
            )| PhysicalScreenLayout {
                logical_x,
                logical_y,
                logical_width,
                logical_height,
                physical_x,
                physical_y,
                physical_width,
                physical_height,
            },
        )
        .collect();
    let (normalized, origin_x, origin_y, width, height) =
        normalize_physical_screen_layouts(&layouts)?;
    Ok((
        normalized
            .into_iter()
            .map(|layout| {
                (
                    layout.physical_x,
                    layout.physical_y,
                    layout.physical_width,
                    layout.physical_height,
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

/// Monitor as (origin, size), the shape `target_overlay_monitor_index` consumes.
type BenchmarkMonitor = ((i32, i32), (u32, u32));

/// Expose `target_overlay_monitor_index` for benchmarks and executable specs.
#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_target_overlay_monitor_index(
    monitors: &[BenchmarkMonitor],
    target_origin: (i32, i32),
    target_size: (u32, u32),
) -> Option<usize> {
    overlay::target_overlay_monitor_index(monitors.iter().copied(), target_origin, target_size)
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

/// Stateful selection-overlay redraw fixture used by Criterion.
///
/// Mirrors `SelectionApp::draw`: [`Self::compose_full_frame`] is the first-frame
/// path (softbuffer age 0), and [`Self::redraw_crosshair`] is the steady state
/// every cursor move takes afterwards (age 1, incremental repaint).
#[doc(hidden)]
pub struct OverlayRedrawBenchmark {
    overlay_width: usize,
    overlay_height: usize,
    pixel_map: overlay::OverlayPixelMapCache,
    output: Vec<u32>,
    last_decoration: overlay::OverlayDecoration,
}

impl OverlayRedrawBenchmark {
    #[doc(hidden)]
    pub fn new(screenshot: Vec<u8>, image_size: (u32, u32), overlay_size: (u32, u32)) -> Self {
        let mut pixel_map = overlay::OverlayPixelMapCache::default();
        pixel_map.update_frame(overlay_size, image_size, &screenshot);
        let output_len = (overlay_size.0 as usize)
            .checked_mul(overlay_size.1 as usize)
            .unwrap_or(0);
        Self {
            overlay_width: overlay_size.0 as usize,
            overlay_height: overlay_size.1 as usize,
            pixel_map,
            output: vec![0; output_len],
            last_decoration: overlay::OverlayDecoration::None,
        }
    }

    fn crosshair(&self, position: (i32, i32)) -> overlay::OverlayDecoration {
        overlay::OverlayDecoration::new(
            None,
            Some(position),
            self.overlay_width,
            self.overlay_height,
        )
    }

    /// Steady-state cursor-move redraw: repaint only the changed pixels.
    #[doc(hidden)]
    pub fn redraw_crosshair(&mut self, position: (i32, i32)) {
        let decoration = self.crosshair(position);
        overlay::redraw_overlay_frame(
            &self.pixel_map.background,
            &mut self.output,
            self.overlay_width,
            self.overlay_height,
            self.last_decoration,
            decoration,
        );
        self.last_decoration = decoration;
    }

    /// First-frame path: rebuild the whole frame from the cached background.
    #[doc(hidden)]
    pub fn compose_full_frame(&mut self, position: (i32, i32)) {
        let decoration = self.crosshair(position);
        overlay::compose_overlay_frame(
            &self.pixel_map.background,
            &mut self.output,
            self.overlay_width,
            self.overlay_height,
            decoration,
        );
        self.last_decoration = decoration;
    }

    #[doc(hidden)]
    pub fn output(&self) -> &[u32] {
        &self.output
    }
}
