//! Per-screen desktop capture and the public selection entry points.

use screenshots::Screen;
use winit::event_loop::{ControlFlow, EventLoop};

use super::*;

#[derive(Debug)]
pub(super) struct VirtualDesktopCapture {
    pub(super) rgba_data: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) origin_x: i32,
    pub(super) origin_y: i32,
    pub(super) screens: Vec<Screen>,
    pub(super) layouts: Vec<PhysicalScreenLayout>,
}

/// Get the current cursor position in virtual screen coordinates.
///
/// Returns `(x, y)` in logical (virtual desktop) pixels.
#[cfg(windows)]
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

/// Find the screen that contains the cursor and capture only that screen.
///
/// Returns a `VirtualDesktopCapture` containing just the one screen,
/// positioned so the overlay window covers exactly that monitor.
///
/// The overlay intentionally covers only the monitor under the cursor. An
/// earlier design composited every screen into one window spanning the whole
/// virtual desktop (see [`capture_virtual_desktop`], kept as dead code). That
/// was abandoned: a single window spanning mixed-DPI monitors gets stretched
/// and misplaced by the Windows compositor, and recording regions have to be
/// single-monitor anyway (see [`resolve_recording_selection`]). Do not restore
/// the spanning overlay without solving the mixed-DPI stretching first.
pub(super) fn capture_screen_at_cursor() -> Result<VirtualDesktopCapture, String> {
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

    #[cfg(windows)]
    let img = crate::record::capture::capture_monitor_image(info.raw_handle.0)?;
    #[cfg(not(windows))]
    let img = screen
        .capture()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;

    let phys_w = img.width();
    let phys_h = img.height();

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
        rgba_data: img.into_raw(),
        width: phys_w,
        height: phys_h,
        origin_x: window_layout.physical_x,
        origin_y: window_layout.physical_y,
        screens: vec![screen],
        layouts: vec![local_layout],
    })
}

pub(super) fn capture_selection_desktop_with<CaptureScreen>(
    capture_screen: CaptureScreen,
) -> Result<VirtualDesktopCapture, String>
where
    CaptureScreen: FnOnce() -> Result<VirtualDesktopCapture, String>,
{
    capture_screen()
}

/// Capture all screens and composite them into a single physical virtual desktop image.
#[allow(dead_code)]
pub(super) fn capture_virtual_desktop() -> Result<VirtualDesktopCapture, String> {
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
        width: total_width,
        height: total_height,
        origin_x,
        origin_y,
        screens: captures.into_iter().map(|(s, _)| s).collect(),
        layouts,
    })
}

/// Show the selection overlay and return the cropped screenshot.
///
/// Captures the screen under the cursor, displays a fullscreen overlay,
/// waits for the user to drag a rectangle, and returns the cropped RGBA image.
pub fn select_region() -> Result<image::RgbaImage, String> {
    let desktop = capture_selection_desktop_with(capture_screen_at_cursor)?;
    let width = desktop.width;
    let height = desktop.height;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = SelectionApp::new(
        desktop.rgba_data,
        width,
        height,
        desktop.origin_x,
        desktop.origin_y,
    );
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("Event loop error: {}", e))?;

    if let Some(err) = app.setup_error {
        return Err(err);
    }

    if app.cancelled {
        return Err("Selection cancelled".to_string());
    }

    let region = app.result.ok_or("No region selected")?;

    let composite_data = std::mem::take(&mut app.screenshot);
    let composite_img = image::RgbaImage::from_raw(width, height, composite_data)
        .ok_or("Failed to create image from composite data")?;

    if region.physical_x < 0 || region.physical_y < 0 {
        return Err("Selection region has negative coordinates".to_string());
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

/// Run the selection UI and return the selected screen and region (screen-local coords).
/// Used by the recording feature.
pub fn select_region_for_recording() -> Result<(screenshots::Screen, Region), String> {
    let desktop = capture_selection_desktop_with(capture_screen_at_cursor)?;
    let width = desktop.width;
    let height = desktop.height;

    let event_loop = EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = SelectionApp::new(
        desktop.rgba_data,
        width,
        height,
        desktop.origin_x,
        desktop.origin_y,
    );
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("Event loop error: {}", e))?;

    if let Some(err) = app.setup_error {
        return Err(err);
    }

    if app.cancelled {
        return Err("Selection cancelled".to_string());
    }

    let region = app.result.ok_or("No region selected")?;
    let (screen_index, local_region) = resolve_recording_selection(&desktop.layouts, region)?;
    let screen = desktop
        .screens
        .into_iter()
        .nth(screen_index)
        .ok_or_else(|| "Selected region does not fall on any screen".to_string())?;

    Ok((screen, local_region))
}
