//! Screenshot capture and file saving.
//!
//! This module handles the core screenshot workflow:
//! 1. Resolve which screen to capture (`resolve_screen_index`, `default_screen_index`).
//! 2. Capture monitor pixels via `screenshots`, or window pixels through the
//!    native Windows Graphics Capture backend.
//! 3. Determine the output path (`resolve_output_path`) and ensure parent dirs exist.
//! 4. Save the image in the requested format (PNG / JPG / BMP).
//!
//! The public entry point is [`capture_screenshot`].  An internal
//! `capture_screenshot_with` variant accepts injected closures for unit testing
//! without real screen hardware.

use image::ImageFormat;
use screenshots::Screen;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use crate::domain::Format;

impl Format {
    /// Map this format to the corresponding [`ImageFormat`] variant.
    pub fn to_image_format(self) -> ImageFormat {
        match self {
            Format::Png => ImageFormat::Png,
            Format::Jpg => ImageFormat::Jpeg,
            Format::Bmp => ImageFormat::Bmp,
        }
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
/// All formats use `BufWriter` for efficient I/O, and `save_image_to_writer`
/// flushes it before returning - see there for why that flush is not optional.
///
/// Deliberately no `File::sync_all`: this runs on the global-hotkey path, and a
/// `FlushFileBuffers` round-trip to the device would be paid on every single
/// shot (milliseconds on an SSD, far worse on a spinning or network drive) to
/// buy durability this tool never promised. The failures a screenshot actually
/// hits do not need it - a full volume and a yanked removable drive are both
/// reported by the `WriteFile` that empties the buffer, which the checked flush
/// below already surfaces. Only an error first discovered during lazy
/// write-back would escape, and that is not worth the per-capture latency.
pub fn save_image_optimized(
    image: &image::RgbaImage,
    path: &Path,
    format: Format,
) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("Failed to create file: {e}"))?;
    // The buffer is wrapped here rather than inside the generic helper because
    // buffering is a property of the file sink: an in-memory caller must not pay
    // for an extra 8 KiB copy, and the unit test that injects a failing writer
    // has to hand over the very writer whose flush error must surface.
    save_image_to_writer(image, std::io::BufWriter::new(file), format)
}

/// Encode `image` into `writer` with the per-format settings documented on
/// [`save_image_optimized`], then flush and report any failure.
///
/// The checked flush is the reason this exists as its own function. Two layers
/// underneath drop write errors on the floor: `BufWriter::drop` ignores the
/// result of its final flush (a destructor must not panic), and
/// `png::Writer::drop` closes the file with `let _ = self.write_iend()` because
/// `image`'s `PngEncoder` never calls `png::Writer::finish`. Nothing in the
/// png, jpeg or bmp encoders flushes the sink, so before this function a full
/// disk or a removed drive made `save_image_optimized` answer `Ok(())` for a
/// truncated screenshot that no viewer can open.
///
/// Every encoder borrows the writer (`&mut writer`) instead of consuming it:
/// `&mut W` implements `Write`, which is all the encoders' `W: Write` bound
/// asks for, and borrowing is what keeps the writer available to flush
/// afterwards. `BmpEncoder` never had a choice - it takes `&'a mut W` by
/// construction - so borrowing is now uniform across the three arms.
fn save_image_to_writer<W: Write>(
    image: &image::RgbaImage,
    mut writer: W,
    format: Format,
) -> Result<(), String> {
    match format {
        Format::Png => {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            let encoder =
                PngEncoder::new_with_quality(&mut writer, CompressionType::Fast, FilterType::Sub);
            image.write_with_encoder(encoder)
        }
        Format::Jpg => {
            use image::codecs::jpeg::JpegEncoder;
            // Encode RGBA block-by-block; JPEG ignores alpha without requiring
            // a full-size temporary RGB allocation.
            //
            // The encoder stays a local of this arm on purpose: it is dropped at
            // the end of the block, which releases its borrow of `writer` before
            // the flush below needs it back.
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, 90);
            encoder.encode_image(image)
        }
        Format::Bmp => {
            use image::codecs::bmp::BmpEncoder;
            let encoder = BmpEncoder::new(&mut writer);
            image.write_with_encoder(encoder)
        }
    }
    .map_err(|e| format!("Failed to save screenshot: {e}"))?;

    // An encoder reporting success only means the bytes reached the buffer. This
    // is where "the volume is full" or "the drive is gone" finally becomes
    // visible, and the message is kept distinct from the encoder's so a truncated
    // file is diagnosable from the log alone.
    writer
        .flush()
        .map_err(|e| format!("Failed to flush screenshot to disk: {e}"))
}

/// Capture a screenshot and save it to the specified path or default location
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
            let cursor_pos = crate::platform::get_cursor_position().ok().map(Into::into);
            let screen_rects: Vec<_> = screens
                .iter()
                .map(|s| {
                    let info = &s.display_info;
                    (info.x, info.y, info.width, info.height, info.is_primary)
                })
                .collect();
            crate::domain::preferred_screen_index(screen_rects, cursor_pos)
                .ok_or_else(|| "No screens detected".to_string())?
        }
    };

    capture_screenshot_with(
        ScreenSelection {
            available: screens.len(),
            index: Some(idx),
        },
        || {
            #[cfg(windows)]
            {
                let info = &screens[idx].display_info;
                crate::platform::monitor::capture_monitor_image(info.raw_handle.0)
            }

            #[cfg(not(windows))]
            let captured = screens[idx]
                .capture()
                .map_err(|e| format!("Failed to capture screen: {}", e))?;
            #[cfg(not(windows))]
            // Forced version bridge, not a stylistic choice: `screenshots` 0.8
            // pins `image` 0.24 internally while RustShot builds against 0.25,
            // so the two `RgbaImage` types are distinct and the buffer has to be
            // moved through their shared raw representation. `into_raw` transfers
            // ownership of the pixel buffer, so this bridge does not copy it.
            image::RgbaImage::from_raw(captured.width(), captured.height(), captured.into_raw())
                .ok_or_else(|| "Failed to convert captured image".to_string())
        },
        ScreenshotDestination {
            output,
            format,
            directory: screenshots_dir,
        },
        ensure_parent_directory,
        |image, path, _image_format| save_image_optimized(image, path, format),
    )
}

/// Get the default screenshots directory
pub fn get_screenshots_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
}

/// A requested monitor index interpreted against the current enumeration.
#[derive(Clone, Copy)]
struct ScreenSelection {
    available: usize,
    index: Option<usize>,
}

/// Output policy shared by destination resolution and image encoding.
struct ScreenshotDestination<'a> {
    output: Option<PathBuf>,
    format: Format,
    directory: &'a Path,
}

/// Internal capture pipeline with dependency injection for unit testing.
fn capture_screenshot_with<CaptureImage, EnsureDir, SaveImage, Img>(
    screen: ScreenSelection,
    capture_image: CaptureImage,
    destination: ScreenshotDestination<'_>,
    ensure_dir: EnsureDir,
    save_image: SaveImage,
) -> Result<PathBuf, String>
where
    CaptureImage: FnOnce() -> Result<Img, String>,
    EnsureDir: Fn(&Path) -> Result<(), String>,
    SaveImage: FnOnce(&Img, &Path, ImageFormat) -> Result<(), String>,
{
    resolve_screen_index(screen.available, screen.index)?;

    let image = capture_image()?;
    let output_path = resolve_output_path_in_dir(
        destination.output,
        destination.format,
        destination.directory,
    );
    ensure_dir(&output_path)?;
    save_image(&image, &output_path, destination.format.to_image_format())?;

    Ok(output_path)
}

/// Capture the currently active (foreground) window and save it to the specified path.
/// Uses Windows Graphics Capture with a GDI fallback. Returns an error on other platforms.
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
    save_active_window_image_in_dir(&rgba_image, output, format, screenshots_dir)
}

/// Save an already captured active-window image using the normal naming policy.
///
/// Keeping capture and persistence separate lets tray mode deliver the exact
/// same pixels to both the clipboard and disk. CLI callers still use
/// [`capture_active_window_in_dir`] and retain their existing file-only flow.
pub(crate) fn save_active_window_image_in_dir(
    rgba_image: &image::RgbaImage,
    output: Option<PathBuf>,
    format: Format,
    screenshots_dir: &Path,
) -> Result<PathBuf, String> {
    let output_path = resolve_output_path_in_dir(output, format, screenshots_dir);
    ensure_parent_directory(&output_path)?;
    save_image_optimized(rgba_image, &output_path, format)?;
    Ok(output_path)
}

/// Capture the active window as an `RgbaImage` (for clipboard use).
pub fn capture_active_window_image() -> Result<image::RgbaImage, String> {
    capture_active_window_platform()
}

/// Snapshot the foreground window's reusable handle identity.
///
/// Keeping the PID/TID beside the raw `HWND` prevents delayed hotkey work from
/// capturing an unrelated window if Windows recycles the handle meanwhile.
pub(crate) fn foreground_window_target() -> Result<WindowTarget, String> {
    platform::foreground_window_target_platform()
}

/// Capture exactly the window represented by an earlier identity snapshot.
pub(crate) fn capture_window_target(target: WindowTarget) -> Result<image::RgbaImage, String> {
    platform::capture_window_target_platform(target)
}

pub use crate::pixels::bgra_to_rgba;
#[cfg(test)]
use crate::pixels::PixelSwizzle;

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

// Submodules split out of this file. The re-exports below keep every
// pre-split path (`crate::capture::…` / `rustshot::capture::…`) resolving
// unchanged, including the `benchmark_*` shims used by benches/benchmarks.rs.
mod bench_support;
use crate::platform::window as platform;

pub use bench_support::*;
use platform::capture_active_window_platform;
#[cfg(windows)]
pub(crate) use platform::capture_hwnd;
pub(crate) use platform::WindowTarget;

#[cfg(test)]
#[path = "../tests/unit/capture/mod.rs"]
mod tests;
