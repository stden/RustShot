//! Multi-monitor layout resolution: logical-to-physical screen placement.

use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct ScreenLayoutInput {
    pub(super) logical_x: i32,
    pub(super) logical_y: i32,
    pub(super) logical_width: u32,
    pub(super) logical_height: u32,
    pub(super) physical_width: u32,
    pub(super) physical_height: u32,
    pub(super) is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhysicalScreenLayout {
    pub(super) logical_x: i32,
    pub(super) logical_y: i32,
    pub(super) logical_width: u32,
    pub(super) logical_height: u32,
    pub(super) physical_x: i32,
    pub(super) physical_y: i32,
    pub(super) physical_width: u32,
    pub(super) physical_height: u32,
}

pub(super) fn infer_physical_x(
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

    if reference_right == i64::from(target.logical_x) {
        Some(reference_x + reference.physical_width as i32)
    } else if target_right == i64::from(reference.logical_x) {
        Some(reference_x - target.physical_width as i32)
    } else {
        None
    }
}

pub(super) fn infer_physical_y(
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

    if reference_bottom == i64::from(target.logical_y) {
        Some(reference_y + reference.physical_height as i32)
    } else if target_bottom == i64::from(reference.logical_y) {
        Some(reference_y - target.physical_height as i32)
    } else {
        None
    }
}

/// Infer the physical Y of a target from a horizontally adjacent reference.
pub(super) fn infer_physical_y_from_horizontal_neighbor(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_y: i32,
) -> Option<i32> {
    let reference_right = logical_end(reference.logical_x, reference.logical_width);
    let target_right = logical_end(target.logical_x, target.logical_width);

    let is_adjacent = reference_right == i64::from(target.logical_x)
        || target_right == i64::from(reference.logical_x);

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
pub(super) fn infer_physical_x_from_vertical_neighbor(
    reference: &ScreenLayoutInput,
    target: &ScreenLayoutInput,
    reference_x: i32,
) -> Option<i32> {
    let reference_bottom = logical_end(reference.logical_y, reference.logical_height);
    let target_bottom = logical_end(target.logical_y, target.logical_height);

    let is_adjacent = reference_bottom == i64::from(target.logical_y)
        || target_bottom == i64::from(reference.logical_y);

    if !is_adjacent {
        return None;
    }

    let logical_x_diff = target.logical_x - reference.logical_x;
    Some(reference_x + scaled_gap(logical_x_diff, target.logical_width, target.physical_width))
}

pub(super) fn fallback_physical_x(
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

pub(super) fn fallback_physical_y(
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

/// Map every screen from logical (`display_info`) space into physical space.
///
/// There is deliberately **no single global scale factor**. `display_info`
/// reports logical coordinates while capture buffers are physical, and on a
/// mixed-DPI setup one shared scale misplaces every non-primary monitor.
///
/// Instead the primary monitor is anchored at physical `(0, 0)` and each
/// neighbour's physical origin is inferred from *its own* physical size via
/// logical adjacency (`infer_physical_x/y_from_*_neighbor`, falling back to
/// `fallback_physical_x/y`). Because a screen can only be placed once one of
/// its neighbours is placed, the resolver loops until a pass makes no further
/// progress, and errors out if a screen is left disconnected from the anchor.
pub(super) fn build_physical_screen_layouts(
    inputs: &[ScreenLayoutInput],
) -> Result<Vec<PhysicalScreenLayout>, String> {
    if inputs.is_empty() {
        return Err("No screens found".to_string());
    }

    let anchor_index = inputs
        .iter()
        .position(|screen| screen.is_primary)
        .unwrap_or(0);
    let mut physical_x = vec![None; inputs.len()];
    let mut physical_y = vec![None; inputs.len()];
    physical_x[anchor_index] = Some(0);
    physical_y[anchor_index] = Some(0);

    loop {
        let mut progress = false;

        for (reference_index, reference) in inputs.iter().enumerate() {
            let (Some(reference_x), Some(reference_y)) =
                (physical_x[reference_index], physical_y[reference_index])
            else {
                continue;
            };

            for (target_index, target) in inputs.iter().enumerate() {
                if reference_index == target_index {
                    continue;
                }

                if physical_x[target_index].is_none() {
                    if let Some(candidate) = infer_physical_x(reference, target, reference_x)
                        .or_else(|| {
                            infer_physical_x_from_vertical_neighbor(reference, target, reference_x)
                        })
                    {
                        physical_x[target_index] = Some(candidate);
                        progress = true;
                    }
                }

                if physical_y[target_index].is_none() {
                    if let Some(candidate) = infer_physical_y(reference, target, reference_y)
                        .or_else(|| {
                            infer_physical_y_from_horizontal_neighbor(
                                reference,
                                target,
                                reference_y,
                            )
                        })
                    {
                        physical_y[target_index] = Some(candidate);
                        progress = true;
                    }
                }
            }
        }

        for index in 0..inputs.len() {
            if physical_x[index].is_none() {
                if let Some(candidate) = fallback_physical_x(inputs, &physical_x, index) {
                    physical_x[index] = Some(candidate);
                    progress = true;
                }
            }

            if physical_y[index].is_none() {
                if let Some(candidate) = fallback_physical_y(inputs, &physical_y, index) {
                    physical_y[index] = Some(candidate);
                    progress = true;
                }
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

#[allow(dead_code)]
pub(super) fn normalize_physical_screen_layouts(
    layouts: &[PhysicalScreenLayout],
) -> Result<(Vec<PhysicalScreenLayout>, i32, i32, u32, u32), String> {
    let min_x = layouts
        .iter()
        .map(|layout| layout.physical_x)
        .min()
        .ok_or_else(|| "No screens found".to_string())?;
    let min_y = layouts
        .iter()
        .map(|layout| layout.physical_y)
        .min()
        .ok_or_else(|| "No screens found".to_string())?;
    let max_x = layouts
        .iter()
        .map(|layout| physical_end(layout.physical_x, layout.physical_width))
        .max()
        .ok_or_else(|| "No screens found".to_string())?;
    let max_y = layouts
        .iter()
        .map(|layout| physical_end(layout.physical_y, layout.physical_height))
        .max()
        .ok_or_else(|| "No screens found".to_string())?;

    let width = u32::try_from(max_x - i64::from(min_x))
        .map_err(|_| "Virtual desktop width is out of range".to_string())?;
    let height = u32::try_from(max_y - i64::from(min_y))
        .map_err(|_| "Virtual desktop height is out of range".to_string())?;

    let normalized = layouts
        .iter()
        .map(|layout| PhysicalScreenLayout {
            physical_x: layout.physical_x - min_x,
            physical_y: layout.physical_y - min_y,
            ..*layout
        })
        .collect();

    Ok((normalized, min_x, min_y, width, height))
}

/// Resolve a physical region to the single screen that fully contains it, and
/// translate the region into that screen's local coordinates.
///
/// A recording region must lie entirely inside one monitor; a region spanning
/// two screens is rejected. The encoder loop captures through
/// `Screen::capture_area` on one screen and has no compositing path, so a
/// cross-monitor region cannot be recorded at all. Failing loudly here — at
/// selection time — is why the check exists: the alternative is a silently
/// cropped or corrupt MP4.
pub(super) fn resolve_recording_selection(
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
    let local_region = Region::from_physical(
        PhysicalPoint::new(
            region.physical_x - layout.physical_x,
            region.physical_y - layout.physical_y,
        ),
        region.size(),
    );

    Ok((screen_index, local_region))
}

pub(super) fn build_screen_layout_inputs<I>(
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
                let physical_size = if index == captured_screen_index {
                    PhysicalSize::from(captured_size)
                } else {
                    LogicalSize::new(logical_width, logical_height)
                        .to_physical(ScaleFactor::try_from(scale_factor)?)?
                };

                Ok(ScreenLayoutInput {
                    logical_x,
                    logical_y,
                    logical_width,
                    logical_height,
                    physical_width: physical_size.width(),
                    physical_height: physical_size.height(),
                    is_primary,
                })
            },
        )
        .collect()
}
