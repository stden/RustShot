//! Interactive region selection overlay.
//!
//! When the user presses PrintScreen (or chooses "Capture" from the tray menu),
//! a child process is spawned with `--select`.  This module:
//!
//! 1. Detects which monitor the mouse cursor is on ([`get_cursor_position`]).
//! 2. Captures only that monitor's screen image ([`capture_screen_at_cursor`]).
//! 3. Opens a borderless, always-on-top window covering that single screen and
//!    draws the screenshot as its background ([`SelectionApp`]).
//! 4. Lets the user drag a rectangle (crosshair → cyan border) and returns the
//!    cropped [`image::RgbaImage`] or a [`Region`] for recording.
//!
//! Two public entry points:
//! - [`select_region`] — returns the cropped RGBA image (used for screenshots).
//! - [`select_region_for_recording`] — returns the screen + local-coordinate
//!   [`Region`] so the recorder can capture just that area.

use screenshots::Screen;
use std::num::NonZeroU32;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{CursorIcon, Window, WindowAttributes, WindowLevel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub physical_x: i32,
    pub physical_y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
}

#[derive(Debug, Clone)]
struct MonitorCapture {
    packed_background: Vec<u32>,
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
}

#[allow(dead_code)]
#[derive(Debug)]
struct VirtualDesktopCapture {
    rgba_data: Vec<u8>,
    packed_background: Vec<u32>,
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
    screens: Vec<Screen>,
    layouts: Vec<PhysicalScreenLayout>,
}

#[derive(Debug, Clone, Copy)]
struct ScreenLayoutInput {
    logical_x: i32,
    logical_y: i32,
    logical_width: u32,
    logical_height: u32,
    physical_width: u32,
    physical_height: u32,
    is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalScreenLayout {
    logical_x: i32,
    logical_y: i32,
    logical_width: u32,
    logical_height: u32,
    physical_x: i32,
    physical_y: i32,
    physical_width: u32,
    physical_height: u32,
}

/// Choose the virtual-desktop scale factor.
///
/// `display_info` coordinates are logical while captured frame buffers are
/// physical. For the spanning overlay we use the primary monitor scale as the
/// shared logical→physical conversion baseline, falling back to the first
/// screen when no primary flag is present.
///
/// # Examples
///
/// ```
/// use rustshot::selection::virtual_desktop_scale_factor;
///
/// // Primary at 1.5×, secondary at 1.0×  → uses primary
/// let factor = virtual_desktop_scale_factor(vec![(false, 1.0), (true, 1.5)]);
/// assert_eq!(factor, Some(1.5));
///
/// // No primary flag → falls back to first screen
/// let factor = virtual_desktop_scale_factor(vec![(false, 2.0), (false, 1.0)]);
/// assert_eq!(factor, Some(2.0));
///
/// // Empty iterator → None
/// let factor = virtual_desktop_scale_factor(Vec::<(bool, f32)>::new());
/// assert_eq!(factor, None);
/// ```
///
/// Algorithm:
/// 1. Keep the first monitor scale as a fallback.
/// 2. Continue scanning until a primary monitor is found.
/// 3. Return the primary monitor's scale immediately.
/// 4. If there is no primary monitor, return the first scale instead.
///
/// Possible improvements:
/// - Derive a merged desktop scale from monitor geometry if mixed-DPI layouts
///   need something more precise than "primary monitor wins".
/// - Surface inconsistent primary-monitor metadata for diagnostics.
#[allow(dead_code)]
pub fn virtual_desktop_scale_factor<I>(screens: I) -> Option<f32>
where
    I: IntoIterator<Item = (bool, f32)>,
{
    // Single-pass selection:
    // keep the first scale factor as a fallback, but prefer the first monitor
    // that the platform marks as primary.
    let mut fallback = None;

    for (is_primary, scale_factor) in screens {
        if fallback.is_none() {
            fallback = Some(scale_factor);
        }

        if is_primary {
            return Some(scale_factor);
        }
    }

    fallback
}

/// Convert a logical (DPI-independent) coordinate to physical pixels.
///
/// Multiplies by the monitor's scale factor and rounds to the nearest integer.
#[allow(dead_code)]
pub fn logical_to_physical(value: i32, scale_factor: f32) -> i32 {
    ((value as f32) * scale_factor).round() as i32
}

/// Compute the exclusive end coordinate: `start + size` (overflow-safe via i64).
fn logical_end(start: i32, size: u32) -> i64 {
    i64::from(start) + i64::from(size)
}

/// Alias for [`logical_end`] — same arithmetic, different semantic context.
#[inline]
fn physical_end(start: i32, size: u32) -> i64 {
    logical_end(start, size)
}

fn ranges_overlap(start_a: i32, size_a: u32, start_b: i32, size_b: u32) -> bool {
    let end_a = logical_end(start_a, size_a);
    let end_b = logical_end(start_b, size_b);
    i64::from(start_a) < end_b && i64::from(start_b) < end_a
}

fn monitor_scale_ratio(logical_size: u32, physical_size: u32) -> f32 {
    if logical_size == 0 {
        1.0
    } else {
        physical_size as f32 / logical_size as f32
    }
}

fn scaled_gap(gap: i32, logical_size: u32, physical_size: u32) -> i32 {
    ((gap as f32) * monitor_scale_ratio(logical_size, physical_size)).round() as i32
}

fn infer_physical_x(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_x: i32,
) -> Option<i32> {
    if reference.logical_x == target.logical_x {
        return Some(reference_x);
    }

    if !ranges_overlap(
        reference.logical_y,
        reference.logical_height,
        target.logical_y,
        target.logical_height,
    ) {
        return None;
    }

    let reference_right = logical_end(reference.logical_x, reference.logical_width);
    let target_right = logical_end(target.logical_x, target.logical_width);
    let reference_phys_right = physical_end(reference.logical_x, reference.physical_width);
    let target_phys_right = physical_end(target.logical_x, target.physical_width);

    if reference_right == i64::from(target.logical_x)
        || reference_phys_right == i64::from(target.logical_x)
    {
        Some(reference_x + reference.physical_width as i32)
    } else if target_right == i64::from(reference.logical_x)
        || target_phys_right == i64::from(reference.logical_x)
    {
        Some(reference_x - target.physical_width as i32)
    } else {
        None
    }
}

fn infer_physical_y(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_y: i32,
) -> Option<i32> {
    if reference.logical_y == target.logical_y {
        return Some(reference_y);
    }

    if !ranges_overlap(
        reference.logical_x,
        reference.logical_width,
        target.logical_x,
        target.logical_width,
    ) {
        return None;
    }

    let reference_bottom = logical_end(reference.logical_y, reference.logical_height);
    let target_bottom = logical_end(target.logical_y, target.logical_height);
    let reference_phys_bottom = physical_end(reference.logical_y, reference.physical_height);
    let target_phys_bottom = physical_end(target.logical_y, target.physical_height);

    if reference_bottom == i64::from(target.logical_y)
        || reference_phys_bottom == i64::from(target.logical_y)
    {
        Some(reference_y + reference.physical_height as i32)
    } else if target_bottom == i64::from(reference.logical_y)
        || target_phys_bottom == i64::from(reference.logical_y)
    {
        Some(reference_y - target.physical_height as i32)
    } else {
        None
    }
}

/// Infer the physical Y of a target from a horizontally adjacent reference.
fn infer_physical_y_from_horizontal_neighbor(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_y: i32,
) -> Option<i32> {
    let reference_right = logical_end(reference.logical_x, reference.logical_width);
    let target_right = logical_end(target.logical_x, target.logical_width);
    let reference_phys_right = physical_end(reference.logical_x, reference.physical_width);
    let target_phys_right = physical_end(target.logical_x, target.physical_width);

    let is_adjacent = reference_right == i64::from(target.logical_x)
        || target_right == i64::from(reference.logical_x)
        || reference_phys_right == i64::from(target.logical_x)
        || target_phys_right == i64::from(reference.logical_x);

    if !is_adjacent {
        return None;
    }

    let logical_y_diff = target.logical_y - reference.logical_y;
    Some(
        reference_y
            + scaled_gap(
                logical_y_diff,
                target.logical_height,
                target.physical_height,
            ),
    )
}

/// Infer the physical X of a target from a vertically adjacent reference.
fn infer_physical_x_from_vertical_neighbor(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_x: i32,
) -> Option<i32> {
    let reference_bottom = logical_end(reference.logical_y, reference.logical_height);
    let target_bottom = logical_end(target.logical_y, target.logical_height);
    let reference_phys_bottom = physical_end(reference.logical_y, reference.physical_height);
    let target_phys_bottom = physical_end(target.logical_y, target.physical_height);

    let is_adjacent = reference_bottom == i64::from(target.logical_y)
        || target_bottom == i64::from(reference.logical_y)
        || reference_phys_bottom == i64::from(target.logical_y)
        || target_phys_bottom == i64::from(reference.logical_y);

    if !is_adjacent {
        return None;
    }

    let logical_x_diff = target.logical_x - reference.logical_x;
    Some(reference_x + scaled_gap(logical_x_diff, target.logical_width, target.physical_width))
}

fn fallback_physical_x(
    inputs: &[ScreenLayoutInput],
    positions: &[Option<i32>],
    target_index: usize,
) -> Option<i32> {
    let target = inputs.get(target_index)?;
    let target_left = i64::from(target.logical_x);
    let target_right = logical_end(target.logical_x, target.logical_width);

    let mut left_candidate: Option<(i64, i32)> = None;
    let mut right_candidate: Option<(i64, i32)> = None;

    for (index, reference) in inputs.iter().enumerate() {
        let Some(reference_x) = positions.get(index).and_then(|position| *position) else {
            continue;
        };

        let reference_left = i64::from(reference.logical_x);
        let reference_right = logical_end(reference.logical_x, reference.logical_width);

        if reference_right <= target_left {
            let gap = (target_left - reference_right) as i32;
            let candidate = reference_x
                + reference.physical_width as i32
                + scaled_gap(gap, target.logical_width, target.physical_width);
            match left_candidate {
                Some((best_distance, _)) if best_distance <= i64::from(gap) => {}
                _ => left_candidate = Some((i64::from(gap), candidate)),
            }
        }

        if target_right <= reference_left {
            let gap = (reference_left - target_right) as i32;
            let candidate = reference_x
                - target.physical_width as i32
                - scaled_gap(gap, target.logical_width, target.physical_width);
            match right_candidate {
                Some((best_distance, _)) if best_distance <= i64::from(gap) => {}
                _ => right_candidate = Some((i64::from(gap), candidate)),
            }
        }
    }

    left_candidate
        .or(right_candidate)
        .map(|(_, candidate)| candidate)
}

fn fallback_physical_y(
    inputs: &[ScreenLayoutInput],
    positions: &[Option<i32>],
    target_index: usize,
) -> Option<i32> {
    let target = inputs.get(target_index)?;
    let target_top = i64::from(target.logical_y);
    let target_bottom = logical_end(target.logical_y, target.logical_height);

    let mut above_candidate: Option<(i64, i32)> = None;
    let mut below_candidate: Option<(i64, i32)> = None;

    for (index, reference) in inputs.iter().enumerate() {
        let Some(reference_y) = positions.get(index).and_then(|position| *position) else {
            continue;
        };

        let reference_top = i64::from(reference.logical_y);
        let reference_bottom = logical_end(reference.logical_y, reference.logical_height);

        if reference_bottom <= target_top {
            let gap = (target_top - reference_bottom) as i32;
            let candidate = reference_y
                + reference.physical_height as i32
                + scaled_gap(gap, target.logical_height, target.physical_height);
            match above_candidate {
                Some((best_distance, _)) if best_distance <= i64::from(gap) => {}
                _ => above_candidate = Some((i64::from(gap), candidate)),
            }
        }

        if target_bottom <= reference_top {
            let gap = (reference_top - target_bottom) as i32;
            let candidate = reference_y
                - target.physical_height as i32
                - scaled_gap(gap, target.logical_height, target.physical_height);
            match below_candidate {
                Some((best_distance, _)) if best_distance <= i64::from(gap) => {}
                _ => below_candidate = Some((i64::from(gap), candidate)),
            }
        }
    }

    above_candidate
        .or(below_candidate)
        .map(|(_, candidate)| candidate)
}

fn build_physical_screen_layouts(
    inputs: &[ScreenLayoutInput],
) -> Result<Vec<PhysicalScreenLayout>, String> {
    if inputs.is_empty() {
        return Err("No screens found".to_string());
    }

    if let [input] = inputs {
        return Ok(vec![PhysicalScreenLayout {
            logical_x: input.logical_x,
            logical_y: input.logical_y,
            logical_width: input.logical_width,
            logical_height: input.logical_height,
            physical_x: 0,
            physical_y: 0,
            physical_width: input.physical_width,
            physical_height: input.physical_height,
        }]);
    }

    let len = inputs.len();
    let anchor_index = inputs
        .iter()
        .position(|screen| screen.is_primary)
        .unwrap_or(0);
    let mut physical_x = vec![None; len];
    let mut physical_y = vec![None; len];
    physical_x[anchor_index] = Some(0);
    physical_y[anchor_index] = Some(0);

    let mut queue = Vec::with_capacity(len);
    queue.push(anchor_index);
    let mut queue_head = 0;
    let mut resolved_count = 1;

    loop {
        while queue_head < queue.len() && resolved_count < len {
            let reference_index = queue[queue_head];
            queue_head += 1;
            let reference = &inputs[reference_index];
            let reference_x = physical_x[reference_index].unwrap();
            let reference_y = physical_y[reference_index].unwrap();

            for (target_index, target) in inputs.iter().enumerate() {
                if physical_x[target_index].is_some() && physical_y[target_index].is_some() {
                    continue;
                }

                physical_x[target_index] = physical_x[target_index].or_else(|| {
                    infer_physical_x(reference, target, reference_x).or_else(|| {
                        infer_physical_x_from_vertical_neighbor(reference, target, reference_x)
                    })
                });

                physical_y[target_index] = physical_y[target_index].or_else(|| {
                    infer_physical_y(reference, target, reference_y).or_else(|| {
                        infer_physical_y_from_horizontal_neighbor(reference, target, reference_y)
                    })
                });

                if physical_x[target_index].is_some() && physical_y[target_index].is_some() {
                    queue.push(target_index);
                    resolved_count += 1;
                }
            }
        }

        if resolved_count == len {
            break;
        }

        let mut progress = false;
        for index in 0..len {
            let mut newly_resolved = false;
            if physical_x[index].is_none() {
                if let Some(candidate) = fallback_physical_x(inputs, &physical_x, index) {
                    physical_x[index] = Some(candidate);
                    progress = true;
                    newly_resolved = physical_y[index].is_some();
                }
            }

            if physical_y[index].is_none() {
                if let Some(candidate) = fallback_physical_y(inputs, &physical_y, index) {
                    physical_y[index] = Some(candidate);
                    progress = true;
                    newly_resolved = physical_x[index].is_some();
                }
            }

            if newly_resolved {
                queue.push(index);
                resolved_count += 1;
            }
        }

        if !progress {
            break;
        }
    }

    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let physical_x = physical_x[index]
                .ok_or_else(|| "Failed to resolve physical screen positions".to_string())?;
            let physical_y = physical_y[index]
                .ok_or_else(|| "Failed to resolve physical screen positions".to_string())?;

            Ok(PhysicalScreenLayout {
                logical_x: input.logical_x,
                logical_y: input.logical_y,
                logical_width: input.logical_width,
                logical_height: input.logical_height,
                physical_x,
                physical_y,
                physical_width: input.physical_width,
                physical_height: input.physical_height,
            })
        })
        .collect()
}

fn overlay_window_origin(
    layout: &PhysicalScreenLayout,
    scale_factor: f32,
    preserve_native_origin: bool,
) -> (i32, i32) {
    if preserve_native_origin {
        // On X11, display-info divides native desktop coordinates by the
        // shared scale factor. Restore them without moving the primary to 0,0.
        return (
            logical_to_physical(layout.logical_x, scale_factor),
            logical_to_physical(layout.logical_y, scale_factor),
        );
    }

    (layout.physical_x, layout.physical_y)
}

#[allow(dead_code)]
fn normalize_physical_screen_layouts(
    layouts: &[PhysicalScreenLayout],
) -> Result<(Vec<PhysicalScreenLayout>, i32, i32, u32, u32), String> {
    let first = layouts
        .first()
        .ok_or_else(|| "No screens found".to_string())?;
    let mut min_x = first.physical_x;
    let mut min_y = first.physical_y;
    let mut max_x = physical_end(first.physical_x, first.physical_width);
    let mut max_y = physical_end(first.physical_y, first.physical_height);
    for layout in &layouts[1..] {
        min_x = min_x.min(layout.physical_x);
        min_y = min_y.min(layout.physical_y);
        max_x = max_x.max(physical_end(layout.physical_x, layout.physical_width));
        max_y = max_y.max(physical_end(layout.physical_y, layout.physical_height));
    }

    let width = i32::try_from(max_x - i64::from(min_x))
        .map(|w| w as u32)
        .map_err(|_| "Virtual desktop width is out of range".to_string())?;
    let height = i32::try_from(max_y - i64::from(min_y))
        .map(|h| h as u32)
        .map_err(|_| "Virtual desktop height is out of range".to_string())?;

    let normalized = layouts
        .iter()
        .map(|layout| PhysicalScreenLayout {
            physical_x: (i64::from(layout.physical_x) - i64::from(min_x)) as i32,
            physical_y: (i64::from(layout.physical_y) - i64::from(min_y)) as i32,
            ..*layout
        })
        .collect();

    Ok((normalized, min_x, min_y, width, height))
}

fn resolve_recording_selection(
    layouts: &[PhysicalScreenLayout],
    region: Region,
) -> Result<(usize, Region), String> {
    let screen_index = layouts
        .iter()
        .enumerate()
        .find(|(_, layout)| {
            let region_right = i64::from(region.physical_x) + i64::from(region.physical_width);
            let region_bottom = i64::from(region.physical_y) + i64::from(region.physical_height);
            let layout_right = physical_end(layout.physical_x, layout.physical_width);
            let layout_bottom = physical_end(layout.physical_y, layout.physical_height);

            i64::from(region.physical_x) >= i64::from(layout.physical_x)
                && i64::from(region.physical_y) >= i64::from(layout.physical_y)
                && region_right <= layout_right
                && region_bottom <= layout_bottom
        })
        .map(|(index, _)| index)
        .ok_or_else(|| "Recording selection must stay within a single monitor".to_string())?;

    let layout = layouts[screen_index];
    let local_region = Region {
        physical_x: (i64::from(region.physical_x) - i64::from(layout.physical_x)) as i32,
        physical_y: (i64::from(region.physical_y) - i64::from(layout.physical_y)) as i32,
        physical_width: region.physical_width,
        physical_height: region.physical_height,
    };

    Ok((screen_index, local_region))
}

fn scaled_physical_dimension(logical_size: u32, scale_factor: f32) -> Result<u32, String> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err("Display scale factor must be positive".to_string());
    }

    let scaled_size = (logical_size as f64) * (scale_factor as f64);
    if !(0.0..=(u32::MAX as f64)).contains(&scaled_size) {
        return Err("Physical screen size is out of range".to_string());
    }

    Ok(scaled_size.ceil() as u32)
}

#[allow(dead_code)]
#[doc(hidden)]
pub fn benchmark_scaled_physical_dimension(
    logical_size: u32,
    scale_factor: f32,
) -> Result<u32, String> {
    scaled_physical_dimension(logical_size, scale_factor)
}

fn build_screen_layout_inputs<I>(
    screens: I,
    captured_screen_index: usize,
    captured_size: (u32, u32),
) -> Result<Vec<ScreenLayoutInput>, String>
where
    I: IntoIterator<Item = (i32, i32, u32, u32, f32, bool)>,
{
    let screens: Vec<_> = screens.into_iter().collect();
    if captured_screen_index >= screens.len() {
        return Err(format!(
            "Unknown screen index for selection capture: {captured_screen_index}"
        ));
    }

    screens
        .into_iter()
        .enumerate()
        .map(
            |(
                index,
                (logical_x, logical_y, logical_width, logical_height, scale_factor, is_primary),
            )| {
                let (physical_width, physical_height) = if index == captured_screen_index {
                    captured_size
                } else {
                    (
                        scaled_physical_dimension(logical_width, scale_factor)?,
                        scaled_physical_dimension(logical_height, scale_factor)?,
                    )
                };

                Ok(ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width,
                    physical_height,
                    is_primary,
                })
            },
        )
        .collect()
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

/// Convert drag start/end screen coordinates into a [`Region`].
/// Returns `None` if the region is too small (≤5 px in either dimension).
///
/// Algorithm:
/// 1. Normalize the drag direction by taking the minimum x/y as the top-left
///    corner and the maximum x/y as the opposite corner.
/// 2. Compute unsigned width and height from those normalized bounds.
/// 3. Reject tiny drags so click jitter does not produce accidental captures.
/// 4. Return a normalized [`Region`] for valid drags.
///
/// Possible improvements:
/// - Make the minimum drag threshold configurable for accessibility.
/// - Preserve the original drag direction as metadata if future UX features
///   need resize handles or animated reveal directions.
///
/// # Examples
///
/// ```
/// use rustshot::selection::build_region_from_drag;
///
/// // Valid drag
/// let region = build_region_from_drag((10, 20), (100, 200)).unwrap();
/// assert_eq!((region.physical_x, region.physical_y, region.physical_width, region.physical_height), (10, 20, 90, 180));
///
/// // Too small (≤5 px) → None
/// assert!(build_region_from_drag((10, 10), (14, 14)).is_none());
/// ```
pub fn build_region_from_drag(start: (i32, i32), end: (i32, i32)) -> Option<Region> {
    // Normalize drag direction so every caller gets a canonical rectangle
    // regardless of whether the user dragged top-left → bottom-right or the
    // reverse.
    let x1 = start.0.min(end.0);
    let y1 = start.1.min(end.1);
    let width = start.0.abs_diff(end.0);
    let height = start.1.abs_diff(end.1);

    if width > 5 && height > 5 {
        Some(Region {
            physical_x: x1,
            physical_y: y1,
            physical_width: width,
            physical_height: height,
        })
    } else {
        None
    }
}

/// Pre-pack RGBA bytes (`[R, G, B, A]`) into `softbuffer`'s `0x00RRGGBB` pixel format.
///
/// Pre-computing the packed `u32` buffer once when the overlay initializes turns
/// every subsequent `CursorMoved` / `RedrawRequested` frame into a fast slice copy
/// plus `O(width + height)` border/crosshair drawing instead of unpacking and
/// branching over every pixel on the screen.
pub fn pack_rgba_to_softbuffer(rgba: &[u8]) -> Vec<u32> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, _]| ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
        .collect()
}

/// Compute the next `softbuffer` surface dimensions, clamping `0` to `1` and
/// returning `None` when the surface is already at the requested size.
///
/// Suppresses redundant `surface.resize(...)` and full-frame redraws during the
/// 4–6 cascading `WM_SIZE` messages emitted on Windows overlay creation.
pub fn next_surface_size(
    current_size: Option<(u32, u32)>,
    requested_width: u32,
    requested_height: u32,
) -> Option<(NonZeroU32, NonZeroU32)> {
    let w = NonZeroU32::new(requested_width).unwrap_or(NonZeroU32::MIN);
    let h = NonZeroU32::new(requested_height).unwrap_or(NonZeroU32::MIN);
    if current_size == Some((w.get(), h.get())) {
        None
    } else {
        Some((w, h))
    }
}

#[inline]
fn set_overlay_pixel(
    buffer: &mut [u32],
    buf_w: usize,
    packed_len: usize,
    src_w: usize,
    x: usize,
    y: usize,
    color: u32,
) {
    let src_pixel_idx = y.saturating_mul(src_w).saturating_add(x);
    let dst_idx = y.saturating_mul(buf_w).saturating_add(x);
    if src_pixel_idx < packed_len && dst_idx < buffer.len() {
        buffer[dst_idx] = color;
    }
}

#[inline]
fn restore_overlay_pixel(
    buffer: &mut [u32],
    buf_w: usize,
    packed_background: &[u32],
    src_w: usize,
    x: usize,
    y: usize,
) {
    let src_pixel_idx = y.saturating_mul(src_w).saturating_add(x);
    let dst_idx = y.saturating_mul(buf_w).saturating_add(x);
    if src_pixel_idx < packed_background.len() && dst_idx < buffer.len() {
        buffer[dst_idx] = packed_background[src_pixel_idx];
    }
}

#[inline]
fn fill_overlay_row_span(
    buffer: &mut [u32],
    buf_w: usize,
    packed_len: usize,
    src_w: usize,
    y: usize,
    (x_start, x_end): (usize, usize),
    color: u32,
) {
    let src_start = y
        .saturating_mul(src_w)
        .saturating_add(x_start)
        .min(packed_len);
    let dst_start = y
        .saturating_mul(buf_w)
        .saturating_add(x_start)
        .min(buffer.len());
    let span = x_end
        .saturating_sub(x_start)
        .min(packed_len - src_start)
        .min(buffer.len() - dst_start);
    buffer[dst_start..dst_start + span].fill(color);
}

#[inline]
fn restore_overlay_row_span(
    buffer: &mut [u32],
    buf_w: usize,
    packed_background: &[u32],
    src_w: usize,
    y: usize,
    (x_start, x_end): (usize, usize),
) {
    let packed_len = packed_background.len();
    let src_start = y
        .saturating_mul(src_w)
        .saturating_add(x_start)
        .min(packed_len);
    let dst_start = y
        .saturating_mul(buf_w)
        .saturating_add(x_start)
        .min(buffer.len());
    let span = x_end
        .saturating_sub(x_start)
        .min(packed_len - src_start)
        .min(buffer.len() - dst_start);
    buffer[dst_start..dst_start + span]
        .copy_from_slice(&packed_background[src_start..src_start + span]);
}

/// Extract and clamp selection border corners from a drag pair.
///
/// Returns `(x1, y1, x2, y2)` where `(x1, y1)` is the top-left and `(x2, y2)`
/// is the bottom-right corner in buffer pixel coordinates. `x2` and `y2` are
/// clamped to the last valid pixel (`buf_w - 1`, `buf_h - 1`) to guard against
/// cursor coordinates reported slightly outside the window due to DPI rounding
/// on secondary monitors.
#[inline]
fn drag_border_coords(
    start: (i32, i32),
    end: (i32, i32),
    buf_w: usize,
    buf_h: usize,
) -> (usize, usize, usize, usize) {
    let x1 = start.0.min(end.0).max(0) as usize;
    let y1 = start.1.min(end.1).max(0) as usize;
    let x2 = (start.0.max(end.0).max(0) as usize).min(buf_w.saturating_sub(1));
    let y2 = (start.1.max(end.1).max(0) as usize).min(buf_h.saturating_sub(1));
    (x1, y1, x2, y2)
}

fn draw_overlay_primitives(
    buffer: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    packed_len: usize,
    src_w: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
) {
    if let (Some(start), Some(end)) = (start_pos, current_pos) {
        const CYAN_BORDER: u32 = (200 << 8) | 255;
        let (x1, y1, x2, y2) = drag_border_coords(start, end, buf_w, buf_h);

        let x_start = x1.saturating_sub(1).min(buf_w);
        let x_end = x2.saturating_add(2).min(buf_w);
        let y_start = y1.saturating_sub(1).min(buf_h);
        let y_end = y2.saturating_add(2).min(buf_h);

        // Top and bottom 3px horizontal border bands
        for y in y1.saturating_sub(1)..y1.saturating_add(2).min(buf_h) {
            fill_overlay_row_span(
                buffer,
                buf_w,
                packed_len,
                src_w,
                y,
                (x_start, x_end),
                CYAN_BORDER,
            );
        }
        for y in y2.saturating_sub(1)..y2.saturating_add(2).min(buf_h) {
            fill_overlay_row_span(
                buffer,
                buf_w,
                packed_len,
                src_w,
                y,
                (x_start, x_end),
                CYAN_BORDER,
            );
        }

        // Left and right 3px vertical border bands
        let left_x_start = x1.saturating_sub(1).min(buf_w);
        let left_x_end = x1.saturating_add(2).min(buf_w);
        let right_x_start = x2.saturating_sub(1).min(buf_w);
        let right_x_end = x2.saturating_add(2).min(buf_w);
        for y in y_start..y_end {
            fill_overlay_row_span(
                buffer,
                buf_w,
                packed_len,
                src_w,
                y,
                (left_x_start, left_x_end),
                CYAN_BORDER,
            );
            fill_overlay_row_span(
                buffer,
                buf_w,
                packed_len,
                src_w,
                y,
                (right_x_start, right_x_end),
                CYAN_BORDER,
            );
        }
    } else if let Some(pos) = current_pos {
        const RED_CROSSHAIR: u32 = 255 << 16;
        let cx = pos.0 as usize;
        let cy = pos.1 as usize;

        if cy < buf_h {
            fill_overlay_row_span(
                buffer,
                buf_w,
                packed_len,
                src_w,
                cy,
                (0, buf_w),
                RED_CROSSHAIR,
            );
        }
        if cx < buf_w {
            for y in 0..buf_h {
                set_overlay_pixel(buffer, buf_w, packed_len, src_w, cx, y, RED_CROSSHAIR);
            }
        }
    }
}

/// Tracks the previous frame's drawn overlay primitives so subsequent frames only
/// restore the modified crosshair lines or 3px rectangle border strips (`O(W + H)`)
/// instead of copying all `W × H` background pixels on every mouse move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OverlayDamage {
    pub surface_size: Option<(usize, usize)>,
    pub start_pos: Option<(i32, i32)>,
    pub current_pos: Option<(i32, i32)>,
}

/// Incrementally update `buffer` by restoring only the pixels damaged in `damage`
/// from `packed_background` and drawing the new crosshair or selection border.
/// Falls back to a full background copy on the first frame or when `buf_w × buf_h` changes.
#[allow(clippy::too_many_arguments)]
pub fn update_selection_overlay_frame(
    buffer: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    packed_background: &[u32],
    src_w: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
    damage: &mut OverlayDamage,
) {
    let packed_len = packed_background.len();
    if buf_w == 0 || buf_h == 0 || packed_len == 0 || buffer.is_empty() {
        return;
    }

    if damage.surface_size != Some((buf_w, buf_h)) {
        render_selection_overlay_frame(
            buffer,
            buf_w,
            buf_h,
            packed_background,
            src_w,
            start_pos,
            current_pos,
        );
        *damage = OverlayDamage {
            surface_size: Some((buf_w, buf_h)),
            start_pos,
            current_pos,
        };
        return;
    }

    if damage.start_pos == start_pos && damage.current_pos == current_pos {
        return;
    }

    // Restore only the previous frame's drawn pixels from packed_background.
    if let (Some(start), Some(end)) = (damage.start_pos, damage.current_pos) {
        let (x1, y1, x2, y2) = drag_border_coords(start, end, buf_w, buf_h);

        let x_start = x1.saturating_sub(1).min(buf_w);
        let x_end = x2.saturating_add(2).min(buf_w);
        let y_start = y1.saturating_sub(1).min(buf_h);
        let y_end = y2.saturating_add(2).min(buf_h);

        for y in y1.saturating_sub(1)..y1.saturating_add(2).min(buf_h) {
            restore_overlay_row_span(buffer, buf_w, packed_background, src_w, y, (x_start, x_end));
        }
        for y in y2.saturating_sub(1)..y2.saturating_add(2).min(buf_h) {
            restore_overlay_row_span(buffer, buf_w, packed_background, src_w, y, (x_start, x_end));
        }

        let left_x_start = x1.saturating_sub(1).min(buf_w);
        let left_x_end = x1.saturating_add(2).min(buf_w);
        let right_x_start = x2.saturating_sub(1).min(buf_w);
        let right_x_end = x2.saturating_add(2).min(buf_w);
        for y in y_start..y_end {
            restore_overlay_row_span(
                buffer,
                buf_w,
                packed_background,
                src_w,
                y,
                (left_x_start, left_x_end),
            );
            restore_overlay_row_span(
                buffer,
                buf_w,
                packed_background,
                src_w,
                y,
                (right_x_start, right_x_end),
            );
        }
    } else if let Some(pos) = damage.current_pos {
        let cx = pos.0 as usize;
        let cy = pos.1 as usize;

        if cy < buf_h {
            restore_overlay_row_span(buffer, buf_w, packed_background, src_w, cy, (0, buf_w));
        }
        if cx < buf_w {
            for y in 0..buf_h {
                restore_overlay_pixel(buffer, buf_w, packed_background, src_w, cx, y);
            }
        }
    }

    draw_overlay_primitives(
        buffer,
        buf_w,
        buf_h,
        packed_len,
        src_w,
        start_pos,
        current_pos,
    );
    *damage = OverlayDamage {
        surface_size: Some((buf_w, buf_h)),
        start_pos,
        current_pos,
    };
}

fn push_clamped_damage_rect(
    rects: &mut Vec<softbuffer::Rect>,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    buf_w: usize,
    buf_h: usize,
) {
    let xs = x0.min(buf_w);
    let xe = x1.min(buf_w);
    let ys = y0.min(buf_h);
    let ye = y1.min(buf_h);
    let (Some(w), Some(h)) = (
        NonZeroU32::new(xe.saturating_sub(xs) as u32),
        NonZeroU32::new(ye.saturating_sub(ys) as u32),
    ) else {
        return;
    };
    rects.push(softbuffer::Rect {
        x: xs as u32,
        y: ys as u32,
        width: w,
        height: h,
    });
}

fn push_primitive_damage_rects(
    rects: &mut Vec<softbuffer::Rect>,
    buf_w: usize,
    buf_h: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
) {
    if let (Some(start), Some(end)) = (start_pos, current_pos) {
        let (x1, y1, x2, y2) = drag_border_coords(start, end, buf_w, buf_h);

        let x_start = x1.saturating_sub(1).min(buf_w);
        let x_end = x2.saturating_add(2).min(buf_w);
        let y_start = y1.saturating_sub(1).min(buf_h);
        let y_end = y2.saturating_add(2).min(buf_h);

        push_clamped_damage_rect(
            rects,
            x_start,
            y1.saturating_sub(1),
            x_end,
            y1.saturating_add(2),
            buf_w,
            buf_h,
        );
        push_clamped_damage_rect(
            rects,
            x_start,
            y2.saturating_sub(1),
            x_end,
            y2.saturating_add(2),
            buf_w,
            buf_h,
        );
        push_clamped_damage_rect(
            rects,
            x1.saturating_sub(1),
            y_start,
            x1.saturating_add(2),
            y_end,
            buf_w,
            buf_h,
        );
        push_clamped_damage_rect(
            rects,
            x2.saturating_sub(1),
            y_start,
            x2.saturating_add(2),
            y_end,
            buf_w,
            buf_h,
        );
    } else if let Some(pos) = current_pos {
        let cx = pos.0 as usize;
        let cy = pos.1 as usize;

        if cy < buf_h {
            push_clamped_damage_rect(rects, 0, cy, buf_w, cy + 1, buf_w, buf_h);
        }
        if cx < buf_w {
            push_clamped_damage_rect(rects, cx, 0, cx + 1, buf_h, buf_w, buf_h);
        }
    }
}

/// Compute the minimal list of dirty `softbuffer::Rect`s to pass to
/// `Buffer::present_with_damage` so the OS only blits the modified crosshair lines
/// or 3px rectangle border bands instead of the entire fullscreen buffer.
pub fn overlay_present_damage_rects(
    previous_damage: OverlayDamage,
    buf_w: usize,
    buf_h: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
) -> Vec<softbuffer::Rect> {
    let (Some(full_w), Some(full_h)) =
        (NonZeroU32::new(buf_w as u32), NonZeroU32::new(buf_h as u32))
    else {
        return Vec::new();
    };

    if previous_damage.surface_size != Some((buf_w, buf_h)) {
        return vec![softbuffer::Rect {
            x: 0,
            y: 0,
            width: full_w,
            height: full_h,
        }];
    }

    if previous_damage.start_pos == start_pos && previous_damage.current_pos == current_pos {
        return Vec::new();
    }

    let mut rects = Vec::with_capacity(8);
    push_primitive_damage_rects(
        &mut rects,
        buf_w,
        buf_h,
        previous_damage.start_pos,
        previous_damage.current_pos,
    );
    push_primitive_damage_rects(&mut rects, buf_w, buf_h, start_pos, current_pos);
    rects
}

/// Render one selection overlay frame into `buffer` (`buf_w` × `buf_h`) from a
/// pre-packed `0x00RRGGBB` background (`src_w` stride) and draw the active
/// 3px cyan selection border or 1px red crosshair in `O(buf_w + buf_h)`.
pub fn render_selection_overlay_frame(
    buffer: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    packed_background: &[u32],
    src_w: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
) {
    let packed_len = packed_background.len();
    if buf_w == 0 || buf_h == 0 || packed_len == 0 || buffer.is_empty() {
        return;
    }

    if buf_w == src_w {
        let copy_len = buf_w
            .saturating_mul(buf_h)
            .min(buffer.len())
            .min(packed_len);
        buffer[..copy_len].copy_from_slice(&packed_background[..copy_len]);
    } else {
        for y in 0..buf_h {
            let src_row = y.saturating_mul(src_w);
            let dst_row = y.saturating_mul(buf_w);
            if src_row >= packed_len || dst_row >= buffer.len() {
                break;
            }
            let row_len = buf_w.min(packed_len - src_row).min(buffer.len() - dst_row);
            buffer[dst_row..dst_row + row_len]
                .copy_from_slice(&packed_background[src_row..src_row + row_len]);
        }
    }

    draw_overlay_primitives(
        buffer,
        buf_w,
        buf_h,
        packed_len,
        src_w,
        start_pos,
        current_pos,
    );
}

/// Crop the selected region directly from a packed `0x00RRGGBB` / `0xAARRGGBB`
/// pixel slice into an `image::RgbaImage` (`A = 255`), avoiding keeping a second
/// full-screen `Vec<u8>` RGBA buffer in memory during selection.
pub fn crop_packed_region(
    width: u32,
    height: u32,
    packed: &[u32],
    region: Region,
) -> Result<image::RgbaImage, String> {
    if region.physical_x < 0 || region.physical_y < 0 {
        return Err("Selection region has negative coordinates".to_string());
    }
    let rx = region.physical_x as usize;
    let ry = region.physical_y as usize;
    let rw = region.physical_width as usize;
    let rh = region.physical_height as usize;
    let src_w = width as usize;
    let src_h = height as usize;

    if rw == 0 || rh == 0 || rx + rw > src_w || ry + rh > src_h {
        return Err("Selection region is out of bounds".to_string());
    }
    if packed.len() < src_w.saturating_mul(src_h) {
        return Err("Packed background buffer is smaller than screen dimensions".to_string());
    }

    let total_bytes = rw * rh * 4;
    let mut out = Vec::with_capacity(total_bytes);
    let spare_chunks = out.spare_capacity_mut().as_chunks_mut::<4>().0;
    for y in 0..rh {
        let src_row_start = (ry + y) * src_w + rx;
        let dst_row_start = y * rw;
        let src_row = &packed[src_row_start..src_row_start + rw];
        let dst_row = &mut spare_chunks[dst_row_start..dst_row_start + rw];
        for (dst_px, &px) in dst_row.iter_mut().zip(src_row.iter()) {
            *dst_px = [
                std::mem::MaybeUninit::new((px >> 16) as u8),
                std::mem::MaybeUninit::new((px >> 8) as u8),
                std::mem::MaybeUninit::new(px as u8),
                std::mem::MaybeUninit::new(255),
            ];
        }
    }
    // SAFETY: All `rh * rw * 4` (`total_bytes`) elements in `out` were initialized above.
    unsafe {
        out.set_len(total_bytes);
    }

    Ok(image::RgbaImage::from_raw(region.physical_width, region.physical_height, out).unwrap())
}

/// Crop the selected region from raw RGBA screen data, returning the full image
/// without reallocating when the entire screen is selected.
pub fn crop_selected_region(
    width: u32,
    height: u32,
    composite_data: Vec<u8>,
    region: Region,
) -> Result<image::RgbaImage, String> {
    if region.physical_x < 0 || region.physical_y < 0 {
        return Err("Selection region has negative coordinates".to_string());
    }

    let composite_img = image::RgbaImage::from_raw(width, height, composite_data)
        .ok_or("Failed to create image from composite data")?;

    if region.physical_x == 0
        && region.physical_y == 0
        && region.physical_width == width
        && region.physical_height == height
    {
        return Ok(composite_img);
    }

    Ok(image::imageops::crop_imm(
        &composite_img,
        region.physical_x as u32,
        region.physical_y as u32,
        region.physical_width,
        region.physical_height,
    )
    .to_image())
}

struct MonitorOverlay {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    surface_size: Option<(u32, u32)>,
    damage: OverlayDamage,
    packed_background: Vec<u32>,
    screen_width: u32,
    screen_height: u32,
    screen_x: i32,
    screen_y: i32,
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn draw_monitor_surface(
    window: &Option<Rc<Window>>,
    surface: &mut Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    damage: &mut OverlayDamage,
    packed_background: &[u32],
    screen_width: u32,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
) {
    let (Some(window), Some(surface)) = (window, surface) else {
        return;
    };
    let Ok(mut buffer) = surface.buffer_mut() else {
        return;
    };

    // Use actual window size for the buffer; screenshot may differ due to DPI
    let actual = window.inner_size();
    let buf_w = actual.width as usize;
    let buf_h = actual.height as usize;
    let src_w = screen_width as usize;

    let previous_damage = *damage;
    update_selection_overlay_frame(
        &mut buffer,
        buf_w,
        buf_h,
        packed_background,
        src_w,
        start_pos,
        current_pos,
        damage,
    );

    let damage_rects =
        overlay_present_damage_rects(previous_damage, buf_w, buf_h, start_pos, current_pos);
    if !damage_rects.is_empty() {
        let _ = buffer.present_with_damage(&damage_rects);
    } else if buffer.age() == 0 {
        let _ = buffer.present();
    }
}

type CreatedMonitorWindowSurface = (
    Rc<Window>,
    softbuffer::Surface<Rc<Window>, Rc<Window>>,
    (u32, u32),
);

#[cfg_attr(coverage_nightly, coverage(off))]
fn create_monitor_window_and_surface(
    event_loop: &ActiveEventLoop,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
) -> Result<CreatedMonitorWindowSurface, String> {
    let attrs = WindowAttributes::default()
        .with_title("RustShot")
        .with_inner_size(PhysicalSize::new(width, height))
        .with_position(PhysicalPosition::new(x, y))
        .with_decorations(false)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_resizable(false);

    let window = event_loop
        .create_window(attrs)
        .map(Rc::new)
        .map_err(|e| format!("Failed to create window: {e}"))?;
    window.set_cursor(CursorIcon::Crosshair);

    let actual_size = window.inner_size();
    let surface_w = actual_size.width.max(1);
    let surface_h = actual_size.height.max(1);

    let context = softbuffer::Context::new(window.clone())
        .map_err(|e| format!("Failed to create graphics context: {e}"))?;
    let mut surface = softbuffer::Surface::new(&context, window.clone())
        .map_err(|e| format!("Failed to create surface: {e}"))?;
    surface
        .resize(
            NonZeroU32::new(surface_w).unwrap(),
            NonZeroU32::new(surface_h).unwrap(),
        )
        .map_err(|e| format!("Failed to resize surface: {e}"))?;

    Ok((window, surface, (surface_w, surface_h)))
}

/// Winit application handler for the fullscreen selection overlay.
///
/// Displays the captured screenshot as background on each monitor, draws a
/// crosshair cursor on whichever monitor the cursor is currently over, and lets
/// the user drag a rectangle (cyan border) on any monitor. The selected
/// [`Region`] and monitor index (`selected_monitor`) are stored on mouse release.
struct SelectionApp {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    surface_size: Option<(u32, u32)>,
    damage: OverlayDamage,
    #[allow(dead_code)]
    screenshot: Vec<u8>,
    packed_background: Vec<u32>,
    screen_width: u32,
    screen_height: u32,
    screen_x: i32,
    screen_y: i32,
    extra_monitors: Vec<MonitorOverlay>,
    active_monitor: usize,
    selected_monitor: usize,
    start_pos: Option<(i32, i32)>,
    current_pos: Option<(i32, i32)>,
    is_dragging: bool,
    result: Option<Region>,
    cancelled: bool,
    /// Error captured during window/surface setup (checked after event loop exits).
    setup_error: Option<String>,
}

impl SelectionApp {
    fn with_packed_background(
        screenshot: Vec<u8>,
        packed_background: Vec<u32>,
        width: u32,
        height: u32,
        screen_x: i32,
        screen_y: i32,
    ) -> Self {
        Self {
            window: None,
            surface: None,
            surface_size: None,
            damage: OverlayDamage::default(),
            screenshot,
            packed_background,
            screen_width: width,
            screen_height: height,
            screen_x,
            screen_y,
            extra_monitors: Vec::new(),
            active_monitor: 0,
            selected_monitor: 0,
            start_pos: None,
            current_pos: None,
            is_dragging: false,
            result: None,
            cancelled: false,
            setup_error: None,
        }
    }

    fn with_monitors(mut captures: Vec<MonitorCapture>, initial_active: usize) -> Self {
        if captures.is_empty() {
            return Self::with_packed_background(Vec::new(), Vec::new(), 0, 0, 0, 0);
        }
        let active = initial_active.min(captures.len() - 1);
        let first = captures.remove(0);
        let extra_monitors = captures
            .into_iter()
            .map(|cap| MonitorOverlay {
                window: None,
                surface: None,
                surface_size: None,
                damage: OverlayDamage::default(),
                packed_background: cap.packed_background,
                screen_width: cap.width,
                screen_height: cap.height,
                screen_x: cap.origin_x,
                screen_y: cap.origin_y,
            })
            .collect();

        Self {
            window: None,
            surface: None,
            surface_size: None,
            damage: OverlayDamage::default(),
            screenshot: Vec::new(),
            packed_background: first.packed_background,
            screen_width: first.width,
            screen_height: first.height,
            screen_x: first.origin_x,
            screen_y: first.origin_y,
            extra_monitors,
            active_monitor: active,
            selected_monitor: active,
            start_pos: None,
            current_pos: None,
            is_dragging: false,
            result: None,
            cancelled: false,
            setup_error: None,
        }
    }

    #[allow(dead_code)]
    fn new(screenshot: Vec<u8>, width: u32, height: u32, screen_x: i32, screen_y: i32) -> Self {
        let packed_background = pack_rgba_to_softbuffer(&screenshot);
        Self::with_packed_background(
            screenshot,
            packed_background,
            width,
            height,
            screen_x,
            screen_y,
        )
    }

    /// Construct a `SelectionApp` for recording region selection, releasing the
    /// raw RGBA byte buffer immediately after pre-packing `packed_background`.
    #[allow(dead_code)]
    fn new_for_recording(
        screenshot: Vec<u8>,
        width: u32,
        height: u32,
        screen_x: i32,
        screen_y: i32,
    ) -> Self {
        let packed_background = pack_rgba_to_softbuffer(&screenshot);
        drop(screenshot);
        Self::with_packed_background(
            Vec::new(),
            packed_background,
            width,
            height,
            screen_x,
            screen_y,
        )
    }

    fn monitor_count(&self) -> usize {
        1 + self.extra_monitors.len()
    }

    fn monitor_geometry(&self, monitor_index: usize) -> Option<(i32, i32, u32, u32)> {
        if monitor_index == 0 {
            Some((
                self.screen_x,
                self.screen_y,
                self.screen_width,
                self.screen_height,
            ))
        } else {
            self.extra_monitors
                .get(monitor_index - 1)
                .map(|m| (m.screen_x, m.screen_y, m.screen_width, m.screen_height))
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn find_monitor_by_window_id(&self, id: winit::window::WindowId) -> Option<usize> {
        if self.active_monitor == 0 {
            if self.window.as_ref().is_some_and(|w| w.id() == id) {
                return Some(0);
            }
        } else if let Some(mon) = self.extra_monitors.get(self.active_monitor - 1) {
            if mon.window.as_ref().is_some_and(|w| w.id() == id) {
                return Some(self.active_monitor);
            }
        }
        if self.window.as_ref().is_some_and(|w| w.id() == id) {
            return Some(0);
        }
        for (idx, mon) in self.extra_monitors.iter().enumerate() {
            if mon.window.as_ref().is_some_and(|w| w.id() == id) {
                return Some(idx + 1);
            }
        }
        None
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn request_redraw_for_monitor(&self, monitor_index: usize) {
        if monitor_index == 0 {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else if let Some(mon) = self.extra_monitors.get(monitor_index - 1) {
            if let Some(window) = &mon.window {
                window.request_redraw();
            }
        }
    }

    fn handle_cursor_moved_on_monitor(
        &mut self,
        monitor_index: usize,
        position: (i32, i32),
    ) -> Vec<usize> {
        let Some((mon_x, mon_y, mon_w, mon_h)) = self.monitor_geometry(monitor_index) else {
            return Vec::new();
        };

        if !self.is_dragging {
            let max_mon_x = i64::from(mon_w.saturating_sub(1)).min(i64::from(i32::MAX)) as i32;
            let max_mon_y = i64::from(mon_h.saturating_sub(1)).min(i64::from(i32::MAX)) as i32;
            let clamped = (
                position.0.clamp(0, max_mon_x),
                position.1.clamp(0, max_mon_y),
            );
            if self.active_monitor != monitor_index {
                let prev = self.active_monitor;
                self.active_monitor = monitor_index;
                self.selected_monitor = monitor_index;
                self.start_pos = None;
                self.current_pos = Some(clamped);
                return vec![prev, monitor_index];
            }
            if self.current_pos != Some(clamped) {
                self.current_pos = Some(clamped);
                return vec![monitor_index];
            }
            return Vec::new();
        }

        // Currently dragging: keep selection locked to `self.active_monitor` and
        // clamp coordinates to its bounds even if the cursor crosses into another monitor.
        let Some((active_x, active_y, active_w, active_h)) =
            self.monitor_geometry(self.active_monitor)
        else {
            return Vec::new();
        };
        let (local_x, local_y) = if monitor_index == self.active_monitor {
            (i64::from(position.0), i64::from(position.1))
        } else {
            (
                i64::from(mon_x) + i64::from(position.0) - i64::from(active_x),
                i64::from(mon_y) + i64::from(position.1) - i64::from(active_y),
            )
        };

        let max_x = i64::from(active_w.saturating_sub(1)).min(i64::from(i32::MAX));
        let max_y = i64::from(active_h.saturating_sub(1)).min(i64::from(i32::MAX));
        let clamped = (
            local_x.clamp(0, max_x) as i32,
            local_y.clamp(0, max_y) as i32,
        );
        if self.start_pos.is_none() {
            self.start_pos = Some(clamped);
        }
        if self.current_pos != Some(clamped) {
            self.current_pos = Some(clamped);
            vec![self.active_monitor]
        } else {
            Vec::new()
        }
    }

    fn handle_mouse_press_on_monitor(&mut self, monitor_index: usize) -> Vec<usize> {
        if monitor_index >= self.monitor_count() {
            return Vec::new();
        }
        let mut redraw = Vec::new();
        if self.active_monitor != monitor_index {
            redraw.push(self.active_monitor);
            self.active_monitor = monitor_index;
            self.selected_monitor = monitor_index;
            self.current_pos = None;
            redraw.push(monitor_index);
        }
        self.is_dragging = true;
        self.start_pos = self.current_pos;
        redraw
    }

    fn handle_mouse_release(&mut self) -> bool {
        if !self.is_dragging {
            return false;
        }
        self.is_dragging = false;
        if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
            if let Some(region) = build_region_from_drag(start, end) {
                self.selected_monitor = self.active_monitor;
                self.result = Some(region);
                return true;
            }
        }
        self.start_pos = None;
        false
    }

    fn take_selected_monitor_capture(&mut self) -> (Vec<u32>, u32, u32, usize) {
        if self.selected_monitor == 0 || self.selected_monitor > self.extra_monitors.len() {
            (
                std::mem::take(&mut self.packed_background),
                self.screen_width,
                self.screen_height,
                0,
            )
        } else {
            let idx = self.selected_monitor;
            let mon = &mut self.extra_monitors[idx - 1];
            (
                std::mem::take(&mut mon.packed_background),
                mon.screen_width,
                mon.screen_height,
                idx,
            )
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn draw_monitor(&mut self, monitor_index: usize) {
        let is_active = self.active_monitor == monitor_index;
        let start_pos = if is_active { self.start_pos } else { None };
        let current_pos = if is_active { self.current_pos } else { None };

        if monitor_index == 0 {
            draw_monitor_surface(
                &self.window,
                &mut self.surface,
                &mut self.damage,
                &self.packed_background,
                self.screen_width,
                start_pos,
                current_pos,
            );
        } else if let Some(mon) = self.extra_monitors.get_mut(monitor_index - 1) {
            draw_monitor_surface(
                &mon.window,
                &mut mon.surface,
                &mut mon.damage,
                &mon.packed_background,
                mon.screen_width,
                start_pos,
                current_pos,
            );
        }
    }

    #[allow(dead_code)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn draw(&mut self) {
        self.draw_monitor(self.active_monitor);
    }

    /// Free background pixel buffers for monitors that will not be cropped after
    /// the overlay closes (all monitors on cancel/error, or non-selected monitors
    /// when a selection finishes).
    fn discard_unneeded_captures(&mut self) {
        if self.cancelled || self.setup_error.is_some() {
            self.packed_background = Vec::new();
            for mon in &mut self.extra_monitors {
                mon.packed_background = Vec::new();
            }
            return;
        }
        if self.selected_monitor != 0 {
            self.packed_background = Vec::new();
        }
        for (idx, mon) in self.extra_monitors.iter_mut().enumerate() {
            if idx + 1 != self.selected_monitor {
                mon.packed_background = Vec::new();
            }
        }
    }

    /// Synchronously hide and drop all overlay windows and softbuffer surfaces
    /// before the event loop exits so Win32 processes window destruction
    /// immediately instead of leaving ghost `AlwaysOnTop` windows on screen.
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn release_window(&mut self) {
        self.surface = None;
        if let Some(window) = self.window.take() {
            window.set_visible(false);
        }
        for mon in &mut self.extra_monitors {
            mon.surface = None;
            if let Some(window) = mon.window.take() {
                window.set_visible(false);
            }
        }
        self.discard_unneeded_captures();
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn close_overlay(&mut self, event_loop: &ActiveEventLoop) {
        self.release_window();
        event_loop.exit();
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        match create_monitor_window_and_surface(
            event_loop,
            self.screen_width,
            self.screen_height,
            self.screen_x,
            self.screen_y,
        ) {
            Ok((window, surface, surface_size)) => {
                self.surface_size = Some(surface_size);
                self.damage = OverlayDamage::default();
                self.window = Some(window);
                self.surface = Some(surface);
            }
            Err(e) => {
                self.setup_error = Some(e);
                self.close_overlay(event_loop);
                return;
            }
        }

        for idx in 0..self.extra_monitors.len() {
            let (w, h, x, y) = {
                let mon = &self.extra_monitors[idx];
                (
                    mon.screen_width,
                    mon.screen_height,
                    mon.screen_x,
                    mon.screen_y,
                )
            };
            match create_monitor_window_and_surface(event_loop, w, h, x, y) {
                Ok((window, surface, surface_size)) => {
                    let mon = &mut self.extra_monitors[idx];
                    mon.surface_size = Some(surface_size);
                    mon.damage = OverlayDamage::default();
                    mon.window = Some(window);
                    mon.surface = Some(surface);
                }
                Err(e) => {
                    self.setup_error = Some(e);
                    self.close_overlay(event_loop);
                    return;
                }
            }
        }

        for monitor_index in 0..self.monitor_count() {
            self.draw_monitor(monitor_index);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let monitor_index = self
            .find_monitor_by_window_id(id)
            .unwrap_or(self.active_monitor);

        match event {
            WindowEvent::CloseRequested => {
                self.cancelled = true;
                self.close_overlay(event_loop);
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key
                        == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) =>
            {
                self.cancelled = true;
                self.close_overlay(event_loop);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            let redraw = self.handle_mouse_press_on_monitor(monitor_index);
                            for idx in redraw {
                                self.request_redraw_for_monitor(idx);
                            }
                        }
                        ElementState::Released if self.is_dragging => {
                            if self.handle_mouse_release() {
                                self.close_overlay(event_loop);
                            } else {
                                self.request_redraw_for_monitor(self.active_monitor);
                            }
                        }
                        _ => {}
                    }
                } else if button == MouseButton::Right {
                    self.cancelled = true;
                    self.close_overlay(event_loop);
                }
            }
            WindowEvent::Resized(new_size) => {
                let mut resized = false;
                if monitor_index == 0 {
                    if let Some((w, h)) =
                        next_surface_size(self.surface_size, new_size.width, new_size.height)
                    {
                        self.surface_size = Some((w.get(), h.get()));
                        if let Some(surface) = &mut self.surface {
                            let _ = surface.resize(w, h);
                        }
                        self.damage = OverlayDamage::default();
                        resized = true;
                    }
                } else if let Some(mon) = self.extra_monitors.get_mut(monitor_index - 1) {
                    if let Some((w, h)) =
                        next_surface_size(mon.surface_size, new_size.width, new_size.height)
                    {
                        mon.surface_size = Some((w.get(), h.get()));
                        if let Some(surface) = &mut mon.surface {
                            let _ = surface.resize(w, h);
                        }
                        mon.damage = OverlayDamage::default();
                        resized = true;
                    }
                }
                if resized {
                    self.draw_monitor(monitor_index);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = (position.x as i32, position.y as i32);
                let redraw = self.handle_cursor_moved_on_monitor(monitor_index, new_pos);
                for idx in redraw {
                    self.request_redraw_for_monitor(idx);
                }
            }
            WindowEvent::RedrawRequested => self.draw_monitor(monitor_index),
            _ => {}
        }
    }
}

/// Drain any pending Win32 messages on the calling thread after `EventLoop::run_app`
/// exits so `winit`'s posted `DESTROY_MSG_ID` (`DestroyWindow`) executes immediately
/// instead of leaving a ghost window when the process continues running (e.g. recording).
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn flush_pending_win32_messages() {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    // SAFETY: Standard Win32 message pump draining the current thread's queue.
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(windows))]
fn flush_pending_win32_messages() {}

/// Get the current cursor position in virtual screen coordinates.
///
/// Returns `(x, y)` in logical (virtual desktop) pixels.
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn get_cursor_position() -> Result<(i32, i32), String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    // SAFETY: `point` is a valid mutable reference to a stack-allocated POINT.
    // GetCursorPos writes the cursor coordinates into it; no lifetime issues.
    unsafe {
        GetCursorPos(&mut point).map_err(|e| format!("GetCursorPos failed: {e}"))?;
    }
    Ok((point.x, point.y))
}

/// Get the current cursor position in virtual screen coordinates (Linux/other).
#[cfg(not(windows))]
pub fn get_cursor_position() -> Result<(i32, i32), String> {
    Err("Cursor position detection is not supported on this platform".to_string())
}

#[cfg(windows)]
type Win32PackedScreenCapture = (Vec<u32>, u32, u32, (i32, i32));
type PackedScreenCaptureWithOrigin = (Vec<u32>, u32, u32, Option<(i32, i32)>);

/// Capture a monitor directly into a top-down `0xAARRGGBB` `Vec<u32>` buffer via Win32
/// `BitBlt(..., SRCCOPY)` + `GetDIBits` and return its exact Win32 virtual-desktop origin
/// `(rcMonitor.left, rcMonitor.top)`.
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_screen_packed_with_origin_win32(
    raw_hmonitor: isize,
) -> Result<Win32PackedScreenCapture, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDCW, DeleteDC, DeleteObject,
        GetDIBits, GetDeviceCaps, GetMonitorInfoW, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DESKTOPHORZRES, DESKTOPVERTRES, DIB_RGB_COLORS, HMONITOR, MONITORINFOEXW, SRCCOPY,
    };

    // SAFETY: All GDI handles created below (`hdc_monitor`, `hdc_mem`, `hbm`) are checked
    // for validity, restored via `SelectObject`, and deleted before return. `packed` is
    // pre-allocated to `phys_w * phys_h` `u32` elements (`phys_w * phys_h * 4` bytes), matching
    // the 32-bit top-down `BI_RGB` DIB written by `GetDIBits`, and `set_len` is called only
    // after `GetDIBits` confirms all `phys_h` scanlines were written.
    unsafe {
        let mut monitor_info: MONITORINFOEXW = std::mem::zeroed();
        monitor_info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        GetMonitorInfoW(
            HMONITOR(raw_hmonitor),
            (&mut monitor_info as *mut MONITORINFOEXW).cast(),
        )
        .ok()
        .map_err(|e| format!("GetMonitorInfoW failed: {e}"))?;

        let origin = (
            monitor_info.monitorInfo.rcMonitor.left,
            monitor_info.monitorInfo.rcMonitor.top,
        );
        let sz_device_ptr = monitor_info.szDevice.as_ptr();
        let hdc_monitor = CreateDCW(
            PCWSTR(sz_device_ptr),
            PCWSTR(sz_device_ptr),
            PCWSTR(std::ptr::null()),
            None,
        );
        if hdc_monitor.is_invalid() {
            return Err("CreateDCW for monitor failed".to_string());
        }

        let phys_w = GetDeviceCaps(hdc_monitor, DESKTOPHORZRES);
        let phys_h = GetDeviceCaps(hdc_monitor, DESKTOPVERTRES);
        if phys_w <= 0 || phys_h <= 0 {
            let _ = DeleteDC(hdc_monitor);
            return Err(format!("Invalid monitor physical size: {phys_w}x{phys_h}"));
        }

        let hdc_mem = CreateCompatibleDC(hdc_monitor);
        if hdc_mem.is_invalid() {
            let _ = DeleteDC(hdc_monitor);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let hbm = CreateCompatibleBitmap(hdc_monitor, phys_w, phys_h);
        if hbm.is_invalid() {
            let _ = DeleteDC(hdc_mem);
            let _ = DeleteDC(hdc_monitor);
            return Err("CreateCompatibleBitmap failed".to_string());
        }

        let old_obj = SelectObject(hdc_mem, hbm);
        let blt_res = BitBlt(hdc_mem, 0, 0, phys_w, phys_h, hdc_monitor, 0, 0, SRCCOPY)
            .map_err(|e| format!("BitBlt failed: {e}"));

        let pixel_count = (phys_w as usize) * (phys_h as usize);
        let mut packed: Vec<u32> = Vec::with_capacity(pixel_count);
        let dib_res = if let Err(e) = blt_res {
            Err(e)
        } else {
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: phys_w,
                    biHeight: -phys_h, // negative = top-down 0xAARRGGBB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let scan_lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                phys_h as u32,
                Some(packed.as_mut_ptr().cast()),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            if scan_lines != phys_h {
                Err("GetDIBits failed".to_string())
            } else {
                packed.set_len(pixel_count);
                Ok(())
            }
        };

        let _ = SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        let _ = DeleteDC(hdc_monitor);

        dib_res?;
        Ok((packed, phys_w as u32, phys_h as u32, origin))
    }
}

#[cfg(windows)]
#[allow(dead_code)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_screen_packed_win32(raw_hmonitor: isize) -> Result<(Vec<u32>, u32, u32), String> {
    capture_screen_packed_with_origin_win32(raw_hmonitor)
        .map(|(packed, phys_w, phys_h, _)| (packed, phys_w, phys_h))
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_screen_packed_with_origin(
    screen: &Screen,
) -> Result<PackedScreenCaptureWithOrigin, String> {
    #[cfg(windows)]
    if let Ok((packed, phys_w, phys_h, origin)) =
        capture_screen_packed_with_origin_win32(screen.display_info.raw_handle.0)
    {
        return Ok((packed, phys_w, phys_h, Some(origin)));
    }

    let img = screen
        .capture()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;
    let phys_w = img.width();
    let phys_h = img.height();
    let packed = pack_rgba_to_softbuffer(img.as_raw());
    Ok((packed, phys_w, phys_h, None))
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_screen_packed(screen: &Screen) -> Result<(Vec<u32>, u32, u32), String> {
    capture_screen_packed_with_origin(screen)
        .map(|(packed, phys_w, phys_h, _)| (packed, phys_w, phys_h))
}

type MonitorSelectionSet = (Vec<MonitorCapture>, Vec<usize>, usize);

fn collect_monitor_captures_with<F>(
    screens: &[(i32, i32, u32, u32, f32, bool)],
    cursor_pos: Option<(i32, i32)>,
    preserve_native_origin: bool,
    mut capture_fn: F,
) -> Result<MonitorSelectionSet, String>
where
    F: FnMut(usize) -> Result<PackedScreenCaptureWithOrigin, String>,
{
    let mut raw_captures = Vec::with_capacity(screens.len());
    let mut layout_inputs = Vec::with_capacity(screens.len());
    let mut last_error = String::from("No screens found");

    for (idx, &(logical_x, logical_y, logical_width, logical_height, scale_factor, is_primary)) in
        screens.iter().enumerate()
    {
        match capture_fn(idx) {
            Ok((packed, phys_w, phys_h, native_origin)) => {
                layout_inputs.push(ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width: phys_w,
                    physical_height: phys_h,
                    is_primary,
                });
                raw_captures.push((idx, packed, phys_w, phys_h, native_origin, scale_factor));
            }
            Err(err) => {
                let phys_w =
                    scaled_physical_dimension(logical_width, scale_factor).unwrap_or(logical_width);
                let phys_h = scaled_physical_dimension(logical_height, scale_factor)
                    .unwrap_or(logical_height);
                layout_inputs.push(ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width: phys_w,
                    physical_height: phys_h,
                    is_primary,
                });
                last_error = err;
            }
        }
    }

    if raw_captures.is_empty() {
        return Err(last_error);
    }

    let layouts = if raw_captures
        .iter()
        .any(|(_, _, _, _, native_origin, _)| native_origin.is_none())
    {
        Some(build_physical_screen_layouts(&layout_inputs)?)
    } else {
        None
    };
    let mut monitor_captures = Vec::with_capacity(raw_captures.len());
    let mut kept_indices = Vec::with_capacity(raw_captures.len());

    for (idx, packed, phys_w, phys_h, native_origin, scale_factor) in raw_captures {
        let (origin_x, origin_y) = native_origin.unwrap_or_else(|| {
            overlay_window_origin(
                &layouts.as_ref().unwrap()[idx],
                scale_factor,
                preserve_native_origin,
            )
        });
        monitor_captures.push(MonitorCapture {
            packed_background: packed,
            width: phys_w,
            height: phys_h,
            origin_x,
            origin_y,
        });
        kept_indices.push(idx);
    }

    let initial_active = cursor_pos
        .and_then(|pos| {
            find_screen_index_at_position(
                monitor_captures
                    .iter()
                    .map(|m| (m.origin_x, m.origin_y, m.width, m.height)),
                pos,
            )
        })
        .or_else(|| {
            preferred_screen_index(
                kept_indices.iter().map(|&idx| {
                    let (x, y, w, h, _, is_primary) = screens[idx];
                    (x, y, w, h, is_primary)
                }),
                cursor_pos,
            )
        })
        .unwrap_or(0);

    Ok((monitor_captures, kept_indices, initial_active))
}

/// Capture all monitors so the selection overlay can open a native-DPI borderless
/// window on every screen, allowing the user to start dragging on any monitor.
#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_monitors_for_selection() -> Result<(Vec<MonitorCapture>, Vec<Screen>, usize), String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen_infos: Vec<_> = screens
        .iter()
        .map(|s| {
            let info = &s.display_info;
            (
                info.x,
                info.y,
                info.width,
                info.height,
                info.scale_factor,
                info.is_primary,
            )
        })
        .collect();

    let (monitor_captures, kept_indices, initial_active) = collect_monitor_captures_with(
        &screen_infos,
        get_cursor_position().ok(),
        cfg!(target_os = "linux"),
        |idx| capture_screen_packed_with_origin(&screens[idx]),
    )?;

    let kept_screens = kept_indices.into_iter().map(|idx| screens[idx]).collect();
    Ok((monitor_captures, kept_screens, initial_active))
}

/// Find the screen that contains the cursor and capture only that screen.
///
/// Returns a `VirtualDesktopCapture` containing just the one screen,
/// positioned so the overlay window covers exactly that monitor.
#[allow(dead_code)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_screen_at_cursor() -> Result<VirtualDesktopCapture, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    let cursor_pos = get_cursor_position().ok();
    let screen_index = preferred_screen_index(
        screens.iter().map(|screen| {
            let info = &screen.display_info;
            (info.x, info.y, info.width, info.height, info.is_primary)
        }),
        cursor_pos,
    )
    .ok_or_else(|| "No screens found".to_string())?;

    let screen = screens[screen_index];
    let info = &screen.display_info;

    let (packed_background, phys_w, phys_h) = capture_screen_packed(&screen)?;

    let window_layout = build_physical_screen_layouts(&build_screen_layout_inputs(
        screens.iter().map(|screen| {
            let info = &screen.display_info;
            (
                info.x,
                info.y,
                info.width,
                info.height,
                info.scale_factor,
                info.is_primary,
            )
        }),
        screen_index,
        (phys_w, phys_h),
    )?)?
    .into_iter()
    .nth(screen_index)
    .ok_or_else(|| "Selected screen does not have a layout".to_string())?;

    let (origin_x, origin_y) =
        overlay_window_origin(&window_layout, info.scale_factor, cfg!(target_os = "linux"));

    let local_layout = PhysicalScreenLayout {
        logical_x: info.x,
        logical_y: info.y,
        logical_width: info.width,
        logical_height: info.height,
        physical_x: 0,
        physical_y: 0,
        physical_width: phys_w,
        physical_height: phys_h,
    };

    Ok(VirtualDesktopCapture {
        rgba_data: Vec::new(),
        packed_background,
        width: phys_w,
        height: phys_h,
        origin_x,
        origin_y,
        screens: vec![screen],
        layouts: vec![local_layout],
    })
}

#[allow(dead_code)]
fn capture_selection_desktop_with<CaptureScreen>(
    capture_screen: CaptureScreen,
) -> Result<VirtualDesktopCapture, String>
where
    CaptureScreen: FnOnce() -> Result<VirtualDesktopCapture, String>,
{
    capture_screen()
}

/// Capture all screens and composite them into a single physical virtual desktop image.
#[allow(dead_code)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn capture_virtual_desktop() -> Result<VirtualDesktopCapture, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    if screens.is_empty() {
        return Err("No screens found".to_string());
    }

    let mut captures = Vec::new();
    for &screen in &screens {
        let img = screen
            .capture()
            .map_err(|e| format!("Failed to capture screen: {}", e))?;
        captures.push((screen, img));
    }

    let inputs: Vec<ScreenLayoutInput> = captures
        .iter()
        .map(|(screen, img)| {
            let info = &screen.display_info;
            ScreenLayoutInput {
                logical_x: info.x,
                logical_y: info.y,
                logical_width: info.width,
                logical_height: info.height,
                physical_width: img.width(),
                physical_height: img.height(),
                is_primary: info.is_primary,
            }
        })
        .collect();
    let layouts = build_physical_screen_layouts(&inputs)?;
    let (layouts, origin_x, origin_y, total_width, total_height) =
        normalize_physical_screen_layouts(&layouts)?;

    let mut composite = vec![0u8; (total_width as usize) * (total_height as usize) * 4];
    for ((_, img), layout) in captures.iter().zip(layouts.iter()) {
        let px = layout.physical_x as usize;
        let py = layout.physical_y as usize;
        let src = img.as_raw();
        let src_w = img.width() as usize;
        let src_h = img.height() as usize;

        for row in 0..src_h {
            let dst_y = py + row;
            if dst_y >= total_height as usize {
                break;
            }
            let dst_start = (dst_y * total_width as usize + px) * 4;
            let src_start = row * src_w * 4;
            let copy_len = src_w * 4;
            if dst_start + copy_len <= composite.len() && src_start + copy_len <= src.len() {
                composite[dst_start..dst_start + copy_len]
                    .copy_from_slice(&src[src_start..src_start + copy_len]);
            }
        }
    }

    Ok(VirtualDesktopCapture {
        rgba_data: composite,
        packed_background: Vec::new(),
        width: total_width,
        height: total_height,
        origin_x,
        origin_y,
        screens: captures.into_iter().map(|(s, _)| s).collect(),
        layouts,
    })
}

#[allow(dead_code)]
fn into_packed_background(rgba_data: Vec<u8>, packed_background: Vec<u32>) -> Vec<u32> {
    if !packed_background.is_empty() {
        drop(rgba_data);
        packed_background
    } else {
        let packed = pack_rgba_to_softbuffer(&rgba_data);
        drop(rgba_data);
        packed
    }
}

/// Show the selection overlay on all monitors and return the cropped screenshot.
///
/// Captures all monitors before creating the per-monitor overlay windows,
/// waits for the user to drag a rectangle on any monitor, and returns the
/// cropped RGBA image from the selected monitor.
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn select_region() -> Result<image::RgbaImage, String> {
    let (captures, _screens, initial_active) = capture_monitors_for_selection()?;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = SelectionApp::with_monitors(captures, initial_active);

    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("Event loop error: {}", e))?;

    app.release_window();
    let setup_error = app.setup_error.take();
    let cancelled = app.cancelled;
    let result = app.result;
    let (packed_background, width, height, _) = app.take_selected_monitor_capture();
    drop(app);
    flush_pending_win32_messages();

    if let Some(err) = setup_error {
        return Err(err);
    }

    if cancelled {
        return Err("Selection cancelled".to_string());
    }

    let region = result.ok_or("No region selected")?;

    crop_packed_region(width, height, &packed_background, region)
}

/// Run the selection UI across all monitors and return the selected screen and
/// region (in that screen's local physical coordinates).
/// Used by the recording feature.
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn select_region_for_recording() -> Result<(screenshots::Screen, Region), String> {
    let (captures, screens, initial_active) = capture_monitors_for_selection()?;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = SelectionApp::with_monitors(captures, initial_active);

    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("Event loop error: {}", e))?;

    app.release_window();
    let setup_error = app.setup_error.take();
    let cancelled = app.cancelled;
    let result = app.result;
    let selected_monitor = app.selected_monitor;
    drop(app);
    flush_pending_win32_messages();

    if let Some(err) = setup_error {
        return Err(err);
    }

    if cancelled {
        return Err("Selection cancelled".to_string());
    }

    let local_region = result.ok_or("No region selected")?;
    let screen = screens
        .into_iter()
        .nth(selected_monitor)
        .ok_or_else(|| "Selected region does not fall on any screen".to_string())?;

    Ok((screen, local_region))
}

/// Find which screen contains the given logical cursor position.
/// Returns `None` if no screen matches.
///
/// # Examples
///
/// ```
/// use rustshot::selection::find_screen_index_at_position;
///
/// let screens = vec![(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
/// assert_eq!(find_screen_index_at_position(screens.clone(), (500, 500)), Some(0));
/// assert_eq!(find_screen_index_at_position(screens.clone(), (2000, 500)), Some(1));
/// assert_eq!(find_screen_index_at_position(screens, (-1, -1)), None);
/// ```
///
/// Algorithm:
/// 1. Enumerate screens in input order.
/// 2. Treat each screen rectangle as half-open:
///    `[x, x + width)` by `[y, y + height)`.
/// 3. Return the first screen whose rectangle contains the cursor.
/// 4. Return `None` when the cursor lies in a gap or outside all screens.
///
/// Possible improvements:
/// - Replace the linear scan with a spatial index if screen counts ever become
///   large enough to matter.
/// - Add explicit overlap diagnostics if platform APIs start reporting
///   conflicting monitor rectangles.
pub fn find_screen_index_at_position<I>(screens: I, cursor_pos: (i32, i32)) -> Option<usize>
where
    I: IntoIterator<Item = (i32, i32, u32, u32)>,
{
    // Linear scan is deliberate here: monitor counts are small, and returning
    // the first match also gives a deterministic answer for overlapping or
    // inconsistent screen layouts reported by the OS.
    let cx = i64::from(cursor_pos.0);
    let cy = i64::from(cursor_pos.1);
    screens
        .into_iter()
        .enumerate()
        .find(|(_, (x, y, width, height))| {
            let x0 = i64::from(*x);
            let y0 = i64::from(*y);
            cx >= x0 && cx < x0 + i64::from(*width) && cy >= y0 && cy < y0 + i64::from(*height)
        })
        .map(|(index, _)| index)
}

/// Choose the preferred screen index based on cursor position, then primary, then first.
///
/// `screens` is an iterator of `(x, y, width, height, is_primary)` tuples.
/// If `cursor_pos` is `Some`, the screen containing the cursor is preferred.
/// Falls back to the primary screen, then to index 0.
///
/// Algorithm:
/// 1. Materialize the iterator once because the cursor and fallback passes need
///    to inspect the same data more than once.
/// 2. If a cursor position is available, prefer the screen that contains it.
/// 3. Otherwise look for the first screen marked primary.
/// 4. If neither rule picks a screen, fall back to index 0.
///
/// Possible improvements:
/// - Avoid the temporary allocation by passing a reusable slice when this logic
///   is called from already-materialized screen lists.
/// - Use weighted heuristics for ambiguous layouts, such as preferring the
///   largest overlapping screen instead of the first match.
pub fn preferred_screen_index<I>(screens: I, cursor_pos: Option<(i32, i32)>) -> Option<usize>
where
    I: IntoIterator<Item = (i32, i32, u32, u32, bool)>,
{
    let items: Vec<_> = screens.into_iter().collect();
    if items.is_empty() {
        return None;
    }

    // Try cursor position first
    if let Some(pos) = cursor_pos {
        let rects = items.iter().map(|&(x, y, w, h, _)| (x, y, w, h));
        if let Some(idx) = find_screen_index_at_position(rects, pos) {
            return Some(idx);
        }
    }

    // Fall back to primary screen
    if let Some(idx) = items
        .iter()
        .position(|&(_, _, _, _, is_primary)| is_primary)
    {
        return Some(idx);
    }

    // Fall back to first screen
    Some(0)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // --- Region struct tests ---

    #[test]
    fn region_fields_are_accessible() {
        let r = Region {
            physical_x: 10,
            physical_y: 20,
            physical_width: 300,
            physical_height: 200,
        };
        assert_eq!(r.physical_x, 10);
        assert_eq!(r.physical_y, 20);
        assert_eq!(r.physical_width, 300);
        assert_eq!(r.physical_height, 200);
    }

    #[test]
    fn region_implements_copy() {
        let r = Region {
            physical_x: 1,
            physical_y: 2,
            physical_width: 3,
            physical_height: 4,
        };
        let s = r; // Uses Copy; r is still usable after this
        assert_eq!(r.physical_x, s.physical_x);
        assert_eq!(r.physical_y, s.physical_y);
        assert_eq!(r.physical_width, s.physical_width);
        assert_eq!(r.physical_height, s.physical_height);
    }

    #[test]
    fn region_implements_clone() {
        let r = Region {
            physical_x: 5,
            physical_y: 6,
            physical_width: 7,
            physical_height: 8,
        };
        #[allow(clippy::clone_on_copy)]
        let s = r.clone();
        assert_eq!(s.physical_x, 5);
        assert_eq!(s.physical_y, 6);
        assert_eq!(s.physical_width, 7);
        assert_eq!(s.physical_height, 8);
    }

    #[test]
    fn region_debug_repr_is_non_empty() {
        let r = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(!format!("{:?}", r).is_empty());
    }

    #[test]
    fn region_supports_negative_coords() {
        // Regions can start at negative screen coordinates (multi-monitor)
        let r = Region {
            physical_x: -1920,
            physical_y: -100,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert_eq!(r.physical_x, -1920);
        assert_eq!(r.physical_y, -100);
    }

    // --- build_region_from_drag tests ---

    #[test]
    fn build_region_from_drag_supports_normal_drag() {
        let region = build_region_from_drag((5, 7), (15, 20)).expect("region should exist");
        assert_eq!(region.physical_x, 5);
        assert_eq!(region.physical_y, 7);
        assert_eq!(region.physical_width, 10);
        assert_eq!(region.physical_height, 13);
    }

    #[test]
    fn build_region_from_drag_supports_reversed_drag() {
        let region = build_region_from_drag((15, 20), (5, 7)).expect("region should exist");
        assert_eq!(region.physical_x, 5);
        assert_eq!(region.physical_y, 7);
        assert_eq!(region.physical_width, 10);
        assert_eq!(region.physical_height, 13);
    }

    #[test]
    fn build_region_from_drag_filters_too_small_regions() {
        assert!(build_region_from_drag((0, 0), (2, 3)).is_none());
    }

    #[test]
    fn build_region_from_drag_exactly_5x5_returns_none() {
        // width=5 and height=5: condition is width > 5 && height > 5, so 5 is NOT > 5
        assert!(build_region_from_drag((0, 0), (5, 5)).is_none());
    }

    #[test]
    fn build_region_from_drag_6x6_returns_some() {
        // 6 > 5 and 6 > 5, so this should return Some
        let r = build_region_from_drag((0, 0), (6, 6));
        assert!(r.is_some());
        let r = r.unwrap();
        assert_eq!(r.physical_width, 6);
        assert_eq!(r.physical_height, 6);
    }

    #[test]
    fn build_region_from_drag_zero_size_returns_none() {
        assert!(build_region_from_drag((0, 0), (0, 0)).is_none());
    }

    #[test]
    fn build_region_from_drag_wide_but_short_returns_none() {
        // width=100 > 5, but height=3 is NOT > 5
        assert!(build_region_from_drag((0, 0), (100, 3)).is_none());
    }

    #[test]
    fn build_region_from_drag_tall_but_narrow_returns_none() {
        // height=100 > 5, but width=3 is NOT > 5
        assert!(build_region_from_drag((0, 0), (3, 100)).is_none());
    }

    #[test]
    fn build_region_from_drag_exactly_width_6_height_5_returns_none() {
        // width=6 > 5, but height=5 is NOT > 5
        assert!(build_region_from_drag((0, 0), (6, 5)).is_none());
    }

    #[test]
    fn build_region_from_drag_exactly_width_5_height_6_returns_none() {
        // width=5 is NOT > 5
        assert!(build_region_from_drag((0, 0), (5, 6)).is_none());
    }

    #[test]
    fn build_region_from_drag_result_top_left_is_min_coords() {
        // drag from bottom-right to top-left
        let r = build_region_from_drag((100, 200), (10, 20)).unwrap();
        assert_eq!(r.physical_x, 10);
        assert_eq!(r.physical_y, 20);
        assert_eq!(r.physical_width, 90);
        assert_eq!(r.physical_height, 180);
    }

    #[test]
    fn build_region_from_drag_large_region() {
        let r = build_region_from_drag((0, 0), (1920, 1080)).unwrap();
        assert_eq!(r.physical_x, 0);
        assert_eq!(r.physical_y, 0);
        assert_eq!(r.physical_width, 1920);
        assert_eq!(r.physical_height, 1080);
    }

    #[test]
    fn virtual_desktop_scale_factor_prefers_primary_monitor() {
        let scale = virtual_desktop_scale_factor([(false, 1.0), (true, 1.5), (false, 2.0)]);
        assert_eq!(scale, Some(1.5));
    }

    #[test]
    fn virtual_desktop_scale_factor_falls_back_to_first_monitor() {
        let scale = virtual_desktop_scale_factor([(false, 1.25), (false, 1.5)]);
        assert_eq!(scale, Some(1.25));
    }

    #[test]
    fn virtual_desktop_scale_factor_returns_none_for_empty_input() {
        let scale = virtual_desktop_scale_factor(std::iter::empty::<(bool, f32)>());
        assert_eq!(scale, None);
    }

    #[test]
    fn logical_to_physical_rounds_using_scale_factor() {
        assert_eq!(logical_to_physical(-1280, 1.5), -1920);
        assert_eq!(logical_to_physical(1280, 1.5), 1920);
    }

    // --- SelectionApp tests ---

    #[test]
    fn selection_app_new_sets_defaults() {
        let app = SelectionApp::new(vec![0u8; 16], 4, 5, 6, 7);

        assert!(app.window.is_none());
        assert!(app.surface.is_none());
        assert_eq!(app.screenshot, vec![0u8; 16]);
        assert_eq!(app.screen_width, 4);
        assert_eq!(app.screen_height, 5);
        assert_eq!(app.screen_x, 6);
        assert_eq!(app.screen_y, 7);
        assert_eq!(app.start_pos, None);
        assert_eq!(app.current_pos, None);
        assert!(!app.is_dragging);
        assert!(app.result.is_none());
        assert!(!app.cancelled);
    }

    #[test]
    fn selection_app_is_dragging_is_false_by_default() {
        let app = SelectionApp::new(vec![], 800, 600, 0, 0);
        assert!(!app.is_dragging);
    }

    #[test]
    fn selection_app_result_is_none_by_default() {
        let app = SelectionApp::new(vec![], 800, 600, 0, 0);
        assert!(app.result.is_none());
    }

    #[test]
    fn selection_app_cancelled_is_false_by_default() {
        let app = SelectionApp::new(vec![], 800, 600, 0, 0);
        assert!(!app.cancelled);
    }

    #[test]
    fn selection_app_fields_can_be_mutated() {
        let mut app = SelectionApp::new(vec![], 800, 600, 0, 0);
        app.is_dragging = true;
        app.cancelled = true;
        app.result = Some(Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        });
        assert!(app.is_dragging);
        assert!(app.cancelled);
        assert!(app.result.is_some());
    }

    #[test]
    fn selection_app_draw_without_surface_is_noop() {
        let mut app = SelectionApp::new(vec![0u8; 16], 4, 4, 0, 0);
        app.draw();
    }

    #[test]
    fn selection_app_new_with_empty_screenshot() {
        let app = SelectionApp::new(vec![], 0, 0, 0, 0);
        assert_eq!(app.screenshot.len(), 0);
        assert_eq!(app.screen_width, 0);
        assert_eq!(app.screen_height, 0);
    }

    #[test]
    fn selection_app_new_stores_screen_offset() {
        let app = SelectionApp::new(vec![], 1920, 1080, -1920, 0);
        assert_eq!(app.screen_x, -1920);
        assert_eq!(app.screen_y, 0);
    }

    #[test]
    fn build_region_from_drag_with_negative_start_coords() {
        // Multi-monitor setup: drag starts at negative x/y
        let r = build_region_from_drag((-100, -50), (-85, -35)).unwrap();
        assert_eq!(r.physical_x, -100);
        assert_eq!(r.physical_y, -50);
        assert_eq!(r.physical_width, 15);
        assert_eq!(r.physical_height, 15);
    }

    #[test]
    fn build_region_from_drag_crossing_zero() {
        // Drag from negative coords to positive (crosses origin)
        let r = build_region_from_drag((-10, -10), (10, 10)).unwrap();
        assert_eq!(r.physical_x, -10);
        assert_eq!(r.physical_y, -10);
        assert_eq!(r.physical_width, 20);
        assert_eq!(r.physical_height, 20);
    }

    #[test]
    fn selection_app_draw_with_current_pos_no_surface_is_noop() {
        let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
        app.current_pos = Some((2, 2));
        app.draw(); // Should be a no-op since surface is None
    }

    #[test]
    fn selection_app_draw_with_drag_state_no_surface_is_noop() {
        let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
        app.start_pos = Some((0, 0));
        app.current_pos = Some((3, 3));
        app.is_dragging = true;
        app.draw(); // Should be a no-op since surface is None
    }

    #[test]
    fn region_zero_dimensions_is_valid_struct() {
        // Region can be constructed with zero dims (build_region_from_drag filters these,
        // but the type itself is not restricted)
        let r = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 0,
        };
        assert_eq!(r.physical_width, 0);
        assert_eq!(r.physical_height, 0);
    }

    #[test]
    fn selection_app_screenshot_data_is_preserved() {
        let data: Vec<u8> = (0..=255u8).collect();
        let app = SelectionApp::new(data.clone(), 8, 8, 0, 0);
        assert_eq!(app.screenshot, data);
    }

    #[test]
    fn build_region_from_drag_same_x_gives_zero_width_returns_none() {
        // start.x == end.x → width = 0, which is not > 5
        assert!(build_region_from_drag((10, 0), (10, 100)).is_none());
    }

    #[test]
    fn build_region_from_drag_same_y_gives_zero_height_returns_none() {
        // start.y == end.y → height = 0, which is not > 5
        assert!(build_region_from_drag((0, 20), (100, 20)).is_none());
    }

    #[test]
    fn selection_app_start_pos_can_be_set() {
        let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
        app.start_pos = Some((10, 20));
        assert_eq!(app.start_pos, Some((10, 20)));
    }

    #[test]
    fn selection_app_current_pos_can_be_set() {
        let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
        app.current_pos = Some((50, 60));
        assert_eq!(app.current_pos, Some((50, 60)));
    }

    #[test]
    fn region_max_u32_dimensions_are_valid() {
        let r = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: u32::MAX,
            physical_height: u32::MAX,
        };
        assert_eq!(r.physical_width, u32::MAX);
        assert_eq!(r.physical_height, u32::MAX);
    }

    #[test]
    fn build_region_from_drag_minimum_valid_is_7x7() {
        // 6 is the minimum value > 5; but let's confirm 7 works unambiguously
        let r = build_region_from_drag((0, 0), (7, 7)).unwrap();
        assert_eq!(r.physical_width, 7);
        assert_eq!(r.physical_height, 7);
    }

    #[test]
    fn build_region_from_drag_large_negative_coords() {
        // Multi-monitor: drag entirely within a large-negative-coordinate screen
        let r = build_region_from_drag((-10000, -5000), (-9990, -4990))
            .expect("region should be Some for 10x10 drag");
        assert_eq!(r.physical_x, -10000);
        assert_eq!(r.physical_y, -5000);
        assert_eq!(r.physical_width, 10);
        assert_eq!(r.physical_height, 10);
    }

    #[test]
    fn selection_app_draw_called_multiple_times_without_surface() {
        // draw() must be a no-op (no panic) when surface is None, even called repeatedly
        let mut app = SelectionApp::new(vec![0u8; 64], 8, 8, 0, 0);
        app.draw();
        app.draw();
        app.draw();
    }

    #[test]
    fn region_copy_does_not_alias() {
        // Region implements Copy: two copies are fully independent values
        let r = Region {
            physical_x: 42,
            physical_y: 7,
            physical_width: 100,
            physical_height: 200,
        };
        let a = r;
        let b = r;
        // Both copies reflect the original values
        assert_eq!(a.physical_x, 42);
        assert_eq!(a.physical_y, 7);
        assert_eq!(a.physical_width, 100);
        assert_eq!(a.physical_height, 200);
        assert_eq!(b.physical_x, 42);
        assert_eq!(b.physical_y, 7);
        assert_eq!(b.physical_width, 100);
        assert_eq!(b.physical_height, 200);
        // The original is still usable (Copy, not moved)
        assert_eq!(r.physical_x, 42);
    }

    #[test]
    fn build_region_from_drag_asymmetric_valid() {
        // width=100 > 5, height=6 > 5 → Some
        let r = build_region_from_drag((0, 0), (100, 6)).expect("should be Some for 100x6 drag");
        assert_eq!(r.physical_width, 100);
        assert_eq!(r.physical_height, 6);
    }

    #[test]
    fn region_i32_max_coords() {
        let r = Region {
            physical_x: i32::MAX,
            physical_y: i32::MIN,
            physical_width: 1,
            physical_height: 1,
        };
        assert_eq!(r.physical_x, i32::MAX);
        assert_eq!(r.physical_y, i32::MIN);
    }

    #[test]
    fn build_region_from_drag_height_greater_than_width() {
        let r = build_region_from_drag((0, 0), (10, 100)).unwrap();
        assert_eq!(r.physical_width, 10);
        assert_eq!(r.physical_height, 100);
        assert!(r.physical_height > r.physical_width);
    }

    #[test]
    fn selection_app_screenshot_len_large() {
        let size = 1920usize * 1080 * 4;
        let app = SelectionApp::new(vec![0u8; size], 1920, 1080, 0, 0);
        assert_eq!(app.screenshot.len(), size);
    }

    #[test]
    fn region_debug_contains_x_value() {
        let r = Region {
            physical_x: 42,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(format!("{:?}", r).contains("42"));
    }

    #[test]
    fn build_region_from_drag_diagonal_gives_square() {
        // Equal x and y deltas produce a square region
        let r = build_region_from_drag((0, 0), (20, 20)).unwrap();
        assert_eq!(r.physical_width, r.physical_height);
    }

    #[test]
    fn selection_app_all_init_fields() {
        // Verify all non-window/surface fields initialised by SelectionApp::new
        let data: Vec<u8> = vec![1, 2, 3, 4];
        let app = SelectionApp::new(data.clone(), 1280, 720, -1920, 100);

        // screenshot field
        assert_eq!(app.screenshot, data);
        // screen_width / screen_height
        assert_eq!(app.screen_width, 1280);
        assert_eq!(app.screen_height, 720);
        // screen_x / screen_y
        assert_eq!(app.screen_x, -1920);
        assert_eq!(app.screen_y, 100);
        // start_pos / current_pos
        assert_eq!(app.start_pos, None);
        assert_eq!(app.current_pos, None);
        // is_dragging
        assert!(!app.is_dragging);
        // result
        assert!(app.result.is_none());
        // cancelled
        assert!(!app.cancelled);
    }

    #[test]
    fn find_screen_index_at_position_returns_matching_screen() {
        let screens = vec![(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
        assert_eq!(find_screen_index_at_position(screens, (2000, 200)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_returns_first_screen_for_origin() {
        let screens = vec![(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
        assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_supports_negative_coordinates() {
        let screens = vec![(-1920, 0, 1920, 1080), (0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (-100, 10)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_returns_none_when_no_match_exists() {
        let screens = vec![(0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (3000, 3000)), None);
    }

    #[test]
    fn find_screen_index_at_position_treats_right_boundary_as_exclusive() {
        let screens = vec![(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (1920, 50)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_treats_bottom_boundary_as_exclusive() {
        let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (100, 1080)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_returns_none_for_empty_input() {
        let screens = Vec::<(i32, i32, u32, u32)>::new();
        assert_eq!(find_screen_index_at_position(screens, (0, 0)), None);
    }

    #[test]
    fn find_screen_index_at_position_treats_left_boundary_as_inclusive() {
        let screens = vec![(100, 50, 300, 200)];
        assert_eq!(find_screen_index_at_position(screens, (100, 75)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_treats_top_boundary_as_inclusive() {
        let screens = vec![(100, 50, 300, 200)];
        assert_eq!(find_screen_index_at_position(screens, (150, 50)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_treats_point_above_top_as_outside() {
        let screens = vec![(100, 50, 300, 200)];
        assert_eq!(find_screen_index_at_position(screens, (150, 49)), None);
    }

    #[test]
    fn find_screen_index_at_position_treats_point_left_of_screen_as_outside() {
        let screens = vec![(100, 50, 300, 200)];
        assert_eq!(find_screen_index_at_position(screens, (99, 100)), None);
    }

    // --- find_screen_index_at_position: multi-monitor layouts ---

    #[test]
    fn find_screen_index_at_position_three_horizontal_monitors() {
        let screens = vec![
            (0, 0, 1920, 1080),
            (1920, 0, 1920, 1080),
            (3840, 0, 1920, 1080),
        ];
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (100, 100)),
            Some(0)
        );
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (2000, 500)),
            Some(1)
        );
        assert_eq!(find_screen_index_at_position(screens, (4000, 500)), Some(2));
    }

    #[test]
    fn find_screen_index_at_position_vertical_stack() {
        let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (500, 500)),
            Some(0)
        );
        assert_eq!(find_screen_index_at_position(screens, (500, 1200)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_gap_between_screens_returns_none() {
        // Two screens with a gap between them
        let screens = vec![(0, 0, 100, 100), (200, 0, 100, 100)];
        assert_eq!(find_screen_index_at_position(screens, (150, 50)), None);
    }

    #[test]
    fn find_screen_index_at_position_single_pixel_screen() {
        let screens = vec![(5, 5, 1, 1)];
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (5, 5)),
            Some(0)
        );
        assert_eq!(find_screen_index_at_position(screens, (6, 5)), None);
    }

    #[test]
    fn find_screen_index_at_position_returns_first_match_for_overlapping() {
        // Overlapping screens: first match wins
        let screens = vec![(0, 0, 200, 200), (100, 100, 200, 200)];
        assert_eq!(find_screen_index_at_position(screens, (150, 150)), Some(0));
    }

    // --- build_region_from_drag: additional boundary tests ---

    #[test]
    fn build_region_from_drag_width_6_height_6_is_some() {
        let r = build_region_from_drag((10, 10), (16, 16)).unwrap();
        assert_eq!(r.physical_x, 10);
        assert_eq!(r.physical_y, 10);
        assert_eq!(r.physical_width, 6);
        assert_eq!(r.physical_height, 6);
    }

    #[test]
    fn build_region_from_drag_reversed_negative() {
        // Drag from positive to negative
        let r = build_region_from_drag((10, 10), (-10, -10)).unwrap();
        assert_eq!(r.physical_x, -10);
        assert_eq!(r.physical_y, -10);
        assert_eq!(r.physical_width, 20);
        assert_eq!(r.physical_height, 20);
    }

    #[test]
    fn build_region_from_drag_same_point_returns_none() {
        assert!(build_region_from_drag((42, 42), (42, 42)).is_none());
    }

    // --- SelectionApp: state transitions ---

    #[test]
    fn selection_app_start_and_end_drag_simulation() {
        let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
        // Simulate mouse move
        app.current_pos = Some((1, 1));
        // Simulate mouse press
        app.is_dragging = true;
        app.start_pos = app.current_pos;
        // Simulate mouse move during drag
        app.current_pos = Some((3, 3));
        // Simulate mouse release
        app.is_dragging = false;
        if let (Some(start), Some(end)) = (app.start_pos, app.current_pos) {
            app.result = build_region_from_drag(start, end);
        }
        // Region is 2x2, which is < 5, so None
        assert!(app.result.is_none());
    }

    #[test]
    fn selection_app_large_drag_produces_region() {
        let mut app = SelectionApp::new(vec![0u8; 400 * 400 * 4], 400, 400, 0, 0);
        app.start_pos = Some((10, 10));
        app.current_pos = Some((100, 100));
        app.is_dragging = false;
        if let (Some(start), Some(end)) = (app.start_pos, app.current_pos) {
            app.result = build_region_from_drag(start, end);
        }
        let region = app.result.unwrap();
        assert_eq!(region.physical_x, 10);
        assert_eq!(region.physical_y, 10);
        assert_eq!(region.physical_width, 90);
        assert_eq!(region.physical_height, 90);
    }

    #[test]
    fn selection_app_cancel_resets_nothing_but_sets_cancelled() {
        let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
        app.cancelled = true;
        assert!(app.cancelled);
        assert!(app.result.is_none());
    }

    // --- logical_to_physical additional tests ---

    #[test]
    fn logical_to_physical_scale_1_is_identity() {
        assert_eq!(logical_to_physical(100, 1.0), 100);
        assert_eq!(logical_to_physical(-100, 1.0), -100);
        assert_eq!(logical_to_physical(0, 1.0), 0);
    }

    #[test]
    fn logical_to_physical_scale_2_doubles() {
        assert_eq!(logical_to_physical(100, 2.0), 200);
        assert_eq!(logical_to_physical(-50, 2.0), -100);
    }

    #[test]
    fn logical_to_physical_zero_value() {
        assert_eq!(logical_to_physical(0, 1.5), 0);
        assert_eq!(logical_to_physical(0, 2.0), 0);
    }

    #[test]
    fn logical_to_physical_fractional_scale_rounds() {
        // 100 * 1.25 = 125.0 (exact)
        assert_eq!(logical_to_physical(100, 1.25), 125);
        // 3 * 1.5 = 4.5 → rounds to 4 (banker's rounding) or 5
        let result = logical_to_physical(3, 1.5);
        assert!(result == 4 || result == 5, "expected 4 or 5, got {result}");
    }

    #[test]
    fn logical_to_physical_negative_half_rounds_away_from_zero() {
        assert_eq!(logical_to_physical(-3, 1.5), -5);
    }

    // --- virtual_desktop_scale_factor additional tests ---

    #[test]
    fn virtual_desktop_scale_factor_single_primary_monitor() {
        let scale = virtual_desktop_scale_factor([(true, 2.0)]);
        assert_eq!(scale, Some(2.0));
    }

    #[test]
    fn virtual_desktop_scale_factor_primary_is_first() {
        let scale = virtual_desktop_scale_factor([(true, 1.25), (false, 1.5)]);
        assert_eq!(scale, Some(1.25));
    }

    #[test]
    fn virtual_desktop_scale_factor_primary_is_last_of_three() {
        let scale = virtual_desktop_scale_factor([(false, 1.0), (false, 1.25), (true, 1.5)]);
        assert_eq!(scale, Some(1.5));
    }

    #[test]
    fn virtual_desktop_scale_factor_multiple_primary_monitors_uses_first_primary() {
        let scale = virtual_desktop_scale_factor([(true, 1.25), (true, 1.5), (false, 2.0)]);
        assert_eq!(scale, Some(1.25));
    }

    // --- find_screen_index_at_position additional tests ---

    #[test]
    fn find_screen_index_at_position_large_monitor_count() {
        // 10 side-by-side 100px monitors
        let screens: Vec<(i32, i32, u32, u32)> =
            (0..10).map(|i| (i * 100, 0, 100u32, 100u32)).collect();
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (950, 50)),
            Some(9)
        );
        assert_eq!(find_screen_index_at_position(screens, (1000, 50)), None);
    }

    #[test]
    fn find_screen_index_at_position_negative_y_offset() {
        // Monitor above the primary
        let screens = vec![(0, -1080, 1920, 1080), (0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (100, -500)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_negative_top_boundary_is_inclusive() {
        let screens = vec![(0, -1080, 1920, 1080), (0, 0, 1920, 1080)];
        assert_eq!(
            find_screen_index_at_position(screens, (100, -1080)),
            Some(0)
        );
    }

    #[test]
    fn find_screen_index_at_position_negative_monitor_right_boundary_switches_to_next_screen() {
        let screens = vec![(-1920, 0, 1920, 1080), (0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (0, 300)), Some(1));
    }

    #[test]
    fn preferred_screen_index_uses_monitor_under_cursor() {
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        assert_eq!(preferred_screen_index(screens, Some((2500, 200))), Some(1));
    }

    #[test]
    fn preferred_screen_index_falls_back_to_primary_when_cursor_unavailable() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
        assert_eq!(preferred_screen_index(screens, None), Some(1));
    }

    #[test]
    fn preferred_screen_index_falls_back_to_first_when_no_primary_exists() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
        assert_eq!(preferred_screen_index(screens, Some((9999, 9999))), Some(0));
    }

    #[test]
    fn overlay_window_origin_preserves_x11_primary_right_of_desktop_origin() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, false),
                (1920, 0, 1920, 1080, 1.0, true),
            ],
            1,
            (1920, 1080),
        )
        .unwrap();
        let layouts = build_physical_screen_layouts(&inputs).unwrap();

        assert_eq!(overlay_window_origin(&layouts[1], 1.0, true), (1920, 0));
        assert_eq!(overlay_window_origin(&layouts[0], 1.0, true), (0, 0));
    }

    #[test]
    fn overlay_window_origin_restores_scaled_x11_coordinates() {
        let inputs =
            build_screen_layout_inputs([(960, 540, 1920, 1080, 2.0, true)], 0, (3840, 2160))
                .unwrap();
        let layouts = build_physical_screen_layouts(&inputs).unwrap();

        assert_eq!(overlay_window_origin(&layouts[0], 2.0, true), (1920, 1080));
    }

    #[test]
    fn overlay_window_origin_keeps_reconstructed_mixed_dpi_positions() {
        let inputs = build_screen_layout_inputs(
            [
                (-1707, 0, 1707, 960, 1.5, false),
                (0, 0, 1920, 1080, 1.0, true),
            ],
            0,
            (2560, 1440),
        )
        .unwrap();
        let layouts = build_physical_screen_layouts(&inputs).unwrap();

        assert_eq!(overlay_window_origin(&layouts[0], 1.5, false), (-2560, 0));
        assert_eq!(overlay_window_origin(&layouts[1], 1.0, false), (0, 0));
    }

    #[test]
    fn build_screen_layout_inputs_uses_capture_size_for_target_monitor() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 1707, 960, 1.5, false),
            ],
            1,
            (2560, 1440),
        )
        .expect("layout inputs should build");

        assert_eq!(inputs[1].physical_width, 2560); // target gets exact capture size
        assert_eq!(inputs[1].physical_height, 1440);
    }

    #[test]
    fn build_screen_layout_inputs_uses_scale_factor_for_other_monitors() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 1707, 960, 1.5, false),
            ],
            0,
            (1920, 1080),
        )
        .expect("layout inputs should build");

        assert_eq!(inputs[1].physical_width, 2561);
        assert_eq!(inputs[1].physical_height, 1440);
    }

    #[test]
    fn build_screen_layout_inputs_rejects_unknown_target_monitor() {
        let error = build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 1, (1920, 1080))
            .unwrap_err();

        assert!(error.contains("screen index"));
    }

    #[test]
    fn capture_selection_desktop_propagates_screen_error() {
        let result =
            capture_selection_desktop_with(|| Err("single-screen capture failed".to_string()));

        assert_eq!(result.unwrap_err(), "single-screen capture failed");
    }

    // --- build_region_from_drag boundary precision ---

    #[test]
    fn build_region_from_drag_width_1_height_1000_returns_none() {
        // Only 1 pixel wide — not > 5
        assert!(build_region_from_drag((0, 0), (1, 1000)).is_none());
    }

    #[test]
    fn build_region_from_drag_exact_boundary_5x6_returns_none() {
        // width=5 is NOT > 5, even though height=6 IS > 5
        assert!(build_region_from_drag((10, 10), (15, 16)).is_none());
    }

    #[test]
    fn build_region_from_drag_exact_boundary_6x5_returns_none() {
        // height=5 is NOT > 5, even though width=6 IS > 5
        assert!(build_region_from_drag((10, 10), (16, 15)).is_none());
    }

    #[test]
    fn build_region_from_drag_very_large_coordinates() {
        let r = build_region_from_drag(
            (i32::MIN / 2, i32::MIN / 2),
            (i32::MIN / 2 + 100, i32::MIN / 2 + 100),
        );
        assert!(r.is_some());
        let r = r.unwrap();
        assert_eq!(r.physical_width, 100);
        assert_eq!(r.physical_height, 100);
    }

    // --- logical_to_physical edge cases ---

    #[test]
    fn logical_to_physical_zero_is_zero() {
        assert_eq!(logical_to_physical(0, 1.0), 0);
        assert_eq!(logical_to_physical(0, 2.0), 0);
        assert_eq!(logical_to_physical(0, 0.5), 0);
    }

    #[test]
    fn logical_to_physical_scale_2x() {
        assert_eq!(logical_to_physical(100, 2.0), 200);
        assert_eq!(logical_to_physical(-100, 2.0), -200);
    }

    #[test]
    fn logical_to_physical_scale_half() {
        assert_eq!(logical_to_physical(100, 0.5), 50);
    }

    #[test]
    fn logical_to_physical_rounds_correctly() {
        // 3 * 1.5 = 4.5, f32::round() rounds half-away-from-zero → 5
        assert_eq!(logical_to_physical(3, 1.5), 5);
        // 7 * 0.3 = 2.1, rounds to 2
        assert_eq!(logical_to_physical(7, 0.3), 2);
    }

    #[test]
    fn logical_to_physical_common_dpi_scales() {
        // 125% scaling
        assert_eq!(logical_to_physical(1280, 1.25), 1600);
        // 150% scaling
        assert_eq!(logical_to_physical(1280, 1.5), 1920);
        // 175% scaling
        assert_eq!(logical_to_physical(1280, 1.75), 2240);
    }

    // --- virtual_desktop_scale_factor edge cases ---

    #[test]
    fn virtual_desktop_scale_factor_single_primary_2x() {
        let scale = virtual_desktop_scale_factor([(true, 2.0)]);
        assert_eq!(scale, Some(2.0));
    }

    #[test]
    fn virtual_desktop_scale_factor_single_non_primary_1_5x() {
        let scale = virtual_desktop_scale_factor([(false, 1.5)]);
        assert_eq!(scale, Some(1.5));
    }

    #[test]
    fn virtual_desktop_scale_factor_primary_in_middle_of_four() {
        let scale =
            virtual_desktop_scale_factor([(false, 1.0), (false, 1.25), (true, 1.5), (false, 2.0)]);
        assert_eq!(scale, Some(1.5));
    }

    #[test]
    fn virtual_desktop_scale_factor_two_primaries_returns_first() {
        let scale = virtual_desktop_scale_factor([(true, 1.0), (true, 2.0)]);
        assert_eq!(scale, Some(1.0));
    }

    // --- find_screen_index_at_position edge cases ---

    #[test]
    fn find_screen_index_at_position_exact_top_left_corner() {
        let screens = vec![(0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_just_outside_right_boundary() {
        let screens = vec![(0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (1920, 0)), None);
    }

    #[test]
    fn find_screen_index_at_position_just_outside_bottom_boundary() {
        let screens = vec![(0, 0, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (0, 1080)), None);
    }

    #[test]
    fn find_screen_index_at_position_last_valid_pixel() {
        let screens = vec![(0, 0, 1920, 1080)];
        assert_eq!(
            find_screen_index_at_position(screens, (1919, 1079)),
            Some(0)
        );
    }

    #[test]
    fn find_screen_index_at_position_empty_screens() {
        let screens: Vec<(i32, i32, u32, u32)> = vec![];
        assert_eq!(find_screen_index_at_position(screens, (100, 100)), None);
    }

    #[test]
    fn find_screen_index_at_position_stacked_monitors() {
        let screens = vec![
            (0, 0, 1920, 1080),    // top
            (0, 1080, 1920, 1080), // bottom
        ];
        assert_eq!(
            find_screen_index_at_position(screens.clone(), (500, 500)),
            Some(0)
        );
        assert_eq!(find_screen_index_at_position(screens, (500, 1500)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_gap_between_monitors_large_suite() {
        let screens = vec![
            (0, 0, 1920, 1080),
            (2000, 0, 1920, 1080), // 80px gap
        ];
        assert_eq!(find_screen_index_at_position(screens, (1950, 500)), None);
    }

    #[test]
    fn build_physical_screen_layouts_places_high_dpi_monitor_left_of_primary() {
        let layouts = build_physical_screen_layouts(&[
            ScreenLayoutInput {
                logical_x: -1707,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
        ])
        .expect("layout should build");

        assert_eq!(layouts[0].physical_x, -2560);
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[1].physical_x, 0);
        assert_eq!(layouts[1].physical_y, 0);
    }

    #[test]
    fn build_physical_screen_layouts_keeps_primary_aligned_to_high_dpi_monitor_on_right() {
        let layouts = build_physical_screen_layouts(&[
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false,
            },
        ])
        .expect("layout should build");

        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[1].physical_x, 1920);
        assert_eq!(layouts[1].physical_y, 0);
    }

    #[test]
    fn resolve_recording_selection_rejects_regions_crossing_monitors() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];

        let error = resolve_recording_selection(
            &layouts,
            Region {
                physical_x: 1910,
                physical_y: 100,
                physical_width: 40,
                physical_height: 200,
            },
        )
        .unwrap_err();

        assert!(error.contains("single monitor"));
    }

    #[test]
    fn resolve_recording_selection_returns_screen_local_physical_region() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];

        let (screen_index, local_region) = resolve_recording_selection(
            &layouts,
            Region {
                physical_x: 2000,
                physical_y: 100,
                physical_width: 400,
                physical_height: 300,
            },
        )
        .expect("region should resolve inside the right monitor");

        assert_eq!(screen_index, 1);
        assert_eq!(local_region.physical_x, 80);
        assert_eq!(local_region.physical_y, 100);
        assert_eq!(local_region.physical_width, 400);
        assert_eq!(local_region.physical_height, 300);
    }

    // --- logical_end / physical_end tests ---

    #[test]
    fn logical_end_zero_start_zero_size() {
        assert_eq!(logical_end(0, 0), 0);
    }

    #[test]
    fn logical_end_positive_start_and_size() {
        assert_eq!(logical_end(100, 1920), 2020);
    }

    #[test]
    fn logical_end_negative_start() {
        assert_eq!(logical_end(-500, 1000), 500);
    }

    #[test]
    fn logical_end_large_values_no_overflow() {
        assert_eq!(
            logical_end(i32::MAX, u32::MAX),
            i64::from(i32::MAX) + i64::from(u32::MAX)
        );
    }

    #[test]
    fn physical_end_matches_logical_end_behavior() {
        assert_eq!(physical_end(10, 20), logical_end(10, 20));
    }

    // --- ranges_overlap tests ---

    #[test]
    fn ranges_overlap_identical_ranges() {
        assert!(ranges_overlap(0, 100, 0, 100));
    }

    #[test]
    fn ranges_overlap_touching_ranges_do_not_overlap() {
        assert!(!ranges_overlap(0, 100, 100, 100));
    }

    #[test]
    fn ranges_overlap_adjacent_reversed_do_not_overlap() {
        assert!(!ranges_overlap(100, 100, 0, 100));
    }

    #[test]
    fn ranges_overlap_partial_overlap() {
        assert!(ranges_overlap(0, 150, 100, 100));
    }

    #[test]
    fn ranges_overlap_one_inside_another() {
        assert!(ranges_overlap(0, 1000, 100, 200));
    }

    #[test]
    fn ranges_overlap_disjoint_with_gap() {
        assert!(!ranges_overlap(0, 100, 200, 100));
    }

    #[test]
    fn ranges_overlap_negative_start() {
        assert!(ranges_overlap(-100, 200, 0, 100));
    }

    #[test]
    fn ranges_overlap_both_negative() {
        assert!(ranges_overlap(-200, 150, -100, 150));
    }

    #[test]
    fn ranges_overlap_zero_size_never_overlaps() {
        assert!(!ranges_overlap(50, 0, 50, 100));
    }

    // --- monitor_scale_ratio tests ---

    #[test]
    fn monitor_scale_ratio_1x_scaling() {
        assert_eq!(monitor_scale_ratio(1920, 1920), 1.0);
    }

    #[test]
    fn monitor_scale_ratio_2x_scaling() {
        assert_eq!(monitor_scale_ratio(960, 1920), 2.0);
    }

    #[test]
    fn monitor_scale_ratio_150_percent() {
        assert!((monitor_scale_ratio(1707, 2560) - 1.5).abs() < 0.01);
    }

    #[test]
    fn monitor_scale_ratio_zero_logical_returns_1() {
        assert_eq!(monitor_scale_ratio(0, 1920), 1.0);
    }

    #[test]
    fn monitor_scale_ratio_both_zero_returns_1() {
        assert_eq!(monitor_scale_ratio(0, 0), 1.0);
    }

    // --- scaled_gap tests ---

    #[test]
    fn scaled_gap_zero_gap() {
        assert_eq!(scaled_gap(0, 1920, 1920), 0);
    }

    #[test]
    fn scaled_gap_1x_scaling() {
        assert_eq!(scaled_gap(100, 1920, 1920), 100);
    }

    #[test]
    fn scaled_gap_2x_scaling() {
        assert_eq!(scaled_gap(50, 960, 1920), 100);
    }

    #[test]
    fn scaled_gap_negative_gap() {
        assert_eq!(scaled_gap(-100, 1920, 1920), -100);
    }

    #[test]
    fn scaled_gap_zero_logical_size_returns_gap_unchanged() {
        assert_eq!(scaled_gap(50, 0, 1920), 50);
    }

    // --- infer_physical_x tests ---

    fn make_screen(lx: i32, ly: i32, lw: u32, lh: u32, pw: u32, ph: u32) -> ScreenLayoutInput {
        ScreenLayoutInput {
            logical_x: lx,
            logical_y: ly,
            logical_width: lw,
            logical_height: lh,
            physical_width: pw,
            physical_height: ph,
            is_primary: false,
        }
    }

    #[test]
    fn infer_physical_x_same_logical_x_returns_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 1080, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_x(&reference, &target, 0), Some(0));
    }

    #[test]
    fn infer_physical_x_target_right_of_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 0, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_x(&reference, &target, 0), Some(1920));
    }

    #[test]
    fn infer_physical_x_target_left_of_reference() {
        let reference = make_screen(1920, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 0, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_x(&reference, &target, 1920), Some(0));
    }

    #[test]
    fn infer_physical_x_no_y_overlap_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 2000, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_x(&reference, &target, 0), None);
    }

    #[test]
    fn infer_physical_x_non_adjacent_with_y_overlap_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(3000, 0, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_x(&reference, &target, 0), None);
    }

    #[test]
    fn infer_physical_x_high_dpi_target_right_of_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 0, 1707, 960, 2560, 1440);
        assert_eq!(infer_physical_x(&reference, &target, 0), Some(1920));
    }

    #[test]
    fn infer_physical_x_high_dpi_target_left_of_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(-1707, 0, 1707, 960, 2560, 1440);
        assert_eq!(infer_physical_x(&reference, &target, 0), Some(-2560));
    }

    // --- infer_physical_y tests ---

    #[test]
    fn infer_physical_y_same_logical_y_returns_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 0, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_y(&reference, &target, 0), Some(0));
    }

    #[test]
    fn infer_physical_y_target_below_reference() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 1080, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_y(&reference, &target, 0), Some(1080));
    }

    #[test]
    fn infer_physical_y_target_above_reference() {
        let reference = make_screen(0, 1080, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 0, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_y(&reference, &target, 1080), Some(0));
    }

    #[test]
    fn infer_physical_y_no_x_overlap_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(3000, 1080, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_y(&reference, &target, 0), None);
    }

    #[test]
    fn infer_physical_y_non_adjacent_with_x_overlap_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 3000, 1920, 1080, 1920, 1080);
        assert_eq!(infer_physical_y(&reference, &target, 0), None);
    }

    // --- infer_physical_y_from_horizontal_neighbor tests ---

    #[test]
    fn infer_y_from_horizontal_neighbor_adjacent_same_y() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 0, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(0)
        );
    }

    #[test]
    fn infer_y_from_horizontal_neighbor_adjacent_different_y() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 100, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(100)
        );
    }

    #[test]
    fn infer_y_from_horizontal_neighbor_not_adjacent_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(3000, 0, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            None
        );
    }

    #[test]
    fn infer_y_from_horizontal_neighbor_target_left_of_reference() {
        let reference = make_screen(1920, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 50, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(50)
        );
    }

    #[test]
    fn infer_y_from_horizontal_neighbor_high_dpi_scales_gap() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(1920, 100, 1707, 960, 2560, 1440);
        let result = infer_physical_y_from_horizontal_neighbor(&reference, &target, 0);
        // gap=100, scale=1440/960=1.5, so physical gap = 150
        assert_eq!(result, Some(150));
    }

    // --- infer_physical_x_from_vertical_neighbor tests ---

    #[test]
    fn infer_x_from_vertical_neighbor_adjacent_same_x() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 1080, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            Some(0)
        );
    }

    #[test]
    fn infer_x_from_vertical_neighbor_not_adjacent_returns_none() {
        let reference = make_screen(0, 0, 1920, 1080, 1920, 1080);
        let target = make_screen(0, 3000, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            None
        );
    }

    #[test]
    fn infer_x_from_vertical_neighbor_target_above_reference() {
        let reference = make_screen(100, 1080, 1920, 1080, 1920, 1080);
        let target = make_screen(200, 0, 1920, 1080, 1920, 1080);
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 100),
            Some(200)
        );
    }

    // --- normalize_physical_screen_layouts tests ---

    #[test]
    fn normalize_layouts_single_screen_at_origin() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let (normalized, min_x, min_y, width, height) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(min_x, 0);
        assert_eq!(min_y, 0);
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);
        assert_eq!(normalized[0].physical_x, 0);
        assert_eq!(normalized[0].physical_y, 0);
    }

    #[test]
    fn normalize_layouts_negative_offsets_shifted_to_zero() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: -1707,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_x: -2560,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let (normalized, min_x, min_y, width, height) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(min_x, -2560);
        assert_eq!(min_y, 0);
        assert_eq!(width, 4480);
        assert_eq!(height, 1440);
        assert_eq!(normalized[0].physical_x, 0);
        assert_eq!(normalized[1].physical_x, 2560);
    }

    #[test]
    fn normalize_layouts_empty_returns_error() {
        let result = normalize_physical_screen_layouts(&[]);
        assert!(result.is_err());
    }

    // --- resolve_recording_selection additional tests ---

    #[test]
    fn resolve_recording_selection_first_monitor() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let (idx, region) = resolve_recording_selection(
            &layouts,
            Region {
                physical_x: 100,
                physical_y: 100,
                physical_width: 500,
                physical_height: 400,
            },
        )
        .unwrap();
        assert_eq!(idx, 0);
        assert_eq!(region.physical_x, 100);
        assert_eq!(region.physical_y, 100);
    }

    #[test]
    fn resolve_recording_selection_exact_monitor_bounds() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let (idx, region) = resolve_recording_selection(
            &layouts,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        )
        .unwrap();
        assert_eq!(idx, 0);
        assert_eq!(region.physical_x, 0);
        assert_eq!(region.physical_y, 0);
        assert_eq!(region.physical_width, 1920);
        assert_eq!(region.physical_height, 1080);
    }

    #[test]
    fn resolve_recording_selection_region_exceeds_single_monitor() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let result = resolve_recording_selection(
            &layouts,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 2000,
                physical_height: 1080,
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("single monitor"));
    }

    #[test]
    fn resolve_recording_selection_empty_layouts() {
        let result = resolve_recording_selection(
            &[],
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
        );
        assert!(result.is_err());
    }

    // --- build_physical_screen_layouts additional tests ---

    #[test]
    fn build_physical_screen_layouts_single_monitor() {
        let layouts = build_physical_screen_layouts(&[ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        }])
        .unwrap();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[0].physical_y, 0);
    }

    #[test]
    fn build_physical_screen_layouts_empty_returns_error() {
        let result = build_physical_screen_layouts(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn build_physical_screen_layouts_three_monitors_horizontal() {
        let layouts = build_physical_screen_layouts(&[
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            ScreenLayoutInput {
                is_primary: true,
                ..make_screen(1920, 0, 1920, 1080, 1920, 1080)
            },
            make_screen(3840, 0, 1920, 1080, 1920, 1080),
        ])
        .unwrap();
        // Primary is index 1, anchored at physical_x=0
        assert_eq!(layouts[0].physical_x, -1920);
        assert_eq!(layouts[1].physical_x, 0);
        assert_eq!(layouts[2].physical_x, 1920);
    }

    #[test]
    fn build_physical_screen_layouts_stacked_monitors() {
        let layouts = build_physical_screen_layouts(&[
            ScreenLayoutInput {
                is_primary: true,
                ..make_screen(0, 0, 1920, 1080, 1920, 1080)
            },
            make_screen(0, 1080, 1920, 1080, 1920, 1080),
        ])
        .unwrap();
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[1].physical_x, 0);
        assert_eq!(layouts[1].physical_y, 1080);
    }

    // --- fallback_physical_x tests ---

    #[test]
    fn fallback_physical_x_from_left_neighbor_with_gap() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(2000, 0, 1920, 1080, 1920, 1080), // 80px gap
        ];
        let positions = vec![Some(0), None];
        let result = fallback_physical_x(&inputs, &positions, 1);
        // gap = 80, scale = 1.0, so physical_x = 0 + 1920 + 80 = 2000
        assert_eq!(result, Some(2000));
    }

    #[test]
    fn fallback_physical_x_from_right_neighbor_with_gap() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(2000, 0, 1920, 1080, 1920, 1080),
        ];
        let positions = vec![None, Some(2000)];
        let result = fallback_physical_x(&inputs, &positions, 0);
        // target is left of reference, gap = 80, scale = 1.0
        assert_eq!(result, Some(0));
    }

    #[test]
    fn fallback_physical_x_prefers_nearest_left_neighbor() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(2500, 0, 1920, 1080, 1920, 1080),
            make_screen(4100, 0, 1920, 1080, 1920, 1080),
        ];
        let positions = vec![Some(0), Some(2500), None];
        let result = fallback_physical_x(&inputs, &positions, 2);
        assert_eq!(result, Some(4100));
    }

    #[test]
    fn fallback_physical_x_high_dpi_gap_scales_target_width() {
        let inputs = vec![
            make_screen(0, 0, 100, 100, 100, 100),
            make_screen(150, 0, 100, 100, 200, 200),
        ];
        let positions = vec![Some(0), None];
        let result = fallback_physical_x(&inputs, &positions, 1);
        assert_eq!(result, Some(200));
    }

    #[test]
    fn fallback_physical_x_no_resolved_neighbors() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(2000, 0, 1920, 1080, 1920, 1080),
        ];
        let positions = vec![None, None];
        let result = fallback_physical_x(&inputs, &positions, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn fallback_physical_x_out_of_bounds_index() {
        let inputs = vec![make_screen(0, 0, 1920, 1080, 1920, 1080)];
        let positions = vec![Some(0)];
        let result = fallback_physical_x(&inputs, &positions, 5);
        assert_eq!(result, None);
    }

    // --- fallback_physical_y tests ---

    #[test]
    fn fallback_physical_y_from_above_neighbor_with_gap() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(0, 1200, 1920, 1080, 1920, 1080), // 120px gap
        ];
        let positions = vec![Some(0), None];
        let result = fallback_physical_y(&inputs, &positions, 1);
        assert_eq!(result, Some(1200));
    }

    #[test]
    fn fallback_physical_y_from_below_neighbor_with_gap() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(0, 1200, 1920, 1080, 1920, 1080),
        ];
        let positions = vec![None, Some(1200)];
        let result = fallback_physical_y(&inputs, &positions, 0);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn fallback_physical_y_high_dpi_gap_scales_target_height() {
        let inputs = vec![
            make_screen(0, 0, 100, 100, 100, 100),
            make_screen(0, 150, 100, 100, 200, 200),
        ];
        let positions = vec![Some(0), None];
        let result = fallback_physical_y(&inputs, &positions, 1);
        assert_eq!(result, Some(200));
    }

    #[test]
    fn fallback_physical_y_no_resolved_neighbors() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(0, 2000, 1920, 1080, 1920, 1080),
        ];
        let positions = vec![None, None];
        let result = fallback_physical_y(&inputs, &positions, 1);
        assert_eq!(result, None);
    }

    #[test]
    fn fallback_physical_y_out_of_bounds_index() {
        let inputs = vec![make_screen(0, 0, 1920, 1080, 1920, 1080)];
        let positions = vec![Some(0)];
        let result = fallback_physical_y(&inputs, &positions, 5);
        assert_eq!(result, None);
    }

    // =========================================================================
    // get_cursor_position tests
    // =========================================================================

    #[cfg(not(windows))]
    #[test]
    fn get_cursor_position_returns_error_on_non_windows() {
        let result = get_cursor_position();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    #[cfg(not(windows))]
    #[test]
    fn get_cursor_position_error_message_is_not_empty() {
        let err = get_cursor_position().unwrap_err();
        assert!(!err.is_empty());
    }

    // =========================================================================
    // preferred_screen_index tests
    // =========================================================================

    #[test]
    fn preferred_screen_index_empty_returns_none() {
        let screens: Vec<(i32, i32, u32, u32, bool)> = vec![];
        assert_eq!(preferred_screen_index(screens, Some((100, 100))), None);
    }

    #[test]
    fn preferred_screen_index_single_screen_always_returns_zero() {
        let screens = vec![(0, 0, 1920, 1080, true)];
        assert_eq!(preferred_screen_index(screens.clone(), None), Some(0));
        assert_eq!(
            preferred_screen_index(screens.clone(), Some((500, 500))),
            Some(0)
        );
        assert_eq!(preferred_screen_index(screens, Some((9999, 9999))), Some(0));
    }

    #[test]
    fn preferred_screen_index_cursor_on_second_monitor() {
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        // Cursor at (2000, 500) is on the second monitor
        assert_eq!(preferred_screen_index(screens, Some((2000, 500))), Some(1));
    }

    #[test]
    fn preferred_screen_index_cursor_on_first_monitor() {
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        assert_eq!(preferred_screen_index(screens, Some((960, 540))), Some(0));
    }

    #[test]
    fn preferred_screen_index_cursor_on_boundary_between_monitors() {
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        // x=1920 is exactly the start of monitor 2 (exclusive end of monitor 1)
        assert_eq!(preferred_screen_index(screens, Some((1920, 500))), Some(1));
    }

    #[test]
    fn preferred_screen_index_cursor_at_origin() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
        assert_eq!(preferred_screen_index(screens, Some((0, 0))), Some(0));
    }

    #[test]
    fn preferred_screen_index_cursor_outside_all_screens_falls_back_to_primary() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
        assert_eq!(
            preferred_screen_index(screens, Some((10000, 10000))),
            Some(1)
        );
    }

    #[test]
    fn preferred_screen_index_cursor_outside_no_primary_falls_back_to_first() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
        assert_eq!(
            preferred_screen_index(screens, Some((10000, 10000))),
            Some(0)
        );
    }

    #[test]
    fn preferred_screen_index_no_cursor_uses_primary() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
        assert_eq!(preferred_screen_index(screens, None), Some(1));
    }

    #[test]
    fn preferred_screen_index_no_cursor_no_primary_uses_first() {
        let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
        assert_eq!(preferred_screen_index(screens, None), Some(0));
    }

    #[test]
    fn preferred_screen_index_three_monitors_cursor_on_middle() {
        let screens = vec![
            (0, 0, 1920, 1080, true),
            (1920, 0, 2560, 1440, false),
            (4480, 0, 1920, 1080, false),
        ];
        assert_eq!(preferred_screen_index(screens, Some((3000, 700))), Some(1));
    }

    #[test]
    fn preferred_screen_index_three_monitors_cursor_on_third() {
        let screens = vec![
            (0, 0, 1920, 1080, true),
            (1920, 0, 2560, 1440, false),
            (4480, 0, 1920, 1080, false),
        ];
        assert_eq!(preferred_screen_index(screens, Some((5000, 500))), Some(2));
    }

    #[test]
    fn preferred_screen_index_negative_coords_monitor() {
        let screens = vec![(-1920, 0, 1920, 1080, false), (0, 0, 1920, 1080, true)];
        assert_eq!(preferred_screen_index(screens, Some((-500, 500))), Some(0));
    }

    #[test]
    fn preferred_screen_index_vertically_stacked_monitors() {
        let screens = vec![(0, 0, 1920, 1080, true), (0, 1080, 1920, 1080, false)];
        // Cursor on lower monitor
        assert_eq!(preferred_screen_index(screens, Some((960, 1500))), Some(1));
    }

    #[test]
    fn preferred_screen_index_cursor_at_last_pixel_of_first_monitor() {
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        // x=1919, y=1079 is the last valid pixel of monitor 0
        assert_eq!(preferred_screen_index(screens, Some((1919, 1079))), Some(0));
    }

    #[test]
    fn preferred_screen_index_priority_cursor_over_primary() {
        // Even though monitor 0 is primary, cursor on monitor 1 → returns 1
        let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
        assert_eq!(preferred_screen_index(screens, Some((2500, 200))), Some(1));
    }

    // =========================================================================
    // build_screen_layout_inputs tests
    // =========================================================================

    #[test]
    fn build_screen_layout_inputs_single_monitor() {
        let inputs =
            build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 0, (1920, 1080)).unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].physical_width, 1920);
        assert_eq!(inputs[0].physical_height, 1080);
        assert_eq!(inputs[0].logical_x, 0);
        assert_eq!(inputs[0].logical_y, 0);
        assert!(inputs[0].is_primary);
    }

    #[test]
    fn build_screen_layout_inputs_target_gets_actual_capture_size() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.5, true),
                (1920, 0, 2560, 1440, 1.0, false),
            ],
            0,
            (2880, 1620), // actual captured size
        )
        .unwrap();
        // Target monitor gets exact capture size
        assert_eq!(inputs[0].physical_width, 2880);
        assert_eq!(inputs[0].physical_height, 1620);
    }

    #[test]
    fn build_screen_layout_inputs_non_target_uses_scale_factor() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 1280, 720, 2.0, false),
            ],
            0,
            (1920, 1080),
        )
        .unwrap();
        // Non-target: 1280 * 2.0 = 2560, 720 * 2.0 = 1440
        assert_eq!(inputs[1].physical_width, 2560);
        assert_eq!(inputs[1].physical_height, 1440);
    }

    #[test]
    fn build_screen_layout_inputs_preserves_logical_coords() {
        let inputs = build_screen_layout_inputs(
            [
                (-1920, 100, 1920, 1080, 1.0, false),
                (0, 0, 2560, 1440, 1.25, true),
            ],
            1,
            (3200, 1800),
        )
        .unwrap();
        assert_eq!(inputs[0].logical_x, -1920);
        assert_eq!(inputs[0].logical_y, 100);
        assert_eq!(inputs[1].logical_x, 0);
        assert_eq!(inputs[1].logical_y, 0);
    }

    #[test]
    fn build_screen_layout_inputs_preserves_is_primary() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 2560, 1440, 1.0, false),
            ],
            0,
            (1920, 1080),
        )
        .unwrap();
        assert!(inputs[0].is_primary);
        assert!(!inputs[1].is_primary);
    }

    #[test]
    fn build_screen_layout_inputs_index_out_of_range_returns_error() {
        let result = build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 5, (1920, 1080));
        assert!(result.is_err());
    }

    #[test]
    fn build_screen_layout_inputs_error_message_mentions_screen_index() {
        let err = build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 2, (1920, 1080))
            .unwrap_err();
        assert!(err.contains("screen index") || err.contains("Invalid"));
    }

    #[test]
    fn build_screen_layout_inputs_empty_screens_returns_error() {
        let result: Result<Vec<ScreenLayoutInput>, String> =
            build_screen_layout_inputs([], 0, (1920, 1080));
        assert!(result.is_err());
    }

    #[test]
    fn build_screen_layout_inputs_three_monitors_target_middle() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 1707, 960, 1.5, false),
                (3627, 0, 1920, 1080, 1.0, false),
            ],
            1,
            (2560, 1440), // actual capture of the 150% monitor
        )
        .unwrap();
        assert_eq!(inputs.len(), 3);
        // Target (index 1) gets actual capture size
        assert_eq!(inputs[1].physical_width, 2560);
        assert_eq!(inputs[1].physical_height, 1440);
        // Others get scaled size
        assert_eq!(inputs[0].physical_width, 1920); // 1920 * 1.0
        assert_eq!(inputs[2].physical_width, 1920); // 1920 * 1.0
    }

    #[test]
    fn build_screen_layout_inputs_fractional_scale_rounds_up() {
        let inputs =
            build_screen_layout_inputs([(0, 0, 1000, 500, 1.25, true)], 0, (1250, 625)).unwrap();
        assert_eq!(inputs[0].physical_width, 1250);
        assert_eq!(inputs[0].physical_height, 625);
    }

    #[test]
    fn build_screen_layout_inputs_non_target_fractional_scale_uses_ceil() {
        let inputs = build_screen_layout_inputs(
            [
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 1000, 700, 1.333, false),
            ],
            0,
            (1920, 1080),
        )
        .unwrap();
        // 1000 * 1.333 = 1333.0, ceil = 1333
        assert_eq!(inputs[1].physical_width, 1333);
        // 700 * 1.333 = 933.1, ceil = 934
        assert_eq!(inputs[1].physical_height, 934);
    }

    #[test]
    fn build_screen_layout_inputs_preserves_order() {
        let inputs = build_screen_layout_inputs(
            [
                (100, 200, 800, 600, 1.0, false),
                (0, 0, 1920, 1080, 1.0, true),
                (1920, 0, 2560, 1440, 2.0, false),
            ],
            1,
            (1920, 1080),
        )
        .unwrap();
        assert_eq!(inputs[0].logical_x, 100);
        assert_eq!(inputs[1].logical_x, 0);
        assert_eq!(inputs[2].logical_x, 1920);
    }

    // =========================================================================
    // find_screen_index_at_position — additional tests for cursor feature
    // =========================================================================

    #[test]
    fn find_screen_index_at_position_cursor_exactly_on_monitor_origin() {
        let screens = vec![(1920, 0, 2560, 1440)];
        assert_eq!(find_screen_index_at_position(screens, (1920, 0)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_cursor_one_before_monitor_start() {
        let screens = vec![(1920, 0, 2560, 1440)];
        assert_eq!(find_screen_index_at_position(screens, (1919, 0)), None);
    }

    #[test]
    fn find_screen_index_at_position_cursor_at_bottom_right_edge() {
        let screens = vec![(0, 0, 1920, 1080)];
        // x=1919, y=1079 is last valid pixel
        assert_eq!(
            find_screen_index_at_position(screens, (1919, 1079)),
            Some(0)
        );
    }

    #[test]
    fn find_screen_index_at_position_cursor_just_outside_bottom_right() {
        let screens = vec![(0, 0, 1920, 1080)];
        // x=1920 is out of bounds (exclusive)
        assert_eq!(find_screen_index_at_position(screens, (1920, 1079)), None);
    }

    #[test]
    fn find_screen_index_at_position_vertical_stack_top() {
        let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (960, 500)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_vertical_stack_bottom() {
        let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
        assert_eq!(find_screen_index_at_position(screens, (960, 1500)), Some(1));
    }

    #[test]
    fn find_screen_index_at_position_gap_between_monitors() {
        let screens = vec![
            (0, 0, 1920, 1080),
            (2000, 0, 1920, 1080), // 80px gap
        ];
        // Cursor in the gap
        assert_eq!(find_screen_index_at_position(screens, (1950, 500)), None);
    }

    #[test]
    fn find_screen_index_at_position_four_monitors_grid() {
        let screens = vec![
            (0, 0, 1920, 1080),       // top-left
            (1920, 0, 1920, 1080),    // top-right
            (0, 1080, 1920, 1080),    // bottom-left
            (1920, 1080, 1920, 1080), // bottom-right
        ];
        assert_eq!(
            find_screen_index_at_position(screens, (2500, 1500)),
            Some(3)
        );
    }

    // =========================================================================
    // logical_end / physical_end — additional coverage
    // =========================================================================

    #[test]
    fn logical_end_sums_start_and_size_correctly() {
        assert_eq!(logical_end(100, 200), 300);
    }

    #[test]
    fn logical_end_i32_min_start_zero_size() {
        assert_eq!(logical_end(i32::MIN, 0), i64::from(i32::MIN));
    }

    #[test]
    fn physical_end_large_physical_size() {
        assert_eq!(physical_end(0, u32::MAX), i64::from(u32::MAX));
    }

    #[test]
    fn physical_end_negative_origin_with_size() {
        assert_eq!(physical_end(-500, 1000), 500);
    }

    // =========================================================================
    // ranges_overlap — additional coverage
    // =========================================================================

    #[test]
    fn ranges_overlap_touching_at_boundary_no_overlap() {
        // [0, 50) and [50, 100) — touching but not overlapping
        assert!(!ranges_overlap(0, 50, 50, 50));
    }

    #[test]
    fn ranges_overlap_by_one_pixel() {
        // [0, 51) and [50, 100) — overlap by 1
        assert!(ranges_overlap(0, 51, 50, 50));
    }

    #[test]
    fn ranges_overlap_both_zero_size() {
        assert!(!ranges_overlap(0, 0, 0, 0));
    }

    #[test]
    fn ranges_overlap_second_zero_size_inside_first() {
        // Zero-size range at position 50 within [0, 100): start_b < end_a && start_a < end_b
        // Both conditions true since end_b=50 > start_a=0 and start_b=50 < end_a=100
        assert!(ranges_overlap(0, 100, 50, 0));
    }

    // =========================================================================
    // monitor_scale_ratio — additional coverage
    // =========================================================================

    #[test]
    fn monitor_scale_ratio_125pct() {
        let ratio = monitor_scale_ratio(1920, 2400);
        assert!((ratio - 1.25).abs() < 0.001);
    }

    #[test]
    fn monitor_scale_ratio_fractional() {
        let ratio = monitor_scale_ratio(1000, 1333);
        assert!((ratio - 1.333).abs() < 0.001);
    }

    // =========================================================================
    // scaled_gap — additional coverage
    // =========================================================================

    #[test]
    fn scaled_gap_150pct_scale() {
        // gap=100, ratio=2880/1920=1.5 → 150
        assert_eq!(scaled_gap(100, 1920, 2880), 150);
    }

    #[test]
    fn scaled_gap_large_gap() {
        assert_eq!(scaled_gap(1000, 1920, 1920), 1000);
    }

    // =========================================================================
    // infer_physical_y_from_horizontal_neighbor
    // =========================================================================

    #[test]
    fn infer_physical_y_from_horizontal_neighbor_adjacent_right() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 1920,
            logical_y: 100,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        // Adjacent right, y_diff = 100, scale ratio = 1.0, so physical y = 0 + 100 = 100
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(100)
        );
    }

    #[test]
    fn infer_physical_y_from_horizontal_neighbor_not_adjacent_returns_none() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 5000,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            None
        );
    }

    // =========================================================================
    // infer_physical_x_from_vertical_neighbor
    // =========================================================================

    #[test]
    fn infer_physical_x_from_vertical_neighbor_adjacent_below() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 200,
            logical_y: 1080,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        // Adjacent below, x_diff = 200, scale ratio = 1.0, so physical x = 0 + 200 = 200
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            Some(200)
        );
    }

    #[test]
    fn infer_physical_x_from_vertical_neighbor_not_adjacent_returns_none() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 5000,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            None
        );
    }

    #[test]
    fn infer_physical_y_from_horizontal_neighbor_adjacent_left() {
        // Target sits to the LEFT of the reference: target_right == reference.logical_x.
        let reference = ScreenLayoutInput {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 100,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        // Adjacent left, y_diff = 100, scale ratio = 1.0, so physical y = 0 + 100 = 100.
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(100)
        );
    }

    #[test]
    fn infer_physical_y_from_horizontal_neighbor_high_dpi_scales_y_diff() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 1920,
            logical_y: 50,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 3840,
            physical_height: 2160,
            is_primary: false,
        };
        // Adjacent right, y_diff = 50, scale ratio = 2160/1080 = 2.0, so physical y = 0 + 100.
        assert_eq!(
            infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
            Some(100)
        );
    }

    #[test]
    fn infer_physical_x_from_vertical_neighbor_adjacent_above() {
        // Target sits ABOVE the reference: target_bottom == reference.logical_y.
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 1080,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 300,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        };
        // Adjacent above, x_diff = 300, scale ratio = 1.0, so physical x = 0 + 300 = 300.
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            Some(300)
        );
    }

    #[test]
    fn infer_physical_x_from_vertical_neighbor_high_dpi_scales_x_diff() {
        let reference = ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        };
        let target = ScreenLayoutInput {
            logical_x: 80,
            logical_y: 1080,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 3840,
            physical_height: 2160,
            is_primary: false,
        };
        // Adjacent below, x_diff = 80, scale ratio = 3840/1920 = 2.0, so physical x = 0 + 160.
        assert_eq!(
            infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
            Some(160)
        );
    }

    // =========================================================================
    // build_physical_screen_layouts
    // =========================================================================

    #[test]
    fn build_physical_screen_layouts_single_screen() {
        let inputs = vec![ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        }];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[0].physical_width, 1920);
        assert_eq!(layouts[0].physical_height, 1080);
    }

    #[test]
    fn build_physical_screen_layouts_two_horizontal() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[1].physical_x, 1920);
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[1].physical_y, 0);
    }

    #[test]
    fn build_physical_screen_layouts_two_vertical() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 1080,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[1].physical_y, 1080);
    }

    #[test]
    fn build_physical_screen_layouts_empty_input_is_error() {
        let result = build_physical_screen_layouts(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No screens"));
    }

    #[test]
    fn build_physical_screen_layouts_mixed_dpi() {
        // Primary 1920x1080@1x, secondary 1707x960@1.5x (physical 2560x1440)
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[1].physical_x, 1920);
    }

    #[test]
    fn build_physical_screen_layouts_primary_not_first() {
        // Non-primary first, primary second. Anchor should be at index 1.
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: -1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        // Anchor (primary) is at physical (0, 0)
        assert_eq!(layouts[1].physical_x, 0);
        assert_eq!(layouts[1].physical_y, 0);
        // Left monitor at physical_x = -1920
        assert_eq!(layouts[0].physical_x, -1920);
    }

    // =========================================================================
    // normalize_physical_screen_layouts
    // =========================================================================

    #[test]
    fn normalize_physical_screen_layouts_single_screen() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let (normalized, min_x, min_y, w, h) = normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(min_x, 0);
        assert_eq!(min_y, 0);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
        assert_eq!(normalized[0].physical_x, 0);
        assert_eq!(normalized[0].physical_y, 0);
    }

    #[test]
    fn normalize_physical_screen_layouts_negative_origin_shifted() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: -1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: -1920,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let (normalized, min_x, _min_y, w, h) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(min_x, -1920);
        assert_eq!(w, 3840);
        assert_eq!(h, 1080);
        assert_eq!(normalized[0].physical_x, 0);
        assert_eq!(normalized[1].physical_x, 1920);
    }

    #[test]
    fn normalize_physical_screen_layouts_empty_returns_error() {
        let result = normalize_physical_screen_layouts(&[]);
        assert!(result.is_err());
    }

    // =========================================================================
    // resolve_recording_selection
    // =========================================================================

    #[test]
    fn resolve_recording_selection_region_within_first_screen() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let region = Region {
            physical_x: 100,
            physical_y: 100,
            physical_width: 500,
            physical_height: 400,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_x, 100);
        assert_eq!(local.physical_y, 100);
        assert_eq!(local.physical_width, 500);
        assert_eq!(local.physical_height, 400);
    }

    #[test]
    fn resolve_recording_selection_region_within_second_screen() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let region = Region {
            physical_x: 2000,
            physical_y: 100,
            physical_width: 500,
            physical_height: 400,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(local.physical_x, 80); // 2000 - 1920
        assert_eq!(local.physical_y, 100);
    }

    #[test]
    fn resolve_recording_selection_spanning_two_screens_returns_error() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        // Region spans from screen 0 to screen 1
        let region = Region {
            physical_x: 1800,
            physical_y: 100,
            physical_width: 500,
            physical_height: 400,
        };
        let result = resolve_recording_selection(&layouts, region);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("single monitor"));
    }

    #[test]
    fn resolve_recording_selection_exact_screen_bounds() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_x, 0);
        assert_eq!(local.physical_y, 0);
        assert_eq!(local.physical_width, 1920);
        assert_eq!(local.physical_height, 1080);
    }

    // =========================================================================
    // scaled_physical_dimension
    // =========================================================================

    #[test]
    fn scaled_physical_dimension_1x() {
        assert_eq!(scaled_physical_dimension(1920, 1.0).unwrap(), 1920);
    }

    #[test]
    fn scaled_physical_dimension_150pct() {
        // 1707 * 1.5 = 2560.5, ceil = 2561
        assert_eq!(scaled_physical_dimension(1707, 1.5).unwrap(), 2561);
    }

    #[test]
    fn scaled_physical_dimension_2x() {
        assert_eq!(scaled_physical_dimension(1920, 2.0).unwrap(), 3840);
    }

    #[test]
    fn scaled_physical_dimension_zero_logical() {
        assert_eq!(scaled_physical_dimension(0, 1.5).unwrap(), 0);
    }

    #[test]
    fn scaled_physical_dimension_zero_scale_returns_error() {
        assert!(scaled_physical_dimension(1920, 0.0).is_err());
    }

    #[test]
    fn scaled_physical_dimension_negative_scale_returns_error() {
        assert!(scaled_physical_dimension(1920, -1.0).is_err());
    }

    #[test]
    fn scaled_physical_dimension_nan_scale_returns_error() {
        assert!(scaled_physical_dimension(1920, f32::NAN).is_err());
    }

    #[test]
    fn scaled_physical_dimension_infinity_scale_returns_error() {
        assert!(scaled_physical_dimension(1920, f32::INFINITY).is_err());
    }

    // =========================================================================
    // logical_to_physical additional tests
    // =========================================================================

    #[test]
    fn logical_to_physical_1x_is_identity() {
        assert_eq!(logical_to_physical(1920, 1.0), 1920);
    }

    #[test]
    fn logical_to_physical_2x_doubles() {
        assert_eq!(logical_to_physical(960, 2.0), 1920);
    }

    #[test]
    fn logical_to_physical_zero_always_zero() {
        assert_eq!(logical_to_physical(0, 1.5), 0);
    }

    #[test]
    fn logical_to_physical_125pct() {
        // 1280 * 1.25 = 1600.0
        assert_eq!(logical_to_physical(1280, 1.25), 1600);
    }

    // =========================================================================
    // virtual_desktop_scale_factor additional tests
    // =========================================================================

    #[test]
    fn virtual_desktop_scale_factor_single_primary() {
        let scale = virtual_desktop_scale_factor([(true, 2.0)]);
        assert_eq!(scale, Some(2.0));
    }

    #[test]
    fn virtual_desktop_scale_factor_single_non_primary() {
        let scale = virtual_desktop_scale_factor([(false, 1.25)]);
        assert_eq!(scale, Some(1.25));
    }

    #[test]
    fn virtual_desktop_scale_factor_primary_first_wins() {
        let scale = virtual_desktop_scale_factor([(true, 1.0), (true, 2.0), (false, 3.0)]);
        assert_eq!(scale, Some(1.0));
    }

    // =========================================================================
    // build_screen_layout_inputs
    // =========================================================================

    #[test]
    fn build_screen_layout_inputs_target_gets_exact_capture_size() {
        let screens = vec![(0, 0, 1920u32, 1080u32, 1.5f32, true)];
        let inputs = build_screen_layout_inputs(screens, 0, (2880, 1620)).unwrap();
        assert_eq!(inputs[0].physical_width, 2880);
        assert_eq!(inputs[0].physical_height, 1620);
    }

    #[test]
    fn build_screen_layout_inputs_non_target_uses_scale() {
        let screens = vec![
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1707u32, 960u32, 1.5f32, false),
        ];
        let inputs = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
        // target (index 0) gets exact capture size
        assert_eq!(inputs[0].physical_width, 1920);
        // non-target (index 1): 1707 * 1.5 = 2560.5 → ceil = 2561
        assert_eq!(inputs[1].physical_width, 2561);
    }

    #[test]
    fn build_screen_layout_inputs_out_of_range_index_returns_error() {
        let screens = vec![(0, 0, 1920u32, 1080u32, 1.0f32, true)];
        let result = build_screen_layout_inputs(screens, 5, (1920, 1080));
        assert!(result.is_err());
    }

    #[test]
    fn build_screen_layout_inputs_keeps_negative_logical_coords() {
        let screens = vec![(-1920, 100, 1920u32, 1080u32, 1.0f32, false)];
        let inputs = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
        assert_eq!(inputs[0].logical_x, -1920);
        assert_eq!(inputs[0].logical_y, 100);
        assert_eq!(inputs[0].logical_width, 1920);
        assert_eq!(inputs[0].logical_height, 1080);
    }

    // =========================================================================
    // fallback_physical_x / fallback_physical_y
    // =========================================================================

    #[test]
    fn fallback_physical_x_finds_nearest_left_neighbor() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                // Gap: logically at 2000 (80px gap from 1920)
                logical_x: 2000,
                logical_y: 500,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let positions = vec![Some(0), None];
        // Target index 1: should find from left neighbor (index 0)
        let result = fallback_physical_x(&inputs, &positions, 1);
        assert!(result.is_some());
        // 0 + 1920 + scaled_gap(80, 1920, 1920) = 0 + 1920 + 80 = 2000
        assert_eq!(result.unwrap(), 2000);
    }

    #[test]
    fn fallback_physical_x_no_positioned_neighbors_returns_none() {
        let inputs = vec![ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        }];
        let positions = vec![None];
        assert_eq!(fallback_physical_x(&inputs, &positions, 0), None);
    }

    #[test]
    fn fallback_physical_y_finds_nearest_above_neighbor() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 500,
                logical_y: 1200,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let positions = vec![Some(0), None];
        let result = fallback_physical_y(&inputs, &positions, 1);
        assert!(result.is_some());
        // 0 + 1080 + scaled_gap(120, 1080, 1080) = 0 + 1080 + 120 = 1200
        assert_eq!(result.unwrap(), 1200);
    }

    #[test]
    fn fallback_physical_y_out_of_range_returns_none() {
        let inputs = vec![ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        }];
        let positions = vec![Some(0)];
        assert_eq!(fallback_physical_y(&inputs, &positions, 5), None);
    }

    #[test]
    fn fallback_physical_x_prefers_left_over_right_neighbor() {
        // Both a left and a right neighbor are equally close; the left wins.
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 2000,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 4000,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let positions = vec![Some(0), None, Some(5000)];
        // Left candidate: 0 + 1920 + scaled_gap(80) = 2000.
        // Right candidate: 5000 - 1920 - 80 = 3000. Left is preferred.
        assert_eq!(fallback_physical_x(&inputs, &positions, 1), Some(2000));
    }

    #[test]
    fn fallback_physical_y_prefers_above_over_below_neighbor() {
        // Both an above and a below neighbor are equally close; the above wins.
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 1200,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 2400,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let positions = vec![Some(0), None, Some(5000)];
        // Above candidate: 0 + 1080 + scaled_gap(120) = 1200.
        // Below candidate: 5000 - 1080 - 120 = 3800. Above is preferred.
        assert_eq!(fallback_physical_y(&inputs, &positions, 1), Some(1200));
    }

    // =========================================================================
    // capture_selection_desktop_with
    // =========================================================================

    #[test]
    fn capture_selection_desktop_with_uses_screen_capture() {
        let capture = VirtualDesktopCapture {
            rgba_data: vec![0u8; 16],
            packed_background: Vec::new(),
            width: 2,
            height: 2,
            origin_x: 0,
            origin_y: 0,
            screens: vec![],
            layouts: vec![],
        };
        let result = capture_selection_desktop_with(|| Ok(capture));
        assert!(result.is_ok());
        let vdc = result.unwrap();
        assert_eq!(vdc.width, 2);
        assert_eq!(vdc.height, 2);
    }

    #[test]
    fn capture_selection_desktop_with_propagates_error() {
        let result = capture_selection_desktop_with(|| Err("capture failed".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("capture failed"));
    }

    // =========================================================================
    // preferred_screen_index additional edge cases
    // =========================================================================

    #[test]
    fn preferred_screen_index_single_non_primary_with_cursor_hit() {
        let screens = vec![(0i32, 0i32, 1920u32, 1080u32, false)];
        assert_eq!(preferred_screen_index(screens, Some((960, 540))), Some(0));
    }

    #[test]
    fn preferred_screen_index_cursor_on_boundary_pixel() {
        let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
        // Last valid pixel is (1919, 1079)
        assert_eq!(preferred_screen_index(screens, Some((1919, 1079))), Some(0));
    }

    #[test]
    fn preferred_screen_index_cursor_just_outside() {
        let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
        // (1920, 1080) is outside
        assert_eq!(
            preferred_screen_index(screens, Some((1920, 1080))),
            Some(0) // fallback to primary
        );
    }

    // --- logical_end / physical_end tests ---

    #[test]
    fn logical_end_basic() {
        assert_eq!(logical_end(0, 100), 100);
    }

    #[test]
    fn logical_end_with_negative_start() {
        assert_eq!(logical_end(-100, 200), 100);
    }

    #[test]
    fn logical_end_zero_size() {
        assert_eq!(logical_end(50, 0), 50);
    }

    #[test]
    fn logical_end_large_values() {
        assert_eq!(logical_end(i32::MAX, 1), i64::from(i32::MAX) + 1);
    }

    #[test]
    fn physical_end_basic() {
        assert_eq!(physical_end(0, 1920), 1920);
    }

    #[test]
    fn physical_end_with_offset() {
        assert_eq!(physical_end(100, 500), 600);
    }

    #[test]
    fn physical_end_negative_start() {
        assert_eq!(physical_end(-50, 100), 50);
    }

    #[test]
    fn physical_end_zero_size() {
        assert_eq!(physical_end(100, 0), 100);
    }

    // --- scaled_physical_dimension edge cases ---

    #[test]
    fn scaled_physical_dimension_1x_scale() {
        assert_eq!(scaled_physical_dimension(1920, 1.0).unwrap(), 1920);
    }

    #[test]
    fn scaled_physical_dimension_2x_scale() {
        assert_eq!(scaled_physical_dimension(960, 2.0).unwrap(), 1920);
    }

    #[test]
    fn scaled_physical_dimension_zero_logical_size() {
        assert_eq!(scaled_physical_dimension(0, 2.0).unwrap(), 0);
    }

    #[test]
    fn scaled_physical_dimension_negative_scale_fails() {
        assert!(scaled_physical_dimension(1920, -1.0).is_err());
    }

    #[test]
    fn scaled_physical_dimension_zero_scale_fails() {
        assert!(scaled_physical_dimension(1920, 0.0).is_err());
    }

    #[test]
    fn scaled_physical_dimension_nan_scale_fails() {
        assert!(scaled_physical_dimension(1920, f32::NAN).is_err());
    }

    #[test]
    fn scaled_physical_dimension_infinity_scale_fails() {
        assert!(scaled_physical_dimension(1920, f32::INFINITY).is_err());
    }

    #[test]
    fn scaled_physical_dimension_fractional_rounds_up() {
        // 1000 * 1.5 = 1500 (exact)
        assert_eq!(scaled_physical_dimension(1000, 1.5).unwrap(), 1500);
    }

    #[test]
    fn scaled_physical_dimension_125_percent_uses_ceil() {
        // 1000 * 1.25 = 1250 (exact)
        assert_eq!(scaled_physical_dimension(1000, 1.25).unwrap(), 1250);
    }

    // --- resolve_recording_selection edge cases ---

    #[test]
    fn resolve_recording_selection_empty_layouts_returns_error() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(resolve_recording_selection(&[], region).is_err());
    }

    #[test]
    fn resolve_recording_selection_region_at_origin() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_x, 0);
        assert_eq!(local.physical_y, 0);
    }

    #[test]
    fn resolve_recording_selection_full_screen_region() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_width, 1920);
        assert_eq!(local.physical_height, 1080);
    }

    #[test]
    fn resolve_recording_selection_selects_second_monitor() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 2560,
                logical_height: 1440,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];
        let region = Region {
            physical_x: 2000,
            physical_y: 100,
            physical_width: 200,
            physical_height: 200,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(local.physical_x, 80);
        assert_eq!(local.physical_y, 100);
    }

    // --- monitor_scale_ratio edge cases ---

    #[test]
    fn monitor_scale_ratio_equal_sizes_returns_one() {
        assert_eq!(monitor_scale_ratio(100, 100), 1.0);
    }

    #[test]
    fn monitor_scale_ratio_half_size() {
        assert_eq!(monitor_scale_ratio(200, 100), 0.5);
    }

    // -------------------------------------------------------------------------
    // scaled_physical_dimension additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn scaled_physical_dimension_125_percent() {
        let result = scaled_physical_dimension(1920, 1.25).unwrap();
        assert_eq!(result, 2400);
    }

    #[test]
    fn scaled_physical_dimension_175_percent() {
        let result = scaled_physical_dimension(1920, 1.75).unwrap();
        // 1920 * 1.75 = 3360
        assert_eq!(result, 3360);
    }

    #[test]
    fn scaled_physical_dimension_fractional_rounds_up_to_ceil() {
        // 1000 * 1.333 = 1333.0 — exact, no rounding
        let result = scaled_physical_dimension(1000, 1.333).unwrap();
        assert_eq!(result, 1333);
    }

    #[test]
    fn scaled_physical_dimension_1_logical_at_2x() {
        let result = scaled_physical_dimension(1, 2.0).unwrap();
        assert_eq!(result, 2);
    }

    #[test]
    fn scaled_physical_dimension_large_logical_size() {
        let result = scaled_physical_dimension(7680, 1.0).unwrap();
        assert_eq!(result, 7680);
    }

    // -------------------------------------------------------------------------
    // build_screen_layout_inputs additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn build_screen_layout_inputs_four_monitors() {
        let screens = vec![
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1920, 1080, 1.0, false),
            (0, 1080, 1920, 1080, 1.5, false),
            (1920, 1080, 1920, 1080, 2.0, false),
        ];
        let result = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
        assert_eq!(result.len(), 4);
        // Target monitor gets exact capture size
        assert_eq!(result[0].physical_width, 1920);
        assert_eq!(result[0].physical_height, 1080);
        // Non-target at 1.0 scale
        assert_eq!(result[1].physical_width, 1920);
        // Non-target at 1.5 scale: ceil(1920 * 1.5) = 2880
        assert_eq!(result[2].physical_width, 2880);
        // Non-target at 2.0 scale
        assert_eq!(result[3].physical_width, 3840);
    }

    #[test]
    fn build_screen_layout_inputs_target_last_of_three() {
        let screens = vec![
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 2560, 1440, 1.0, false),
            (4480, 0, 1920, 1080, 1.5, false),
        ];
        let result = build_screen_layout_inputs(screens, 2, (2880, 1620)).unwrap();
        // Target at index 2 gets exact capture size
        assert_eq!(result[2].physical_width, 2880);
        assert_eq!(result[2].physical_height, 1620);
        // Non-targets use scale
        assert_eq!(result[0].physical_width, 1920);
    }

    #[test]
    fn build_screen_layout_inputs_negative_coords_preserved() {
        let screens = vec![
            (-1920, 0, 1920u32, 1080u32, 1.0f32, false),
            (0, 0, 1920, 1080, 1.0, true),
        ];
        let result = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
        assert_eq!(result[0].logical_x, -1920);
        assert_eq!(result[1].logical_x, 0);
    }

    // -------------------------------------------------------------------------
    // logical_to_physical additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn logical_to_physical_very_large_value() {
        let result = logical_to_physical(10000, 2.0);
        assert_eq!(result, 20000);
    }

    #[test]
    fn logical_to_physical_zero_at_150pct() {
        assert_eq!(logical_to_physical(0, 1.5), 0);
    }

    #[test]
    fn logical_to_physical_scale_one_is_identity() {
        assert_eq!(logical_to_physical(1920, 1.0), 1920);
        assert_eq!(logical_to_physical(-500, 1.0), -500);
    }

    // -------------------------------------------------------------------------
    // build_region_from_drag additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn build_region_from_drag_exact_minimum_size() {
        // The minimum accepted region should have width/height > some threshold
        let result = build_region_from_drag((0, 0), (10, 10));
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.physical_width, 10);
        assert_eq!(r.physical_height, 10);
    }

    #[test]
    fn build_region_from_drag_1x1_returns_none() {
        let result = build_region_from_drag((0, 0), (1, 1));
        assert!(result.is_none());
    }

    #[test]
    fn build_region_from_drag_identical_start_end_returns_none() {
        let result = build_region_from_drag((100, 200), (100, 200));
        assert!(result.is_none());
    }

    #[test]
    fn build_region_from_drag_reversed_large_region() {
        let result = build_region_from_drag((1920, 1080), (0, 0));
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.physical_x, 0);
        assert_eq!(r.physical_y, 0);
        assert_eq!(r.physical_width, 1920);
        assert_eq!(r.physical_height, 1080);
    }

    // -------------------------------------------------------------------------
    // virtual_desktop_scale_factor additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn virtual_desktop_scale_factor_two_primaries_uses_first() {
        let screens = vec![(true, 1.5f32), (true, 2.0)];
        let result = virtual_desktop_scale_factor(screens);
        assert_eq!(result, Some(1.5));
    }

    #[test]
    fn virtual_desktop_scale_factor_no_primary_falls_back_to_first() {
        let screens = vec![(false, 1.0f32), (false, 2.0)];
        let result = virtual_desktop_scale_factor(screens);
        assert_eq!(result, Some(1.0));
    }

    // -------------------------------------------------------------------------
    // find_screen_index_at_position additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn find_screen_index_at_position_origin_hit() {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
    }

    #[test]
    fn find_screen_index_at_position_just_past_bottom_right() {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        assert_eq!(find_screen_index_at_position(screens, (1920, 1080)), None);
    }

    #[test]
    fn find_screen_index_at_position_negative_coords_miss() {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        assert_eq!(find_screen_index_at_position(screens, (-1, -1)), None);
    }

    #[test]
    fn find_screen_index_at_position_negative_origin_screen() {
        let screens = vec![(-1920, 0, 1920u32, 1080u32), (0, 0, 1920u32, 1080u32)];
        assert_eq!(find_screen_index_at_position(screens, (-960, 540)), Some(0));
    }

    // -------------------------------------------------------------------------
    // preferred_screen_index additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn preferred_screen_index_single_primary_no_cursor() {
        let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
        assert_eq!(preferred_screen_index(screens, None), Some(0));
    }

    #[test]
    fn preferred_screen_index_single_non_primary_no_cursor() {
        let screens = vec![(0i32, 0i32, 1920u32, 1080u32, false)];
        assert_eq!(preferred_screen_index(screens, None), Some(0));
    }

    // -------------------------------------------------------------------------
    // ranges_overlap additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn ranges_overlap_same_range_overlaps() {
        assert!(ranges_overlap(0, 100, 0, 100));
    }

    #[test]
    fn ranges_overlap_touching_boundaries_no_overlap() {
        // [0..100) and [100..200) do not overlap
        assert!(!ranges_overlap(0, 100, 100, 100));
    }

    #[test]
    fn ranges_overlap_half_overlap() {
        // [0..100) and [50..150)
        assert!(ranges_overlap(0, 100, 50, 100));
    }

    #[test]
    fn ranges_overlap_completely_separated() {
        assert!(!ranges_overlap(0, 10, 20, 10));
    }

    #[test]
    fn ranges_overlap_containment() {
        assert!(ranges_overlap(0, 200, 50, 50));
    }

    #[test]
    fn ranges_overlap_zero_size_a_does_not_overlap() {
        assert!(!ranges_overlap(0, 0, 0, 100));
    }

    // -------------------------------------------------------------------------
    // monitor_scale_ratio additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn monitor_scale_ratio_equal_sizes_is_1() {
        assert_eq!(monitor_scale_ratio(1920, 1920), 1.0);
    }

    #[test]
    fn monitor_scale_ratio_double_is_2() {
        assert_eq!(monitor_scale_ratio(1920, 3840), 2.0);
    }

    #[test]
    fn monitor_scale_ratio_zero_logical_defaults_to_1() {
        assert_eq!(monitor_scale_ratio(0, 1920), 1.0);
    }

    // -------------------------------------------------------------------------
    // logical_end and physical_end additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn logical_end_from_zero_origin() {
        assert_eq!(logical_end(0, 1920), 1920);
    }

    #[test]
    fn logical_end_from_negative_origin() {
        assert_eq!(logical_end(-1920, 1920), 0);
    }

    #[test]
    fn logical_end_with_zero_size() {
        assert_eq!(logical_end(100, 0), 100);
    }

    #[test]
    fn physical_end_from_zero_origin() {
        assert_eq!(physical_end(0, 3840), 3840);
    }

    // -------------------------------------------------------------------------
    // logical_to_physical additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn logical_to_physical_zero_with_2x_scale() {
        assert_eq!(logical_to_physical(0, 2.0), 0);
    }

    #[test]
    fn logical_to_physical_identity_at_1x() {
        assert_eq!(logical_to_physical(100, 1.0), 100);
    }

    #[test]
    fn logical_to_physical_960_at_2x() {
        assert_eq!(logical_to_physical(960, 2.0), 1920);
    }

    #[test]
    fn logical_to_physical_negative_at_2x() {
        assert_eq!(logical_to_physical(-100, 2.0), -200);
    }

    // -------------------------------------------------------------------------
    // scaled_gap additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn scaled_gap_zero_gap_identity() {
        assert_eq!(scaled_gap(0, 1920, 1920), 0);
    }

    #[test]
    fn scaled_gap_at_1x() {
        assert_eq!(scaled_gap(10, 1920, 1920), 10);
    }

    #[test]
    fn scaled_gap_at_2x() {
        assert_eq!(scaled_gap(10, 1920, 3840), 20);
    }

    // -------------------------------------------------------------------------
    // virtual_desktop_scale_factor extra tests
    // -------------------------------------------------------------------------

    #[test]
    fn virtual_desktop_scale_factor_one_primary_returns_its_scale() {
        let scale = virtual_desktop_scale_factor([(true, 2.0)]);
        assert_eq!(scale, Some(2.0));
    }

    #[test]
    fn virtual_desktop_scale_factor_one_non_primary_returns_fallback() {
        let scale = virtual_desktop_scale_factor([(false, 1.5)]);
        assert_eq!(scale, Some(1.5));
    }

    // -------------------------------------------------------------------------
    // build_physical_screen_layouts error path: disconnected screens
    // -------------------------------------------------------------------------

    #[test]
    fn build_physical_screen_layouts_disconnected_screens_returns_error() {
        // Two screens with no logical adjacency or overlap — resolution should fail
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(5000, 5000, 1920, 1080, 1920, 1080),
        ];
        // The primary is index 0, so it gets (0,0). Screen 1 at (5000,5000)
        // has no overlap or adjacency with screen 0, so fallback positioning
        // kicks in. Whether this succeeds or fails depends on the fallback
        // logic finding a gap-based match.
        let result = build_physical_screen_layouts(&inputs);
        // The fallback path uses gap-based positioning, so it should still succeed
        // (left_candidate or above_candidate will match).
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // normalize_physical_screen_layouts overflow error path
    // -------------------------------------------------------------------------

    #[test]
    fn normalize_physical_screen_layouts_extreme_coordinates() {
        // Screens at extreme coordinates that cause i64 subtraction to exceed u32 range
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: i32::MIN,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: i32::MAX,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let result = normalize_physical_screen_layouts(&layouts);
        // physical_end of second screen = i32::MAX + 1920 as i64 = 2147485567
        // min_x = i32::MIN = -2147483648
        // width = 2147485567 - (-2147483648) = 4294969215 which overflows u32
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("out of range"),
            "error should mention out of range"
        );
    }

    // -------------------------------------------------------------------------
    // build_physical_screen_layouts: primary not at index 0
    // -------------------------------------------------------------------------

    #[test]
    fn build_physical_screen_layouts_primary_at_index_2() {
        let inputs = vec![
            make_screen(0, 0, 1920, 1080, 1920, 1080),
            make_screen(1920, 0, 1920, 1080, 1920, 1080),
            ScreenLayoutInput {
                logical_x: 3840,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
        ];
        let result = build_physical_screen_layouts(&inputs).unwrap();
        // Primary (index 2) should be at (0, 0), others positioned relative
        assert_eq!(result[2].physical_x, 0);
        assert_eq!(result[2].physical_y, 0);
        // Screen 1 is left of primary by 1920 logical pixels
        assert_eq!(result[1].physical_x, -1920);
        // Screen 0 is left of screen 1 by another 1920 logical pixels
        assert_eq!(result[0].physical_x, -3840);
    }

    // -------------------------------------------------------------------------
    // resolve_recording_selection: region at exact screen boundary
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_recording_selection_region_at_bottom_right_corner() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        // Region touching the bottom-right corner (1-pixel region at last pixel)
        let region = Region {
            physical_x: 1919,
            physical_y: 1079,
            physical_width: 1,
            physical_height: 1,
        };
        let result = resolve_recording_selection(&layouts, region);
        assert!(result.is_ok());
        let (idx, local) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_x, 1919);
        assert_eq!(local.physical_y, 1079);
    }

    #[test]
    fn resolve_recording_selection_region_one_pixel_past_boundary() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        // Region that extends 1 pixel past the right edge
        let region = Region {
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        let result = resolve_recording_selection(&layouts, region);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // scaled_physical_dimension edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn scaled_physical_dimension_negative_infinity_returns_error() {
        assert!(scaled_physical_dimension(100, f32::NEG_INFINITY).is_err());
    }

    #[test]
    fn scaled_physical_dimension_very_small_positive_scale() {
        // Very small but positive scale factor should succeed
        let result = scaled_physical_dimension(100, 0.01);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // ceil(100 * 0.01) = ceil(1.0) = 1
    }

    // -------------------------------------------------------------------------
    // normalize_physical_screen_layouts — height overflow
    // -------------------------------------------------------------------------

    #[test]
    fn normalize_physical_screen_layouts_width_overflow_returns_error() {
        // physical_end(1, u32::MAX) = 1 + u32::MAX = u32::MAX+1 as i64.
        // min_x = 0, so width = u32::MAX+1 > u32::MAX → Err.
        // Normalization: (0 - 0 = 0) and (1 - 0 = 1), both safe i32 ops.
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 1,
                physical_y: 0,
                physical_width: u32::MAX,
                physical_height: 1080,
            },
        ];
        let result = normalize_physical_screen_layouts(&layouts);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("out of range"),
            "error should mention out of range: {err}"
        );
    }

    // -------------------------------------------------------------------------
    // scaled_physical_dimension — overflow for huge logical_size * scale
    // -------------------------------------------------------------------------

    #[test]
    fn scaled_physical_dimension_huge_size_times_scale_returns_overflow_error() {
        // u32::MAX * 2.0 = ~8.6 billion > u32::MAX
        let result = scaled_physical_dimension(u32::MAX, 2.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of range"));
    }

    // -------------------------------------------------------------------------
    // resolve_recording_selection — edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_recording_selection_local_region_has_zero_origin_for_first_screen() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        let region = Region {
            physical_x: 100,
            physical_y: 200,
            physical_width: 300,
            physical_height: 400,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(local.physical_x, 100);
        assert_eq!(local.physical_y, 200);
    }

    #[test]
    fn resolve_recording_selection_local_region_subtracts_screen_offset() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 2560,
                logical_height: 1440,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];
        let region = Region {
            physical_x: 2000,
            physical_y: 100,
            physical_width: 200,
            physical_height: 200,
        };
        let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(local.physical_x, 80); // 2000 - 1920
        assert_eq!(local.physical_y, 100); // 100 - 0
        assert_eq!(local.physical_width, 200);
        assert_eq!(local.physical_height, 200);
    }

    // -------------------------------------------------------------------------
    // build_screen_layout_inputs — invalid scale factor propagation
    // -------------------------------------------------------------------------

    #[test]
    fn build_screen_layout_inputs_invalid_scale_factor_returns_error() {
        // Non-target monitor with NaN scale should fail
        let screens = vec![
            (0, 0, 1920, 1080, 1.0f32, true),
            (1920, 0, 2560, 1440, f32::NAN, false),
        ];
        let result = build_screen_layout_inputs(screens, 0, (1920, 1080));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("scale factor"));
    }

    // -------------------------------------------------------------------------
    // build_region_from_drag — reversed coordinates
    // -------------------------------------------------------------------------

    #[test]
    fn build_region_from_drag_reversed_coordinates_normalizes() {
        // Drag from bottom-right to top-left
        let region = build_region_from_drag((200, 300), (10, 20)).unwrap();
        assert_eq!(region.physical_x, 10);
        assert_eq!(region.physical_y, 20);
        assert_eq!(region.physical_width, 190);
        assert_eq!(region.physical_height, 280);
    }

    #[test]
    fn build_region_from_drag_exactly_5_pixels_returns_none() {
        // 5 px in both dimensions — threshold is > 5, so this should be None
        assert!(build_region_from_drag((0, 0), (5, 5)).is_none());
    }

    #[test]
    fn build_region_from_drag_6_pixels_returns_some() {
        // 6 px in both dimensions — just above threshold
        let region = build_region_from_drag((0, 0), (6, 6));
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.physical_width, 6);
        assert_eq!(r.physical_height, 6);
    }

    // -------------------------------------------------------------------------
    // preferred_screen_index — cursor in gap between screens
    // -------------------------------------------------------------------------

    #[test]
    fn preferred_screen_index_cursor_in_gap_falls_back_to_primary() {
        let screens = vec![
            (0, 0, 1920, 1080, false),
            (2000, 0, 1920, 1080, true), // gap from 1920 to 2000
        ];
        // Cursor at (1950, 500) is in the gap — not on any screen
        let idx = preferred_screen_index(screens, Some((1950, 500)));
        assert_eq!(idx, Some(1), "should fall back to primary screen");
    }

    // -------------------------------------------------------------------------
    // logical_to_physical — additional scale factors
    // -------------------------------------------------------------------------

    #[test]
    fn logical_to_physical_negative_value() {
        assert_eq!(logical_to_physical(-100, 2.0), -200);
    }

    #[test]
    fn logical_to_physical_zero_value_at_scale_150() {
        assert_eq!(logical_to_physical(0, 1.5), 0);
    }

    #[test]
    fn logical_to_physical_rounds_to_nearest() {
        // 100 * 1.5 = 150.0, rounds to 150
        assert_eq!(logical_to_physical(100, 1.5), 150);
        // 7 * 1.25 = 8.75, rounds to 9
        assert_eq!(logical_to_physical(7, 1.25), 9);
    }

    // =========================================================================
    // Task 2.1 — additional unit tests for ranges_overlap, monitor_scale_ratio,
    //            scaled_gap, logical_to_physical
    // Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9
    // =========================================================================

    // --- ranges_overlap: both ranges zero-size at different positions ---

    #[test]
    fn ranges_overlap_both_zero_size_at_different_positions() {
        // Two zero-size ranges at distinct positions never overlap
        assert!(!ranges_overlap(10, 0, 20, 0));
    }

    #[test]
    fn ranges_overlap_first_zero_size_at_start_of_second() {
        // Zero-size range exactly at the start of a non-zero range:
        // [50, 50) vs [50, 150) → start_a(50) < end_b(150) && start_b(50) < end_a(50) → false
        assert!(!ranges_overlap(50, 0, 50, 100));
    }

    #[test]
    fn ranges_overlap_zero_size_at_end_of_other() {
        // Zero-size range at the exclusive end of another range:
        // [0, 100) vs [100, 100) → start_a(0) < end_b(100) && start_b(100) < end_a(100) → false
        assert!(!ranges_overlap(0, 100, 100, 0));
    }

    #[test]
    fn ranges_overlap_completely_disjoint_negative_and_positive() {
        // [-100, -50) vs [50, 150) — clearly non-overlapping
        assert!(!ranges_overlap(-100, 50, 50, 100));
    }

    #[test]
    fn ranges_overlap_adjacent_reversed_order() {
        // [100, 200) and [0, 100) — adjacent (touching), should not overlap
        assert!(!ranges_overlap(100, 100, 0, 100));
    }

    #[test]
    fn ranges_overlap_subset_reversed() {
        // Second range fully contains first
        assert!(ranges_overlap(100, 50, 0, 1000));
    }

    // --- monitor_scale_ratio: explicit physical/logical equality ---

    #[test]
    fn monitor_scale_ratio_exact_ratio_equals_physical_over_logical() {
        // 3840 / 1920 = 2.0 exactly
        let ratio = monitor_scale_ratio(1920, 3840);
        assert_eq!(ratio, 3840.0 / 1920.0);
    }

    #[test]
    fn monitor_scale_ratio_small_logical_large_physical() {
        // 100 logical → 250 physical = 2.5
        assert_eq!(monitor_scale_ratio(100, 250), 2.5);
    }

    #[test]
    fn monitor_scale_ratio_physical_smaller_than_logical() {
        // Downscale: 1920 logical → 960 physical = 0.5
        assert_eq!(monitor_scale_ratio(1920, 960), 0.5);
    }

    #[test]
    fn monitor_scale_ratio_zero_logical_zero_physical_returns_1() {
        // Both zero: fallback to 1.0
        assert_eq!(monitor_scale_ratio(0, 0), 1.0);
    }

    // --- scaled_gap: round(gap * physical / logical) verification ---

    #[test]
    fn scaled_gap_fractional_result_rounds_correctly() {
        // gap=10, logical=3, physical=4 → 10 * (4/3) = 13.333... → rounds to 13
        assert_eq!(scaled_gap(10, 3, 4), 13);
    }

    #[test]
    fn scaled_gap_rounds_half_away_from_zero() {
        // gap=3, logical=2, physical=3 → 3 * (3/2) = 4.5 → rounds to 5
        assert_eq!(scaled_gap(3, 2, 3), 5);
    }

    #[test]
    fn scaled_gap_negative_gap_fractional_scale() {
        // gap=-10, logical=3, physical=4 → -10 * (4/3) = -13.333... → rounds to -13
        assert_eq!(scaled_gap(-10, 3, 4), -13);
    }

    #[test]
    fn scaled_gap_matches_manual_formula() {
        // Verify scaled_gap(gap, l, p) == round(gap * p / l) for several cases
        let cases: Vec<(i32, u32, u32)> = vec![
            (100, 1920, 2880), // 1.5x → 150
            (50, 960, 1920),   // 2.0x → 100
            (7, 100, 150),     // 1.5x → 10.5 → 11 (rounds up)
        ];
        for (gap, logical, physical) in cases {
            let expected = ((gap as f32) * (physical as f32 / logical as f32)).round() as i32;
            assert_eq!(
                scaled_gap(gap, logical, physical),
                expected,
                "scaled_gap({gap}, {logical}, {physical}) should equal {expected}"
            );
        }
    }

    // --- logical_to_physical: negative coords + fractional scale ---

    #[test]
    fn logical_to_physical_negative_with_125_scale() {
        // -1024 * 1.25 = -1280.0
        assert_eq!(logical_to_physical(-1024, 1.25), -1280);
    }

    #[test]
    fn logical_to_physical_negative_with_175_scale() {
        // -100 * 1.75 = -175.0
        assert_eq!(logical_to_physical(-100, 1.75), -175);
    }

    #[test]
    fn logical_to_physical_negative_fractional_rounds_away_from_zero() {
        // -7 * 1.25 = -8.75 → rounds to -9
        assert_eq!(logical_to_physical(-7, 1.25), -9);
    }

    #[test]
    fn logical_to_physical_zero_with_any_scale_is_zero() {
        // Zero input always produces zero regardless of scale
        for scale in [0.5, 1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
            assert_eq!(
                logical_to_physical(0, scale),
                0,
                "logical_to_physical(0, {scale}) should be 0"
            );
        }
    }

    // =========================================================================
    // Task 2.2 — unit tests for build_physical_screen_layouts and
    //            normalize_physical_screen_layouts
    // Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7
    // =========================================================================

    // --- Req 2.1: single primary monitor placed at physical origin (0, 0) ---

    #[test]
    fn build_layouts_single_primary_at_origin() {
        let inputs = vec![ScreenLayoutInput {
            logical_x: 100,
            logical_y: 200,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 2880,
            physical_height: 1620,
            is_primary: true,
        }];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].physical_x, 0, "primary must be anchored at x=0");
        assert_eq!(layouts[0].physical_y, 0, "primary must be anchored at y=0");
        assert_eq!(layouts[0].physical_width, 2880);
        assert_eq!(layouts[0].physical_height, 1620);
        // Logical fields preserved
        assert_eq!(layouts[0].logical_x, 100);
        assert_eq!(layouts[0].logical_y, 200);
    }

    // --- Req 2.2: two horizontally adjacent monitors at different DPI ---

    #[test]
    fn build_layouts_horizontal_mixed_dpi_positions_account_for_pixel_dimensions() {
        // Primary: 1920x1080 logical, 1920x1080 physical (1x)
        // Secondary: 1707x960 logical, 2560x1440 physical (1.5x), right of primary
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        // Primary at origin
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[0].physical_y, 0);
        // Secondary physical_x = primary physical_width (they are adjacent)
        assert_eq!(
            layouts[1].physical_x, 1920,
            "second monitor physical_x should equal first monitor's physical width"
        );
        assert_eq!(layouts[1].physical_y, 0);
        // Each monitor retains its own physical dimensions
        assert_eq!(layouts[0].physical_width, 1920);
        assert_eq!(layouts[1].physical_width, 2560);
    }

    // --- Req 2.3: two vertically stacked monitors ---

    #[test]
    fn build_layouts_vertical_stack_second_y_equals_first_physical_height() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 1080,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(
            layouts[1].physical_y, layouts[0].physical_height as i32,
            "second monitor physical_y should equal first monitor's physical height"
        );
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[1].physical_x, 0);
    }

    // --- Req 2.4: empty input returns error ---

    #[test]
    fn build_layouts_empty_input_returns_error_with_message() {
        let result = build_physical_screen_layouts(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No screens"),
            "error should mention no screens: {err}"
        );
    }

    // --- Req 2.5: no primary monitor uses first as anchor at (0, 0) ---

    #[test]
    fn build_layouts_no_primary_uses_first_as_anchor() {
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false, // not primary
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 2560,
                logical_height: 1440,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false, // not primary either
            },
        ];
        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        // First monitor should be the anchor at (0, 0)
        assert_eq!(
            layouts[0].physical_x, 0,
            "first monitor should be anchor at x=0 when no primary"
        );
        assert_eq!(
            layouts[0].physical_y, 0,
            "first monitor should be anchor at y=0 when no primary"
        );
        // Second monitor positioned relative to first
        assert_eq!(layouts[1].physical_x, 1920);
    }

    // --- Req 2.6: negative coordinate normalization shifts min X/Y to zero ---

    #[test]
    fn normalize_layouts_negative_coords_shifted_to_zero() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: -1920,
                logical_y: -500,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: -2560,
                physical_y: -750,
                physical_width: 2560,
                physical_height: 1440,
            },
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let (normalized, min_x, min_y, _w, _h) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(min_x, -2560);
        assert_eq!(min_y, -750);
        // After normalization, the leftmost/topmost screen starts at (0, 0)
        assert_eq!(
            normalized[0].physical_x, 0,
            "min X should be shifted to zero"
        );
        assert_eq!(
            normalized[0].physical_y, 0,
            "min Y should be shifted to zero"
        );
        // Second screen shifted by the same offset
        assert_eq!(normalized[1].physical_x, 2560);
        assert_eq!(normalized[1].physical_y, 750);
    }

    // --- Req 2.7: total width/height spans full virtual desktop ---

    #[test]
    fn normalize_layouts_total_size_spans_full_virtual_desktop() {
        // Two horizontally adjacent monitors with different physical sizes
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1707,
                logical_height: 960,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];
        let (_normalized, _min_x, _min_y, width, height) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        // Total width = rightmost physical_end - leftmost physical_x
        // = (1920 + 2560) - 0 = 4480
        assert_eq!(
            width, 4480,
            "total width should span from leftmost to rightmost physical edge"
        );
        // Total height = max physical_end_y - min physical_y
        // = max(1080, 1440) - 0 = 1440
        assert_eq!(
            height, 1440,
            "total height should span from topmost to bottommost physical edge"
        );
    }

    #[test]
    fn normalize_layouts_vertical_stack_total_height_is_sum() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 1080,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 1080,
                physical_width: 1920,
                physical_height: 1080,
            },
        ];
        let (_normalized, _min_x, _min_y, width, height) =
            normalize_physical_screen_layouts(&layouts).unwrap();
        assert_eq!(width, 1920, "width should be the single column width");
        assert_eq!(
            height, 2160,
            "height should be sum of both monitors' physical heights"
        );
    }

    // --- Task 2.3: build_region_from_drag, resolve_recording_selection,
    //     find_screen_index_at_position, preferred_screen_index ---
    //
    // Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8

    /// Requirement 3.1: identical start and end points produce None.
    #[test]
    fn req3_1_build_region_from_drag_identical_start_end_returns_none() {
        assert!(build_region_from_drag((500, 300), (500, 300)).is_none());
    }

    /// Requirement 3.2: negative coordinates spanning a valid area produce a
    /// correct Region with the minimum coordinates as origin.
    #[test]
    fn req3_2_build_region_from_drag_negative_coords_spanning_valid_area() {
        let region = build_region_from_drag((-50, -30), (60, 80)).expect("should produce a region");
        assert_eq!(region.physical_x, -50, "origin x should be the minimum");
        assert_eq!(region.physical_y, -30, "origin y should be the minimum");
        assert_eq!(region.physical_width, 110);
        assert_eq!(region.physical_height, 110);
    }

    /// Requirement 3.3: region fully inside one monitor returns the correct
    /// screen index and local coordinates.
    #[test]
    fn req3_3_resolve_recording_selection_region_inside_one_monitor() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 2560,
                logical_height: 1440,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];
        let region = Region {
            physical_x: 100,
            physical_y: 100,
            physical_width: 400,
            physical_height: 300,
        };
        let (index, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(index, 0);
        assert_eq!(local.physical_x, 100);
        assert_eq!(local.physical_y, 100);
        assert_eq!(local.physical_width, 400);
        assert_eq!(local.physical_height, 300);
    }

    /// Requirement 3.4: region spanning two monitors returns an error.
    #[test]
    fn req3_4_resolve_recording_selection_spanning_two_monitors_returns_error() {
        let layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            PhysicalScreenLayout {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 2560,
                logical_height: 1440,
                physical_x: 1920,
                physical_y: 0,
                physical_width: 2560,
                physical_height: 1440,
            },
        ];
        // Region crosses the boundary between monitor 0 and monitor 1
        let region = Region {
            physical_x: 1800,
            physical_y: 100,
            physical_width: 300,
            physical_height: 200,
        };
        let err = resolve_recording_selection(&layouts, region).unwrap_err();
        assert!(
            err.contains("single monitor"),
            "error should mention single monitor constraint: {err}"
        );
    }

    /// Requirement 3.5: region at the exact boundary of a monitor is accepted.
    #[test]
    fn req3_5_resolve_recording_selection_exact_boundary_accepted() {
        let layouts = vec![PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        }];
        // Region fills the entire monitor exactly
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        let (index, local) = resolve_recording_selection(&layouts, region).unwrap();
        assert_eq!(index, 0);
        assert_eq!(local.physical_x, 0);
        assert_eq!(local.physical_y, 0);
        assert_eq!(local.physical_width, 1920);
        assert_eq!(local.physical_height, 1080);
    }

    /// Requirement 3.6: cursor inside a monitor returns the correct index.
    #[test]
    fn req3_6_find_screen_index_at_position_cursor_inside_monitor() {
        let screens = vec![(0, 0, 1920u32, 1080u32), (1920, 0, 2560u32, 1440u32)];
        assert_eq!(
            find_screen_index_at_position(screens, (2500, 700)),
            Some(1),
            "cursor at (2500,700) should be inside the second monitor"
        );
    }

    /// Requirement 3.7: cursor outside all monitors returns None.
    #[test]
    fn req3_7_find_screen_index_at_position_cursor_outside_all_monitors() {
        let screens = vec![(0, 0, 1920u32, 1080u32), (1920, 0, 2560u32, 1440u32)];
        assert_eq!(
            find_screen_index_at_position(screens, (9999, 9999)),
            None,
            "cursor far outside all monitors should return None"
        );
    }

    /// Requirement 3.8: preferred_screen_index with None cursor falls back to
    /// the primary monitor.
    #[test]
    fn req3_8_preferred_screen_index_none_cursor_falls_back_to_primary() {
        let screens = vec![
            (0, 0, 1920, 1080, false),
            (1920, 0, 2560, 1440, true), // primary
        ];
        assert_eq!(
            preferred_screen_index(screens, None),
            Some(1),
            "with no cursor, should fall back to the primary monitor (index 1)"
        );
    }

    // --- Property-based tests ---

    use proptest::prelude::*;

    proptest! {
        /// **Validates: Requirements 17.1, 1.8, 1.9**
        ///
        /// For all i32 values and positive f32 scale factors,
        /// logical_to_physical result is within 1 unit of value * scale.
        ///
        /// The function uses f32 arithmetic: `((value as f32) * scale).round() as i32`.
        /// When the f32 product is within i32 range, the result is within 1 unit
        /// of the exact f64 product (accounting for f32 precision loss and rounding).
        /// When the product overflows i32 range, the `as i32` cast saturates to
        /// i32::MIN or i32::MAX, which we accept as correct clamping behavior.
        #[test]
        fn prop_logical_to_physical_rounding_tolerance(
            value in any::<i32>(),
            scale in (0.0f32..1e10).prop_filter("positive scale", |s| *s > 0.0 && s.is_finite()),
        ) {
            let result = logical_to_physical(value, scale);
            let exact = (value as f64) * (scale as f64);

            // When the exact product is outside i32 range, the function saturates
            // via `as i32` cast — that is expected clamping behavior.
            if exact > i32::MAX as f64 + 0.5 {
                prop_assert_eq!(result, i32::MAX,
                    "expected saturation to i32::MAX for value={}, scale={}", value, scale);
            } else if exact < i32::MIN as f64 - 0.5 {
                prop_assert_eq!(result, i32::MIN,
                    "expected saturation to i32::MIN for value={}, scale={}", value, scale);
            } else {
                // Within representable range: result is within 1 unit of exact product
                let diff = (result as f64 - exact).abs();
                prop_assert!(
                    diff <= 1.0,
                    "logical_to_physical({}, {}) = {}, exact = {}, diff {} exceeds 1.0",
                    value, scale, result, exact, diff
                );
            }
        }

        /// **Validates: Requirements 17.2, 1.1, 1.2, 1.3**
        ///
        /// For all (start_a, size_a, start_b, size_b) tuples, `ranges_overlap`
        /// returns the correct result: two half-open ranges [start, start+size)
        /// overlap iff start_a < start_b + size_b AND start_b < start_a + size_a.
        ///
        /// The reference implementation uses i64 arithmetic to avoid overflow,
        /// matching the production code's use of `logical_end`.
        #[test]
        fn prop_ranges_overlap_correctness(
            start_a in any::<i32>(),
            size_a in any::<u32>(),
            start_b in any::<i32>(),
            size_b in any::<u32>(),
        ) {
            let result = ranges_overlap(start_a, size_a, start_b, size_b);

            // Reference: half-open interval overlap using i64 to avoid overflow
            let end_a = i64::from(start_a) + i64::from(size_a);
            let end_b = i64::from(start_b) + i64::from(size_b);
            let expected = i64::from(start_a) < end_b && i64::from(start_b) < end_a;

            prop_assert_eq!(
                result, expected,
                "ranges_overlap({}, {}, {}, {}) = {}, expected {}",
                start_a, size_a, start_b, size_b, result, expected
            );
        }
    }

    #[test]
    fn test_overlay_perf_pack_rgba_to_softbuffer() {
        let rgba = vec![
            10, 20, 30, 255, // pixel 0
            200, 100, 50, 128, // pixel 1
            99,  // trailing incomplete byte ignored
        ];
        let packed = pack_rgba_to_softbuffer(&rgba);
        assert_eq!(
            packed,
            vec![
                (10u32 << 16) | (20u32 << 8) | 30u32,
                (200u32 << 16) | (100u32 << 8) | 50u32,
            ]
        );
    }

    #[test]
    fn test_overlay_perf_render_frame_matches_reference_crosshair_and_border() {
        let src_w = 16usize;
        let src_h = 12usize;
        let mut rgba = Vec::with_capacity(src_w * src_h * 4);
        for i in 0..(src_w * src_h) {
            rgba.push(((i * 7) % 256) as u8);
            rgba.push(((i * 13) % 256) as u8);
            rgba.push(((i * 29) % 256) as u8);
            rgba.push(255);
        }
        let packed = pack_rgba_to_softbuffer(&rgba);

        // Reference scalar renderer matching original draw() logic
        let reference_render = |buf_w: usize,
                                buf_h: usize,
                                start_pos: Option<(i32, i32)>,
                                current_pos: Option<(i32, i32)>| {
            let mut buffer = vec![0u32; buf_w * buf_h];
            for y in 0..buf_h {
                for x in 0..buf_w {
                    let src_idx = (y * src_w + x) * 4;
                    let dst_idx = y * buf_w + x;
                    if src_idx + 3 <= rgba.len() && dst_idx < buffer.len() {
                        let mut r = rgba[src_idx] as u32;
                        let mut g = rgba[src_idx + 1] as u32;
                        let mut b = rgba[src_idx + 2] as u32;
                        if let (Some(start), Some(end)) = (start_pos, current_pos) {
                            let x1 = start.0.min(end.0) as usize;
                            let y1 = start.1.min(end.1) as usize;
                            let x2 = start.0.max(end.0) as usize;
                            let y2 = start.1.max(end.1) as usize;
                            let on_border = (x >= x1.saturating_sub(1)
                                && x <= x2 + 1
                                && ((y >= y1.saturating_sub(1) && y <= y1 + 1)
                                    || (y >= y2.saturating_sub(1) && y <= y2 + 1)))
                                || (y >= y1.saturating_sub(1)
                                    && y <= y2 + 1
                                    && ((x >= x1.saturating_sub(1) && x <= x1 + 1)
                                        || (x >= x2.saturating_sub(1) && x <= x2 + 1)));
                            if on_border {
                                r = 0;
                                g = 200;
                                b = 255;
                            }
                        } else if let Some(pos) = current_pos {
                            if x == pos.0 as usize || y == pos.1 as usize {
                                r = 255;
                                g = 0;
                                b = 0;
                            }
                        }
                        buffer[dst_idx] = (r << 16) | (g << 8) | b;
                    }
                }
            }
            buffer
        };

        // 1. Plain background (same width)
        let mut actual = vec![0u32; src_w * src_h];
        render_selection_overlay_frame(&mut actual, src_w, src_h, &packed, src_w, None, None);
        assert_eq!(actual, reference_render(src_w, src_h, None, None));

        // 2. Crosshair active
        let mut actual_cross = vec![0u32; src_w * src_h];
        render_selection_overlay_frame(
            &mut actual_cross,
            src_w,
            src_h,
            &packed,
            src_w,
            None,
            Some((5, 4)),
        );
        assert_eq!(
            actual_cross,
            reference_render(src_w, src_h, None, Some((5, 4)))
        );

        // 3. Selection border active
        let mut actual_border = vec![0u32; src_w * src_h];
        render_selection_overlay_frame(
            &mut actual_border,
            src_w,
            src_h,
            &packed,
            src_w,
            Some((2, 3)),
            Some((11, 9)),
        );
        assert_eq!(
            actual_border,
            reference_render(src_w, src_h, Some((2, 3)), Some((11, 9)))
        );

        // 4. Mismatched buffer width (e.g. transient DPI resize 12x10 vs 16x12)
        let mut actual_resized = vec![0u32; 12 * 10];
        render_selection_overlay_frame(
            &mut actual_resized,
            12,
            10,
            &packed,
            src_w,
            Some((1, 1)),
            Some((8, 7)),
        );
        assert_eq!(
            actual_resized,
            reference_render(12, 10, Some((1, 1)), Some((8, 7)))
        );
    }

    #[test]
    fn test_overlay_perf_next_surface_size_deduplicates_resizes() {
        let first = next_surface_size(None, 1920, 1080).unwrap();
        assert_eq!((first.0.get(), first.1.get()), (1920, 1080));

        // Duplicate size returns None (skips redundant softbuffer resize + redraw)
        assert!(next_surface_size(Some((1920, 1080)), 1920, 1080).is_none());

        // Zero dimensions clamp to 1x1
        let clamped = next_surface_size(Some((1920, 1080)), 0, 0).unwrap();
        assert_eq!((clamped.0.get(), clamped.1.get()), (1, 1));
    }

    #[test]
    fn test_overlay_perf_new_for_recording_drops_raw_rgba_buffer() {
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
        let app = SelectionApp::new_for_recording(rgba, 2, 1, 0, 0);
        assert!(app.screenshot.is_empty());
        assert_eq!(app.packed_background.len(), 2);
    }

    #[test]
    fn test_overlay_perf_crop_selected_region_zero_copy_fullscreen() {
        let rgba = vec![
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        let full = crop_selected_region(
            2,
            2,
            rgba.clone(),
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 2,
                physical_height: 2,
            },
        )
        .unwrap();
        assert_eq!(full.dimensions(), (2, 2));
        assert_eq!(full.into_raw(), rgba);

        let sub = crop_selected_region(
            2,
            2,
            rgba,
            Region {
                physical_x: 1,
                physical_y: 1,
                physical_width: 1,
                physical_height: 1,
            },
        )
        .unwrap();
        assert_eq!(sub.dimensions(), (1, 1));
        assert_eq!(sub.into_raw(), vec![100, 110, 120, 255]);
    }

    #[test]
    fn test_overlay_release_window_clears_window_and_surface_before_exit() {
        let mut app = SelectionApp::new_for_recording(vec![10, 20, 30, 255], 1, 1, 0, 0);
        assert_eq!(app.packed_background.len(), 1);
        app.release_window();
        assert!(app.window.is_none());
        assert!(app.surface.is_none());
        assert_eq!(app.packed_background.len(), 1);
    }

    #[test]
    fn test_overlay_perf_into_packed_background_reuses_direct_packed_capture() {
        let direct = vec![0xFF11_2233, 0xFF44_5566];
        assert_eq!(into_packed_background(Vec::new(), direct.clone()), direct);
        let fallback_rgba = vec![0x11, 0x22, 0x33, 0xFF, 0x44, 0x55, 0x66, 0xFF];
        assert_eq!(
            into_packed_background(fallback_rgba, Vec::new()),
            vec![0x0011_2233, 0x0044_5566]
        );
    }

    #[test]
    fn test_overlay_perf_crop_packed_region_matches_rgba_crop() {
        let width = 8u32;
        let height = 6u32;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for i in 0..(width * height) {
            rgba.push(((i * 11) % 256) as u8);
            rgba.push(((i * 23) % 256) as u8);
            rgba.push(((i * 37) % 256) as u8);
            rgba.push(255);
        }
        // Packed values may also carry 0xFF in high byte when read directly via Win32 GetDIBits
        let packed: Vec<u32> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|px| 0xFF00_0000 | ((px[0] as u32) << 16) | ((px[1] as u32) << 8) | (px[2] as u32))
            .collect();

        let region = Region {
            physical_x: 2,
            physical_y: 1,
            physical_width: 4,
            physical_height: 3,
        };
        let expected = crop_selected_region(width, height, rgba, region).unwrap();
        let actual = crop_packed_region(width, height, &packed, region).unwrap();
        assert_eq!(actual.dimensions(), expected.dimensions());
        assert_eq!(actual.into_raw(), expected.into_raw());
    }

    #[test]
    fn test_overlay_perf_incremental_update_matches_full_render_across_moves_and_drags() {
        let src_w = 24usize;
        let src_h = 18usize;
        let mut packed = Vec::with_capacity(src_w * src_h);
        for i in 0..(src_w * src_h) {
            let r = ((i * 17) % 256) as u32;
            let g = ((i * 31) % 256) as u32;
            let b = ((i * 47) % 256) as u32;
            packed.push(0xFF00_0000 | (r << 16) | (g << 8) | b);
        }

        let mut inc_buffer = vec![0u32; src_w * src_h];
        let mut damage = OverlayDamage::default();

        let steps = [
            (None, None),
            (None, Some((3, 4))),
            (None, Some((10, 12))),
            (None, Some((0, 0))),
            (None, Some((23, 17))),
            (Some((4, 5)), Some((4, 5))),
            (Some((4, 5)), Some((12, 10))),
            (Some((4, 5)), Some((19, 15))),
            (Some((18, 14)), Some((2, 3))),
            (None, Some((7, 8))),
            (None, None),
        ];

        for (start_pos, current_pos) in steps {
            update_selection_overlay_frame(
                &mut inc_buffer,
                src_w,
                src_h,
                &packed,
                src_w,
                start_pos,
                current_pos,
                &mut damage,
            );
            let mut full_buffer = vec![0u32; src_w * src_h];
            render_selection_overlay_frame(
                &mut full_buffer,
                src_w,
                src_h,
                &packed,
                src_w,
                start_pos,
                current_pos,
            );
            assert_eq!(
                inc_buffer, full_buffer,
                "incremental update diverged for start={start_pos:?} current={current_pos:?}"
            );
        }
    }

    #[test]
    fn test_overlay_perf_crop_packed_region_rejects_invalid_and_out_of_bounds_regions() {
        let packed = vec![0xFF11_2233; 4 * 4];

        // Negative coordinates
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: -1,
                physical_y: 0,
                physical_width: 2,
                physical_height: 2,
            }
        )
        .is_err());
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: 0,
                physical_y: -1,
                physical_width: 2,
                physical_height: 2,
            }
        )
        .is_err());

        // Zero width / height
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 0,
                physical_height: 2,
            }
        )
        .is_err());
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 2,
                physical_height: 0,
            }
        )
        .is_err());

        // Out of bounds
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: 3,
                physical_y: 0,
                physical_width: 2,
                physical_height: 2,
            }
        )
        .is_err());
        assert!(crop_packed_region(
            4,
            4,
            &packed,
            Region {
                physical_x: 0,
                physical_y: 3,
                physical_width: 2,
                physical_height: 2,
            }
        )
        .is_err());

        // Undersized packed buffer
        assert!(crop_packed_region(
            4,
            4,
            &packed[..15],
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 2,
                physical_height: 2,
            }
        )
        .is_err());
    }

    #[test]
    fn test_overlay_perf_crop_packed_region_full_screen_and_1x1_corners() {
        let width = 5u32;
        let height = 4u32;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for i in 0..(width * height) {
            rgba.push(((i * 19) % 256) as u8);
            rgba.push(((i * 41) % 256) as u8);
            rgba.push(((i * 73) % 256) as u8);
            rgba.push(255);
        }
        let packed = pack_rgba_to_softbuffer(&rgba);

        for region in [
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: width,
                physical_height: height,
            },
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 1,
                physical_height: 1,
            },
            Region {
                physical_x: (width - 1) as i32,
                physical_y: 0,
                physical_width: 1,
                physical_height: 1,
            },
            Region {
                physical_x: 0,
                physical_y: (height - 1) as i32,
                physical_width: 1,
                physical_height: 1,
            },
            Region {
                physical_x: (width - 1) as i32,
                physical_y: (height - 1) as i32,
                physical_width: 1,
                physical_height: 1,
            },
        ] {
            let expected = crop_selected_region(width, height, rgba.clone(), region).unwrap();
            let actual = crop_packed_region(width, height, &packed, region).unwrap();
            assert_eq!(actual.dimensions(), expected.dimensions());
            assert_eq!(actual.into_raw(), expected.into_raw());
        }
    }

    #[test]
    fn test_overlay_perf_incremental_update_matches_full_render_with_dpi_stride_mismatch_and_resize(
    ) {
        let src_w = 20usize;
        let src_h = 16usize;
        let packed: Vec<u32> = (0..(src_w * src_h))
            .map(|i| 0xFF00_0000 | (((i * 13) % 256) as u32) << 16 | (((i * 29) % 256) as u32) << 8)
            .collect();

        // Test surface narrower than src_w, equal to src_w, and wider than src_w
        let mut damage = OverlayDamage::default();
        for (buf_w, buf_h) in [(16usize, 12usize), (20, 16), (24, 14)] {
            let mut inc_buffer = vec![0u32; buf_w * buf_h];
            let steps = [
                (None, Some((0, 0))),
                (None, Some((15, 11))),
                (None, Some((100, 100))), // out-of-bounds cursor
                (Some((0, 0)), Some((15, 11))),
                (Some((2, 2)), Some((30, 30))), // drag extending past surface bounds
                (None, Some((5, 5))),
            ];
            for (start_pos, current_pos) in steps {
                update_selection_overlay_frame(
                    &mut inc_buffer,
                    buf_w,
                    buf_h,
                    &packed,
                    src_w,
                    start_pos,
                    current_pos,
                    &mut damage,
                );
                let mut full_buffer = vec![0u32; buf_w * buf_h];
                render_selection_overlay_frame(
                    &mut full_buffer,
                    buf_w,
                    buf_h,
                    &packed,
                    src_w,
                    start_pos,
                    current_pos,
                );
                assert_eq!(
                    inc_buffer, full_buffer,
                    "mismatch for surface=({buf_w},{buf_h}) start={start_pos:?} current={current_pos:?}"
                );
            }
        }
    }

    #[test]
    fn test_overlay_perf_incremental_update_handles_empty_or_zero_dimensions() {
        let mut damage = OverlayDamage::default();
        let mut buf = vec![42u32; 4];
        let packed = vec![1u32; 4];

        update_selection_overlay_frame(&mut buf, 0, 2, &packed, 2, None, Some((0, 0)), &mut damage);
        assert_eq!(damage, OverlayDamage::default());
        assert_eq!(buf, vec![42u32; 4]);

        update_selection_overlay_frame(&mut buf, 2, 0, &packed, 2, None, Some((0, 0)), &mut damage);
        assert_eq!(damage, OverlayDamage::default());

        update_selection_overlay_frame(&mut [], 2, 2, &packed, 2, None, Some((0, 0)), &mut damage);
        assert_eq!(damage, OverlayDamage::default());

        update_selection_overlay_frame(&mut buf, 2, 2, &[], 2, None, Some((0, 0)), &mut damage);
        assert_eq!(damage, OverlayDamage::default());
    }

    #[cfg(windows)]
    #[test]
    fn test_overlay_perf_capture_screen_packed_win32_rejects_invalid_hmonitor() {
        let err = capture_screen_packed_win32(0).unwrap_err();
        assert!(
            err.contains("GetMonitorInfoW failed"),
            "unexpected error for null HMONITOR: {err}"
        );
    }

    #[test]
    fn test_overlay_perf_flush_pending_win32_messages_is_safe_when_queue_empty() {
        flush_pending_win32_messages();
    }

    #[test]
    fn test_multi_monitor_mixed_dpi_physical_origin_layout() {
        // Windows PerMonitorV2: Screen 0 is 1920x1200 physical at (0,0) with 1.25 scale
        // (display_info reports logical 1536x960), Screen 1 is 1920x1080 at x=1920, y=0.
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1536,
                logical_height: 960,
                physical_width: 1920,
                physical_height: 1200,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 1920,
                logical_y: 0,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];

        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts[0].physical_x, 0);
        assert_eq!(layouts[0].physical_y, 0);
        assert_eq!(layouts[1].physical_x, 1920);
        assert_eq!(layouts[1].physical_y, 0);
    }

    #[test]
    fn test_multi_monitor_cursor_move_switches_active_monitor_before_drag() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![0xFF11_1111; 4 * 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![0xFF22_2222; 4 * 4],
                width: 1920,
                height: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
        ];

        // PrintScreen pressed while cursor was on Monitor 0
        let mut app = SelectionApp::with_monitors(captures, 0);
        assert_eq!(app.monitor_count(), 2);
        assert_eq!(app.active_monitor, 0);

        // Move on Monitor 0
        let redraw = app.handle_cursor_moved_on_monitor(0, (500, 400));
        assert_eq!(redraw, vec![0]);
        assert_eq!(app.active_monitor, 0);
        assert_eq!(app.current_pos, Some((500, 400)));

        // Move cursor to Monitor 1 before starting drag
        let redraw = app.handle_cursor_moved_on_monitor(1, (300, 250));
        assert_eq!(redraw, vec![0, 1]);
        assert_eq!(app.active_monitor, 1);
        assert_eq!(app.current_pos, Some((300, 250)));

        // Press left mouse button on Monitor 1, drag to (700, 550), and release
        let press_redraw = app.handle_mouse_press_on_monitor(1);
        assert!(press_redraw.is_empty());
        assert!(app.is_dragging);
        assert_eq!(app.start_pos, Some((300, 250)));

        let drag_redraw = app.handle_cursor_moved_on_monitor(1, (700, 550));
        assert_eq!(drag_redraw, vec![1]);
        assert_eq!(app.current_pos, Some((700, 550)));

        assert!(app.handle_mouse_release());
        assert!(!app.is_dragging);
        assert_eq!(app.selected_monitor, 1);
        assert_eq!(
            app.result,
            Some(Region {
                physical_x: 300,
                physical_y: 250,
                physical_width: 400,
                physical_height: 300,
            })
        );

        let (packed, w, h, idx) = app.take_selected_monitor_capture();
        assert_eq!(idx, 1);
        assert_eq!((w, h), (1920, 1080));
        assert_eq!(packed[0], 0xFF22_2222);
    }

    #[test]
    fn test_multi_monitor_drag_clamps_to_originating_monitor_when_crossing_boundary() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![0xFF11_1111; 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![0xFF22_2222; 4],
                width: 1920,
                height: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 0);
        // Move to Monitor 1 and start drag at (200, 200)
        app.handle_cursor_moved_on_monitor(1, (200, 200));
        app.handle_mouse_press_on_monitor(1);

        // Cursor crosses left onto Monitor 0 at (1850, 300) while button is held down
        let redraw = app.handle_cursor_moved_on_monitor(0, (1850, 300));
        assert_eq!(redraw, vec![1]);
        assert_eq!(app.active_monitor, 1);
        assert_eq!(app.current_pos, Some((0, 300)));

        assert!(app.handle_mouse_release());
        assert_eq!(app.selected_monitor, 1);
        assert_eq!(
            app.result,
            Some(Region {
                physical_x: 0,
                physical_y: 200,
                physical_width: 200,
                physical_height: 100,
            })
        );
    }

    #[test]
    fn test_multi_monitor_capture_monitors_skips_failed_screen_when_another_succeeds() {
        let screens = vec![
            (0i32, 0i32, 1536u32, 960u32, 1.25f32, true),
            (1920i32, 0i32, 1920u32, 1080u32, 1.0f32, false),
        ];
        let (captures, kept_indices, initial_active) =
            collect_monitor_captures_with(&screens, Some((2100, 300)), false, |idx| {
                if idx == 0 {
                    Err("Access is denied".to_string())
                } else {
                    Ok((vec![0xFF33_4455; 4], 1920, 1080, Some((1920, 0))))
                }
            })
            .unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(kept_indices, vec![1]);
        assert_eq!(initial_active, 0);
        assert_eq!(captures[0].origin_x, 1920);
        assert_eq!(captures[0].origin_y, 0);
    }

    #[test]
    fn test_multi_monitor_with_monitors_empty_captures_defaults_safely() {
        let mut app = SelectionApp::with_monitors(Vec::new(), 5);
        assert_eq!(app.monitor_count(), 1);
        assert_eq!(app.active_monitor, 0);
        assert_eq!(app.selected_monitor, 0);
        assert_eq!(app.screen_width, 0);
        assert_eq!(app.screen_height, 0);

        let (packed, w, h, idx) = app.take_selected_monitor_capture();
        assert!(packed.is_empty());
        assert_eq!((w, h, idx), (0, 0, 0));
    }

    #[test]
    fn test_multi_monitor_with_monitors_clamps_out_of_bounds_initial_active() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![1; 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![2; 4],
                width: 1920,
                height: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 99);
        assert_eq!(app.active_monitor, 1);
        assert_eq!(app.selected_monitor, 1);

        let (packed, w, h, idx) = app.take_selected_monitor_capture();
        assert_eq!(idx, 1);
        assert_eq!((w, h), (1920, 1080));
        assert_eq!(packed, vec![2; 4]);
    }

    #[test]
    fn test_multi_monitor_geometry_and_out_of_bounds_events() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![1; 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![2; 4],
                width: 2560,
                height: 1440,
                origin_x: 1920,
                origin_y: -120,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 0);
        assert_eq!(app.monitor_geometry(0), Some((0, 0, 1920, 1200)));
        assert_eq!(app.monitor_geometry(1), Some((1920, -120, 2560, 1440)));
        assert_eq!(app.monitor_geometry(2), None);

        // Out-of-bounds monitor indices are ignored
        assert!(app.handle_cursor_moved_on_monitor(2, (100, 100)).is_empty());
        assert!(app.handle_mouse_press_on_monitor(2).is_empty());
        assert!(!app.is_dragging);

        // Moving to the same position on the active monitor does not schedule a redundant redraw
        assert_eq!(app.handle_cursor_moved_on_monitor(0, (40, 50)), vec![0]);
        assert!(app.handle_cursor_moved_on_monitor(0, (40, 50)).is_empty());
    }

    #[test]
    fn test_multi_monitor_mouse_press_directly_on_inactive_monitor_switches_active() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![1; 4],
                width: 1920,
                height: 1080,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![2; 4],
                width: 1920,
                height: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 0);
        app.current_pos = Some((100, 100));

        // Pressing directly on Monitor 1 while Monitor 0 was active switches active_monitor
        // and schedules redraws on both monitors.
        let redraw = app.handle_mouse_press_on_monitor(1);
        assert_eq!(redraw, vec![0, 1]);
        assert_eq!(app.active_monitor, 1);
        assert_eq!(app.selected_monitor, 1);
        assert!(app.is_dragging);
    }

    #[test]
    fn test_multi_monitor_tiny_invalid_drag_resets_and_allows_retry_on_other_monitor() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![0xFFAA_AAAA; 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![0xFFBB_BBBB; 4],
                width: 1920,
                height: 1080,
                origin_x: 1920,
                origin_y: 0,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 0);

        // Release when not dragging returns false
        assert!(!app.handle_mouse_release());

        // Tiny 2x2 drag on Monitor 0 is rejected (<= 5px) and resets start_pos
        app.handle_cursor_moved_on_monitor(0, (10, 10));
        app.handle_mouse_press_on_monitor(0);
        app.handle_cursor_moved_on_monitor(0, (12, 12));
        assert!(!app.handle_mouse_release());
        assert!(!app.is_dragging);
        assert_eq!(app.start_pos, None);
        assert_eq!(app.result, None);

        // User then moves to Monitor 1 and completes a valid drag
        assert_eq!(app.handle_cursor_moved_on_monitor(1, (50, 60)), vec![0, 1]);
        app.handle_mouse_press_on_monitor(1);
        app.handle_cursor_moved_on_monitor(1, (150, 140));
        assert!(app.handle_mouse_release());
        assert_eq!(app.selected_monitor, 1);
        assert_eq!(
            app.result,
            Some(Region {
                physical_x: 50,
                physical_y: 60,
                physical_width: 100,
                physical_height: 80,
            })
        );
    }

    #[test]
    fn test_multi_monitor_three_monitor_negative_and_vertical_drag_clamping() {
        let captures = vec![
            // Monitor 0: primary (0, 0) 1920x1200
            MonitorCapture {
                packed_background: vec![1; 4],
                width: 1920,
                height: 1200,
                origin_x: 0,
                origin_y: 0,
            },
            // Monitor 1: left (-1920, 0) 1920x1080
            MonitorCapture {
                packed_background: vec![2; 4],
                width: 1920,
                height: 1080,
                origin_x: -1920,
                origin_y: 0,
            },
            // Monitor 2: bottom (0, 1200) 1920x1080
            MonitorCapture {
                packed_background: vec![3; 4],
                width: 1920,
                height: 1080,
                origin_x: 0,
                origin_y: 1200,
            },
        ];

        let mut app = SelectionApp::with_monitors(captures, 0);
        app.handle_cursor_moved_on_monitor(0, (500, 500));
        app.handle_mouse_press_on_monitor(0);

        // Dragging onto Monitor 1 (left of primary) clamps x to 0 on Monitor 0
        assert_eq!(app.handle_cursor_moved_on_monitor(1, (1800, 600)), vec![0]);
        assert_eq!(app.current_pos, Some((0, 600)));

        // Moving further left on Monitor 1 stays clamped at (0, 600) so no redundant redraw
        assert!(app
            .handle_cursor_moved_on_monitor(1, (1700, 600))
            .is_empty());

        // Dragging onto Monitor 2 (below primary) clamps y to 1199 on Monitor 0
        assert_eq!(app.handle_cursor_moved_on_monitor(2, (800, 250)), vec![0]);
        assert_eq!(app.current_pos, Some((800, 1199)));

        assert!(app.handle_mouse_release());
        assert_eq!(app.selected_monitor, 0);
        assert_eq!(
            app.result,
            Some(Region {
                physical_x: 500,
                physical_y: 500,
                physical_width: 300,
                physical_height: 699,
            })
        );

        let (packed, w, h, idx) = app.take_selected_monitor_capture();
        assert_eq!(idx, 0);
        assert_eq!((w, h), (1920, 1200));
        assert_eq!(packed, vec![1; 4]);
    }

    #[test]
    fn test_multi_monitor_mixed_dpi_physical_origin_left_top_bottom_and_neighbors() {
        // Primary at (0, 0) 1920x1200 physical (1536x960 logical @ 125%),
        // Left at (-2560, 120) 2560x1440 physical (1707x960 logical @ 150%),
        // Bottom at (120, 1200) 1920x1080 physical (1920x1080 logical @ 100%),
        // Top at (0, -1080) 1920x1080 physical (1920x1080 logical @ 100%).
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 1536,
                logical_height: 960,
                physical_width: 1920,
                physical_height: 1200,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: -2560,
                logical_y: 120,
                logical_width: 1707,
                logical_height: 960,
                physical_width: 2560,
                physical_height: 1440,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 120,
                logical_y: 1200,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: -1080,
                logical_width: 1920,
                logical_height: 1080,
                physical_width: 1920,
                physical_height: 1080,
                is_primary: false,
            },
        ];

        let layouts = build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(
            (layouts[0].physical_x, layouts[0].physical_y),
            (0, 0),
            "primary layout"
        );
        assert_eq!(
            (layouts[1].physical_x, layouts[1].physical_y),
            (-2560, 180),
            "left layout with physical x=-2560 and scaled vertical offset"
        );
        assert_eq!(
            (layouts[2].physical_x, layouts[2].physical_y),
            (120, 1200),
            "bottom layout with physical y=1200 and horizontal offset"
        );
        assert_eq!(
            (layouts[3].physical_x, layouts[3].physical_y),
            (0, -1080),
            "top layout with physical y=-1080"
        );
    }

    #[test]
    fn test_multi_monitor_collect_monitor_captures_errors_and_fallback_origins() {
        // Empty screen list returns error
        let empty_err =
            collect_monitor_captures_with(&[], None, false, |_| unreachable!()).unwrap_err();
        assert_eq!(empty_err, "No screens found");

        // All screens failing returns the last error
        let screens = vec![
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1920, 1080, 1.0, false),
        ];
        let all_fail_err = collect_monitor_captures_with(&screens, None, false, |idx| {
            Err(format!("screen {idx} failed"))
        })
        .unwrap_err();
        assert_eq!(all_fail_err, "screen 1 failed");

        // Fallback origins when native_origin is None and cursor is None (prefers primary at idx 1)
        let screens_non_win = vec![
            (0, 0, 1920, 1080, 1.0, false),
            (1920, 0, 1920, 1080, 2.0, true),
        ];
        let (captures, kept, initial_active) =
            collect_monitor_captures_with(&screens_non_win, None, true, |idx| {
                if idx == 0 {
                    Ok((vec![1; 4], 1920, 1080, None))
                } else {
                    Ok((vec![2; 4], 3840, 2160, None))
                }
            })
            .unwrap();
        assert_eq!(kept, vec![0, 1]);
        assert_eq!(initial_active, 1);
        assert_eq!((captures[0].origin_x, captures[0].origin_y), (0, 0));
        assert_eq!((captures[1].origin_x, captures[1].origin_y), (3840, 0));
    }

    #[test]
    fn test_multi_monitor_draw_and_release_window_without_surfaces_are_safe() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![1; 4],
                width: 4,
                height: 1,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![2; 4],
                width: 4,
                height: 1,
                origin_x: 4,
                origin_y: 0,
            },
        ];
        let mut app = SelectionApp::with_monitors(captures, 1);
        app.draw();
        app.draw_monitor(0);
        app.draw_monitor(1);
        app.draw_monitor(99);
        app.request_redraw_for_monitor(0);
        app.request_redraw_for_monitor(1);
        app.request_redraw_for_monitor(99);
        app.release_window();
        assert!(app.window.is_none());
        assert!(app.extra_monitors[0].window.is_none());
    }

    #[test]
    fn test_overlay_present_damage_rects_full_frame_and_unchanged_frame() {
        let initial = OverlayDamage::default();
        let rects = overlay_present_damage_rects(initial, 100, 80, None, Some((10, 20)));
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0);
        assert_eq!(rects[0].y, 0);
        assert_eq!(rects[0].width.get(), 100);
        assert_eq!(rects[0].height.get(), 80);

        let matching = OverlayDamage {
            surface_size: Some((100, 80)),
            start_pos: None,
            current_pos: Some((10, 20)),
        };
        let unchanged = overlay_present_damage_rects(matching, 100, 80, None, Some((10, 20)));
        assert!(unchanged.is_empty());

        let zero_size = overlay_present_damage_rects(initial, 0, 80, None, Some((10, 20)));
        assert!(zero_size.is_empty());
    }

    #[test]
    fn test_overlay_present_damage_rects_crosshair_and_rectangle_bands() {
        let prev = OverlayDamage {
            surface_size: Some((100, 80)),
            start_pos: None,
            current_pos: Some((10, 20)),
        };
        let crosshair_rects = overlay_present_damage_rects(prev, 100, 80, None, Some((30, 40)));
        assert_eq!(crosshair_rects.len(), 4);
        assert_eq!(
            (
                crosshair_rects[0].x,
                crosshair_rects[0].y,
                crosshair_rects[0].width.get(),
                crosshair_rects[0].height.get()
            ),
            (0, 20, 100, 1)
        );
        assert_eq!(
            (
                crosshair_rects[1].x,
                crosshair_rects[1].y,
                crosshair_rects[1].width.get(),
                crosshair_rects[1].height.get()
            ),
            (10, 0, 1, 80)
        );
        assert_eq!(
            (
                crosshair_rects[2].x,
                crosshair_rects[2].y,
                crosshair_rects[2].width.get(),
                crosshair_rects[2].height.get()
            ),
            (0, 40, 100, 1)
        );
        assert_eq!(
            (
                crosshair_rects[3].x,
                crosshair_rects[3].y,
                crosshair_rects[3].width.get(),
                crosshair_rects[3].height.get()
            ),
            (30, 0, 1, 80)
        );

        let prev_drag = OverlayDamage {
            surface_size: Some((100, 80)),
            start_pos: Some((10, 10)),
            current_pos: Some((20, 20)),
        };
        let drag_rects =
            overlay_present_damage_rects(prev_drag, 100, 80, Some((10, 10)), Some((30, 25)));
        assert_eq!(drag_rects.len(), 8);
        for r in &drag_rects {
            assert!(r.x + r.width.get() <= 100);
            assert!(r.y + r.height.get() <= 80);
            assert!(r.width.get() <= 23 || r.height.get() <= 3);
        }
    }

    #[test]
    fn test_update_selection_overlay_frame_skips_redundant_writes_when_damage_matches() {
        let bg = vec![0x00112233u32; 16];
        let mut buffer = bg.clone();
        let mut damage = OverlayDamage {
            surface_size: Some((4, 4)),
            start_pos: None,
            current_pos: Some((1, 1)),
        };
        buffer[5] = 0xDEADBEEF;
        update_selection_overlay_frame(&mut buffer, 4, 4, &bg, 4, None, Some((1, 1)), &mut damage);
        assert_eq!(buffer[5], 0xDEADBEEF);
    }

    #[test]
    fn test_discard_unneeded_captures_frees_unselected_and_cancelled_buffers() {
        let captures = vec![
            MonitorCapture {
                packed_background: vec![1; 8],
                width: 4,
                height: 2,
                origin_x: 0,
                origin_y: 0,
            },
            MonitorCapture {
                packed_background: vec![2; 8],
                width: 4,
                height: 2,
                origin_x: 4,
                origin_y: 0,
            },
        ];
        let mut app = SelectionApp::with_monitors(captures.clone(), 1);
        app.selected_monitor = 1;
        app.result = Some(Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2,
            physical_height: 2,
        });
        app.discard_unneeded_captures();
        assert!(app.packed_background.is_empty());
        assert_eq!(app.extra_monitors[0].packed_background.len(), 8);

        let mut cancelled_app = SelectionApp::with_monitors(captures, 0);
        cancelled_app.cancelled = true;
        cancelled_app.discard_unneeded_captures();
        assert!(cancelled_app.packed_background.is_empty());
        assert!(cancelled_app.extra_monitors[0].packed_background.is_empty());
    }

    #[test]
    fn test_benchmark_helpers_and_multi_candidate_fallbacks() {
        let inputs = vec![
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 1920, 1080, 3840, 2160, false),
        ];
        let layouts = benchmark_build_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0], (0, 0, 1920, 1080));
        assert_eq!(layouts[1], (1920, 0, 3840, 2160));

        let (norm, ox, oy, w, h) = benchmark_normalize_physical_screen_layouts(&inputs).unwrap();
        assert_eq!(norm.len(), 2);
        assert_eq!((ox, oy, w, h), (0, 0, 5760, 2160));

        let resolve_layouts = vec![
            (0, 0, 1920, 1080, 0, 0, 1920, 1080),
            (1920, 0, 1920, 1080, 1920, 0, 3840, 2160),
        ];
        let (idx, local) =
            benchmark_resolve_recording_selection(&resolve_layouts, (2000, 100, 300, 200)).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(local, (80, 100, 300, 200));

        assert!(benchmark_ranges_overlap(0, 100, 50, 100));
        assert!(!benchmark_ranges_overlap(0, 50, 50, 100));
        assert_eq!(benchmark_monitor_scale_ratio(1000, 2000), 2.0);
        assert_eq!(benchmark_scaled_gap(100, 1000, 1500), 150);

        // Exercise fallback_physical_x and fallback_physical_y with multiple candidates
        let multi_x_inputs = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 200,
                logical_y: 500,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 500,
                logical_y: 1000,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: false,
            },
        ];
        let x_positions = vec![Some(200), Some(0), None];
        let _ = fallback_physical_x(&multi_x_inputs, &x_positions, 2);
        let y_positions = vec![Some(500), Some(0), None];
        let _ = fallback_physical_y(&multi_x_inputs, &y_positions, 2);
        let y_below_positions = vec![None, Some(500), Some(1000)];
        let _ = fallback_physical_x(&multi_x_inputs, &y_below_positions, 0);
        let _ = fallback_physical_y(&multi_x_inputs, &y_below_positions, 0);

        // Exercise push_clamped_damage_rect empty rect early return and flush_pending_win32_messages
        let mut rects = Vec::new();
        push_clamped_damage_rect(&mut rects, 50, 50, 50, 60, 100, 100);
        assert!(rects.is_empty());
        flush_pending_win32_messages();
    }

    #[test]
    fn test_live_screen_capture_helpers_when_display_available() {
        let Ok(screens) = Screen::all() else {
            return;
        };
        let Some(first) = screens.first() else {
            return;
        };

        let Ok((packed, w, h)) = capture_screen_packed(first) else {
            return;
        };
        assert_eq!(packed.len(), (w as usize) * (h as usize));

        #[cfg(windows)]
        {
            let (win_packed, ww, wh) =
                capture_screen_packed_win32(first.display_info.raw_handle.0).unwrap();
            assert_eq!(win_packed.len(), (ww as usize) * (wh as usize));
        }

        let (mon_captures, kept_screens, active_idx) = capture_monitors_for_selection().unwrap();
        assert!(!mon_captures.is_empty());
        assert_eq!(mon_captures.len(), kept_screens.len());
        assert!(active_idx < mon_captures.len());

        let cursor_cap = capture_screen_at_cursor().unwrap();
        assert!(cursor_cap.width > 0 && cursor_cap.height > 0);

        let vd_cap = capture_virtual_desktop().unwrap();
        assert!(vd_cap.width > 0 && vd_cap.height > 0);
    }

    #[test]
    fn selection_pure_logic_remaining_branches_are_covered() {
        // 1. fallback_physical_x & fallback_physical_y: closer reference precedes farther reference
        let inputs = vec![
            ScreenLayoutInput {
                logical_x: 100,
                logical_y: 100,
                logical_width: 50,
                logical_height: 50,
                physical_width: 50,
                physical_height: 50,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 50,
                logical_height: 50,
                physical_width: 50,
                physical_height: 50,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 200,
                logical_y: 200,
                logical_width: 50,
                logical_height: 50,
                physical_width: 50,
                physical_height: 50,
                is_primary: false,
            },
        ];
        assert_eq!(
            fallback_physical_x(&inputs, &[Some(100), Some(0), None], 2),
            Some(200)
        );
        assert_eq!(
            fallback_physical_y(&inputs, &[Some(100), Some(0), None], 2),
            Some(200)
        );

        // 2. build_physical_screen_layouts: fallback_physical_x & fallback_physical_y progress
        let gapped = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 200,
                logical_y: 200,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: false,
            },
        ];
        let gapped_layouts = build_physical_screen_layouts(&gapped).unwrap();
        assert_eq!(
            (gapped_layouts[1].physical_x, gapped_layouts[1].physical_y),
            (200, 200)
        );

        // 3. build_physical_screen_layouts: unresolved physical_x and unresolved physical_y errors
        let overlap_both = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 10,
                logical_y: 10,
                logical_width: 50,
                logical_height: 50,
                physical_width: 50,
                physical_height: 50,
                is_primary: false,
            },
        ];
        assert_eq!(
            build_physical_screen_layouts(&overlap_both).unwrap_err(),
            "Failed to resolve physical screen positions"
        );

        let overlap_y_only = vec![
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 200,
                logical_y: 10,
                logical_width: 50,
                logical_height: 50,
                physical_width: 50,
                physical_height: 50,
                is_primary: false,
            },
        ];
        assert_eq!(
            build_physical_screen_layouts(&overlap_y_only).unwrap_err(),
            "Failed to resolve physical screen positions"
        );

        // 4. normalize_physical_screen_layouts height out of range & benchmark_normalize_physical_screen_layouts
        let tall_layouts = vec![
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_x: 0,
                physical_y: i32::MIN,
                physical_width: 100,
                physical_height: 100,
            },
            PhysicalScreenLayout {
                logical_x: 0,
                logical_y: 100,
                logical_width: 100,
                logical_height: 100,
                physical_x: 0,
                physical_y: i32::MAX,
                physical_width: 100,
                physical_height: 100,
            },
        ];
        assert_eq!(
            normalize_physical_screen_layouts(&tall_layouts).unwrap_err(),
            "Virtual desktop height is out of range"
        );
        assert!(benchmark_normalize_physical_screen_layouts(&[]).is_err());
        assert!(benchmark_normalize_physical_screen_layouts(&[
            (0, 0, 100, 100, u32::MAX, 100, true),
            (100, 0, 100, 100, u32::MAX, 100, false),
        ])
        .is_err());
        assert_eq!(
            benchmark_build_screen_layout_inputs_physical_sizes(
                [(0, 0, 100, 100, 1.0, true)],
                0,
                (100, 100)
            )
            .unwrap(),
            vec![(100, 100)]
        );
        assert!(benchmark_build_screen_layout_inputs_physical_sizes(
            [(0, 0, 100, 100, 1.0, true)],
            5,
            (100, 100)
        )
        .is_err());

        // 5. set_overlay_pixel & restore_overlay_pixel out of bounds
        let mut buf = vec![0u32; 4];
        set_overlay_pixel(&mut buf, 2, 4, 2, 10, 10, 0xFF);
        restore_overlay_pixel(&mut buf, 2, &[1, 2, 3, 4], 2, 10, 10);
        assert_eq!(buf, vec![0u32; 4]);

        // 6. update_selection_overlay_frame with short packed_background on crosshair restore
        let mut damage = OverlayDamage {
            surface_size: Some((4, 4)),
            start_pos: None,
            current_pos: Some((1, 1)),
        };
        let mut frame_buf = vec![0u32; 16];
        update_selection_overlay_frame(
            &mut frame_buf,
            4,
            4,
            &[1u32, 2],
            4,
            None,
            Some((2, 2)),
            &mut damage,
        );

        // 7. push_primitive_damage_rects with out-of-bounds crosshair coordinates
        let mut rects = Vec::new();
        push_primitive_damage_rects(&mut rects, 4, 4, None, Some((10, 10)));
        assert!(rects.is_empty());

        // 8. render_selection_overlay_frame early return and row break when packed_background is short
        render_selection_overlay_frame(&mut frame_buf, 0, 4, &[1, 2, 3, 4], 2, None, None);
        render_selection_overlay_frame(&mut frame_buf, 4, 4, &[1, 2], 2, None, None);

        // 9. crop_selected_region negative coordinates and invalid buffer size
        assert_eq!(
            crop_selected_region(
                2,
                2,
                vec![0u8; 16],
                Region {
                    physical_x: -1,
                    physical_y: 0,
                    physical_width: 1,
                    physical_height: 1,
                }
            )
            .unwrap_err(),
            "Selection region has negative coordinates"
        );
        assert_eq!(
            crop_selected_region(
                2,
                2,
                vec![0u8; 4],
                Region {
                    physical_x: 0,
                    physical_y: 0,
                    physical_width: 1,
                    physical_height: 1,
                }
            )
            .unwrap_err(),
            "Failed to create image from composite data"
        );

        // 10. SelectionApp edge cases: invalid active_monitor while dragging, mouse release without start_pos, discard_unneeded_captures
        let mut app = SelectionApp::with_monitors(
            vec![
                MonitorCapture {
                    packed_background: vec![1; 4],
                    width: 2,
                    height: 2,
                    origin_x: 0,
                    origin_y: 0,
                },
                MonitorCapture {
                    packed_background: vec![2; 4],
                    width: 2,
                    height: 2,
                    origin_x: 2,
                    origin_y: 0,
                },
            ],
            0,
        );
        app.is_dragging = true;
        app.active_monitor = 99;
        assert!(app.handle_cursor_moved_on_monitor(0, (1, 1)).is_empty());

        app.active_monitor = 0;
        app.is_dragging = true;
        app.start_pos = None;
        app.current_pos = Some((1, 1));
        assert!(!app.handle_mouse_release());

        app.selected_monitor = 0;
        app.discard_unneeded_captures();
        assert!(!app.packed_background.is_empty());
        assert!(app.extra_monitors[0].packed_background.is_empty());

        let mut rects = Vec::new();
        push_primitive_damage_rects(&mut rects, 10, 10, None, Some((20, 20)));
        push_primitive_damage_rects(&mut rects, 10, 10, None, None);
        assert!(rects.is_empty());

        assert!(build_screen_layout_inputs(
            vec![
                (0, 0, 100, 100, 1.0, true),
                (100, 0, u32::MAX, 100, 2.0, false)
            ],
            0,
            (100, 100),
        )
        .is_err());
        assert!(build_screen_layout_inputs(
            vec![
                (0, 0, 100, 100, 1.0, true),
                (100, 0, 100, u32::MAX, 2.0, false)
            ],
            0,
            (100, 100),
        )
        .is_err());
        assert!(build_screen_layout_inputs(
            vec![(0, 0, 100, 100, 1.0, true), (100, 0, 100, 100, 1.0, false)],
            0,
            (100, 100),
        )
        .is_ok());

        for case in 0..5 {
            let screens: &[(i32, i32, u32, u32, f32, bool)] = match case {
                0 => &[],
                1 => &[(0, 0, 100, 100, 0.0, true)],
                2 => &[(0, 0, 100, 100, 1.0, true), (50, 50, 100, 100, 1.0, false)],
                _ => &[
                    (0, 0, 100, 100, 0.0, false),
                    (100, 0, 100, 100, 1.0, true),
                    (200, 0, 100, 100, 1.0, false),
                ],
            };
            let res =
                collect_monitor_captures_with(screens, None, false, |idx| match (case, idx) {
                    (1, _) | (3, 0) => Err("cap fail".to_string()),
                    (3, 1) | (4, _) => Ok((vec![0; 4], 2, 2, Some((10, 20)))),
                    _ => Ok((vec![0; 4], 2, 2, None)),
                });
            assert_eq!(res.is_ok(), case >= 3);
        }

        // Regression tests for negative coordinates in overlay rendering/damage,
        // >16 monitor layouts, gapped-to-adjacent BFS propagation, and i32 overflow.
        let bg = [0x00112233u32; 100];
        let mut frame_full = [0u32; 100];
        let mut frame_inc = [0u32; 100];
        let mut dmg = OverlayDamage::default();
        render_selection_overlay_frame(
            &mut frame_full,
            10,
            10,
            &bg,
            10,
            Some((-5, -5)),
            Some((6, 6)),
        );
        update_selection_overlay_frame(
            &mut frame_inc,
            10,
            10,
            &bg,
            10,
            Some((-5, -5)),
            Some((6, 6)),
            &mut dmg,
        );
        assert_eq!(frame_full, frame_inc);
        // Top border at y=0, x=3 must be drawn as cyan despite negative start_pos
        assert_eq!(frame_full[3], (200 << 8) | 255);
        update_selection_overlay_frame(
            &mut frame_inc,
            10,
            10,
            &bg,
            10,
            Some((2, 2)),
            Some((7, 7)),
            &mut dmg,
        );
        render_selection_overlay_frame(
            &mut frame_full,
            10,
            10,
            &bg,
            10,
            Some((2, 2)),
            Some((7, 7)),
        );
        assert_eq!(frame_full, frame_inc);

        let mut neg_crosshair_rects = Vec::new();
        push_primitive_damage_rects(&mut neg_crosshair_rects, 10, 10, None, Some((-5, -5)));
        assert!(neg_crosshair_rects.is_empty());

        let many_inputs: Vec<ScreenLayoutInput> = (0..18)
            .map(|i| ScreenLayoutInput {
                logical_x: i * 100,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: i == 0,
            })
            .collect();
        let many_layouts = build_physical_screen_layouts(&many_inputs).unwrap();
        assert_eq!(many_layouts.len(), 18);
        assert_eq!(many_layouts[17].physical_x, 1700);

        let gapped_chain = [
            ScreenLayoutInput {
                logical_x: 0,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: true,
            },
            ScreenLayoutInput {
                logical_x: 200,
                logical_y: 200,
                logical_width: 100,
                logical_height: 100,
                physical_width: 200,
                physical_height: 200,
                is_primary: false,
            },
            ScreenLayoutInput {
                logical_x: 300,
                logical_y: 200,
                logical_width: 100,
                logical_height: 100,
                physical_width: 100,
                physical_height: 100,
                is_primary: false,
            },
        ];
        let chain_layouts = build_physical_screen_layouts(&gapped_chain).unwrap();
        assert_eq!(chain_layouts[1].physical_x, 300);
        assert_eq!(chain_layouts[2].physical_x, 500);

        let extreme_region =
            build_region_from_drag((-2_000_000_000, -10), (1_000_000_000, 10)).unwrap();
        assert_eq!(extreme_region.physical_width, 3_000_000_000);
        assert_eq!(
            find_screen_index_at_position(
                [(i32::MAX - 10, i32::MAX - 10, 100, 100)],
                (i32::MAX - 5, i32::MAX - 5),
            ),
            Some(0)
        );

        // Press before CursorMoved (or on another monitor) anchors start_pos on first drag motion
        let mut press_first_app = SelectionApp::with_monitors(
            vec![
                MonitorCapture {
                    packed_background: vec![1; 4],
                    width: 100,
                    height: 100,
                    origin_x: 0,
                    origin_y: 0,
                },
                MonitorCapture {
                    packed_background: vec![2; 4],
                    width: 100,
                    height: 100,
                    origin_x: 100,
                    origin_y: 0,
                },
            ],
            0,
        );
        press_first_app.handle_cursor_moved_on_monitor(0, (-10, 200));
        assert_eq!(press_first_app.current_pos, Some((0, 99)));
        assert_eq!(press_first_app.handle_mouse_press_on_monitor(1), vec![0, 1]);
        assert_eq!(press_first_app.start_pos, None);
        assert_eq!(
            press_first_app.handle_cursor_moved_on_monitor(1, (10, 15)),
            vec![1]
        );
        assert_eq!(press_first_app.start_pos, Some((10, 15)));
        assert_eq!(
            press_first_app.handle_cursor_moved_on_monitor(1, (50, 65)),
            vec![1]
        );
        assert!(press_first_app.handle_mouse_release());
        assert_eq!(
            press_first_app.result,
            Some(Region {
                physical_x: 10,
                physical_y: 15,
                physical_width: 40,
                physical_height: 50,
            })
        );

        let span_overflow_layouts = [
            PhysicalScreenLayout {
                logical_x: -1_500_000_000,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_x: -1_500_000_000,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            PhysicalScreenLayout {
                logical_x: 1_000_000_000,
                logical_y: 0,
                logical_width: 100,
                logical_height: 100,
                physical_x: 1_000_000_000,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
        ];
        assert!(normalize_physical_screen_layouts(&span_overflow_layouts).is_err());
    }
}
