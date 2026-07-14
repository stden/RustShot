//! Screenshot capture and file saving.
//!
//! This module handles the core screenshot workflow:
//! 1. Resolve which screen to capture (`resolve_screen_index`, `default_screen_index`).
//! 2. Capture the screen pixels via the `screenshots` crate.
//! 3. Determine the output path (`resolve_output_path`) and ensure parent dirs exist.
//! 4. Save the image in the requested format (PNG / JPG / BMP).
//!
//! The public entry point is [`capture_screenshot`].  An internal
//! [`capture_screenshot_with`] variant accepts injected closures for unit testing
//! without real screen hardware.

use image::ImageFormat;
use screenshots::Screen;
use std::fs;
use std::path::{Path, PathBuf};

/// Supported output image formats.
///
/// This enum is the single source of truth for "which formats exist". Settings
/// store it directly rather than a `String`, so an unsupported format is not
/// representable once parsed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Png,
    Jpg,
    Bmp,
}

impl Format {
    /// Every supported format, in settings/UI order. The first entry is the
    /// default used as the fallback for unrecognised input.
    pub const ALL: [Format; 3] = [Format::Png, Format::Jpg, Format::Bmp];

    /// Map this format to the corresponding [`image::ImageFormat`] variant.
    pub fn to_image_format(self) -> ImageFormat {
        match self {
            Format::Png => ImageFormat::Png,
            Format::Jpg => ImageFormat::Jpeg,
            Format::Bmp => ImageFormat::Bmp,
        }
    }

    /// File extension string (without leading dot) for this format. This is
    /// also exactly how the format is written to `settings.json`.
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::Bmp => "bmp",
        }
    }

    /// Parse a format as stored in `settings.json`.
    ///
    /// Deliberately **case-sensitive** and forgiving: anything unrecognised
    /// falls back to PNG rather than failing, because `settings.json` is
    /// hand-editable and a bad value should not brick the app. Note the
    /// contrast with [`crate::cli::format_from_extension`], which *is*
    /// case-insensitive because a user may well type `--output shot.PNG`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustshot::capture::Format;
    ///
    /// assert_eq!(Format::from_settings_str("jpg"), Format::Jpg);
    /// assert_eq!(Format::from_settings_str("webp"), Format::Png); // unknown → default
    /// assert_eq!(Format::from_settings_str("JPG"), Format::Png);  // case-sensitive
    /// ```
    pub fn from_settings_str(value: &str) -> Format {
        Format::ALL
            .into_iter()
            .find(|fmt| fmt.extension() == value)
            .unwrap_or(Format::Png)
    }
}

/// Validate and resolve the screen index. Returns an error if out of range or no screens.
///
/// # Examples
///
/// ```
/// use rustshot::capture::resolve_screen_index;
///
/// // No explicit screen → defaults to primary (index 0)
/// assert_eq!(resolve_screen_index(3, None), Ok(0));
///
/// // Explicit valid index
/// assert_eq!(resolve_screen_index(3, Some(2)), Ok(2));
///
/// // Out of range
/// assert!(resolve_screen_index(2, Some(5)).is_err());
/// ```
///
/// Algorithm:
/// 1. Reject the call immediately if there are no screens.
/// 2. If the user requested an explicit index, bounds-check it.
/// 3. Otherwise fall back to index 0, which matches the project's current
///    default-screen policy for CLI capture.
///
/// Possible improvements:
/// - Delegate the implicit-screen choice to [`default_screen_index`] so CLI and
///   GUI behavior share the same primary-monitor logic.
/// - Return a dedicated error type if callers ever need to distinguish
///   "no screens" from "bad index" without parsing strings.
pub fn resolve_screen_index(
    screen_count: usize,
    requested: Option<usize>,
) -> Result<usize, String> {
    if screen_count == 0 {
        return Err("No screens detected".to_string());
    }

    match requested {
        Some(index) if index >= screen_count => Err(format!(
            "Screen index {} is out of range (0-{})",
            index,
            screen_count - 1
        )),
        Some(index) => Ok(index),
        None => Ok(0),
    }
}

/// Find the primary screen index, falling back to the first screen.
///
/// Algorithm:
/// 1. Walk the iterator once from left to right.
/// 2. Remember the first index as a fallback candidate.
/// 3. Return immediately when a primary screen is encountered.
/// 4. If no primary flag is present, return the fallback index.
///
/// This keeps the function single-pass and allocation-free, which matters more
/// here than asymptotic cleverness because monitor counts are tiny in practice.
///
/// Possible improvements:
/// - Return richer metadata when multiple primary flags are present so callers
///   can detect inconsistent platform data instead of silently using the first.
/// - Accept screen objects directly if the monitor-selection logic is ever
///   consolidated into one shared screen abstraction.
#[allow(dead_code)]
pub fn default_screen_index<I>(is_primary_flags: I) -> Option<usize>
where
    I: IntoIterator<Item = bool>,
{
    let mut fallback = None;

    for (index, is_primary) in is_primary_flags.into_iter().enumerate() {
        fallback.get_or_insert(index);
        if is_primary {
            return Some(index);
        }
    }

    fallback
}

/// Resolve the output file path: use explicit path if given, otherwise generate a timestamped default.
///
/// # Examples
///
/// ```
/// use rustshot::capture::{resolve_output_path, Format};
/// use std::path::PathBuf;
///
/// // Explicit path is returned as-is
/// let p = resolve_output_path(Some(PathBuf::from("/tmp/shot.png")), Format::Png);
/// assert_eq!(p, PathBuf::from("/tmp/shot.png"));
/// ```
#[allow(dead_code)]
pub fn resolve_output_path(output: Option<PathBuf>, format: Format) -> PathBuf {
    let screenshots_dir = get_screenshots_dir();
    resolve_output_path_in_dir(output, format, &screenshots_dir)
}

/// Resolve the output file path using an explicit screenshots directory.
///
/// Algorithm:
/// 1. If the caller provided an explicit output path, return it unchanged.
/// 2. Otherwise generate a timestamp string.
/// 3. Build `screenshot_<timestamp>.<ext>` inside `screenshots_dir`.
///
/// The function intentionally does not try to rewrite or "fix" explicit paths.
/// That keeps CLI behavior predictable and avoids surprising users who pass a
/// custom filename without an extension on purpose.
///
/// Possible improvements:
/// - Detect collisions and append a suffix when two captures happen inside the
///   same timestamp granularity window.
/// - Allow a configurable filename template if the settings format grows.
pub fn resolve_output_path_in_dir(
    output: Option<PathBuf>,
    format: Format,
    screenshots_dir: &Path,
) -> PathBuf {
    output.unwrap_or_else(|| {
        let timestamp = crate::settings::generate_timestamp();
        screenshots_dir.join(format!("screenshot_{}.{}", timestamp, format.extension()))
    })
}

/// Create parent directories for the given path if they don't exist.
pub(crate) fn ensure_parent_directory(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    Ok(())
}

/// Save an image using optimized encoder settings per format.
///
/// - **PNG:** `CompressionType::Fast` + `FilterType::Sub` (faster than default with minimal size cost)
/// - **JPG:** Quality 90 via explicit `JpegEncoder`
/// - **BMP:** Standard `BmpEncoder`
///
/// All formats use `BufWriter` for efficient I/O.
pub fn save_image_optimized(
    image: &image::RgbaImage,
    path: &Path,
    format: Format,
) -> Result<(), String> {
    use std::io::BufWriter;
    let file = std::fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut writer = BufWriter::new(file);
    match format {
        Format::Png => {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Fast, FilterType::Sub);
            image.write_with_encoder(encoder)
        }
        Format::Jpg => {
            use image::codecs::jpeg::JpegEncoder;
            // JPEG doesn't support RGBA; convert to RGB by dropping the alpha channel.
            let raw: Vec<u8> = image.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
            let rgb = image::RgbImage::from_raw(image.width(), image.height(), raw)
                .ok_or("Failed to convert RGBA to RGB")?;
            let encoder = JpegEncoder::new_with_quality(writer, 90);
            return rgb
                .write_with_encoder(encoder)
                .map_err(|e| format!("Failed to save screenshot: {e}"));
        }
        Format::Bmp => {
            use image::codecs::bmp::BmpEncoder;
            let encoder = BmpEncoder::new(&mut writer);
            image.write_with_encoder(encoder)
        }
    }
    .map_err(|e| format!("Failed to save screenshot: {e}"))
}

/// Capture a screenshot and save it to the specified path or default location
#[allow(dead_code)]
pub fn capture_screenshot(
    output: Option<PathBuf>,
    format: Format,
    screen_index: Option<usize>,
) -> Result<PathBuf, String> {
    let screenshots_dir = get_screenshots_dir();
    capture_screenshot_in_dir(output, format, screen_index, &screenshots_dir)
}

/// Capture a screenshot and save it using the provided screenshots directory
pub fn capture_screenshot_in_dir(
    output: Option<PathBuf>,
    format: Format,
    screen_index: Option<usize>,
    screenshots_dir: &Path,
) -> Result<PathBuf, String> {
    log::debug!(
        "capture_screenshot: format={:?}, screen={:?}",
        format,
        screen_index
    );
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let idx = match screen_index {
        Some(requested) => resolve_screen_index(screens.len(), Some(requested))?,
        None => {
            // Prefer the screen under the cursor, fall back to primary
            let cursor_pos = crate::selection::get_cursor_position().ok();
            let screen_rects: Vec<_> = screens
                .iter()
                .map(|s| {
                    let info = &s.display_info;
                    (info.x, info.y, info.width, info.height, info.is_primary)
                })
                .collect();
            crate::selection::preferred_screen_index(screen_rects, cursor_pos)
                .ok_or_else(|| "No screens detected".to_string())?
        }
    };

    capture_screenshot_with(
        screens.len(),
        Some(idx),
        || {
            let captured = screens[idx]
                .capture()
                .map_err(|e| format!("Failed to capture screen: {}", e))?;
            // Forced version bridge, not a stylistic choice: `screenshots` 0.8
            // pins `image` 0.24 internally while RustShot builds against 0.25,
            // so the two `RgbaImage` types are distinct and the buffer has to be
            // round-tripped through raw bytes. This costs one full-buffer copy
            // per capture; removing it means migrating off `screenshots`
            // (e.g. to `xcap`).
            image::RgbaImage::from_raw(captured.width(), captured.height(), captured.into_raw())
                .ok_or_else(|| "Failed to convert captured image".to_string())
        },
        output,
        format,
        screenshots_dir,
        ensure_parent_directory,
        |image, path, _image_format| save_image_optimized(image, path, format),
    )
}

/// Get the default screenshots directory
#[allow(dead_code)]
pub fn get_screenshots_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
}

/// Internal capture pipeline with dependency injection for unit testing.
#[allow(clippy::too_many_arguments)]
fn capture_screenshot_with<CaptureImage, EnsureDir, SaveImage, Img>(
    screen_count: usize,
    screen_index: Option<usize>,
    capture_image: CaptureImage,
    output: Option<PathBuf>,
    format: Format,
    screenshots_dir: &Path,
    ensure_dir: EnsureDir,
    save_image: SaveImage,
) -> Result<PathBuf, String>
where
    CaptureImage: FnOnce() -> Result<Img, String>,
    EnsureDir: Fn(&Path) -> Result<(), String>,
    SaveImage: FnOnce(&Img, &Path, ImageFormat) -> Result<(), String>,
{
    resolve_screen_index(screen_count, screen_index)?;

    let image = capture_image()?;
    let output_path = resolve_output_path_in_dir(output, format, screenshots_dir);
    ensure_dir(&output_path)?;
    save_image(&image, &output_path, format.to_image_format())?;

    Ok(output_path)
}

/// Capture the currently active (foreground) window and save it to the specified path.
/// Uses WinAPI BitBlt on Windows. Returns an error on other platforms.
#[allow(dead_code)]
pub fn capture_active_window(output: Option<PathBuf>, format: Format) -> Result<PathBuf, String> {
    let screenshots_dir = get_screenshots_dir();
    capture_active_window_in_dir(output, format, &screenshots_dir)
}

/// Capture the active window and save it using the provided screenshots directory.
pub fn capture_active_window_in_dir(
    output: Option<PathBuf>,
    format: Format,
    screenshots_dir: &Path,
) -> Result<PathBuf, String> {
    log::debug!("capture_active_window: format={:?}", format);
    let rgba_image = capture_active_window_image()?;
    let output_path = resolve_output_path_in_dir(output, format, screenshots_dir);
    ensure_parent_directory(&output_path)?;
    save_image_optimized(&rgba_image, &output_path, format)?;
    Ok(output_path)
}

/// Capture the active window as an RgbaImage (for clipboard use).
pub fn capture_active_window_image() -> Result<image::RgbaImage, String> {
    capture_active_window_platform()
}

#[cfg(windows)]
fn capture_active_window_platform() -> Result<image::RgbaImage, String> {
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
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    // SAFETY: every GDI call below is passed handles that are checked immediately
    // after creation, and all GDI objects are released before returning on every
    // path.
    unsafe {
        let mut rect = RECT::default();
        // GetWindowRect gives the full window including its title bar.
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return Err("Failed to get window rect".to_string());
        }

        let (left, top, right, bottom) = (rect.left, rect.top, rect.right, rect.bottom);
        if right <= left || bottom <= top {
            return Err("Window has zero or negative size".to_string());
        }
        let width = (right - left) as u32;
        let height = (bottom - top) as u32;

        // `GetDC(None)` is deliberate: a null window handle returns the DC for
        // the entire screen, which is what BitBlt needs here. Passing
        // `Some(HWND::default())` is NOT equivalent — that is a real (zero)
        // handle, not "no window" — and is incorrect Win32 usage.
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbm = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
        let old_obj = SelectObject(hdc_mem, hbm);

        let _ = BitBlt(
            hdc_mem,
            0,
            0,
            width as i32,
            height as i32,
            hdc_screen,
            left,
            top,
            SRCCOPY,
        );

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

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let scan_lines = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup GDI objects.
        SelectObject(hdc_mem, old_obj);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        if scan_lines == 0 {
            return Err("GetDIBits failed to read window pixels".to_string());
        }

        bgra_to_rgba(&mut pixels);

        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Failed to create image from captured pixels".to_string())
    }
}

#[cfg(not(windows))]
fn capture_active_window_platform() -> Result<image::RgbaImage, String> {
    Err("Active window capture is only supported on Windows".to_string())
}

/// Convert BGRA pixel data to RGBA in place.
///
/// Algorithm:
/// 1. Interpret the byte slice as a sequence of 4-byte pixels.
/// 2. For each complete pixel, swap byte 0 (blue) with byte 2 (red).
/// 3. Leave green and alpha unchanged.
/// 4. Ignore any trailing incomplete chunk to avoid out-of-bounds work.
///
/// The operation is in-place and allocation-free, which keeps the hot path
/// cheap for full-screen captures.
///
/// Possible improvements:
/// - Use SIMD or wider chunking if profiling ever shows this channel swap to be
///   a measurable bottleneck on large monitors.
/// - Validate producer pixel format up front if future capture backends can
///   return layouts other than BGRA.
///
/// # Examples
///
/// ```
/// use rustshot::capture::bgra_to_rgba;
///
/// let mut pixels = vec![0u8, 100, 200, 255]; // B=0, G=100, R=200, A=255
/// bgra_to_rgba(&mut pixels);
/// assert_eq!(pixels, vec![200, 100, 0, 255]); // R=200, G=100, B=0, A=255
/// ```
#[allow(dead_code)]
pub fn bgra_to_rgba(pixels: &mut [u8]) {
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2); // swap B and R
    }
}

/// List all available screens as human-readable strings (e.g. `"Screen 0: 1920x1080"`).
pub fn list_screens() -> Vec<String> {
    Screen::all()
        .map(|screens| {
            screens
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    format!(
                        "Screen {}: {}x{}",
                        i, s.display_info.width, s.display_info.height
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Convert screen enumeration results into an explicit benchmark screen count.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_screen_count_from_result<T>(
    screens: Result<Vec<T>, String>,
) -> Result<usize, String> {
    screens.and_then(|screens| {
        if screens.is_empty() {
            Err("No screens detected".to_string())
        } else {
            Ok(screens.len())
        }
    })
}

/// Extract the first screen for benchmarks or return an explicit error.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_first_screen_from_result<T>(screens: Result<Vec<T>, String>) -> Result<T, String> {
    screens.and_then(|screens| {
        screens
            .into_iter()
            .next()
            .ok_or_else(|| "No screens detected".to_string())
    })
}

/// Convert RGBA benchmark images into JPEG-compatible RGB data.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_rgb_image_from_rgba(image: &image::RgbaImage) -> image::RgbImage {
    image::DynamicImage::ImageRgba8(image.clone()).to_rgb8()
}

/// Expose `save_image_optimized` for benchmarks.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_save_image_optimized(
    image: &image::RgbaImage,
    path: &Path,
    format: Format,
) -> Result<(), String> {
    save_image_optimized(image, path, format)
}

/// Expose `ensure_parent_directory` for benchmarks.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_ensure_parent_directory(path: &Path) -> Result<(), String> {
    ensure_parent_directory(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn format_extension_is_correct() {
        assert_eq!(Format::Png.extension(), "png");
        assert_eq!(Format::Jpg.extension(), "jpg");
        assert_eq!(Format::Bmp.extension(), "bmp");
    }

    #[test]
    fn format_to_image_format_is_correct() {
        assert_eq!(Format::Png.to_image_format(), ImageFormat::Png);
        assert_eq!(Format::Jpg.to_image_format(), ImageFormat::Jpeg);
        assert_eq!(Format::Bmp.to_image_format(), ImageFormat::Bmp);
    }

    #[test]
    fn format_default_is_png() {
        assert!(matches!(Format::default(), Format::Png));
    }

    #[test]
    fn format_clone_and_copy_work() {
        let f = Format::Jpg;
        let g = f; // Copy
        #[allow(clippy::clone_on_copy)]
        let h = f.clone(); // Clone
        assert!(matches!(g, Format::Jpg));
        assert!(matches!(h, Format::Jpg));
    }

    #[test]
    fn format_debug_repr_is_non_empty() {
        assert!(!format!("{:?}", Format::Png).is_empty());
        assert!(!format!("{:?}", Format::Jpg).is_empty());
        assert!(!format!("{:?}", Format::Bmp).is_empty());
    }

    #[test]
    fn screen_index_selection_handles_default_and_bounds() {
        assert_eq!(resolve_screen_index(2, None).unwrap(), 0);
        assert_eq!(resolve_screen_index(2, Some(1)).unwrap(), 1);
        assert!(resolve_screen_index(2, Some(3)).is_err());
        assert_eq!(
            resolve_screen_index(2, Some(3)).unwrap_err(),
            "Screen index 3 is out of range (0-1)"
        );
        assert!(resolve_screen_index(0, None).is_err());
        assert_eq!(
            resolve_screen_index(0, None).unwrap_err(),
            "No screens detected"
        );
    }

    #[test]
    fn resolve_screen_index_boundary_exactly_one_screen_index_zero_is_ok() {
        // screen_count=1, requested=Some(0): index 0 < 1, should succeed
        assert_eq!(resolve_screen_index(1, Some(0)).unwrap(), 0);
    }

    #[test]
    fn resolve_screen_index_last_valid_index_is_ok() {
        // screen_count=3, requested=Some(2): last valid index
        assert_eq!(resolve_screen_index(3, Some(2)).unwrap(), 2);
    }

    #[test]
    fn default_screen_index_prefers_primary_screen() {
        let index = default_screen_index([false, true, false]).unwrap();
        assert_eq!(index, 1);
    }

    #[test]
    fn default_screen_index_falls_back_to_first_screen() {
        let index = default_screen_index([false, false, false]).unwrap();
        assert_eq!(index, 0);
    }

    #[test]
    fn default_screen_index_returns_none_for_empty_input() {
        assert_eq!(default_screen_index(Vec::<bool>::new()), None);
    }

    #[test]
    fn resolve_screen_index_one_screen_index_one_is_err() {
        // screen_count=1, requested=Some(1): 1 >= 1, out of range
        assert!(resolve_screen_index(1, Some(1)).is_err());
        assert_eq!(
            resolve_screen_index(1, Some(1)).unwrap_err(),
            "Screen index 1 is out of range (0-0)"
        );
    }

    #[test]
    fn resolve_screen_index_none_always_returns_zero() {
        // With any positive screen count and None, always returns 0
        assert_eq!(resolve_screen_index(1, None).unwrap(), 0);
        assert_eq!(resolve_screen_index(10, None).unwrap(), 0);
        assert_eq!(resolve_screen_index(100, None).unwrap(), 0);
    }

    #[test]
    fn resolve_screen_index_error_message_contains_range() {
        let err = resolve_screen_index(3, Some(5)).unwrap_err();
        assert!(err.contains("5"));
        assert!(err.contains("0-2"));
    }

    #[test]
    fn output_path_uses_default_format_extension() {
        let default_path = resolve_output_path(None, Format::Bmp);
        assert_eq!(default_path.parent().unwrap(), &get_screenshots_dir());
        assert_eq!(
            default_path.extension().and_then(|ext| ext.to_str()),
            Some("bmp")
        );
        assert!(default_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .starts_with("screenshot_"));

        let explicit = resolve_output_path(Some(PathBuf::from("custom.png")), Format::Png);
        assert_eq!(explicit, PathBuf::from("custom.png"));
    }

    #[test]
    fn resolve_output_path_png_has_png_extension() {
        let path = resolve_output_path(None, Format::Png);
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("png"));
    }

    #[test]
    fn resolve_output_path_jpg_has_jpg_extension() {
        let path = resolve_output_path(None, Format::Jpg);
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("jpg"));
    }

    #[test]
    fn resolve_output_path_bmp_has_bmp_extension() {
        let path = resolve_output_path(None, Format::Bmp);
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("bmp"));
    }

    #[test]
    fn resolve_output_path_explicit_overrides_format() {
        // When an explicit path is given, it is returned verbatim regardless of format
        let explicit = PathBuf::from("my_shot.bmp");
        let result = resolve_output_path(Some(explicit.clone()), Format::Png);
        assert_eq!(result, explicit);
    }

    #[test]
    fn resolve_output_path_default_filename_starts_with_screenshot() {
        let path = resolve_output_path(None, Format::Png);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name.starts_with("screenshot_"));
    }

    #[test]
    fn resolve_output_path_default_parent_is_screenshots_dir() {
        let path = resolve_output_path(None, Format::Jpg);
        assert_eq!(path.parent().unwrap(), &get_screenshots_dir());
    }

    #[test]
    fn resolve_output_path_in_dir_uses_provided_directory() {
        let dir = PathBuf::from("/tmp/custom-shots");
        let path = resolve_output_path_in_dir(None, Format::Bmp, &dir);
        assert_eq!(path.parent(), Some(dir.as_path()));
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("bmp"));
    }

    #[test]
    fn resolve_output_path_in_dir_explicit_path_ignores_directory_and_format() {
        let dir = PathBuf::from("/tmp/custom-shots");
        let explicit = PathBuf::from("nested/final-name.jpg");
        let path = resolve_output_path_in_dir(Some(explicit.clone()), Format::Bmp, &dir);
        assert_eq!(path, explicit);
    }

    #[test]
    fn resolve_output_path_in_dir_default_filename_starts_with_screenshot() {
        let dir = PathBuf::from("custom-shots");
        let path = resolve_output_path_in_dir(None, Format::Png, &dir);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name.starts_with("screenshot_"));
    }

    #[test]
    fn ensure_parent_directory_creates_missing_parents() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rustshot-capture-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file_path = temp_dir.join("nested").join("screenshot.png");
        ensure_parent_directory(&file_path).expect("parent dir should be created");
        assert!(file_path.parent().unwrap().exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn ensure_parent_directory_handles_empty_parent() {
        let path = PathBuf::from("screenshot.png");
        assert!(ensure_parent_directory(&path).is_ok());
    }

    #[test]
    fn ensure_parent_directory_creates_deeply_nested_path() {
        let temp_dir = std::env::temp_dir().join(format!(
            "rustshot-deep-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let deep_path = temp_dir
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .join("shot.png");
        ensure_parent_directory(&deep_path).expect("deep parent dirs should be created");
        assert!(deep_path.parent().unwrap().exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn ensure_parent_directory_idempotent_on_existing_dir() {
        // Calling twice on the same path should succeed both times
        let temp_dir = std::env::temp_dir().join(format!(
            "rustshot-idem-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file_path = temp_dir.join("shot.png");
        ensure_parent_directory(&file_path).expect("first call should succeed");
        ensure_parent_directory(&file_path).expect("second call should also succeed");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn screenshots_dir_is_named_rustshot() {
        let dir = get_screenshots_dir();
        assert_eq!(
            dir.file_name().and_then(|name| name.to_str()),
            Some("RustShot")
        );
    }

    #[test]
    fn get_screenshots_dir_is_under_pictures_or_dot() {
        let dir = get_screenshots_dir();
        // The parent must be either the system pictures dir or "." (current dir)
        let parent = dir.parent().expect("screenshots dir should have a parent");
        let expected = dirs::picture_dir().unwrap_or_else(|| PathBuf::from("."));
        assert_eq!(parent, expected.as_path());
    }

    #[test]
    fn get_screenshots_dir_is_absolute_or_relative_dot() {
        let dir = get_screenshots_dir();
        // Either absolute path (from pictures_dir) or starts with "." (fallback)
        let display = dir.display().to_string();
        let is_absolute = dir.is_absolute();
        let is_relative_dot = display.starts_with('.');
        assert!(
            is_absolute || is_relative_dot,
            "dir should be absolute or relative-to-dot: {}",
            display
        );
    }

    #[test]
    fn resolve_screen_index_zero_count_with_some_index_is_no_screens_error() {
        // Even when a specific index is requested, 0 screens always gives "No screens detected"
        let err = resolve_screen_index(0, Some(5)).unwrap_err();
        assert_eq!(err, "No screens detected");
    }

    #[test]
    fn resolve_screen_index_zero_count_with_none_is_no_screens_error() {
        let err = resolve_screen_index(0, None).unwrap_err();
        assert_eq!(err, "No screens detected");
    }

    #[test]
    fn resolve_screen_index_five_screens_last_valid_is_ok() {
        assert_eq!(resolve_screen_index(5, Some(4)).unwrap(), 4);
    }

    #[test]
    fn resolve_screen_index_five_screens_index_five_is_err() {
        // index == screen_count is out of range
        let err = resolve_screen_index(5, Some(5)).unwrap_err();
        assert!(err.contains("5"), "error should mention index: {err}");
        assert!(err.contains("0-4"), "error should show valid range: {err}");
    }

    #[test]
    fn resolve_output_path_default_filename_contains_only_valid_chars() {
        let path = resolve_output_path(None, Format::Png);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Filename should only contain alphanumeric, underscores and the dot before extension
        assert!(name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
    }

    #[test]
    fn resolve_output_path_explicit_path_ignores_format_extension() {
        // Passing a .txt path with Jpg format should return the .txt path unchanged
        let explicit = PathBuf::from("my_file.txt");
        let result = resolve_output_path(Some(explicit.clone()), Format::Jpg);
        assert_eq!(result.extension().and_then(|e| e.to_str()), Some("txt"));
    }

    #[test]
    fn ensure_parent_directory_with_root_relative_path_is_ok() {
        // A path with no directory component (just a filename at current dir)
        let path = PathBuf::from("just_a_file.png");
        assert!(ensure_parent_directory(&path).is_ok());
    }

    #[test]
    fn resolve_screen_index_usize_max_is_out_of_range() {
        let err = resolve_screen_index(5, Some(usize::MAX)).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn resolve_screen_index_called_twice_same_result() {
        assert_eq!(resolve_screen_index(3, Some(0)).unwrap(), 0);
        assert_eq!(resolve_screen_index(3, Some(0)).unwrap(), 0);
    }

    #[test]
    fn resolve_output_path_two_calls_same_format_same_prefix() {
        let p1 = resolve_output_path(None, Format::Png);
        let p2 = resolve_output_path(None, Format::Png);
        let name1 = p1.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name2 = p2.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name1.starts_with("screenshot_"));
        assert!(name2.starts_with("screenshot_"));
        assert!(name1.ends_with(".png"));
        assert!(name2.ends_with(".png"));
    }

    #[test]
    fn ensure_parent_directory_relative_nested_path() {
        // "a/b.png" has parent "a" which is relative — create_dir_all handles it
        let unique = format!(
            "rustshot-rel-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(&unique).join("b.png");
        assert!(ensure_parent_directory(&path).is_ok());
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join(&unique));
    }

    #[test]
    fn format_extension_returns_static_str() {
        // Verify the returned &str is truly static / compiles as such
        let ext: &str = Format::Png.extension();
        assert_eq!(ext, "png");
        let ext: &str = Format::Jpg.extension();
        assert_eq!(ext, "jpg");
        let ext: &str = Format::Bmp.extension();
        assert_eq!(ext, "bmp");
    }

    #[test]
    fn format_to_image_format_distinct() {
        // Each Format variant must map to a different ImageFormat value
        assert_ne!(Format::Png.to_image_format(), Format::Jpg.to_image_format());
        assert_ne!(Format::Png.to_image_format(), Format::Bmp.to_image_format());
        assert_ne!(Format::Jpg.to_image_format(), Format::Bmp.to_image_format());
    }

    #[test]
    fn get_screenshots_dir_consistent_across_calls() {
        // Two successive calls must return the same path
        let first = get_screenshots_dir();
        let second = get_screenshots_dir();
        assert_eq!(first, second);
    }

    #[test]
    fn resolve_screen_index_returns_exact_requested_value() {
        // screen_count=10, requested=Some(7): 7 < 10, so Ok(7)
        assert_eq!(resolve_screen_index(10, Some(7)).unwrap(), 7);
    }

    #[test]
    fn resolve_output_path_explicit_path_no_parent_is_unchanged() {
        // An explicit path with no directory component is returned verbatim
        let result = resolve_output_path(Some(PathBuf::from("file.png")), Format::Bmp);
        assert_eq!(result, PathBuf::from("file.png"));
    }

    #[test]
    fn format_extension_is_all_lowercase() {
        for ext in [
            Format::Png.extension(),
            Format::Jpg.extension(),
            Format::Bmp.extension(),
        ] {
            assert_eq!(
                ext,
                ext.to_lowercase(),
                "extension '{ext}' should be all lowercase"
            );
        }
    }

    #[test]
    fn resolve_screen_index_large_count_last_index() {
        assert_eq!(resolve_screen_index(1000, Some(999)).unwrap(), 999);
    }

    #[test]
    fn resolve_output_path_filename_contains_digits() {
        // Default filename contains the timestamp, which consists of digits and underscores
        let path = resolve_output_path(None, Format::Png);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            name.chars().any(|c| c.is_ascii_digit()),
            "filename should contain timestamp digits: {name}"
        );
    }

    #[test]
    fn get_screenshots_dir_path_has_multiple_components() {
        let dir = get_screenshots_dir();
        assert!(
            dir.components().count() >= 2,
            "path should have at least parent + 'RustShot'"
        );
    }

    #[test]
    fn list_screens_returns_vec_of_strings() {
        let screens = list_screens();
        // May be empty in CI/headless env, but each entry must follow the "Screen N: WxH" pattern
        for screen in &screens {
            assert!(
                screen.starts_with("Screen "),
                "expected 'Screen N: WxH', got: {screen}"
            );
            assert!(
                screen.contains('x'),
                "entry should contain dimensions: {screen}"
            );
        }
    }

    #[test]
    fn list_screens_does_not_panic() {
        // Calling list_screens() must never panic, even in a headless environment
        let _ = list_screens();
    }

    #[test]
    fn ensure_parent_directory_three_levels_deep() {
        // Create a 3-level-deep temp directory via ensure_parent_directory
        let unique = format!(
            "rustshot-3lvl-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(&unique);
        let file_path = root
            .join("level1")
            .join("level2")
            .join("level3")
            .join("shot.png");
        ensure_parent_directory(&file_path).expect("three-level parent dir should be created");
        assert!(
            file_path.parent().unwrap().exists(),
            "level3 directory should exist"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn capture_screenshot_with_saves_to_explicit_path() {
        let captured = Cell::new(false);
        let ensured = Cell::new(false);
        let saved = Cell::new(false);
        let output = PathBuf::from("custom.jpg");

        let result = capture_screenshot_with(
            2,
            Some(1),
            || {
                captured.set(true);
                Ok("fake-image")
            },
            Some(output.clone()),
            Format::Jpg,
            &PathBuf::from("."),
            |path| {
                ensured.set(true);
                assert_eq!(path, output.as_path());
                Ok(())
            },
            |image, path, image_format| {
                saved.set(true);
                assert_eq!(*image, "fake-image");
                assert_eq!(path, output.as_path());
                assert_eq!(image_format, ImageFormat::Jpeg);
                Ok(())
            },
        )
        .unwrap();

        assert!(captured.get());
        assert!(ensured.get());
        assert!(saved.get());
        assert_eq!(result, output);
    }

    #[test]
    fn capture_screenshot_with_reports_capture_error() {
        let result = capture_screenshot_with(
            1,
            None,
            || Err("capture failed".to_string()),
            None,
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_: &(), _, _| Ok(()),
        );

        assert_eq!(result, Err("capture failed".to_string()));
    }

    #[test]
    fn capture_screenshot_with_reports_save_error() {
        let result = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("out.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, _| Err("save failed".to_string()),
        );

        assert_eq!(result, Err("save failed".to_string()));
    }

    #[test]
    fn capture_screenshot_with_reports_directory_error() {
        let save_called = Cell::new(false);
        let result = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("out.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Err("mkdir failed".to_string()),
            |_, _, _| {
                save_called.set(true);
                Ok(())
            },
        );

        assert_eq!(result, Err("mkdir failed".to_string()));
        assert!(!save_called.get());
    }

    #[test]
    fn capture_screenshot_with_reports_invalid_screen_index() {
        let capture_called = Cell::new(false);
        let result = capture_screenshot_with(
            1,
            Some(5),
            || {
                capture_called.set(true);
                Ok(())
            },
            None,
            Format::Bmp,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, _| Ok(()),
        );

        assert_eq!(
            result,
            Err("Screen index 5 is out of range (0-0)".to_string())
        );
        assert!(!capture_called.get());
    }

    #[test]
    fn capture_screenshot_with_reports_no_screens_detected() {
        let capture_called = Cell::new(false);
        let result = capture_screenshot_with(
            0,
            None,
            || {
                capture_called.set(true);
                Ok(())
            },
            None,
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, _| Ok(()),
        );

        assert_eq!(result, Err("No screens detected".to_string()));
        assert!(!capture_called.get());
    }

    #[test]
    fn capture_screenshot_with_uses_default_path_when_output_is_none() {
        let ensured_path = RefCell::new(None::<PathBuf>);
        let saved_path = RefCell::new(None::<PathBuf>);
        let screenshots_dir = PathBuf::from("custom-default-shots");

        let result = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            None,
            Format::Bmp,
            &screenshots_dir,
            |path| {
                ensured_path.replace(Some(path.to_path_buf()));
                Ok(())
            },
            |_, path, image_format| {
                saved_path.replace(Some(path.to_path_buf()));
                assert_eq!(image_format, ImageFormat::Bmp);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(ensured_path.borrow().as_ref(), Some(&result));
        assert_eq!(saved_path.borrow().as_ref(), Some(&result));
        assert_eq!(result.extension().and_then(|ext| ext.to_str()), Some("bmp"));
        assert_eq!(result.parent(), Some(screenshots_dir.as_path()));
        assert!(result
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .starts_with("screenshot_"));
    }

    #[test]
    fn capture_screenshot_with_passes_same_path_to_ensure_and_save() {
        let ensured_path = RefCell::new(None::<PathBuf>);
        let saved_path = RefCell::new(None::<PathBuf>);
        let output = PathBuf::from("nested\\shot.png");

        let result = capture_screenshot_with(
            1,
            Some(0),
            || Ok(()),
            Some(output.clone()),
            Format::Png,
            &PathBuf::from("."),
            |path| {
                ensured_path.replace(Some(path.to_path_buf()));
                Ok(())
            },
            |_, path, _| {
                saved_path.replace(Some(path.to_path_buf()));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result, output);
        assert_eq!(ensured_path.borrow().as_ref(), Some(&output));
        assert_eq!(saved_path.borrow().as_ref(), Some(&output));
    }

    #[test]
    fn capture_screenshot_with_uses_png_image_format_for_png() {
        let observed_format = RefCell::new(None::<ImageFormat>);

        let result = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("shot.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, image_format| {
                observed_format.replace(Some(image_format));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(result, PathBuf::from("shot.png"));
        assert_eq!(*observed_format.borrow(), Some(ImageFormat::Png));
    }

    #[test]
    fn capture_screenshot_with_accepts_explicit_zero_screen_index() {
        let capture_called = Cell::new(false);

        let result = capture_screenshot_with(
            1,
            Some(0),
            || {
                capture_called.set(true);
                Ok(())
            },
            Some(PathBuf::from("shot.bmp")),
            Format::Bmp,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, _| Ok(()),
        )
        .unwrap();

        assert!(capture_called.get());
        assert_eq!(result, PathBuf::from("shot.bmp"));
    }

    #[test]
    fn resolve_screen_index_count_two_index_two_is_err() {
        // index == count is out of range
        let err = resolve_screen_index(2, Some(2)).unwrap_err();
        assert!(err.contains("2"), "error should mention index: {err}");
        assert!(err.contains("0-1"), "error should show valid range: {err}");
    }

    #[test]
    fn capture_screenshot_with_image_format_for_bmp() {
        let observed_format = RefCell::new(None::<ImageFormat>);

        let _ = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("shot.bmp")),
            Format::Bmp,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, image_format| {
                observed_format.replace(Some(image_format));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(*observed_format.borrow(), Some(ImageFormat::Bmp));
    }

    #[test]
    fn capture_screenshot_with_image_format_for_jpg() {
        let observed_format = RefCell::new(None::<ImageFormat>);

        let _ = capture_screenshot_with(
            1,
            None,
            || Ok(()),
            Some(PathBuf::from("shot.jpg")),
            Format::Jpg,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, image_format| {
                observed_format.replace(Some(image_format));
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(*observed_format.borrow(), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn get_screenshots_dir_last_component_is_rustshot() {
        let dir = get_screenshots_dir();
        let last = dir.file_name().and_then(|n| n.to_str());
        assert_eq!(last, Some("RustShot"));
    }

    // --- bgra_to_rgba tests ---

    #[test]
    fn bgra_to_rgba_swaps_red_and_blue() {
        let mut pixels = vec![255, 0, 0, 255]; // B=255, G=0, R=0, A=255
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![0, 0, 255, 255]); // R=0, G=0, B=255, A=255
    }

    #[test]
    fn bgra_to_rgba_preserves_green_and_alpha() {
        let mut pixels = vec![10, 128, 200, 50];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels[1], 128); // green unchanged
        assert_eq!(pixels[3], 50); // alpha unchanged
    }

    #[test]
    fn bgra_to_rgba_identity_for_symmetric_values() {
        // When R == B, swapping is a no-op for those channels
        let mut pixels = vec![100, 50, 100, 255];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![100, 50, 100, 255]);
    }

    #[test]
    fn bgra_to_rgba_handles_multiple_pixels() {
        let mut pixels = vec![
            255, 0, 0, 255, // pixel 0: B=255
            0, 255, 0, 128, // pixel 1: G=255
            0, 0, 255, 64, // pixel 2: R=255
        ];
        bgra_to_rgba(&mut pixels);
        // pixel 0: R and B swapped
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[2], 255);
        // pixel 1: no change (G only)
        assert_eq!(pixels[4], 0);
        assert_eq!(pixels[5], 255);
        assert_eq!(pixels[6], 0);
        // pixel 2: R and B swapped
        assert_eq!(pixels[8], 255);
        assert_eq!(pixels[10], 0);
    }

    #[test]
    fn bgra_to_rgba_empty_input_is_noop() {
        let mut pixels: Vec<u8> = vec![];
        bgra_to_rgba(&mut pixels);
        assert!(pixels.is_empty());
    }

    #[test]
    fn bgra_to_rgba_double_swap_is_identity() {
        let original = vec![10, 20, 30, 40, 50, 60, 70, 80];
        let mut pixels = original.clone();
        bgra_to_rgba(&mut pixels);
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, original);
    }

    #[test]
    fn bgra_to_rgba_all_zeros() {
        let mut pixels = vec![0, 0, 0, 0];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn bgra_to_rgba_all_max() {
        let mut pixels = vec![255, 255, 255, 255];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![255, 255, 255, 255]);
    }

    // --- capture_active_window_platform tests ---

    #[cfg(not(windows))]
    #[test]
    fn capture_active_window_returns_unsupported_on_non_windows() {
        let result = capture_active_window_image();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Active window capture is only supported on Windows"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_active_window_save_returns_unsupported_on_non_windows() {
        let result = capture_active_window(None, Format::Png);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on Windows"));
    }

    // --- bgra_to_rgba edge cases ---

    #[test]
    fn bgra_to_rgba_trailing_bytes_ignored() {
        // chunks_exact_mut skips trailing bytes that don't form a full 4-byte chunk
        let mut pixels = vec![255, 0, 0, 255, 10, 20]; // 4 + 2 trailing
        bgra_to_rgba(&mut pixels);
        // First chunk swapped
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[2], 255);
        // Trailing bytes untouched
        assert_eq!(pixels[4], 10);
        assert_eq!(pixels[5], 20);
    }

    #[test]
    fn bgra_to_rgba_three_bytes_is_noop() {
        // Less than 4 bytes: no chunks to process
        let mut pixels = vec![1, 2, 3];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![1, 2, 3]);
    }

    #[test]
    fn bgra_to_rgba_large_image_all_swapped() {
        // 100 pixels, all with B=10, G=20, R=30, A=40
        let mut pixels = [10, 20, 30, 40].repeat(100);
        bgra_to_rgba(&mut pixels);
        for chunk in pixels.chunks(4) {
            assert_eq!(chunk[0], 30); // was R, now in B position
            assert_eq!(chunk[1], 20); // G unchanged
            assert_eq!(chunk[2], 10); // was B, now in R position
            assert_eq!(chunk[3], 40); // A unchanged
        }
    }

    // --- capture_screenshot_with additional edge cases ---

    #[test]
    fn capture_screenshot_with_multiple_screens_last_index() {
        let result = capture_screenshot_with(
            5,
            Some(4),
            || Ok("img"),
            Some(PathBuf::from("shot.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_, _, _| Ok(()),
        )
        .unwrap();
        assert_eq!(result, PathBuf::from("shot.png"));
    }

    #[test]
    fn capture_screenshot_with_capture_error_prevents_save() {
        let save_called = Cell::new(false);
        let _ = capture_screenshot_with(
            1,
            None,
            || Err::<(), _>("capture boom".to_string()),
            Some(PathBuf::from("out.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Ok(()),
            |_: &(), _, _| {
                save_called.set(true);
                Ok(())
            },
        );
        assert!(
            !save_called.get(),
            "save should not be called after capture error"
        );
    }

    // --- resolve_output_path with different formats ---

    #[test]
    fn resolve_output_path_two_different_formats_have_different_extensions() {
        let png = resolve_output_path(None, Format::Png);
        let jpg = resolve_output_path(None, Format::Jpg);
        assert_ne!(
            png.extension().and_then(|e| e.to_str()),
            jpg.extension().and_then(|e| e.to_str())
        );
    }

    // --- ensure_parent_directory error path ---

    #[test]
    fn ensure_parent_directory_with_only_filename_is_ok() {
        // "file.png" has parent "" which is empty, so no mkdir
        assert!(ensure_parent_directory(Path::new("file.png")).is_ok());
    }

    #[test]
    fn ensure_parent_directory_root_path_is_ok() {
        // "/file.png" parent is "/" which exists
        assert!(ensure_parent_directory(Path::new("/file.png")).is_ok());
    }

    // --- Format trait coverage ---

    #[test]
    fn format_debug_contains_variant_name() {
        assert!(format!("{:?}", Format::Png).contains("Png"));
        assert!(format!("{:?}", Format::Jpg).contains("Jpg"));
        assert!(format!("{:?}", Format::Bmp).contains("Bmp"));
    }

    // --- default_screen_index additional tests ---

    #[test]
    fn default_screen_index_single_primary_returns_zero() {
        assert_eq!(default_screen_index([true]), Some(0));
    }

    #[test]
    fn default_screen_index_single_non_primary_returns_zero() {
        assert_eq!(default_screen_index([false]), Some(0));
    }

    #[test]
    fn default_screen_index_multiple_primaries_returns_first() {
        // Rare but possible: two screens both marked primary
        assert_eq!(default_screen_index([true, true, false]), Some(0));
    }

    #[test]
    fn default_screen_index_primary_in_middle_of_five() {
        assert_eq!(
            default_screen_index([false, false, true, false, false]),
            Some(2)
        );
    }

    // --- bgra_to_rgba additional tests ---

    #[test]
    fn bgra_to_rgba_single_pixel_specific_values() {
        // BGRA [100, 150, 200, 128] → RGBA [200, 150, 100, 128]
        let mut pixels = vec![100, 150, 200, 128];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![200, 150, 100, 128]);
    }

    #[test]
    fn bgra_to_rgba_two_pixels() {
        let mut pixels = vec![
            100, 200, 50, 255, // pixel 0: B=100, G=200, R=50, A=255
            10, 20, 30, 128, // pixel 1: B=10, G=20, R=30, A=128
        ];
        bgra_to_rgba(&mut pixels);
        // pixel 0: R=50, G=200, B=100, A=255
        assert_eq!(&pixels[0..4], &[50, 200, 100, 255]);
        // pixel 1: R=30, G=20, B=10, A=128
        assert_eq!(&pixels[4..8], &[30, 20, 10, 128]);
    }

    // --- capture_screenshot_with: ensure closure order ---

    #[test]
    fn capture_screenshot_with_calls_closures_in_order() {
        let order = RefCell::new(Vec::<&str>::new());

        let _ = capture_screenshot_with(
            1,
            None,
            || {
                order.borrow_mut().push("capture");
                Ok(())
            },
            Some(PathBuf::from("test.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| {
                order.borrow_mut().push("ensure_dir");
                Ok(())
            },
            |_, _, _| {
                order.borrow_mut().push("save");
                Ok(())
            },
        );

        assert_eq!(*order.borrow(), vec!["capture", "ensure_dir", "save"]);
    }

    // --- resolve_output_path consistency ---

    #[test]
    fn resolve_output_path_default_filenames_all_start_with_screenshot() {
        for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
            let path = resolve_output_path(None, fmt);
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            assert!(
                name.starts_with("screenshot_"),
                "expected 'screenshot_' prefix for {fmt:?}, got: {name}"
            );
        }
    }

    #[test]
    fn resolve_output_path_explicit_with_no_extension() {
        let explicit = PathBuf::from("no_ext");
        let result = resolve_output_path(Some(explicit.clone()), Format::Png);
        assert_eq!(result, explicit);
        assert_eq!(result.extension(), None);
    }

    // --- ensure_parent_directory with temp dir ---

    #[test]
    fn ensure_parent_directory_succeeds_for_existing_temp_dir() {
        let path = std::env::temp_dir().join("test_file.png");
        assert!(ensure_parent_directory(&path).is_ok());
    }

    // --- resolve_screen_index property tests ---

    #[test]
    fn resolve_screen_index_always_returns_requested_when_in_range() {
        for count in 1..=10usize {
            for idx in 0..count {
                assert_eq!(
                    resolve_screen_index(count, Some(idx)).unwrap(),
                    idx,
                    "count={count}, idx={idx}"
                );
            }
        }
    }

    #[test]
    fn resolve_screen_index_always_errors_when_out_of_range() {
        for count in 1..=5usize {
            for idx in count..count + 3 {
                assert!(
                    resolve_screen_index(count, Some(idx)).is_err(),
                    "count={count}, idx={idx} should be out of range"
                );
            }
        }
    }

    // =========================================================================
    // Format — additional coverage
    // =========================================================================

    #[test]
    fn format_default_variant_extension_is_png() {
        let f = Format::default();
        assert_eq!(f.extension(), "png");
    }

    #[test]
    fn format_to_image_format_png_roundtrip() {
        assert_eq!(Format::Png.to_image_format(), ImageFormat::Png);
    }

    #[test]
    fn format_to_image_format_jpg_is_jpeg() {
        assert_eq!(Format::Jpg.to_image_format(), ImageFormat::Jpeg);
    }

    #[test]
    fn format_to_image_format_bmp() {
        assert_eq!(Format::Bmp.to_image_format(), ImageFormat::Bmp);
    }

    #[test]
    fn format_clone_preserves_variant() {
        let f = Format::Jpg;
        let f2 = f;
        assert_eq!(f.extension(), f2.extension());
    }

    #[test]
    fn format_debug_is_non_empty() {
        assert!(!format!("{:?}", Format::Png).is_empty());
        assert!(!format!("{:?}", Format::Jpg).is_empty());
        assert!(!format!("{:?}", Format::Bmp).is_empty());
    }

    // =========================================================================
    // ensure_parent_directory — additional coverage
    // =========================================================================

    #[test]
    fn ensure_parent_directory_root_path_succeeds() {
        // Path with no parent (just filename) should succeed
        assert!(ensure_parent_directory(std::path::Path::new("file.png")).is_ok());
    }

    #[test]
    fn ensure_parent_directory_nested_tmp_path() {
        let path = std::path::Path::new("/tmp/rustshot_test_ensure/nested/deep/file.png");
        let result = ensure_parent_directory(path);
        assert!(result.is_ok());
        // Cleanup
        let _ = std::fs::remove_dir_all("/tmp/rustshot_test_ensure");
    }

    // =========================================================================
    // resolve_output_path_in_dir — additional coverage
    // =========================================================================

    #[test]
    fn resolve_output_path_in_dir_explicit_path_ignores_dir() {
        let dir = PathBuf::from("/should/be/ignored");
        let explicit = PathBuf::from("/custom/shot.png");
        let result = resolve_output_path_in_dir(Some(explicit.clone()), Format::Png, &dir);
        assert_eq!(result, explicit);
    }

    #[test]
    fn resolve_output_path_in_dir_none_uses_dir() {
        let dir = PathBuf::from("/my/screenshots");
        let result = resolve_output_path_in_dir(None, Format::Png, &dir);
        assert_eq!(result.parent().unwrap(), dir.as_path());
    }

    #[test]
    fn resolve_output_path_in_dir_jpg_extension() {
        let dir = PathBuf::from("/tmp");
        let result = resolve_output_path_in_dir(None, Format::Jpg, &dir);
        assert_eq!(result.extension().and_then(|e| e.to_str()), Some("jpg"));
    }

    #[test]
    fn resolve_output_path_in_dir_bmp_extension() {
        let dir = PathBuf::from("/tmp");
        let result = resolve_output_path_in_dir(None, Format::Bmp, &dir);
        assert_eq!(result.extension().and_then(|e| e.to_str()), Some("bmp"));
    }

    #[test]
    fn resolve_output_path_in_dir_png_extension() {
        let dir = PathBuf::from("/tmp");
        let result = resolve_output_path_in_dir(None, Format::Png, &dir);
        assert_eq!(result.extension().and_then(|e| e.to_str()), Some("png"));
    }

    #[test]
    fn resolve_output_path_in_dir_filename_starts_with_screenshot() {
        let dir = PathBuf::from("/tmp");
        let result = resolve_output_path_in_dir(None, Format::Png, &dir);
        let stem = result.file_stem().and_then(|n| n.to_str()).unwrap();
        assert!(stem.starts_with("screenshot_"));
    }

    #[test]
    fn resolve_output_path_in_dir_timestamp_is_25_chars() {
        let dir = PathBuf::from("/tmp");
        let result = resolve_output_path_in_dir(None, Format::Png, &dir);
        let stem = result.file_stem().and_then(|n| n.to_str()).unwrap();
        let ts = &stem["screenshot_".len()..];
        assert_eq!(ts.len(), 25); // YYYYMMDD_HHMMSS_NNNNNNNNN
    }

    // =========================================================================
    // default_screen_index — additional coverage
    // =========================================================================

    #[test]
    fn default_screen_index_all_primary_returns_first() {
        assert_eq!(default_screen_index(vec![true, true, true]), Some(0));
    }

    #[test]
    fn default_screen_index_single_non_primary_returns_some_0() {
        assert_eq!(default_screen_index(vec![false]), Some(0));
    }

    #[test]
    fn default_screen_index_empty_returns_none() {
        assert_eq!(default_screen_index(Vec::<bool>::new()), None);
    }

    #[test]
    fn default_screen_index_primary_at_index_2() {
        assert_eq!(
            default_screen_index(vec![false, false, true, false]),
            Some(2)
        );
    }

    // =========================================================================
    // bgra_to_rgba — additional coverage
    // =========================================================================

    #[test]
    fn bgra_to_rgba_single_pixel() {
        let mut data = vec![30u8, 20, 10, 255]; // BGRA
        bgra_to_rgba(&mut data);
        assert_eq!(data, vec![10, 20, 30, 255]); // RGBA
    }

    #[test]
    fn bgra_to_rgba_two_swaps_restore_original() {
        let original = vec![100u8, 150, 200, 255, 10, 20, 30, 128];
        let mut data = original.clone();
        bgra_to_rgba(&mut data);
        bgra_to_rgba(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn bgra_to_rgba_preserves_alpha() {
        let mut data = vec![0u8, 0, 0, 42];
        bgra_to_rgba(&mut data);
        assert_eq!(data[3], 42);
    }

    #[test]
    fn bgra_to_rgba_empty_buffer_is_noop() {
        let mut data: Vec<u8> = vec![];
        bgra_to_rgba(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn bgra_to_rgba_preserves_green() {
        let mut data = vec![0u8, 128, 0, 255];
        bgra_to_rgba(&mut data);
        assert_eq!(data[1], 128); // green unchanged
    }

    // =========================================================================
    // capture_screenshot_with — DI tests
    // =========================================================================

    #[test]
    fn capture_screenshot_with_save_failure_propagates() {
        let result = capture_screenshot_with(
            1,
            Some(0),
            || Ok(vec![0u8; 4]),
            None,
            Format::Png,
            std::path::Path::new("/tmp"),
            |_| Ok(()),
            |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| {
                Err("disk full".to_string())
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disk full"));
    }

    #[test]
    fn capture_screenshot_with_capture_failure_propagates() {
        let result = capture_screenshot_with::<_, _, _, Vec<u8>>(
            1,
            Some(0),
            || Err("no screen".to_string()),
            None,
            Format::Png,
            std::path::Path::new("/tmp"),
            |_| Ok(()),
            |_img, _path, _fmt| Ok(()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no screen"));
    }

    #[test]
    fn capture_screenshot_with_ensure_dir_failure_propagates() {
        let result = capture_screenshot_with(
            1,
            Some(0),
            || Ok(vec![0u8; 4]),
            None,
            Format::Png,
            std::path::Path::new("/tmp"),
            |_| Err("permission denied".to_string()),
            |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| Ok(()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission denied"));
    }

    #[test]
    fn capture_screenshot_with_zero_screens_returns_error() {
        let result = capture_screenshot_with(
            0,
            None,
            || Ok(vec![0u8; 4]),
            None,
            Format::Png,
            std::path::Path::new("/tmp"),
            |_| Ok(()),
            |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| Ok(()),
        );
        assert!(result.is_err());
    }

    // =========================================================================
    // get_screenshots_dir
    // =========================================================================

    #[test]
    fn get_screenshots_dir_ends_with_rustshot() {
        let dir = get_screenshots_dir();
        assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("RustShot"));
    }

    #[test]
    fn get_screenshots_dir_is_consistent() {
        let d1 = get_screenshots_dir();
        let d2 = get_screenshots_dir();
        assert_eq!(d1, d2);
    }

    // =========================================================================
    // list_screens
    // =========================================================================

    #[test]
    fn list_screens_returns_string() {
        let result = list_screens();
        // Should either succeed with screen info or fail gracefully
        // On CI/headless it might fail, but should not panic
        let _ = result;
    }

    // =========================================================================
    // save_image_optimized
    // =========================================================================

    #[test]
    fn save_image_optimized_png() {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255]));
        let dir = std::env::temp_dir().join("rustshot_test_save_opt_png");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");

        save_image_optimized(&img, &path, Format::Png).unwrap();
        assert!(path.exists());
        // Verify it's a valid PNG by re-reading
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (8, 8));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_jpg() {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 255, 0, 255]));
        let dir = std::env::temp_dir().join("rustshot_test_save_opt_jpg");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jpg");

        save_image_optimized(&img, &path, Format::Jpg).unwrap();
        assert!(path.exists());
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (8, 8));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_bmp() {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 255, 255]));
        let dir = std::env::temp_dir().join("rustshot_test_save_opt_bmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bmp");

        save_image_optimized(&img, &path, Format::Bmp).unwrap();
        assert!(path.exists());
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (8, 8));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_invalid_path() {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
        let path = std::path::Path::new("/nonexistent_dir_xyz/impossible/test.png");

        let result = save_image_optimized(&img, path, Format::Png);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to create file"));
    }

    // -------------------------------------------------------------------------
    // capture_active_window platform tests
    // -------------------------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn capture_active_window_image_returns_unsupported_on_linux() {
        let result = capture_active_window_image();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on Windows"));
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_active_window_in_dir_returns_unsupported_on_linux() {
        let dir = std::env::temp_dir();
        let result = capture_active_window_in_dir(None, Format::Png, &dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on Windows"));
    }

    // -------------------------------------------------------------------------
    // save_image_optimized format tests
    // -------------------------------------------------------------------------

    #[test]
    fn save_image_optimized_png_creates_valid_file() {
        let dir = std::env::temp_dir().join("rustshot_test_save_png");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");

        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        save_image_optimized(&img, &path, Format::Png).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 8);
        // PNG magic bytes
        assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_jpg_creates_valid_file() {
        let dir = std::env::temp_dir().join("rustshot_test_save_jpg");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jpg");

        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        save_image_optimized(&img, &path, Format::Jpg).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 2);
        // JPEG magic bytes
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_bmp_creates_valid_file() {
        let dir = std::env::temp_dir().join("rustshot_test_save_bmp");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bmp");

        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
        save_image_optimized(&img, &path, Format::Bmp).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 2);
        // BMP magic bytes
        assert_eq!(&bytes[..2], b"BM");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_1x1_pixel_png() {
        let dir = std::env::temp_dir().join("rustshot_test_save_1x1");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1x1.png");

        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
        save_image_optimized(&img, &path, Format::Png).unwrap();

        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // capture_screenshot_in_dir / capture_screenshot delegation
    // -------------------------------------------------------------------------

    #[test]
    fn capture_screenshot_with_explicit_path_in_dir_returns_explicit_path() {
        let dir = std::env::temp_dir().join("rustshot_test_csid");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let explicit_path = dir.join("explicit.png");
        let result = capture_screenshot_with(
            2,
            Some(0),
            || {
                Ok(image::RgbaImage::from_pixel(
                    2,
                    2,
                    image::Rgba([0, 0, 0, 255]),
                ))
            },
            Some(explicit_path.clone()),
            Format::Png,
            &dir,
            |_| Ok(()),
            |_img, _path, _fmt| Ok(()),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), explicit_path);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_screenshot_with_no_path_generates_path_in_dir() {
        let dir = std::env::temp_dir().join("rustshot_test_csid2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = capture_screenshot_with(
            1,
            Some(0),
            || {
                Ok(image::RgbaImage::from_pixel(
                    2,
                    2,
                    image::Rgba([0, 0, 0, 255]),
                ))
            },
            None,
            Format::Jpg,
            &dir,
            |_| Ok(()),
            |_img, _path, _fmt| Ok(()),
        );
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.extension().unwrap(), "jpg");
        assert_eq!(path.parent().unwrap(), dir);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // bgra_to_rgba additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn bgra_to_rgba_exact_8_pixels() {
        let mut buf: Vec<u8> = (0..32).collect();
        let original = buf.clone();
        bgra_to_rgba(&mut buf);
        // R and B swapped for each pixel
        for i in 0..8 {
            let base = i * 4;
            assert_eq!(buf[base], original[base + 2]); // R = old B
            assert_eq!(buf[base + 1], original[base + 1]); // G unchanged
            assert_eq!(buf[base + 2], original[base]); // B = old R
            assert_eq!(buf[base + 3], original[base + 3]); // A unchanged
        }
    }

    #[test]
    fn bgra_to_rgba_5_extra_bytes_ignored() {
        let mut buf = vec![10, 20, 30, 40, 50, 60, 70, 80, 99];
        bgra_to_rgba(&mut buf);
        // First pixel swapped
        assert_eq!(buf[0], 30);
        assert_eq!(buf[2], 10);
        // Second pixel swapped
        assert_eq!(buf[4], 70);
        assert_eq!(buf[6], 50);
        // Extra byte untouched
        assert_eq!(buf[8], 99);
    }

    // -------------------------------------------------------------------------
    // Format additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn format_all_variants_have_distinct_extensions() {
        let exts: Vec<&str> = [Format::Png, Format::Jpg, Format::Bmp]
            .iter()
            .map(|f| f.extension())
            .collect();
        assert_eq!(exts.len(), 3);
        assert_ne!(exts[0], exts[1]);
        assert_ne!(exts[1], exts[2]);
        assert_ne!(exts[0], exts[2]);
    }

    #[test]
    fn format_all_variants_have_distinct_image_formats() {
        let fmts: Vec<image::ImageFormat> = [Format::Png, Format::Jpg, Format::Bmp]
            .iter()
            .map(|f| f.to_image_format())
            .collect();
        assert_ne!(fmts[0], fmts[1]);
        assert_ne!(fmts[1], fmts[2]);
    }

    // -------------------------------------------------------------------------
    // ensure_parent_directory additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn ensure_parent_directory_unicode_path() {
        let dir = std::env::temp_dir().join("rustshot_тест_юникод");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("file.png");
        assert!(ensure_parent_directory(&path).is_ok());
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_parent_directory_already_exists_is_ok() {
        let dir = std::env::temp_dir().join("rustshot_existing_dir_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("file.png");
        assert!(ensure_parent_directory(&path).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // bgra_to_rgba edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn bgra_to_rgba_single_pixel_swaps_r_and_b() {
        // BGRA [10, 20, 30, 255] -> RGBA [30, 20, 10, 255]
        let mut pixels = vec![10u8, 20, 30, 255];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![30, 20, 10, 255]);
    }

    #[test]
    fn bgra_to_rgba_preserves_alpha_channel() {
        let mut pixels = vec![0u8, 0, 0, 128];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels[3], 128);
    }

    #[test]
    fn bgra_to_rgba_white_pixel_unchanged() {
        // White: [255,255,255,255] - swapping R and B when both are 255 is identity
        let mut pixels = vec![255u8, 255, 255, 255];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![255, 255, 255, 255]);
    }

    #[test]
    fn bgra_to_rgba_empty_slice_is_noop() {
        let mut pixels: Vec<u8> = vec![];
        bgra_to_rgba(&mut pixels);
        assert!(pixels.is_empty());
    }

    #[test]
    fn bgra_to_rgba_two_pixels_verify_both_swapped() {
        let mut pixels = vec![
            10, 20, 30, 255, // pixel 1: B=10, G=20, R=30, A=255
            40, 50, 60, 128, // pixel 2: B=40, G=50, R=60, A=128
        ];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels[0], 30); // R was at index 2
        assert_eq!(pixels[2], 10); // B was at index 0
        assert_eq!(pixels[4], 60); // R was at index 6
        assert_eq!(pixels[6], 40); // B was at index 4
    }

    // -------------------------------------------------------------------------
    // benchmark_screen_count_from_result edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn benchmark_screen_count_from_result_error_passthrough() {
        let result: Result<Vec<i32>, String> = Err("enumeration failed".to_string());
        let count = benchmark_screen_count_from_result(result);
        assert_eq!(count, Err("enumeration failed".to_string()));
    }

    #[test]
    fn benchmark_screen_count_from_result_empty_vec_is_error() {
        let result: Result<Vec<i32>, String> = Ok(vec![]);
        let count = benchmark_screen_count_from_result(result);
        assert_eq!(count, Err("No screens detected".to_string()));
    }

    #[test]
    fn benchmark_screen_count_from_result_three_screens() {
        let result: Result<Vec<i32>, String> = Ok(vec![1, 2, 3]);
        assert_eq!(benchmark_screen_count_from_result(result), Ok(3));
    }

    // -------------------------------------------------------------------------
    // benchmark_first_screen_from_result edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn benchmark_first_screen_from_result_returns_first_element() {
        let result: Result<Vec<&str>, String> = Ok(vec!["primary", "secondary"]);
        assert_eq!(benchmark_first_screen_from_result(result), Ok("primary"));
    }

    #[test]
    fn benchmark_first_screen_from_result_empty_vec_is_error() {
        let result: Result<Vec<i32>, String> = Ok(vec![]);
        assert_eq!(
            benchmark_first_screen_from_result(result),
            Err("No screens detected".to_string())
        );
    }

    // -------------------------------------------------------------------------
    // save_image_optimized to real files
    // -------------------------------------------------------------------------

    #[test]
    fn save_image_optimized_png_creates_file() {
        let dir = std::env::temp_dir().join("rustshot_save_png_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        save_image_optimized(&img, &path, Format::Png).unwrap();
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_jpg_creates_file() {
        let dir = std::env::temp_dir().join("rustshot_save_jpg_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jpg");
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
        save_image_optimized(&img, &path, Format::Jpg).unwrap();
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_bmp_creates_file() {
        let dir = std::env::temp_dir().join("rustshot_save_bmp_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bmp");
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
        save_image_optimized(&img, &path, Format::Bmp).unwrap();
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // benchmark_rgb_image_from_rgba
    // -------------------------------------------------------------------------

    #[test]
    fn benchmark_rgb_image_from_rgba_drops_alpha() {
        let rgba = image::RgbaImage::from_pixel(2, 2, image::Rgba([100, 150, 200, 128]));
        let rgb = benchmark_rgb_image_from_rgba(&rgba);
        assert_eq!(rgb.width(), 2);
        assert_eq!(rgb.height(), 2);
        // RGB image has 3 bytes per pixel
        assert_eq!(rgb.as_raw().len(), 2 * 2 * 3);
        // First pixel should be [100, 150, 200]
        assert_eq!(rgb.as_raw()[0], 100);
        assert_eq!(rgb.as_raw()[1], 150);
        assert_eq!(rgb.as_raw()[2], 200);
    }

    #[test]
    fn benchmark_rgb_image_from_rgba_preserves_dimensions() {
        let rgba = image::RgbaImage::new(10, 20);
        let rgb = benchmark_rgb_image_from_rgba(&rgba);
        assert_eq!(rgb.width(), 10);
        assert_eq!(rgb.height(), 20);
    }

    // -------------------------------------------------------------------------
    // default_screen_index edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn default_screen_index_single_primary() {
        assert_eq!(default_screen_index([true]), Some(0));
    }

    #[test]
    fn default_screen_index_single_non_primary() {
        assert_eq!(default_screen_index([false]), Some(0));
    }

    #[test]
    fn default_screen_index_first_is_primary() {
        assert_eq!(default_screen_index([true, false, false]), Some(0));
    }

    // -------------------------------------------------------------------------
    // Format edge cases
    // -------------------------------------------------------------------------

    // -------------------------------------------------------------------------
    // resolve_screen_index additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_screen_index_zero_screens_none_is_err() {
        assert!(resolve_screen_index(0, None).is_err());
    }

    #[test]
    fn resolve_screen_index_zero_screens_some_is_err() {
        assert!(resolve_screen_index(0, Some(0)).is_err());
    }

    #[test]
    fn resolve_screen_index_zero_screens_mentions_no_screens() {
        let err = resolve_screen_index(0, None).unwrap_err();
        assert!(err.contains("No screens"));
    }

    #[test]
    fn resolve_screen_index_last_valid() {
        assert_eq!(resolve_screen_index(3, Some(2)), Ok(2));
    }

    #[test]
    fn resolve_screen_index_first_invalid() {
        assert!(resolve_screen_index(3, Some(3)).is_err());
    }

    #[test]
    fn resolve_screen_index_error_shows_valid_range() {
        let err = resolve_screen_index(3, Some(5)).unwrap_err();
        assert!(
            err.contains("0-2"),
            "error should mention valid range: {err}"
        );
    }

    #[test]
    fn resolve_screen_index_one_screen_default_is_zero() {
        assert_eq!(resolve_screen_index(1, None), Ok(0));
    }

    #[test]
    fn resolve_screen_index_one_screen_explicit_zero_ok() {
        assert_eq!(resolve_screen_index(1, Some(0)), Ok(0));
    }

    #[test]
    fn resolve_screen_index_one_screen_explicit_one_fails() {
        assert!(resolve_screen_index(1, Some(1)).is_err());
    }

    // -------------------------------------------------------------------------
    // resolve_output_path_in_dir tests
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_output_path_in_dir_explicit_path_returned() {
        let p = resolve_output_path_in_dir(
            Some(PathBuf::from("/tmp/my.png")),
            Format::Png,
            Path::new("/screenshots"),
        );
        assert_eq!(p, PathBuf::from("/tmp/my.png"));
    }

    #[test]
    fn resolve_output_path_in_dir_generated_has_correct_extension() {
        let p = resolve_output_path_in_dir(None, Format::Jpg, Path::new("/screenshots"));
        assert!(
            p.extension().and_then(|e| e.to_str()) == Some("jpg"),
            "expected .jpg extension, got: {p:?}"
        );
    }

    #[test]
    fn resolve_output_path_in_dir_generated_starts_with_dir() {
        let dir = Path::new("/my/screenshots");
        let p = resolve_output_path_in_dir(None, Format::Png, dir);
        assert!(
            p.starts_with(dir),
            "path should start with {dir:?}, got: {p:?}"
        );
    }

    #[test]
    fn resolve_output_path_in_dir_generated_contains_screenshot() {
        let p = resolve_output_path_in_dir(None, Format::Bmp, Path::new("/tmp"));
        let name = p.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("screenshot_"),
            "name should start with screenshot_, got: {name}"
        );
    }

    #[test]
    fn resolve_output_path_in_dir_each_format_extension() {
        for (fmt, ext) in [
            (Format::Png, "png"),
            (Format::Jpg, "jpg"),
            (Format::Bmp, "bmp"),
        ] {
            let p = resolve_output_path_in_dir(None, fmt, Path::new("/tmp"));
            assert_eq!(p.extension().and_then(|e| e.to_str()), Some(ext));
        }
    }

    // -------------------------------------------------------------------------
    // ensure_parent_directory tests
    // -------------------------------------------------------------------------

    #[test]
    fn ensure_parent_directory_creates_missing_dir() {
        let dir = std::env::temp_dir().join("rustshot_test_ensure_parent");
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("subdir").join("file.txt");
        ensure_parent_directory(&file).unwrap();
        assert!(file.parent().unwrap().exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_parent_directory_existing_dir_ok() {
        let dir = std::env::temp_dir();
        let file = dir.join("existing_file.txt");
        assert!(ensure_parent_directory(&file).is_ok());
    }

    #[test]
    fn ensure_parent_directory_root_path_ok() {
        // A path like "file.txt" with no parent should succeed
        assert!(ensure_parent_directory(Path::new("file.txt")).is_ok());
    }

    // -------------------------------------------------------------------------
    // default_screen_index additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn default_screen_index_empty() {
        let empty: Vec<bool> = vec![];
        assert_eq!(default_screen_index(empty), None);
    }

    #[test]
    fn default_screen_index_primary_in_middle() {
        assert_eq!(default_screen_index([false, true, false]), Some(1));
    }

    #[test]
    fn default_screen_index_primary_last() {
        assert_eq!(default_screen_index([false, false, true]), Some(2));
    }

    #[test]
    fn default_screen_index_two_primaries_picks_earlier() {
        assert_eq!(default_screen_index([false, true, true]), Some(1));
    }

    #[test]
    fn default_screen_index_no_primary_returns_first() {
        assert_eq!(default_screen_index([false, false, false]), Some(0));
    }

    // -------------------------------------------------------------------------
    // save_image_optimized additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn save_image_optimized_1x1_pixel_all_formats() {
        for (fmt, ext) in [
            (Format::Png, "png"),
            (Format::Jpg, "jpg"),
            (Format::Bmp, "bmp"),
        ] {
            let dir = std::env::temp_dir().join(format!("rustshot_test_1x1_{ext}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("pixel.{ext}"));

            let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
            save_image_optimized(&img, &path, fmt).unwrap();
            assert!(path.exists(), "file should exist for format {ext}");
            assert!(std::fs::metadata(&path).unwrap().len() > 0);

            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn save_image_optimized_transparent_pixel_png() {
        let dir = std::env::temp_dir().join("rustshot_test_transparent");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("transparent.png");

        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0]));
        save_image_optimized(&img, &path, Format::Png).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_invalid_path_returns_error() {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        let result =
            save_image_optimized(&img, Path::new("/nonexistent/dir/file.png"), Format::Png);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to create file"));
    }

    // -------------------------------------------------------------------------
    // save_image_optimized — JPEG with transparent pixel (alpha channel drop)
    // -------------------------------------------------------------------------

    #[test]
    fn save_image_optimized_jpg_drops_alpha_channel() {
        let dir = std::env::temp_dir().join(format!(
            "rustshot-jpg-alpha-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("alpha.jpg");
        // Image with 50% transparent red pixel
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128]));
        save_image_optimized(&img, &path, Format::Jpg).unwrap();
        // JPEG file should exist and be non-empty
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0, "JPEG file should be non-empty");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // default_screen_index — multiple primaries picks first primary
    // -------------------------------------------------------------------------

    #[test]
    fn default_screen_index_multiple_primaries_picks_first() {
        let index = default_screen_index([false, true, true, false]).unwrap();
        assert_eq!(index, 1, "should pick the first primary screen");
    }

    #[test]
    fn default_screen_index_single_screen_primary_returns_zero() {
        let index = default_screen_index([true]).unwrap();
        assert_eq!(index, 0);
    }

    #[test]
    fn default_screen_index_single_screen_not_primary_returns_zero() {
        // Fallback to first screen when no primary flag
        let index = default_screen_index([false]).unwrap();
        assert_eq!(index, 0);
    }

    // -------------------------------------------------------------------------
    // capture_screenshot_with — screen_index: None
    // -------------------------------------------------------------------------

    #[test]
    fn capture_screenshot_with_none_screen_index_uses_default() {
        let result = capture_screenshot_with(
            3,
            None, // None triggers resolve_screen_index(3, None) → Ok(0)
            || Ok("fake"),
            None,
            Format::Png,
            &PathBuf::from("/tmp/shots"),
            |_| Ok(()),
            |_, _, _| Ok(()),
        );
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // benchmark helpers — additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn benchmark_screen_count_from_result_single_screen() {
        assert_eq!(benchmark_screen_count_from_result(Ok(vec![42])).unwrap(), 1);
    }

    #[test]
    fn benchmark_first_screen_from_result_error_passthrough() {
        let result: Result<i32, String> =
            benchmark_first_screen_from_result(Err("no screens".to_string()));
        assert_eq!(result.unwrap_err(), "no screens");
    }

    #[test]
    fn benchmark_rgb_image_from_rgba_preserves_pixel_colors() {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([100, 150, 200, 255]));
        let rgb = benchmark_rgb_image_from_rgba(&img);
        let pixel = rgb.get_pixel(0, 0);
        assert_eq!(pixel[0], 100); // R
        assert_eq!(pixel[1], 150); // G
        assert_eq!(pixel[2], 200); // B
    }

    // -------------------------------------------------------------------------
    // bgra_to_rgba — 3-byte remainder is ignored
    // -------------------------------------------------------------------------

    #[test]
    fn bgra_to_rgba_three_trailing_bytes_untouched() {
        let mut pixels = vec![10, 20, 30, 255, 1, 2, 3]; // 1 full pixel + 3 remainder
        bgra_to_rgba(&mut pixels);
        // Full pixel: B=10, G=20, R=30, A=255 → R=30, G=20, B=10, A=255
        assert_eq!(pixels[0], 30);
        assert_eq!(pixels[2], 10);
        // Remainder bytes unchanged
        assert_eq!(pixels[4], 1);
        assert_eq!(pixels[5], 2);
        assert_eq!(pixels[6], 3);
    }

    // -------------------------------------------------------------------------
    // save_image_optimized — 2x2 image produces valid files
    // -------------------------------------------------------------------------

    #[test]
    fn save_image_optimized_2x2_all_formats() {
        let dir = std::env::temp_dir().join(format!(
            "rustshot-2x2-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let img = image::RgbaImage::from_fn(2, 2, |x, y| {
            image::Rgba([(x * 100) as u8, (y * 100) as u8, 128, 255])
        });

        for (fmt, ext) in [
            (Format::Png, "png"),
            (Format::Jpg, "jpg"),
            (Format::Bmp, "bmp"),
        ] {
            let path = dir.join(format!("test.{ext}"));
            save_image_optimized(&img, &path, fmt).unwrap();
            assert!(path.exists(), "{ext} file should exist");
            assert!(
                std::fs::metadata(&path).unwrap().len() > 0,
                "{ext} file should be non-empty"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // -------------------------------------------------------------------------
    // capture_active_window_platform — error message on non-windows
    // -------------------------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn capture_active_window_error_message_mentions_windows() {
        let err = capture_active_window_image().unwrap_err();
        assert!(
            err.contains("Windows"),
            "error should mention Windows: {err}"
        );
    }

    // -------------------------------------------------------------------------
    // resolve_output_path_in_dir — two consecutive calls produce different names
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_output_path_in_dir_two_calls_produce_different_names() {
        let dir = PathBuf::from("/tmp/test-shots");
        let p1 = resolve_output_path_in_dir(None, Format::Png, &dir);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let p2 = resolve_output_path_in_dir(None, Format::Png, &dir);
        assert_ne!(p1, p2, "two default paths should differ due to timestamp");
    }

    // -------------------------------------------------------------------------
    // list_screens — does not panic and returns consistent results
    // -------------------------------------------------------------------------

    #[test]
    fn list_screens_returns_consistent_results() {
        let first = list_screens();
        let second = list_screens();
        assert_eq!(first, second);
    }

    // --- save_image_optimized (additional coverage) ---

    fn make_patterned_image(width: u32, height: u32) -> image::RgbaImage {
        let mut img = image::RgbaImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]);
        }
        img
    }

    #[test]
    fn save_image_optimized_png_is_lossless_roundtrip() {
        let dir = std::env::temp_dir().join("rustshot_test_png_lossless");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("exact.png");
        let img = make_patterned_image(16, 16);
        save_image_optimized(&img, &path, Format::Png).unwrap();
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(img.as_raw(), loaded.as_raw());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_large_256x256() {
        let dir = std::env::temp_dir().join("rustshot_test_large256");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("large.png");
        let img = make_patterned_image(256, 256);
        save_image_optimized(&img, &path, Format::Png).unwrap();
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (256, 256));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_bmp_non_square_dimensions() {
        let dir = std::env::temp_dir().join("rustshot_test_bmp_nonsquare");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("rect.bmp");
        let img = make_patterned_image(13, 7);
        save_image_optimized(&img, &path, Format::Bmp).unwrap();
        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 13);
        assert_eq!(loaded.height(), 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_jpg_file_is_smaller_than_raw() {
        let dir = std::env::temp_dir().join("rustshot_test_jpg_size");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("compressed.jpg");
        let img = make_patterned_image(64, 64);
        save_image_optimized(&img, &path, Format::Jpg).unwrap();
        let raw_size = 64 * 64 * 4; // RGBA raw bytes
        let file_size = std::fs::metadata(&path).unwrap().len();
        assert!(file_size < raw_size as u64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_png_file_size_is_positive() {
        let dir = std::env::temp_dir().join("rustshot_test_png_size");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("size.png");
        let img = make_patterned_image(4, 4);
        save_image_optimized(&img, &path, Format::Png).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_all_formats_same_image() {
        let dir = std::env::temp_dir().join("rustshot_test_all_fmts");
        let _ = std::fs::create_dir_all(&dir);
        let img = make_patterned_image(8, 8);
        for (fmt, ext) in [
            (Format::Png, "png"),
            (Format::Jpg, "jpg"),
            (Format::Bmp, "bmp"),
        ] {
            let path = dir.join(format!("out.{ext}"));
            save_image_optimized(&img, &path, fmt).unwrap();
            let loaded = image::open(&path).unwrap();
            assert_eq!(loaded.width(), 8);
            assert_eq!(loaded.height(), 8);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- bgra_to_rgba: Requirement 14.5 — large buffer length preserved ---

    #[test]
    fn bgra_to_rgba_large_buffer_length_preserved() {
        // 1000 pixels = 4000 bytes, plus 3 trailing bytes
        let len = 1000 * 4 + 3;
        let mut buf: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
        let original_len = buf.len();
        bgra_to_rgba(&mut buf);
        assert_eq!(buf.len(), original_len);
    }

    // =========================================================================
    // Task 1.5 — Gap-filling tests for capture engine requirements
    // Requirements: 1.7, 1.8, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3,
    //              3.4, 3.5, 3.6
    // =========================================================================

    // --- Format: PartialEq not yet exercised (Req 3.1, 3.5, 3.6) ---

    #[test]
    fn format_each_variant_matches_itself() {
        // Verify each variant can be pattern-matched correctly
        assert!(matches!(Format::Png, Format::Png));
        assert!(matches!(Format::Jpg, Format::Jpg));
        assert!(matches!(Format::Bmp, Format::Bmp));
    }

    #[test]
    fn format_variants_are_distinct() {
        // Verify variants don't overlap — each maps to a unique extension and ImageFormat
        let variants = [Format::Png, Format::Jpg, Format::Bmp];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        a.extension(),
                        b.extension(),
                        "{:?} and {:?} should have different extensions",
                        a,
                        b
                    );
                    assert_ne!(
                        a.to_image_format(),
                        b.to_image_format(),
                        "{:?} and {:?} should have different ImageFormats",
                        a,
                        b
                    );
                }
            }
        }
    }

    // --- Format: extension has no leading dot (Req 3.4) ---

    #[test]
    fn format_extension_has_no_leading_dot() {
        // Requirement 3.4: extension() returns lowercase string WITHOUT leading dot
        for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
            let ext = fmt.extension();
            assert!(
                !ext.starts_with('.'),
                "extension for {fmt:?} should not start with dot: {ext}"
            );
        }
    }

    // --- save_image_optimized: 1x1 pixel JPG roundtrip (Req 3.3) ---

    #[test]
    fn save_image_optimized_1x1_jpg_roundtrip_dimensions() {
        // Verify 1x1 pixel edge case for JPG specifically (RGBA→RGB conversion)
        let dir = std::env::temp_dir().join(format!(
            "rustshot-1x1-jpg-rt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1x1.jpg");

        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 100, 50, 255]));
        save_image_optimized(&img, &path, Format::Jpg).unwrap();

        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 1);
        assert_eq!(loaded.height(), 1);
        // JPG is lossy, so we only check dimensions, not exact pixel values

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_image_optimized_1x1_bmp_roundtrip_pixel() {
        // Verify 1x1 pixel edge case for BMP (lossless, should preserve pixel)
        let dir = std::env::temp_dir().join(format!(
            "rustshot-1x1-bmp-rt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1x1.bmp");

        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
        save_image_optimized(&img, &path, Format::Bmp).unwrap();

        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.dimensions(), (1, 1));
        assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- save_image_optimized: JPG drops alpha channel (Req 3.3) ---

    #[test]
    fn save_image_optimized_jpg_output_is_rgb_not_rgba() {
        // Requirement 3.3: JPG converts RGBA to RGB before encoding
        let dir = std::env::temp_dir().join(format!(
            "rustshot-jpg-rgb-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rgb_check.jpg");

        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 128, 64, 200]));
        save_image_optimized(&img, &path, Format::Jpg).unwrap();

        // Re-read as DynamicImage — JPEG files are always RGB (no alpha)
        let loaded = image::open(&path).unwrap();
        // The loaded image should be convertible to RGB without loss
        let rgb = loaded.to_rgb8();
        assert_eq!(rgb.dimensions(), (4, 4));
        // Verify the color channels are approximately correct (JPEG is lossy)
        let pixel = rgb.get_pixel(0, 0);
        assert!((pixel[0] as i16 - 255).unsigned_abs() < 10, "R channel");
        assert!((pixel[1] as i16 - 128).unsigned_abs() < 10, "G channel");
        assert!((pixel[2] as i16 - 64).unsigned_abs() < 10, "B channel");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- bgra_to_rgba: 1-byte and 2-byte inputs (Req 3.1) ---

    #[test]
    fn bgra_to_rgba_one_byte_is_noop() {
        // Less than 4 bytes: no complete pixel, nothing to swap
        let mut pixels = vec![42u8];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![42]);
    }

    #[test]
    fn bgra_to_rgba_two_bytes_is_noop() {
        let mut pixels = vec![10u8, 20];
        bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![10, 20]);
    }

    // --- capture_screenshot_with: capture error prevents ensure_dir (Req 1.7) ---

    #[test]
    fn capture_screenshot_with_capture_error_prevents_ensure_dir() {
        // When capture fails, ensure_dir should never be called
        let ensure_called = Cell::new(false);
        let _ = capture_screenshot_with(
            1,
            None,
            || Err::<(), _>("hardware failure".to_string()),
            Some(PathBuf::from("out.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| {
                ensure_called.set(true);
                Ok(())
            },
            |_: &(), _, _| Ok(()),
        );
        assert!(
            !ensure_called.get(),
            "ensure_dir should not be called when capture fails"
        );
    }

    // --- capture_screenshot_with: directory error prevents save (Req 2.4) ---

    #[test]
    fn capture_screenshot_with_dir_error_prevents_save_and_returns_error() {
        let save_called = Cell::new(false);
        let result = capture_screenshot_with(
            2,
            Some(0),
            || Ok("image-data"),
            Some(PathBuf::from("nested/shot.png")),
            Format::Png,
            &PathBuf::from("."),
            |_| Err("permission denied".to_string()),
            |_, _, _| {
                save_called.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission denied"));
        assert!(
            !save_called.get(),
            "save should not be called when ensure_dir fails"
        );
    }

    // --- resolve_output_path: generated filename pattern (Req 2.2, 2.6) ---

    #[test]
    fn resolve_output_path_generated_filename_matches_pattern() {
        // Requirement 2.2: screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>
        // Requirement 2.6: only alphanumeric, underscores, and dots
        let path = resolve_output_path(None, Format::Png);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap();

        // Must start with "screenshot_"
        assert!(name.starts_with("screenshot_"));

        // Strip prefix and extension to get the timestamp part
        let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
        let timestamp = &stem["screenshot_".len()..];

        // Timestamp format: YYYYMMDD_HHMMSS_NNNNNNNNN (25 chars)
        assert_eq!(
            timestamp.len(),
            25,
            "timestamp should be 25 chars: {timestamp}"
        );

        // Verify only digits and underscores in timestamp
        assert!(
            timestamp.chars().all(|c| c.is_ascii_digit() || c == '_'),
            "timestamp should contain only digits and underscores: {timestamp}"
        );

        // Verify underscore positions: index 8 and 15
        let chars: Vec<char> = timestamp.chars().collect();
        assert_eq!(chars[8], '_', "underscore at position 8");
        assert_eq!(chars[15], '_', "underscore at position 15");
    }

    // --- resolve_output_path: explicit path verbatim (Req 2.3) ---

    #[test]
    fn resolve_output_path_explicit_path_with_spaces_returned_verbatim() {
        // Requirement 2.3: explicit path returned verbatim
        let explicit = PathBuf::from("path with spaces/my screenshot.png");
        let result = resolve_output_path(Some(explicit.clone()), Format::Jpg);
        assert_eq!(result, explicit);
    }

    // --- ensure_parent_directory: idempotency with nested (Req 2.4, 2.5) ---

    #[test]
    fn ensure_parent_directory_idempotent_three_calls() {
        // Requirement 2.5: idempotent — calling multiple times always succeeds
        let dir = std::env::temp_dir().join(format!(
            "rustshot-idem3-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file_path = dir.join("a").join("b").join("shot.png");

        ensure_parent_directory(&file_path).expect("first call");
        ensure_parent_directory(&file_path).expect("second call");
        ensure_parent_directory(&file_path).expect("third call");
        assert!(file_path.parent().unwrap().exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- resolve_screen_index: error message format (Req 1.7) ---

    #[test]
    fn resolve_screen_index_error_contains_requested_index_and_range() {
        // Requirement 1.7: error contains requested index and valid range
        let err = resolve_screen_index(4, Some(10)).unwrap_err();
        assert!(
            err.contains("10"),
            "error should contain requested index 10: {err}"
        );
        assert!(
            err.contains("0-3"),
            "error should contain valid range 0-3: {err}"
        );
    }

    // --- resolve_screen_index: "No screens detected" exact message (Req 1.8) ---

    #[test]
    fn resolve_screen_index_no_screens_exact_message() {
        // Requirement 1.8: exact error message
        assert_eq!(
            resolve_screen_index(0, None).unwrap_err(),
            "No screens detected"
        );
        assert_eq!(
            resolve_screen_index(0, Some(0)).unwrap_err(),
            "No screens detected"
        );
        assert_eq!(
            resolve_screen_index(0, Some(99)).unwrap_err(),
            "No screens detected"
        );
    }

    // --- capture_screenshot_with: all three formats pass correct ImageFormat ---

    #[test]
    fn capture_screenshot_with_all_formats_pass_correct_image_format() {
        // Verify each Format variant passes the correct ImageFormat to save closure
        for (fmt, expected_if) in [
            (Format::Png, ImageFormat::Png),
            (Format::Jpg, ImageFormat::Jpeg),
            (Format::Bmp, ImageFormat::Bmp),
        ] {
            let observed = RefCell::new(None::<ImageFormat>);
            let _ = capture_screenshot_with(
                1,
                None,
                || Ok(()),
                Some(PathBuf::from("test.out")),
                fmt,
                &PathBuf::from("."),
                |_| Ok(()),
                |_, _, image_format| {
                    observed.replace(Some(image_format));
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(
                *observed.borrow(),
                Some(expected_if),
                "Format::{fmt:?} should pass {expected_if:?}"
            );
        }
    }

    // --- resolve_output_path: default directory is Pictures/RustShot (Req 2.1) ---

    #[test]
    fn resolve_output_path_default_directory_ends_with_rustshot() {
        // Requirement 2.1: default directory is Pictures/RustShot/
        let path = resolve_output_path(None, Format::Png);
        let parent = path.parent().unwrap();
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("RustShot")
        );
    }
}
