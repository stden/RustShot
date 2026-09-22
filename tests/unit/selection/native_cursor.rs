//! Native cursor confinement cannot be verified with a windowless SelectionApp.

use super::*;
use std::rc::Rc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetClientRect, GetClipCursor, GetCursorPos, GetForegroundWindow, SetCursorPos,
    SetForegroundWindow,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{CursorGrabMode, Window, WindowId};

fn cursor_position() -> POINT {
    let mut position = POINT::default();
    // SAFETY: the output pointer remains writable for this synchronous query.
    unsafe { GetCursorPos(&mut position) }.expect("interactive cursor position");
    position
}

fn cursor_clip() -> RECT {
    let mut rectangle = RECT::default();
    // SAFETY: the output pointer remains writable for this synchronous query.
    unsafe { GetClipCursor(&mut rectangle) }.expect("interactive cursor clipping");
    rectangle
}

struct RestoreDesktop {
    position: POINT,
    clip: RECT,
    foreground: HWND,
}

impl RestoreDesktop {
    fn capture() -> Self {
        Self {
            position: cursor_position(),
            clip: cursor_clip(),
            // SAFETY: this query returns a handle by value without retaining data.
            foreground: unsafe { GetForegroundWindow() },
        }
    }
}

impl Drop for RestoreDesktop {
    fn drop(&mut self) {
        // Best-effort restoration must run on assertion failure without masking
        // the failure. The test window is dropped before this guard, so its
        // teardown cannot overwrite the restored cursor clip; only the event
        // loop's message-pump utility window remains alive.
        // SAFETY: clip/position were captured before mutation; a foreground
        // window closed during the test makes SetForegroundWindow fail safely.
        unsafe {
            let _ = ClipCursor(None);
            if !self.foreground.is_invalid() {
                let _ = SetForegroundWindow(self.foreground);
            }
            let _ = SetCursorPos(self.position.x, self.position.y);
            let _ = ClipCursor(Some(&self.clip));
        }
    }
}

struct NativeFixture {
    app: SelectionApp,
}

impl ApplicationHandler for NativeFixture {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.window.is_some() {
            return;
        }
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("RustShot native cursor regression")
                    .with_decorations(false)
                    .with_position(PhysicalPosition::new(80, 80))
                    .with_inner_size(PhysicalSize::new(240, 160)),
            )
            .expect("native test window");
        // Match selection startup: the screenshot renderer supplies its own
        // crosshair, so the native pointer is initially hidden.
        window.set_cursor_visible(false);
        self.app.window = Some(Rc::new(window));
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

impl Drop for NativeFixture {
    fn drop(&mut self) {
        if let Some(window) = &self.app.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
    }
}

#[test]
#[ignore = "requires an interactive desktop and exclusive foreground/cursor control"]
fn native_drag_confines_to_client_without_warping_and_releases_on_exit() {
    // Winit establishes process DPI awareness during build. Snapshot after it
    // so restoration and assertions use physical coordinates throughout.
    let mut event_loop = EventLoop::builder()
        .with_any_thread(true)
        .build()
        .expect("native event loop on the Rust test thread");
    let _restore = RestoreDesktop::capture();
    // SAFETY: None releases an existing clip; the guard restores its old value.
    unsafe { ClipCursor(None) }.expect("release preexisting cursor clip");
    let unrestricted = cursor_clip();
    let mut fixture = NativeFixture {
        app: SelectionApp::new(vec![], 240, 160, 0, 0),
    };
    event_loop.pump_app_events(Some(Duration::ZERO), &mut fixture);
    let window = fixture.app.window.as_ref().expect("window was created");
    let RawWindowHandle::Win32(raw) = window.window_handle().unwrap().as_raw() else {
        panic!("expected a Win32 window");
    };
    let hwnd = HWND(raw.hwnd.get() as *mut std::ffi::c_void);
    // SAFETY: the fixture keeps hwnd alive until this test completes.
    let _ = unsafe { SetForegroundWindow(hwnd) };
    let deadline = Instant::now() + Duration::from_secs(2);
    // SAFETY: this query reads the foreground handle without retaining data.
    while unsafe { GetForegroundWindow() } != hwnd && Instant::now() < deadline {
        event_loop.pump_app_events(Some(Duration::from_millis(10)), &mut fixture);
    }
    assert_eq!(
        // SAFETY: same handle-only query, after the bounded activation wait.
        unsafe { GetForegroundWindow() },
        hwnd,
        "test window must own focus"
    );
    let mut client = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: hwnd is live and both outputs stay writable for these calls.
    unsafe {
        GetClientRect(hwnd, &mut client).expect("native client bounds");
        assert!(ClientToScreen(hwnd, &mut origin).as_bool());
    }
    client.left += origin.x;
    client.right += origin.x;
    client.top += origin.y;
    client.bottom += origin.y;
    let start = POINT {
        x: client.left + 20,
        y: client.top + 25,
    };
    // SAFETY: the point lies inside the visible test window and will be restored.
    unsafe { SetCursorPos(start.x, start.y) }.expect("place test cursor");

    // One event loop per process: cover a rejected short drag, cancellation,
    // and a completed drag with the same window rather than separate GUI tests.
    for finish in ["short release", "cancel", "completed release"] {
        fixture
            .app
            .on_cursor_moved(PhysicalPosition::new(20.0, 25.0));
        fixture.app.on_press();
        assert_eq!(
            cursor_clip(),
            client,
            "{finish}: drag must retain the whole client area"
        );
        assert_eq!(
            cursor_position(),
            start,
            "{finish}: pressing must not warp the cursor"
        );
        match finish {
            "short release" => assert!(!fixture.app.on_release()),
            "cancel" => fixture.app.on_cancel(),
            _ => {
                fixture
                    .app
                    .on_cursor_moved(PhysicalPosition::new(100.0, 110.0));
                assert!(fixture.app.on_release());
            }
        }
        assert_eq!(
            cursor_clip(),
            unrestricted,
            "{finish}: cursor must be released"
        );
    }
}
