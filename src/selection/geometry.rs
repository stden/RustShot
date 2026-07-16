//! Pure coordinate math: scale factors, ranges, drag rectangles, and screen lookup.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub physical_x: i32,
    pub physical_y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
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
pub(super) fn logical_end(start: i32, size: u32) -> i64 {
    i64::from(start) + i64::from(size)
}

/// Alias for [`logical_end`] — same arithmetic, different semantic context.
#[inline]
pub(super) fn physical_end(start: i32, size: u32) -> i64 {
    logical_end(start, size)
}

pub(super) fn ranges_overlap(start_a: i32, size_a: u32, start_b: i32, size_b: u32) -> bool {
    let end_a = logical_end(start_a, size_a);
    let end_b = logical_end(start_b, size_b);
    i64::from(start_a) < end_b && i64::from(start_b) < end_a
}

pub(super) fn monitor_scale_ratio(logical_size: u32, physical_size: u32) -> f32 {
    if logical_size == 0 {
        1.0
    } else {
        physical_size as f32 / logical_size as f32
    }
}

pub(super) fn scaled_gap(gap: i32, logical_size: u32, physical_size: u32) -> i32 {
    ((gap as f32) * monitor_scale_ratio(logical_size, physical_size)).round() as i32
}

pub(super) fn scaled_physical_dimension(
    logical_size: u32,
    scale_factor: f32,
) -> Result<u32, String> {
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err("Display scale factor must be positive".to_string());
    }

    let scaled_size = (logical_size as f64) * (scale_factor as f64);
    if !(0.0..=(u32::MAX as f64)).contains(&scaled_size) {
        return Err("Physical screen size is out of range".to_string());
    }

    Ok(scaled_size.ceil() as u32)
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
    let width = (start.0.max(end.0) - x1) as u32;
    let height = (start.1.max(end.1) - y1) as u32;

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
    screens
        .into_iter()
        .enumerate()
        .find(|(_, (x, y, width, height))| {
            cursor_pos.0 >= *x
                && cursor_pos.0 < *x + *width as i32
                && cursor_pos.1 >= *y
                && cursor_pos.1 < *y + *height as i32
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
