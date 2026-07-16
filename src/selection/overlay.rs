//! Fullscreen selection overlay window (winit + softbuffer).

use std::num::NonZeroU32;
use std::rc::Rc;

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{CursorIcon, Window, WindowAttributes, WindowLevel},
};

use super::*;

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
        }
    }

    pub(super) fn draw(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        // Use actual window size for the buffer; screenshot may differ due to DPI
        let actual = window.inner_size();
        let buf_w = actual.width as usize;
        let buf_h = actual.height as usize;
        let src_w = self.screen_width as usize;

        for y in 0..buf_h {
            for x in 0..buf_w {
                let src_idx = (y * src_w + x) * 4;
                let dst_idx = y * buf_w + x;

                if src_idx + 3 <= self.screenshot.len() && dst_idx < buffer.len() {
                    let mut r = self.screenshot[src_idx] as u32;
                    let mut g = self.screenshot[src_idx + 1] as u32;
                    let mut b = self.screenshot[src_idx + 2] as u32;

                    // Draw selection border
                    if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
                        let x1 = start.0.min(end.0) as usize;
                        let y1 = start.1.min(end.1) as usize;
                        let x2 = start.0.max(end.0) as usize;
                        let y2 = start.1.max(end.1) as usize;

                        // Border (3px cyan)
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
                    } else if let Some(pos) = self.current_pos {
                        // Crosshair
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

        let _ = buffer.present();
    }
}

impl ApplicationHandler for SelectionApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("RustShot")
            .with_inner_size(PhysicalSize::new(self.screen_width, self.screen_height))
            .with_position(PhysicalPosition::new(self.screen_x, self.screen_y))
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
                self.cancelled = true;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.physical_key
                    == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) =>
            {
                self.cancelled = true;
                event_loop.exit();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.is_dragging = true;
                            self.start_pos = self.current_pos;
                        }
                        ElementState::Released if self.is_dragging => {
                            self.is_dragging = false;
                            if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
                                if let Some(region) = build_region_from_drag(start, end) {
                                    self.result = Some(region);
                                    event_loop.exit();
                                } else {
                                    // Reset for next attempt
                                    self.start_pos = None;
                                }
                            } else {
                                self.start_pos = None;
                            }
                        }
                        _ => {}
                    }
                } else if button == MouseButton::Right {
                    self.cancelled = true;
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(surface) = &mut self.surface {
                    let w = new_size.width.max(1);
                    let h = new_size.height.max(1);
                    let _ =
                        surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap());
                }
                self.draw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.current_pos = Some((position.x as i32, position.y as i32));
                self.draw();
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }
}
