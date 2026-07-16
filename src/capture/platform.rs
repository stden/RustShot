//! Platform-specific active-window capture: WinAPI on Windows, an explicit error elsewhere.

#[cfg(windows)]
mod wgc;

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
            let _ = windows::Win32::Graphics::Gdi::DeleteObject(self.0);
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
/// list), so the GDI blit lives in exactly one place.
///
/// The capture is a screen-DC `BitBlt` of the window's screen rectangle, which
/// means it grabs whatever is currently on screen at those coordinates. A window
/// that is minimised or fully occluded therefore cannot be captured this way;
/// callers are expected to surface a foreground window first.
#[cfg(windows)]
pub(crate) fn capture_hwnd(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<image::RgbaImage, String> {
    let raw_hwnd = hwnd.0;
    let worker = std::thread::Builder::new()
        .name("rustshot-window-capture".to_string())
        .spawn(move || wgc::capture_hwnd(windows::Win32::Foundation::HWND(raw_hwnd)))
        .map_err(|error| format!("Failed to start the window capture worker: {error}"))?;

    match worker
        .join()
        .map_err(|_| "Window capture worker panicked".to_string())?
    {
        Ok(image) => Ok(image),
        Err(wgc::WgcCaptureError::Unsupported(reason)) => {
            log::debug!("Windows Graphics Capture unavailable ({reason}); using GDI fallback");
            capture_hwnd_gdi(hwnd)
        }
        Err(wgc::WgcCaptureError::Failed(error)) => Err(error),
    }
}

/// Legacy visible-pixel fallback for Windows versions without Graphics Capture.
#[cfg(windows)]
fn capture_hwnd_gdi(hwnd: windows::Win32::Foundation::HWND) -> Result<image::RgbaImage, String> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, GetDC, GetDIBits, SelectObject,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, ROP_CODE, SRCCOPY,
    };

    // SAFETY: handles returned by GDI are checked before use. RAII guards restore
    // the selected bitmap and release every resource on all return paths.
    unsafe {
        let bounds = get_window_bounds(hwnd)?;
        let (width, height) = bounds.dimensions()?;

        // `GetDC(None)` is deliberate: a null window handle returns the DC for
        // the entire screen, which is what BitBlt needs here. Passing
        // `Some(HWND::default())` is NOT equivalent — that is a real (zero)
        // handle, not "no window" — and is incorrect Win32 usage.
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("Failed to acquire the screen device context".to_string());
        }
        let hdc_screen = ScreenDc(hdc_screen);

        let hdc_mem = CreateCompatibleDC(hdc_screen.0);
        if hdc_mem.is_invalid() {
            return Err("Failed to create a compatible device context".to_string());
        }
        let hdc_mem = MemoryDc(hdc_mem);

        let hbm = CreateCompatibleBitmap(hdc_screen.0, width as i32, height as i32);
        if hbm.is_invalid() {
            return Err("Failed to create a compatible bitmap".to_string());
        }
        let hbm = GdiBitmap(hbm);

        let old_obj = SelectObject(hdc_mem.0, hbm.0);
        if old_obj.is_invalid() {
            return Err("Failed to select the capture bitmap".to_string());
        }
        let mut selected = SelectedObject {
            dc: hdc_mem.0,
            previous: Some(old_obj),
        };

        let raster_operation = ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0);
        BitBlt(
            hdc_mem.0,
            0,
            0,
            width as i32,
            height as i32,
            hdc_screen.0,
            bounds.left,
            bounds.top,
            raster_operation,
        )
        .map_err(|error| format!("Failed to copy window pixels from the screen: {error}"))?;

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
        gdi_bgrx_to_rgba(&mut pixels);

        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Failed to create image from captured pixels".to_string())
    }
}

#[cfg(not(windows))]
pub(super) fn capture_active_window_platform() -> Result<image::RgbaImage, String> {
    Err("Active window capture is only supported on Windows".to_string())
}
