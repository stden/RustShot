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
        // SAFETY: `CoIncrementMTAUsage` takes no arguments and no pointers we
        // own, and is documented as callable from any thread at any time -
        // including before this thread has entered an apartment. The returned
        // cookie is dropped without calling `CoDecrementMTAUsage`; that is the
        // intended leak, and it is a no-op drop because windows-rs models the
        // cookie as a bare `#[repr(transparent)]` handle with no `Drop` impl.
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
    receive_worker_message(receiver, timeout, backend).and_then(|result| result)
}

/// Wait for one message from a capture worker, or give up.
///
/// The payload is returned untouched, so this works for workers whose result is
/// not a `Result<_, String>`. Like [`receive_capture_result`], the point is that
/// the caller never joins: a synchronous Win32 or GPU-driver call inside the
/// worker can block indefinitely, and dropping the `JoinHandle` detaches the
/// thread so the timeout is real rather than advisory.
pub(super) fn receive_worker_message<T>(
    receiver: std::sync::mpsc::Receiver<T>,
    timeout: std::time::Duration,
    backend: &str,
) -> Result<T, String> {
    match receiver.recv_timeout(timeout) {
        Ok(message) => Ok(message),
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
    // Resolved once for the frame, not once per row.
    let swizzle = super::PixelSwizzle::detect();
    for row in 0..height as usize {
        let source = &mapped[row * row_pitch..row * row_pitch + row_bytes];
        let destination = &mut rgba[row * row_bytes..(row + 1) * row_bytes];
        swizzle.convert(source, destination);
    }
    Ok(rgba)
}

#[cfg(windows)]
struct ScreenDc(windows::Win32::Graphics::Gdi::HDC);

#[cfg(windows)]
impl Drop for ScreenDc {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the DC returned by the `GetDC(None)` in
        // `capture_hwnd_gdi`, checked non-invalid before the guard was built and
        // released nowhere else. `None` mirrors the `None` window passed to
        // `GetDC`, which is what `ReleaseDC` requires for a screen DC. `Drop`
        // runs once because the guard is neither `Clone` nor `Copy`.
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
        // SAFETY: `self.0` came from a `CreateCompatibleDC` whose result was
        // checked non-invalid, so `DeleteDC` (which is only valid for DCs the
        // caller created, never for a `GetDC` handle) is the right release call.
        // In `capture_hwnd_gdi` the `SelectedObject` guard is declared after this
        // one and therefore drops first, so the capture bitmap has already been
        // deselected from this DC by the time we get here. Runs once: not
        // `Clone`/`Copy`.
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
        // SAFETY: `self.0` is the bitmap from a checked `CreateCompatibleBitmap`,
        // owned solely by this guard. `DeleteObject` requires the object not to be
        // selected into a DC: in `capture_hwnd_gdi` the `SelectedObject` guard is
        // declared after this one, so it has already restored the memory DC's
        // original bitmap (or `restore()` did so explicitly before `GetDIBits`). In
        // the pathological case where both re-selections fail, GDI refuses the
        // delete and returns FALSE — a leak until process exit, not UB. Runs once:
        // not `Clone`/`Copy`.
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
        // SAFETY: `self.dc` is the memory DC owned by the `MemoryDc` guard, which
        // is declared before this one in `capture_hwnd_gdi` and so outlives it,
        // and `previous` is exactly the handle that DC's `SelectObject` returned,
        // so putting it back is valid. `previous` is cleared below, which keeps
        // `Drop` from selecting the same object a second time.
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
            // SAFETY: as in `restore` - `self.dc` is still live (its `MemoryDc`
            // guard is declared earlier and drops later) and `previous` is that
            // DC's own original object. Reaching here means `restore` was not
            // called or failed, so this is the only re-selection.
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
            // SAFETY: the out-buffer is the `rect` local, which lives across the
            // whole call, and the size argument is `size_of::<RECT>()` — exactly
            // the type `DWMWA_EXTENDED_FRAME_BOUNDS` is documented to write, so
            // DWM cannot write past it. A stale `hwnd` is reported as an error
            // HRESULT (`E_HANDLE`), which the `map_err` below turns into `Err`.
            unsafe {
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_EXTENDED_FRAME_BOUNDS,
                    std::ptr::from_mut(&mut rect).cast(),
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
            // SAFETY: `&mut rect` is the single out-parameter and borrows a live
            // stack `RECT` for the duration of the call, so the `*mut RECT`
            // windows-rs passes is valid and correctly sized. A window that has
            // been destroyed makes `GetWindowRect` fail rather than write
            // anywhere, and the failure is mapped to `Err` here.
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
    // Generous, because the worker enforces its own 3-second deadline on the
    // frame itself: this bound exists for the calls around that deadline —
    // creating the capture item, the D3D11 device, starting the session, and
    // mapping the staging texture — none of which time out on their own. The
    // GDI path below never joins its worker for exactly this reason; joining
    // here left a wedged GPU driver able to hang the caller forever.
    const WGC_CAPTURE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

    let sendable = SendableHwnd::from(hwnd);
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("rustshot-window-capture".to_string())
        .spawn(move || {
            let _ = sender.send(wgc::capture_hwnd(sendable.hwnd()));
        })
        .map_err(|error| format!("Failed to start the window capture worker: {error}"))?;

    // Never join: detaching keeps the timeout meaningful if the worker is stuck
    // inside a synchronous driver call.
    drop(worker);

    let outcome = receive_worker_message(
        receiver,
        WGC_CAPTURE_TIMEOUT,
        "Windows Graphics Capture window capture",
    );

    match outcome {
        Ok(Ok(image)) => Ok(image),
        Ok(Err(wgc::WgcCaptureError::Unsupported(reason))) => {
            log::debug!("Windows Graphics Capture unavailable ({reason}); using GDI fallback");
            capture_hwnd_gdi_with_timeout(hwnd)
        }
        Ok(Err(wgc::WgcCaptureError::Failed(error))) => {
            log::warn!("Windows Graphics Capture failed ({error}); using GDI fallback");
            capture_hwnd_gdi_with_timeout(hwnd).map_err(|gdi_error| {
                format!(
                    "Windows Graphics Capture failed ({error}); GDI fallback also failed: {gdi_error}"
                )
            })
        }
        Err(error) => {
            log::warn!("{error}; using GDI fallback");
            capture_hwnd_gdi_with_timeout(hwnd)
                .map_err(|gdi_error| format!("{error}; GDI fallback also failed: {gdi_error}"))
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
    //
    // SAFETY: no pointers are passed, and `None` means the call needs no valid
    // window handle. The returned DC is checked immediately below and then handed
    // to the `ScreenDc` guard, which is the only owner and releases it once.
    let hdc_screen = unsafe { GetDC(None) };
    if hdc_screen.is_invalid() {
        return Err("Failed to acquire the screen device context".to_string());
    }
    let hdc_screen = ScreenDc(hdc_screen);

    // SAFETY: `hdc_screen.0` is the screen DC just checked non-invalid and kept
    // alive by its guard, so it is a valid template for the new memory DC; the
    // call takes no pointers. The result is checked before it is used.
    let hdc_mem = unsafe { CreateCompatibleDC(Some(hdc_screen.0)) };
    if hdc_mem.is_invalid() {
        return Err("Failed to create a compatible device context".to_string());
    }
    let hdc_mem = MemoryDc(hdc_mem);

    // SAFETY: same live screen DC, and no pointers. `width`/`height` come from
    // `window.dimensions()`, which already rejected non-positive sizes; the casts
    // are pure size arguments, and a size GDI cannot honour makes the call return
    // a null bitmap, which the check below rejects rather than using.
    let hbm = unsafe { CreateCompatibleBitmap(hdc_screen.0, width as i32, height as i32) };
    if hbm.is_invalid() {
        return Err("Failed to create a compatible bitmap".to_string());
    }
    let hbm = GdiBitmap(hbm);

    // SAFETY: both arguments are the checked, guard-owned memory DC and bitmap,
    // and the bitmap is not selected into any other DC (it was created two lines
    // above and never shared). The returned previous object is immediately moved
    // into a `SelectedObject` guard, so it is restored on every exit path.
    let old_obj = unsafe { SelectObject(hdc_mem.0, hbm.0.into()) };
    if old_obj.is_invalid() {
        return Err("Failed to select the capture bitmap".to_string());
    }
    let mut selected = SelectedObject {
        dc: hdc_mem.0,
        previous: Some(old_obj),
    };

    // SAFETY: `IsHungAppWindow` only inspects the handle and takes no pointers; a
    // handle that no longer names a window is reported as "not hung" rather than
    // faulting, and the `PrintWindow` below then fails into the BitBlt fallback.
    let printed = if unsafe { IsHungAppWindow(hwnd) }.as_bool() {
        log::warn!("Target window is not responding; skipping PrintWindow");
        false
    } else {
        // SAFETY: `hdc_mem.0` is the live memory DC that still has the capture
        // bitmap selected (`selected.restore()` runs only further down), so
        // PrintWindow draws into a `width`x`height` surface it cannot overrun.
        // No pointers are passed, and a window that vanished between the hang
        // check and here makes the call return FALSE.
        unsafe { PrintWindow(hwnd, hdc_mem.0, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)) }.as_bool()
    };
    if !printed {
        // Last resort: copy the visible frame pixels from the screen into
        // the same window-local position, so the crop below stays valid.
        let raster_operation = ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0);
        // SAFETY: source and destination are the two live, guard-owned DCs. The
        // destination rectangle is the crop returned by `frame_crop_within_window`,
        // which clamped the frame into the `width`x`height` window rectangle the
        // selected bitmap was created from, so the blit stays inside that bitmap.
        // GDI clips the source read to the virtual screen itself. No pointers.
        unsafe {
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
        }
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
    // SAFETY: `bmi` describes an uncompressed 32-bpp top-down DIB of exactly
    // `width`x`height`, whose rows are therefore `width * 4` bytes with no
    // padding, and `height` scan lines are requested starting at line 0 — that is
    // `width * height * 4` == `buffer_len` bytes, the exact length of `pixels`.
    // Both `pixels` and `bmi` are live locals borrowed only by this call. The
    // bitmap was just deselected by `restore()`, which is what GetDIBits demands,
    // and `hdc_mem.0`/`hbm.0` are the checked, guard-owned DC and bitmap.
    let scan_lines = unsafe {
        GetDIBits(
            hdc_mem.0,
            hbm.0,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

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

/// The full window rectangle (`GetWindowRect`), which is the surface
/// `PrintWindow` renders — including the invisible resize borders.
#[cfg(windows)]
fn get_window_rect_bounds(hwnd: windows::Win32::Foundation::HWND) -> Result<WindowBounds, String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT::default();
    // SAFETY: `&mut rect` borrows a live stack `RECT` for the whole call and is
    // the only out-parameter, so the pointer windows-rs derives from it is valid
    // and exactly `size_of::<RECT>()` bytes wide. A destroyed window makes the
    // call fail, which is mapped to `Err` instead of yielding a garbage rect.
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
