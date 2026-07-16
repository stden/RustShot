//! Fullscreen selection overlay window (winit + softbuffer).

use std::num::NonZeroU32;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
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
}

fn pixel_axis_map(overlay_extent: u32, image_extent: u32) -> Vec<u32> {
    (0..overlay_extent)
        .filter_map(|pixel| map_overlay_pixel_to_image(pixel, overlay_extent, image_extent))
        .collect()
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
    /// Error captured during window/surface setup (checked after event loop exits).
    pub(super) setup_error: Option<String>,
    pixel_map: OverlayPixelMapCache,
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
            setup_error: None,
            pixel_map: OverlayPixelMapCache::default(),
        }
    }

    pub(super) fn draw(&mut self) {
        let Some(actual) = self.window.as_ref().map(|window| window.inner_size()) else {
            return;
        };
        self.pixel_map.update(
            (actual.width, actual.height),
            (self.screen_width, self.screen_height),
        );

        let Some(surface) = &mut self.surface else {
            return;
        };
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        // Use actual window size for the buffer; screenshot may differ due to DPI
        let buf_w = actual.width as usize;
        let buf_h = actual.height as usize;
        let src_w = self.screen_width as usize;
        let decoration = OverlayDecoration::new(self.start_pos, self.current_pos, buf_w, buf_h);

        for (y, &src_y) in self.pixel_map.y.iter().take(buf_h).enumerate() {
            for (x, &src_x) in self.pixel_map.x.iter().take(buf_w).enumerate() {
                let src_idx = (src_y as usize * src_w + src_x as usize) * 4;
                let dst_idx = y * buf_w + x;

                if src_idx + 3 <= self.screenshot.len() && dst_idx < buffer.len() {
                    let mut r = self.screenshot[src_idx] as u32;
                    let mut g = self.screenshot[src_idx + 1] as u32;
                    let mut b = self.screenshot[src_idx + 2] as u32;

                    if let Some((overlay_r, overlay_g, overlay_b)) = decoration.color_at(x, y) {
                        r = overlay_r;
                        g = overlay_g;
                        b = overlay_b;
                    }

                    buffer[dst_idx] = (r << 16) | (g << 8) | b;
                }
            }
        }

        let _ = buffer.present();
    }

    fn set_cursor_grab(&self, mode: CursorGrabMode) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(mode);
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
        }
    }

    /// Left-button release: finish the drag.
    ///
    /// Returns `true` when a [`Region`] was stored in `result` and the event
    /// loop should exit; `false` when the drag was too small (state is reset
    /// for the next attempt) or no drag was in progress.
    pub(super) fn on_release(&mut self) -> bool {
        if !self.is_dragging {
            return false;
        }
        self.is_dragging = false;
        self.set_cursor_grab(CursorGrabMode::None);
        if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
            let overlay_size = self
                .window
                .as_ref()
                .map(|window| window.inner_size())
                .unwrap_or(PhysicalSize::new(self.screen_width, self.screen_height));
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
        false
    }

    /// Cancel the selection (Escape, right click, or window close).
    pub(super) fn on_cancel(&mut self) {
        self.set_cursor_grab(CursorGrabMode::None);
        self.cancelled = true;
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
        self.draw();
    }

    /// Cursor move: clamp the position into the overlay bounds and redraw.
    pub(super) fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let actual = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or(PhysicalSize::new(self.screen_width, self.screen_height));
        let max_x = f64::from(actual.width.min(i32::MAX as u32));
        let max_y = f64::from(actual.height.min(i32::MAX as u32));
        self.current_pos = Some((
            position.x.round().clamp(0.0, max_x) as i32,
            position.y.round().clamp(0.0, max_y) as i32,
        ));
        self.draw();
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
        window.set_cursor(CursorIcon::Crosshair);

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
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => self.on_press(),
                        ElementState::Released => {
                            if self.on_release() {
                                event_loop.exit();
                            }
                        }
                    }
                } else if button == MouseButton::Right {
                    self.on_cancel();
                    event_loop.exit();
                }
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
}
