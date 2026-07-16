//! Platform-specific active-window capture: WinAPI on Windows, an explicit error elsewhere.

#[cfg(windows)]
mod wgc;

/// Pin the process-wide COM multithreaded apartment for the lifetime of the
/// process.
///
/// The WGC backends initialize WinRT on short-lived worker threads and
/// uninitialize it when the thread exits. If that `RoUninitialize` closes the
/// LAST thread in the MTA, the apartment is destroyed - and COM objects still
/// cached elsewhere in the process (DXGI factories, the `screenshots` crate's
/// internals, frame-pool callbacks) become dangling, so the next capture or
/// the process teardown dies with STATUS_ACCESS_VIOLATION. This was observed
/// as: CI test binaries crashing on the capture test AFTER a window-capture
/// test passed, criterion crashing at exit, and the app crashing after
/// --window-id captures.
///
/// `CoIncrementMTAUsage` is the documented fix: it keeps the MTA alive without
/// dedicating a thread. The cookie is deliberately leaked - the apartment must
/// live until process exit, where the OS reclaims everything.
#[cfg(windows)]
pub(crate) fn pin_process_mta() {
    use std::sync::OnceLock;
    static MTA_PIN: OnceLock<()> = OnceLock::new();
    MTA_PIN.get_or_init(|| {
        // Failure (ancient Windows) degrades to the old per-thread behavior.
        let _ = unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() };
    });
}

/// A window handle detached from windows-rs's raw-pointer wrapper so it can
/// cross threads.
///
/// `HWND` wraps `*mut c_void` in windows 0.62 and is therefore not `Send`,
/// but the underlying value is a process-global identifier with no thread
/// affinity. Storing it as `isize` makes the carrier `Send` by construction -
/// no `unsafe impl` needed - and the only way back out is [`Self::hwnd`],
/// which re-wraps it for the destination thread.
#[cfg(windows)]
#[derive(Clone, Copy)]
pub(super) struct SendableHwnd(isize);

#[cfg(windows)]
impl From<windows::Win32::Foundation::HWND> for SendableHwnd {
    fn from(hwnd: windows::Win32::Foundation::HWND) -> Self {
        Self(hwnd.0 as isize)
    }
}

#[cfg(windows)]
impl SendableHwnd {
    pub(super) fn hwnd(self) -> windows::Win32::Foundation::HWND {
        windows::Win32::Foundation::HWND(self.0 as *mut core::ffi::c_void)
    }
}

/// Physical screen-space bounds used by the Windows capture backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowBounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl WindowBounds {
    pub(super) const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub(super) fn dimensions(self) -> Result<(u32, u32), String> {
        let width = i64::from(self.right) - i64::from(self.left);
        let height = i64::from(self.bottom) - i64::from(self.top);
        if width <= 0 || height <= 0 {
            return Err("Window has zero or negative size".to_string());
        }

        let width = u32::try_from(width).map_err(|_| "Window width is too large".to_string())?;
        let height = u32::try_from(height).map_err(|_| "Window height is too large".to_string())?;
        Ok((width, height))
    }
}

/// Prefer compositor-provided visible bounds and fall back to classic Win32 bounds.
pub(super) fn resolve_window_bounds<Preferred, Fallback>(
    preferred: Preferred,
    fallback: Fallback,
) -> Result<WindowBounds, String>
where
    Preferred: FnOnce() -> Result<WindowBounds, String>,
    Fallback: FnOnce() -> Result<WindowBounds, String>,
{
    match preferred().and_then(|bounds| bounds.dimensions().map(|_| bounds)) {
        Ok(bounds) => Ok(bounds),
        Err(_) => fallback().and_then(|bounds| bounds.dimensions().map(|_| bounds)),
    }
}

/// Convert GDI's 32-bit BGRX output to an opaque RGBA buffer.
pub(super) fn gdi_bgrx_to_rgba(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = u8::MAX;
    }
}

/// Locate the DWM extended-frame rectangle inside the full window rectangle,
/// as a crop in window-local pixels: `(x, y, width, height)`.
///
/// `PrintWindow` renders the entire `GetWindowRect` surface (including the
/// invisible resize borders), while screenshots should show only the visible
/// extended frame. DWM occasionally reports frame bounds a pixel outside the
/// window rectangle, so the frame is clamped to the rendered surface rather
/// than trusted blindly.
pub(super) fn frame_crop_within_window(
    window: WindowBounds,
    frame: WindowBounds,
) -> Result<(u32, u32, u32, u32), String> {
    window.dimensions()?;
    let clamped = WindowBounds::new(
        frame.left.max(window.left),
        frame.top.max(window.top),
        frame.right.min(window.right),
        frame.bottom.min(window.bottom),
    );
    if clamped.left >= clamped.right || clamped.top >= clamped.bottom {
        return Err(format!(
            "Window frame ({}, {}, {}, {}) lies outside the window rectangle ({}, {}, {}, {})",
            frame.left,
            frame.top,
            frame.right,
            frame.bottom,
            window.left,
            window.top,
            window.right,
            window.bottom,
        ));
    }
    let (width, height) = clamped.dimensions()?;
    Ok((
        (clamped.left - window.left) as u32,
        (clamped.top - window.top) as u32,
        width,
        height,
    ))
}

/// `GetDIBits` is successful only when every requested scan line was copied.
pub(super) fn validate_gdi_scan_lines(actual: i32, expected: u32) -> Result<(), String> {
    if actual == expected as i32 {
        Ok(())
    } else {
        Err(format!(
            "GetDIBits copied {actual} of {expected} requested scan lines"
        ))
    }
}

/// Wait for a capture worker without ever joining a potentially blocked native call.
///
/// Dropping the worker's `JoinHandle` detaches it, so a synchronous Win32 API
/// such as `PrintWindow` cannot hold up the caller indefinitely.
pub(super) fn receive_capture_result<T>(
    receiver: std::sync::mpsc::Receiver<Result<T, String>>,
    timeout: std::time::Duration,
    backend: &str,
) -> Result<T, String> {
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(format!(
            "{backend} timed out after {} ms",
            timeout.as_millis()
        )),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(format!(
            "{backend} worker stopped without returning a result"
        )),
    }
}

/// Copy a mapped BGRA texture into tightly packed RGBA rows.
pub(super) fn rgba_from_bgra_rows(
    mapped: &[u8],
    row_pitch: usize,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("Captured frame has zero width or height".to_string());
    }

    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Captured frame row is too large".to_string())?;
    if row_pitch < row_bytes {
        return Err(format!(
            "Mapped row pitch {row_pitch} is smaller than the {row_bytes}-byte content row"
        ));
    }

    let last_row_offset = row_pitch
        .checked_mul(height.saturating_sub(1) as usize)
        .ok_or_else(|| "Mapped frame buffer is too large".to_string())?;
    let required_len = last_row_offset
        .checked_add(row_bytes)
        .ok_or_else(|| "Mapped frame buffer is too large".to_string())?;
    if mapped.len() < required_len {
        return Err(format!(
            "Mapped frame buffer has {} bytes but {required_len} are required",
            mapped.len()
        ));
    }

    let output_len = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(|| "Captured image is too large".to_string())?;
    let mut rgba = vec![0u8; output_len];
    for row in 0..height as usize {
        let source = &mapped[row * row_pitch..row * row_pitch + row_bytes];
        let destination = &mut rgba[row * row_bytes..(row + 1) * row_bytes];
        for (bgra, rgba) in source
            .as_chunks::<4>()
            .0
            .iter()
            .zip(destination.as_chunks_mut::<4>().0.iter_mut())
        {
            *rgba = [bgra[2], bgra[1], bgra[0], bgra[3]];
        }
    }
    Ok(rgba)
}

#[cfg(windows)]
struct ScreenDc(windows::Win32::Graphics::Gdi::HDC);

#[cfg(windows)]
impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, self.0);
        }
    }
}

#[cfg(windows)]
struct MemoryDc(windows::Win32::Graphics::Gdi::HDC);

#[cfg(windows)]
impl Drop for MemoryDc {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::DeleteDC(self.0);
        }
    }
}

#[cfg(windows)]
struct GdiBitmap(windows::Win32::Graphics::Gdi::HBITMAP);

#[cfg(windows)]
impl Drop for GdiBitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::DeleteObject(self.0.into());
        }
    }
}

#[cfg(windows)]
struct SelectedObject {
    dc: windows::Win32::Graphics::Gdi::HDC,
    previous: Option<windows::Win32::Graphics::Gdi::HGDIOBJ>,
}

#[cfg(windows)]
impl SelectedObject {
    fn restore(&mut self) -> Result<(), String> {
        let Some(previous) = self.previous else {
            return Ok(());
        };
        let restored = unsafe { windows::Win32::Graphics::Gdi::SelectObject(self.dc, previous) };
        if restored.is_invalid() {
            return Err("Failed to restore the previous GDI object".to_string());
        }
        self.previous = None;
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for SelectedObject {
    fn drop(&mut self) {
        if let Some(previous) = self.previous {
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::SelectObject(self.dc, previous);
            }
        }
    }
}

#[cfg(windows)]
pub(super) fn capture_active_window_platform() -> Result<image::RgbaImage, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    // SAFETY: `GetForegroundWindow` has no preconditions and returns a handle we
    // null-check before use.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd == HWND::default() {
        return Err("No foreground window found".to_string());
    }
    capture_hwnd(hwnd)
}

#[cfg(windows)]
fn get_window_bounds(hwnd: windows::Win32::Foundation::HWND) -> Result<WindowBounds, String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    resolve_window_bounds(
        || {
            let mut rect = RECT::default();
            unsafe {
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_EXTENDED_FRAME_BOUNDS,
                    (&mut rect as *mut RECT).cast(),
                    std::mem::size_of::<RECT>() as u32,
                )
            }
            .map_err(|error| format!("Failed to get DWM window bounds: {error}"))?;
            Ok(WindowBounds::new(
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
            ))
        },
        || {
            let mut rect = RECT::default();
            unsafe { GetWindowRect(hwnd, &mut rect) }
                .map_err(|error| format!("Failed to get window rect: {error}"))?;
            Ok(WindowBounds::new(
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
            ))
        },
    )
}

/// Capture a single top-level window by its handle into an RGBA image.
///
/// Shared by active-window capture (foreground `HWND`) and
/// [`crate::window::capture_window_image`] (an `HWND` chosen from the window
/// list). Windows Graphics Capture is the primary backend, so compositor-owned
/// window pixels remain available when another window overlaps the target.
/// Older Windows versions and recoverable WGC failures use a PrintWindow GDI
/// fallback.
#[cfg(windows)]
pub(crate) fn capture_hwnd(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<image::RgbaImage, String> {
    let sendable = SendableHwnd::from(hwnd);
    let worker = std::thread::Builder::new()
        .name("rustshot-window-capture".to_string())
        .spawn(move || wgc::capture_hwnd(sendable.hwnd()))
        .map_err(|error| format!("Failed to start the window capture worker: {error}"))?;

    match worker
        .join()
        .map_err(|_| "Window capture worker panicked".to_string())?
    {
        Ok(image) => Ok(image),
        Err(wgc::WgcCaptureError::Unsupported(reason)) => {
            log::debug!("Windows Graphics Capture unavailable ({reason}); using GDI fallback");
            capture_hwnd_gdi_with_timeout(hwnd)
        }
        Err(wgc::WgcCaptureError::Failed(error)) => {
            log::warn!("Windows Graphics Capture failed ({error}); using GDI fallback");
            capture_hwnd_gdi_with_timeout(hwnd).map_err(|gdi_error| {
                format!(
                    "Windows Graphics Capture failed ({error}); GDI fallback also failed: {gdi_error}"
                )
            })
        }
    }
}

#[cfg(windows)]
fn capture_hwnd_gdi_with_timeout(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<image::RgbaImage, String> {
    const GDI_CAPTURE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

    let sendable = SendableHwnd::from(hwnd);
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("rustshot-gdi-window-capture".to_string())
        .spawn(move || {
            let result = capture_hwnd_gdi(sendable.hwnd());
            let _ = sender.send(result);
        })
        .map_err(|error| format!("Failed to start the GDI capture worker: {error}"))?;

    // Never join here: PrintWindow is synchronous and may remain blocked in
    // another process. Dropping the handle detaches the worker on timeout.
    drop(worker);
    receive_capture_result(receiver, GDI_CAPTURE_TIMEOUT, "GDI window capture")
}

/// Legacy GDI fallback for Windows versions without Graphics Capture.
///
/// Renders the window itself via `PrintWindow(PW_RENDERFULLCONTENT)`, so the
/// capture stays correct even when other windows overlap the target — the
/// screen-DC `BitBlt` used previously copied whatever was visible at those
/// coordinates. The blit remains as a last resort for windows that refuse
/// `PrintWindow` (some DirectX-exclusive or protected surfaces).
#[cfg(windows)]
fn capture_hwnd_gdi(hwnd: windows::Win32::Foundation::HWND) -> Result<image::RgbaImage, String> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, GetDC, GetDIBits, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, ROP_CODE, SRCCOPY,
    };
    use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::{IsHungAppWindow, PW_RENDERFULLCONTENT};

    // SAFETY: handles returned by GDI are checked before use. RAII guards restore
    // the selected bitmap and release every resource on all return paths.
    unsafe {
        // PrintWindow renders the full GetWindowRect surface; the visible
        // extended frame is cropped out of it afterwards.
        let window = get_window_rect_bounds(hwnd)?;
        let (width, height) = window.dimensions()?;
        let frame = get_window_bounds(hwnd)?;
        let (crop_x, crop_y, crop_w, crop_h) = frame_crop_within_window(window, frame)?;

        // `GetDC(None)` is deliberate: a null window handle returns the DC for
        // the entire screen, which is what BitBlt needs here. Passing
        // `Some(HWND::default())` is NOT equivalent — that is a real (zero)
        // handle, not "no window" — and is incorrect Win32 usage.
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("Failed to acquire the screen device context".to_string());
        }
        let hdc_screen = ScreenDc(hdc_screen);

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen.0));
        if hdc_mem.is_invalid() {
            return Err("Failed to create a compatible device context".to_string());
        }
        let hdc_mem = MemoryDc(hdc_mem);

        let hbm = CreateCompatibleBitmap(hdc_screen.0, width as i32, height as i32);
        if hbm.is_invalid() {
            return Err("Failed to create a compatible bitmap".to_string());
        }
        let hbm = GdiBitmap(hbm);

        let old_obj = SelectObject(hdc_mem.0, hbm.0.into());
        if old_obj.is_invalid() {
            return Err("Failed to select the capture bitmap".to_string());
        }
        let mut selected = SelectedObject {
            dc: hdc_mem.0,
            previous: Some(old_obj),
        };

        let printed = if IsHungAppWindow(hwnd).as_bool() {
            log::warn!("Target window is not responding; skipping PrintWindow");
            false
        } else {
            PrintWindow(hwnd, hdc_mem.0, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)).as_bool()
        };
        if !printed {
            // Last resort: copy the visible frame pixels from the screen into
            // the same window-local position, so the crop below stays valid.
            let raster_operation = ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0);
            BitBlt(
                hdc_mem.0,
                crop_x as i32,
                crop_y as i32,
                crop_w as i32,
                crop_h as i32,
                Some(hdc_screen.0),
                frame.left,
                frame.top,
                raster_operation,
            )
            .map_err(|error| format!("Failed to copy window pixels from the screen: {error}"))?;
        }

        // Read pixels via GetDIBits.
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let buffer_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "Window pixel buffer is too large".to_string())?;
        let mut pixels = vec![0u8; buffer_len];

        // Microsoft requires the bitmap not to be selected into any DC while
        // GetDIBits reads it.
        selected.restore()?;
        let scan_lines = GetDIBits(
            hdc_mem.0,
            hbm.0,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        validate_gdi_scan_lines(scan_lines, height)?;

        // Crop the visible frame out of the full window surface, then force
        // opaque alpha (the GDI X byte is undefined padding).
        let window_row = width as usize * 4;
        let crop_row = crop_w as usize * 4;
        let mut cropped = vec![0u8; crop_row * crop_h as usize];
        for row in 0..crop_h as usize {
            let src_start = (crop_y as usize + row) * window_row + crop_x as usize * 4;
            cropped[row * crop_row..(row + 1) * crop_row]
                .copy_from_slice(&pixels[src_start..src_start + crop_row]);
        }
        gdi_bgrx_to_rgba(&mut cropped);

        image::RgbaImage::from_raw(crop_w, crop_h, cropped)
            .ok_or_else(|| "Failed to create image from captured pixels".to_string())
    }
}

/// The full window rectangle (`GetWindowRect`), which is the surface
/// `PrintWindow` renders — including the invisible resize borders.
#[cfg(windows)]
fn get_window_rect_bounds(hwnd: windows::Win32::Foundation::HWND) -> Result<WindowBounds, String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|error| format!("Failed to get window rectangle: {error}"))?;
    Ok(WindowBounds::new(
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
    ))
}

#[cfg(not(windows))]
pub(super) fn capture_active_window_platform() -> Result<image::RgbaImage, String> {
    Err("Active window capture is only supported on Windows".to_string())
}
