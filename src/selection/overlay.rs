//! Fullscreen selection overlay window (winit + softbuffer).

use std::num::NonZeroU32;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{CursorGrabMode, CursorIcon, Fullscreen, Window, WindowAttributes, WindowLevel},
};

use super::*;

pub(super) fn target_overlay_monitor_index<I>(
    monitors: I,
    target_origin: (i32, i32),
    target_size: (u32, u32),
) -> Option<usize>
where
    I: IntoIterator<Item = ((i32, i32), (u32, u32))>,
{
    let monitors: Vec<_> = monitors.into_iter().collect();
    monitors
        .iter()
        .position(|&(origin, size)| origin == target_origin && size == target_size)
        .or_else(|| {
            monitors
                .iter()
                .position(|&(origin, _)| origin == target_origin)
        })
}

/// Cached nearest-neighbour lookups from overlay pixels to screenshot pixels.
///
/// The overlay redraws whenever the cursor moves. Keeping one lookup per axis
/// avoids repeating the DPI-scaling division for every pixel in every frame.
#[derive(Default)]
pub(super) struct OverlayPixelMapCache {
    key: Option<((u32, u32), (u32, u32))>,
    pub(super) x: Vec<u32>,
    pub(super) y: Vec<u32>,
    pub(super) background: Vec<u32>,
}

impl OverlayPixelMapCache {
    /// Refresh the lookup tables when either the overlay or image size changes.
    /// Returns `true` when new tables were built.
    pub(super) fn update(&mut self, overlay_size: (u32, u32), image_size: (u32, u32)) -> bool {
        let key = (overlay_size, image_size);
        if self.key == Some(key) {
            return false;
        }

        self.x = pixel_axis_map(overlay_size.0, image_size.0);
        self.y = pixel_axis_map(overlay_size.1, image_size.1);
        self.key = Some(key);
        true
    }

    /// Refresh both coordinate maps and the immutable screenshot background.
    ///
    /// The screenshot does not change during one selection session, so the
    /// converted softbuffer pixels only need rebuilding after a resize.
    pub(super) fn update_frame(
        &mut self,
        overlay_size: (u32, u32),
        image_size: (u32, u32),
        screenshot: &[u8],
    ) -> bool {
        let mappings_changed = self.update(overlay_size, image_size);
        let expected_background_len = self.x.len().checked_mul(self.y.len());
        if !mappings_changed && expected_background_len == Some(self.background.len()) {
            return false;
        }

        build_overlay_background_into(
            &mut self.background,
            screenshot,
            image_size.0,
            &self.x,
            &self.y,
        );
        true
    }
}

fn pixel_axis_map(overlay_extent: u32, image_extent: u32) -> Vec<u32> {
    (0..overlay_extent)
        .filter_map(|pixel| map_overlay_pixel_to_image(pixel, overlay_extent, image_extent))
        .collect()
}

/// How one overlay row draws its pixels from the screenshot.
///
/// The overlay is a physical-pixel window sized to match the captured frame, so
/// on a single-DPI monitor the two line up exactly and every column maps to
/// itself. Naming that case separately is what lets the common path copy a whole
/// row as a contiguous slice — no per-pixel coordinate lookup and no per-pixel
/// bounds check, which is the difference between a vectorised loop and a scalar
/// one. Mixed-DPI setups still take the general nearest-neighbour walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnSampling {
    /// Overlay column *n* is screenshot column *n*.
    Identity,
    /// Columns come from a nearest-neighbour lookup table.
    NearestNeighbour,
}

impl ColumnSampling {
    fn of(x_map: &[u32]) -> Self {
        if x_map
            .iter()
            .enumerate()
            .all(|(index, &source_x)| source_x as usize == index)
        {
            Self::Identity
        } else {
            Self::NearestNeighbour
        }
    }
}

/// Pack RGBA bytes into softbuffer's `0RGB` pixels.
///
/// Expressed as a little-endian byte reorder rather than shift-and-or: the two
/// are arithmetically identical, but this form is a pure byte shuffle that the
/// compiler vectorises, and this loop runs once per pixel of the whole frame.
/// Kept as a tight zip over two slices so the bounds checks stay outside it.
#[inline]
fn pack_rgba_row(source: &[u8], output: &mut [u32]) {
    for (pixel, output) in source.as_chunks::<4>().0.iter().zip(output.iter_mut()) {
        *output = u32::from_le_bytes([pixel[2], pixel[1], pixel[0], 0]);
    }
}

/// Render the screenshot into `background`, reusing its existing allocation.
///
/// Takes the buffer by reference rather than returning a fresh `Vec` because a
/// resize rebuilds it: handing back a new 8 MB zeroed vector each time costs
/// more in first-touch page faults than the pixel conversion itself, and an
/// overlay gets several resizes between the keypress and appearing on screen.
pub(super) fn build_overlay_background_into(
    background: &mut Vec<u32>,
    screenshot: &[u8],
    image_width: u32,
    x_map: &[u32],
    y_map: &[u32],
) {
    let Some(output_len) = x_map.len().checked_mul(y_map.len()) else {
        background.clear();
        return;
    };
    // Two different cheapest options, so pick per case. When the buffer is
    // already the right size, clearing it in place avoids dropping and
    // re-faulting 8 MB of pages. When it is not, `vec![0; n]` beats a resize:
    // the allocator hands back pre-zeroed pages instead of memsetting them.
    // Either way, pixels that no row supplies stay zero.
    if background.len() == output_len {
        background.fill(0);
    } else {
        *background = vec![0; output_len];
    }
    let image_width = image_width as usize;
    if image_width == 0 {
        return;
    }

    // Overlay startup and every resize run this, so it sits inside the delay
    // between the PrintScreen keypress and the overlay appearing.
    let output_width = x_map.len();
    let sampling = ColumnSampling::of(x_map);
    for (output_y, &source_y) in y_map.iter().enumerate() {
        let Some(row_start) = (source_y as usize)
            .checked_mul(image_width)
            .and_then(|row| row.checked_mul(4))
        else {
            continue;
        };
        let output_row = &mut background[output_y * output_width..(output_y + 1) * output_width];

        if sampling == ColumnSampling::Identity {
            // The whole row is one contiguous run of source pixels. A row the
            // screenshot cannot supply is left at its initial value.
            if let Some(source_row) = screenshot
                .get(row_start..)
                .and_then(|rest| rest.get(..output_width * 4))
            {
                pack_rgba_row(source_row, output_row);
                continue;
            }
        }

        for (output_pixel, &source_x) in output_row.iter_mut().zip(x_map.iter()) {
            let Some(source_index) = (source_x as usize)
                .checked_mul(4)
                .and_then(|offset| row_start.checked_add(offset))
            else {
                continue;
            };
            let Some(pixel) = screenshot
                .get(source_index..)
                .and_then(|remaining| remaining.get(..3))
            else {
                continue;
            };
            *output_pixel = u32::from_le_bytes([pixel[2], pixel[1], pixel[0], 0]);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OverlayDecoration {
    None,
    Border {
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
    },
    Crosshair {
        x: usize,
        y: usize,
    },
}

impl OverlayDecoration {
    pub(super) fn new(
        start: Option<(i32, i32)>,
        current: Option<(i32, i32)>,
        width: usize,
        height: usize,
    ) -> Self {
        if width == 0 || height == 0 {
            return Self::None;
        }
        let clamp_x = |value: i32| {
            usize::try_from(value)
                .unwrap_or(0)
                .min(width.saturating_sub(1))
        };
        let clamp_y = |value: i32| {
            usize::try_from(value)
                .unwrap_or(0)
                .min(height.saturating_sub(1))
        };

        match (start, current) {
            (Some(start), Some(end)) => Self::Border {
                x1: clamp_x(start.0.min(end.0)),
                y1: clamp_y(start.1.min(end.1)),
                x2: clamp_x(start.0.max(end.0)),
                y2: clamp_y(start.1.max(end.1)),
            },
            (None, Some(position)) => Self::Crosshair {
                x: clamp_x(position.0),
                y: clamp_y(position.1),
            },
            _ => Self::None,
        }
    }

    #[cfg(test)]
    pub(super) fn color_at(self, x: usize, y: usize) -> Option<(u32, u32, u32)> {
        match self {
            Self::None => None,
            Self::Crosshair {
                x: cursor_x,
                y: cursor_y,
            } if x == cursor_x || y == cursor_y => Some((255, 0, 0)),
            Self::Crosshair { .. } => None,
            Self::Border { x1, y1, x2, y2 } => {
                let on_border = (x >= x1.saturating_sub(1)
                    && x <= x2.saturating_add(1)
                    && ((y >= y1.saturating_sub(1) && y <= y1.saturating_add(1))
                        || (y >= y2.saturating_sub(1) && y <= y2.saturating_add(1))))
                    || (y >= y1.saturating_sub(1)
                        && y <= y2.saturating_add(1)
                        && ((x >= x1.saturating_sub(1) && x <= x1.saturating_add(1))
                            || (x >= x2.saturating_sub(1) && x <= x2.saturating_add(1))));
                on_border.then_some((0, 200, 255))
            }
        }
    }
}

/// A run of overlay pixels, already clamped into the frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct OverlayRect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

/// The rectangles covered by one or two decorations, without heap allocation.
///
/// A crosshair paints two bars and a selection border four, so an incremental
/// redraw — restore the previous decoration, paint the current one — never
/// needs more than eight.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct OverlayDamage {
    rects: [OverlayRect; Self::CAPACITY],
    len: usize,
}

impl OverlayDamage {
    const CAPACITY: usize = 8;

    fn push(&mut self, rect: Option<OverlayRect>) {
        if let Some(rect) = rect {
            if self.len < Self::CAPACITY {
                self.rects[self.len] = rect;
                self.len += 1;
            }
        }
    }

    fn extend(&mut self, other: &Self) {
        for rect in other.as_slice() {
            self.push(Some(*rect));
        }
    }

    pub(super) fn as_slice(&self) -> &[OverlayRect] {
        &self.rects[..self.len]
    }
}

/// Clamp a pair of inclusive pixel ranges into the frame, or `None` when the
/// rectangle lies entirely outside it.
fn clamped_overlay_rect(
    frame_size: (usize, usize),
    x_bounds: (usize, usize),
    y_bounds: (usize, usize),
) -> Option<OverlayRect> {
    let (width, height) = frame_size;
    let (x_start, x_end) = x_bounds;
    let (y_start, y_end) = y_bounds;
    if width == 0
        || height == 0
        || x_start > x_end
        || y_start > y_end
        || x_start >= width
        || y_start >= height
    {
        return None;
    }

    let x_end = x_end.min(width - 1);
    let y_end = y_end.min(height - 1);
    Some(OverlayRect {
        x: x_start,
        y: y_start,
        width: x_end - x_start + 1,
        height: y_end - y_start + 1,
    })
}

/// The rectangles `decoration` paints, clamped into a `width` x `height` frame.
///
/// Painting and damage tracking both derive from this one list, so the reported
/// damage cannot drift out of sync with the pixels actually written.
fn decoration_rects(decoration: OverlayDecoration, width: usize, height: usize) -> OverlayDamage {
    let frame = (width, height);
    let mut damage = OverlayDamage::default();

    match decoration {
        OverlayDecoration::None => {}
        OverlayDecoration::Crosshair { x, y } => {
            damage.push(clamped_overlay_rect(
                frame,
                (0, width.saturating_sub(1)),
                (y, y),
            ));
            damage.push(clamped_overlay_rect(
                frame,
                (x, x),
                (0, height.saturating_sub(1)),
            ));
        }
        OverlayDecoration::Border { x1, y1, x2, y2 } if x1 <= x2 && y1 <= y2 => {
            let outer_x1 = x1.saturating_sub(1);
            let outer_x2 = x2.saturating_add(1);
            let outer_y1 = y1.saturating_sub(1);
            let outer_y2 = y2.saturating_add(1);

            damage.push(clamped_overlay_rect(
                frame,
                (outer_x1, outer_x2),
                (outer_y1, y1.saturating_add(1)),
            ));
            damage.push(clamped_overlay_rect(
                frame,
                (outer_x1, outer_x2),
                (y2.saturating_sub(1), outer_y2),
            ));
            damage.push(clamped_overlay_rect(
                frame,
                (outer_x1, x1.saturating_add(1)),
                (outer_y1, outer_y2),
            ));
            damage.push(clamped_overlay_rect(
                frame,
                (x2.saturating_sub(1), outer_x2),
                (outer_y1, outer_y2),
            ));
        }
        OverlayDecoration::Border { .. } => {}
    }

    damage
}

/// The colour a decoration paints, or `None` when it paints nothing.
fn decoration_color(decoration: OverlayDecoration) -> Option<u32> {
    const CROSSHAIR_COLOR: u32 = 0x00ff_0000;
    const BORDER_COLOR: u32 = 0x0000_c8ff;

    match decoration {
        OverlayDecoration::None => None,
        OverlayDecoration::Crosshair { .. } => Some(CROSSHAIR_COLOR),
        OverlayDecoration::Border { x1, y1, x2, y2 } if x1 <= x2 && y1 <= y2 => Some(BORDER_COLOR),
        OverlayDecoration::Border { .. } => None,
    }
}

fn fill_overlay_rect(frame: &mut [u32], frame_width: usize, rect: OverlayRect, color: u32) {
    for row in rect.y..rect.y + rect.height {
        let start = row * frame_width + rect.x;
        frame[start..start + rect.width].fill(color);
    }
}

fn restore_overlay_rect(
    frame: &mut [u32],
    background: &[u32],
    frame_width: usize,
    rect: OverlayRect,
) {
    for row in rect.y..rect.y + rect.height {
        let start = row * frame_width + rect.x;
        frame[start..start + rect.width].copy_from_slice(&background[start..start + rect.width]);
    }
}

fn paint_overlay_decoration(
    frame: &mut [u32],
    width: usize,
    height: usize,
    decoration: OverlayDecoration,
) {
    let Some(color) = decoration_color(decoration) else {
        return;
    };
    for &rect in decoration_rects(decoration, width, height).as_slice() {
        fill_overlay_rect(frame, width, rect, color);
    }
}

/// Rebuild the whole frame: the cached screenshot background plus `decoration`.
///
/// Returns `false`, having touched nothing, when the frame does not fit both
/// buffers. The caller has to know rather than presenting anyway: softbuffer
/// only vouches for a buffer's contents through `Buffer::age()`, so presenting
/// a frame nobody wrote shows recycled pixels - a black screen on the Win32
/// backend, whose DIB section starts zeroed - instead of the screenshot. The
/// mismatch is real: the window reports a new `inner_size()` before the
/// `Resized` event that rebuilds the surface for it, so a fullscreen or DPI
/// transition right after window creation lands here on the first frame.
///
/// This is a *length* check and so catches only the direction where the output is
/// too short. It is not the whole guard, and callers must not read it as one: a
/// buffer that is long enough but has different dimensions would be filled at the
/// wrong stride, which no length can detect. [`SelectionApp::draw`] rejects that
/// case up front by comparing the surface's own width and height, and only then
/// calls in here.
pub(super) fn compose_overlay_frame(
    background: &[u32],
    output: &mut [u32],
    width: usize,
    height: usize,
    decoration: OverlayDecoration,
) -> bool {
    let Some(frame_len) = width.checked_mul(height) else {
        return false;
    };
    if background.len() < frame_len || output.len() < frame_len {
        return false;
    }

    output[..frame_len].copy_from_slice(&background[..frame_len]);
    paint_overlay_decoration(&mut output[..frame_len], width, height, decoration);
    true
}

/// Repaint only the pixels the previous and current decorations cover.
///
/// The overlay redraws on every cursor move, but a crosshair or selection
/// border touches a few thousand of the frame's millions of pixels. Restoring
/// just the previous decoration's rectangles from the cached background and
/// painting the new one turns a full-frame copy into a handful of thin row
/// spans, which is what keeps dragging responsive at 4K.
///
/// `output` must still hold the frame `previous` was painted into. softbuffer
/// reports exactly that as `Buffer::age() == 1`; when it reports anything else
/// the buffer contents are unspecified and [`compose_overlay_frame`] has to
/// rebuild the whole frame instead.
///
/// A buffer-size mismatch returns empty damage. That is also a legitimate
/// result (an inverted border paints nothing), so damage cannot double as a
/// failure signal or authorize presentation of an otherwise invalid frame.
///
/// What keeps this path safe is not the length check below but the dimension
/// check in [`SelectionApp::draw`], which returns before either composition
/// function is reached unless the surface is exactly the size being composed for.
/// Without it, an output that is long enough but shaped differently would be
/// repainted at the wrong stride here, and `present_with_damage` would blit those
/// rectangles - so do not weaken that check on the assumption that reaching this
/// function implies a frame was already composed at this size.
///
/// Returns the rectangles that changed, for `present_with_damage`.
pub(super) fn redraw_overlay_frame(
    background: &[u32],
    output: &mut [u32],
    size: (usize, usize),
    previous: OverlayDecoration,
    decoration: OverlayDecoration,
) -> OverlayDamage {
    let (width, height) = size;
    let mut damage = OverlayDamage::default();
    let Some(frame_len) = width.checked_mul(height) else {
        return damage;
    };
    if background.len() < frame_len || output.len() < frame_len {
        return damage;
    }
    let frame = &mut output[..frame_len];

    let restored = decoration_rects(previous, width, height);
    for &rect in restored.as_slice() {
        restore_overlay_rect(frame, background, width, rect);
    }

    let painted = decoration_rects(decoration, width, height);
    if let Some(color) = decoration_color(decoration) {
        for &rect in painted.as_slice() {
            fill_overlay_rect(frame, width, rect, color);
        }
    }

    damage.extend(&restored);
    damage.extend(&painted);
    damage
}

/// Choose the native presentation regions independently of CPU composition.
///
/// softbuffer's Win32 backend issues a separate `BitBlt` for each damage rectangle.
/// Sending the thin restore/paint spans individually exposes partially erased
/// crosshairs and borders between blits. It also validates the entire window,
/// so decoration bounds alone miss an OS repaint outside them. Present the full
/// completed frame in one blit on Windows, while still composing only the thin
/// changed spans. Other backends can keep using their damage hints.
pub(super) fn damage_rects(
    damage: &OverlayDamage,
    width: NonZeroU32,
    height: NonZeroU32,
) -> Vec<softbuffer::Rect> {
    #[cfg(windows)]
    {
        let _ = damage;
        vec![softbuffer::Rect {
            x: 0,
            y: 0,
            width,
            height,
        }]
    }
    #[cfg(not(windows))]
    {
        let _ = (width, height);
        damage
            .as_slice()
            .iter()
            .filter_map(|rect| {
                Some(softbuffer::Rect {
                    x: u32::try_from(rect.x).ok()?,
                    y: u32::try_from(rect.y).ok()?,
                    width: NonZeroU32::new(u32::try_from(rect.width).ok()?)?,
                    height: NonZeroU32::new(u32::try_from(rect.height).ok()?)?,
                })
            })
            .collect()
    }
}

/// Winit application handler for the fullscreen selection overlay.
///
/// Displays the captured screenshot as background, draws a crosshair cursor,
/// and lets the user drag a rectangle (cyan border).  The selected [`Region`]
/// is stored in `result` on mouse release.
pub(super) struct SelectionApp {
    pub(super) window: Option<Rc<Window>>,
    pub(super) surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    pub(super) screenshot: Vec<u8>,
    pub(super) screen_width: u32,
    pub(super) screen_height: u32,
    pub(super) screen_x: i32,
    pub(super) screen_y: i32,
    pub(super) start_pos: Option<(i32, i32)>,
    pub(super) current_pos: Option<(i32, i32)>,
    pub(super) is_dragging: bool,
    pub(super) result: Option<Region>,
    pub(super) cancelled: bool,
    pub(super) cancellation_path: Option<std::path::PathBuf>,
    /// Error captured during window/surface setup (checked after event loop exits).
    pub(super) setup_error: Option<String>,
    pixel_map: OverlayPixelMapCache,
    /// Decoration painted into the presented frame, when that frame is still
    /// reusable as the base for an incremental redraw. `None` forces a full
    /// recompose on the next draw.
    last_decoration: Option<OverlayDecoration>,
    first_frame_presented: bool,
    /// Whether the error that skipped a frame has been reported already.
    /// Redraws follow cursor movement and one mismatch is permanent
    /// (a capture with a zero extent leaves the cached background empty for the
    /// whole session), so logging every occurrence would write one identical
    /// line per mouse motion and bury everything else in the log.
    frame_skip_logged: bool,
}

fn log_skipped_frame(
    logged: &mut bool,
    size: (usize, usize),
    background_len: usize,
    output_len: usize,
) {
    if *logged {
        return;
    }
    *logged = true;
    log::warn!(
        "overlay frame {}x{} not presented: background {} px, surface buffer {} px",
        size.0,
        size.1,
        background_len,
        output_len,
    );
}

impl SelectionApp {
    pub(super) fn new(
        screenshot: Vec<u8>,
        width: u32,
        height: u32,
        screen_x: i32,
        screen_y: i32,
    ) -> Self {
        Self {
            window: None,
            surface: None,
            screenshot,
            screen_width: width,
            screen_height: height,
            screen_x,
            screen_y,
            start_pos: None,
            current_pos: None,
            is_dragging: false,
            result: None,
            cancelled: false,
            cancellation_path: None,
            setup_error: None,
            pixel_map: OverlayPixelMapCache::default(),
            last_decoration: None,
            first_frame_presented: false,
            frame_skip_logged: false,
        }
    }

    /// The recording parent can stop a child while it still waits for a drag.
    pub(super) fn poll_cancellation(&mut self) -> bool {
        if self
            .cancellation_path
            .as_ref()
            .is_some_and(|path| path.exists())
        {
            self.on_cancel();
        }
        self.cancelled
    }

    pub(super) fn draw(&mut self) {
        let Some(actual) = self.window.as_ref().map(|window| window.inner_size()) else {
            return;
        };
        if self.pixel_map.update_frame(
            (actual.width, actual.height),
            (self.screen_width, self.screen_height),
            &self.screenshot,
        ) {
            // A rebuilt background invalidates whatever is on screen.
            self.last_decoration = None;
        }

        let Some(surface) = &mut self.surface else {
            return;
        };
        let mut buffer = match surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(error) => {
                if !self.frame_skip_logged {
                    log::warn!("overlay buffer unavailable: {error}");
                    self.frame_skip_logged = true;
                }
                self.last_decoration = None;
                return;
            }
        };

        // Use actual window size for the buffer; screenshot may differ due to DPI
        let buf_w = actual.width as usize;
        let buf_h = actual.height as usize;

        // The surface has to already be the size we are about to compose for, and
        // matching *areas* are not enough to establish that. softbuffer's Win32
        // buffer is a DIB whose stride is the surface width, so a buffer that
        // merely happens to be big enough gets filled at one stride and blitted at
        // another: every row lands progressively further off, and the tail nobody
        // wrote keeps whatever was recycled. Two reachable ways in, both the same
        // "inner_size() changes before the Resized event that applies it" race
        // that motivates the length check below - a shrink or a return from
        // fullscreen leaves a larger buffer, and a display rotation from 1920x1080
        // to 1080x1920 changes neither the area nor the length. A sheared frame is
        // no better than an unwritten one, so it takes the same exit.
        let surface_size = (
            buffer.width().get() as usize,
            buffer.height().get() as usize,
        );
        if surface_size != (buf_w, buf_h) {
            if !self.frame_skip_logged {
                self.frame_skip_logged = true;
                log::warn!(
                    "overlay frame not presented: window is {buf_w}x{buf_h} but the surface buffer is {}x{}",
                    surface_size.0,
                    surface_size.1,
                );
            }
            // Same reasoning as the compose-failed arm: a decoration cached here
            // would let the next incremental redraw restore against a frame that
            // never reached the screen.
            self.last_decoration = None;
            return;
        }

        let decoration = OverlayDecoration::new(self.start_pos, self.current_pos, buf_w, buf_h);

        // age 1 means the buffer still holds the frame presented last, which is
        // the precondition for repainting only what changed.
        let reusable = buffer.age() == 1;
        let presentation = match self.last_decoration.filter(|_| reusable) {
            // An OS repaint can request an unchanged frame after exposure.
            // Buffer age describes the backing pixels, not the window contents.
            Some(previous) if previous == decoration => buffer.present(),
            Some(previous) => {
                let damage = redraw_overlay_frame(
                    &self.pixel_map.background,
                    &mut buffer,
                    (buf_w, buf_h),
                    previous,
                    decoration,
                );
                let rectangles = damage_rects(&damage, buffer.width(), buffer.height());
                buffer.present_with_damage(&rectangles)
            }
            None => {
                if !compose_overlay_frame(
                    &self.pixel_map.background,
                    &mut buffer,
                    buf_w,
                    buf_h,
                    decoration,
                ) {
                    // Nothing was written, so presenting would hand the user
                    // whatever softbuffer recycled into the buffer instead of the
                    // screenshot. Leaving `last_decoration` unset is the other
                    // half of the fix: caching `decoration` here would let the
                    // next incremental redraw restore against a frame that was
                    // never drawn. The condition is normally transient - the
                    // window reports its new size before `Resized` rebuilds the
                    // surface - so skip this frame rather than fail the whole
                    // selection.
                    log_skipped_frame(
                        &mut self.frame_skip_logged,
                        (buf_w, buf_h),
                        self.pixel_map.background.len(),
                        buffer.len(),
                    );
                    self.last_decoration = None;
                    return;
                }
                buffer.present()
            }
        };
        match presentation {
            Ok(()) => {
                // A skipped or failed startup draw is not a visible frame.
                // Record readiness only after the first successful presentation.
                if !self.first_frame_presented {
                    self.first_frame_presented = true;
                    crate::timing::mark(crate::timing::CaptureMilestone::OverlayFirstFrame);
                }
                self.last_decoration = Some(decoration);
                self.frame_skip_logged = false;
            }
            Err(error) => {
                if !self.frame_skip_logged {
                    log::warn!("overlay frame not presented: {error}");
                    self.frame_skip_logged = true;
                }
                self.last_decoration = None;
            }
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            // Winit coalesces requests, so queued mouse events produce one
            // completed frame at the latest position, not intermediate frames.
            window.request_redraw();
        }
    }

    fn set_cursor_grab(&self, mode: CursorGrabMode) {
        if let Some(window) = &self.window {
            // Winit's Win32 backend turns hidden + confined into a 1x1 lock
            // at the window center. During a drag the border replaces the red
            // crosshair, so show the native pointer before confining it.
            if mode != CursorGrabMode::None {
                window.set_cursor_visible(true);
            }
            let _ = window.set_cursor_grab(mode);
            // Release confinement before hiding, or mouse-up would warp too.
            if mode == CursorGrabMode::None {
                window.set_cursor_visible(false);
            }
        }
    }

    // --- Pure event transitions -------------------------------------------
    //
    // The `ApplicationHandler` implementation below is a thin dispatcher into
    // these methods so the selection state machine can be driven directly in
    // unit tests without an `ActiveEventLoop` or a real `Window`. Each method
    // is `Window`-safe: with `window`/`surface` unset it falls back to the
    // captured screen dimensions or skips the surface work.

    /// Left-button press: start a drag at the current cursor position.
    pub(super) fn on_press(&mut self) {
        if self.current_pos.is_some() {
            self.is_dragging = true;
            self.start_pos = self.current_pos;
            self.set_cursor_grab(CursorGrabMode::Confined);
            self.request_redraw();
        }
    }

    /// Left-button release: finish the drag.
    ///
    /// Returns `true` when a [`Region`] was stored in `result` and the event
    /// loop should exit; `false` when the drag was too small (state is reset
    /// for the next attempt) or no drag was in progress.
    pub(super) fn on_release(&mut self) -> bool {
        if self.poll_cancellation() || !self.is_dragging {
            return false;
        }
        self.is_dragging = false;
        self.set_cursor_grab(CursorGrabMode::None);
        if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
            let overlay_size = self
                .window
                .as_ref()
                .map(|window| window.inner_size())
                .unwrap_or_else(|| PhysicalSize::new(self.screen_width, self.screen_height));
            if let Some(region) = map_overlay_drag_to_image(
                start,
                end,
                (overlay_size.width, overlay_size.height),
                (self.screen_width, self.screen_height),
            ) {
                self.result = Some(region);
                return true;
            } else {
                // Reset for next attempt
                self.start_pos = None;
            }
        } else {
            self.start_pos = None;
        }
        self.request_redraw();
        false
    }

    /// Cancel the selection (Escape, right click, or window close).
    pub(super) fn on_cancel(&mut self) {
        self.set_cursor_grab(CursorGrabMode::None);
        self.cancelled = true;
        self.is_dragging = false;
        self.start_pos = None;
        self.current_pos = None;
        self.result = None;
    }

    /// Window resize (e.g. a DPI/scale change): resize the surface and abort
    /// any in-progress drag, whose coordinates are stale in the new size.
    pub(super) fn on_resize(&mut self, new_size: PhysicalSize<u32>) {
        log::info!(
            "overlay resized to {}x{} (screenshot {}x{})",
            new_size.width,
            new_size.height,
            self.screen_width,
            self.screen_height,
        );
        if let Some(surface) = &mut self.surface {
            let w = new_size.width.max(1);
            let h = new_size.height.max(1);
            let _ = surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap());
        }
        if self.is_dragging {
            self.set_cursor_grab(CursorGrabMode::None);
            self.is_dragging = false;
            self.start_pos = None;
            self.current_pos = None;
        }
        self.request_redraw();
    }

    /// Cursor move: clamp the position into the overlay bounds and redraw.
    pub(super) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let actual = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or_else(|| PhysicalSize::new(self.screen_width, self.screen_height));
        let max_x = f64::from(actual.width.min(i32::MAX as u32));
        let max_y = f64::from(actual.height.min(i32::MAX as u32));
        let current_pos = Some((
            position.x.round().clamp(0.0, max_x) as i32,
            position.y.round().clamp(0.0, max_y) as i32,
        ));
        if self.current_pos != current_pos {
            self.current_pos = current_pos;
            self.request_redraw();
        }
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.poll_cancellation() {
            event_loop.exit();
            return;
        }
        if self.window.is_some() {
            return;
        }

        let monitors: Vec<_> = event_loop.available_monitors().collect();
        let target_monitor = target_overlay_monitor_index(
            monitors.iter().map(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                ((position.x, position.y), (size.width, size.height))
            }),
            (self.screen_x, self.screen_y),
            (self.screen_width, self.screen_height),
        )
        .and_then(|index| monitors.get(index).cloned());

        let attrs = WindowAttributes::default()
            .with_title("RustShot")
            .with_inner_size(PhysicalSize::new(self.screen_width, self.screen_height))
            .with_position(PhysicalPosition::new(self.screen_x, self.screen_y))
            .with_fullscreen(target_monitor.map(|monitor| Fullscreen::Borderless(Some(monitor))))
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                self.setup_error = Some(format!("Failed to create window: {e}"));
                event_loop.exit();
                return;
            }
        };
        // Only show this native cursor while dragging a border. Before the
        // press, the frame supplies the red crosshair without a second cursor.
        window.set_cursor(CursorIcon::Crosshair);
        window.set_cursor_visible(false);

        // Use the actual window size (may differ from requested due to DPI)
        let actual_size = window.inner_size();
        log::info!(
            "overlay window: requested {}x{} at ({}, {}), got {}x{} (scale {}), screenshot {}x{}",
            self.screen_width,
            self.screen_height,
            self.screen_x,
            self.screen_y,
            actual_size.width,
            actual_size.height,
            window.scale_factor(),
            self.screen_width,
            self.screen_height,
        );
        let surface_w = actual_size.width.max(1);
        let surface_h = actual_size.height.max(1);

        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                self.setup_error = Some(format!("Failed to create graphics context: {e}"));
                event_loop.exit();
                return;
            }
        };
        let mut surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                self.setup_error = Some(format!("Failed to create surface: {e}"));
                event_loop.exit();
                return;
            }
        };
        if let Err(e) = surface.resize(
            NonZeroU32::new(surface_w).unwrap(),
            NonZeroU32::new(surface_h).unwrap(),
        ) {
            self.setup_error = Some(format!("Failed to resize surface: {e}"));
            event_loop.exit();
            return;
        }

        self.window = Some(window);
        self.surface = Some(surface);
        crate::timing::mark(crate::timing::CaptureMilestone::OverlayWindowCreated);
        self.draw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.on_cancel();
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.physical_key
                    == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) =>
            {
                self.on_cancel();
                event_loop.exit();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.on_press(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.on_release() {
                    event_loop.exit();
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                ..
            } => {
                self.on_cancel();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => self.on_resize(new_size),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                mut inner_size_writer,
            } => {
                // Winit's default DPI handling preserves the LOGICAL size: on a
                // mixed-DPI setup the overlay would shrink to 1/scale of the
                // monitor (e.g. 80% at 125%) and the frozen screenshot with it.
                // The overlay is a physical-pixel window matching the captured
                // frame exactly - reassert that size instead.
                log::info!(
                    "overlay scale factor changed to {scale_factor}; reasserting {}x{}",
                    self.screen_width,
                    self.screen_height,
                );
                let _ = inner_size_writer
                    .request_inner_size(PhysicalSize::new(self.screen_width, self.screen_height));
            }
            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(position),
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.poll_cancellation() {
            event_loop.exit();
        } else if self.cancellation_path.is_some() {
            // A sleeping overlay otherwise receives no events after tray Exit.
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(100),
            ));
        }
    }
}
