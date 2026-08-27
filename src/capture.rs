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
            let cursor_pos = crate::selection::get_cursor_position().ok().map(Into::into);
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
            #[cfg(windows)]
            {
                let info = &screens[idx].display_info;
                crate::record::capture::capture_monitor_image(info.raw_handle.0)
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
/// Uses Windows Graphics Capture with a GDI fallback. Returns an error on other platforms.
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
/// The x86/x86-64 implementation uses AVX2 when the CPU supports it and falls
/// back to the portable scalar loop everywhere else. The runtime feature check
/// keeps the binary compatible with older processors.
///
/// Possible improvement:
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
    PixelSwizzle::detect().convert_in_place(pixels);
}

#[inline]
fn bgra_to_rgba_scalar(pixels: &mut [u8]) {
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2); // swap B and R
    }
}

/// AVX2 byte-shuffle control that swaps red and blue inside each 4-byte pixel,
/// duplicated across both 128-bit lanes. Shared by the in-place and copying
/// converters so the two cannot drift apart.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const RED_BLUE_SHUFFLE: [i8; 32] = [
    2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15, 2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14,
    13, 12, 15,
];

/// Swap red and blue inside `pixels` in place, 32 bytes per `pshufb`.
///
/// # Safety
///
/// The CPU must actually support AVX2. That is the whole of this function's
/// contract - it has no requirement on `pixels`, because the length arithmetic
/// below derives every offset from `pixels.len()` and the scalar tail finishes
/// whatever the 32-byte loop could not reach. [`PixelSwizzle::detect`] is what
/// establishes the feature for the only caller.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn bgra_to_rgba_avx2(pixels: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};

    // SAFETY: `RED_BLUE_SHUFFLE` is a 32-byte `[i8; 32]` const, so the read of
    // 32 bytes from its start is in bounds, and the intrinsic permits unaligned
    // loads - the array carries no 32-byte alignment guarantee.
    let shuffle = unsafe { _mm256_loadu_si256(RED_BLUE_SHUFFLE.as_ptr().cast()) };
    let vectorized_len = pixels.len() / 32 * 32;
    let mut offset = 0;
    while offset < vectorized_len {
        // SAFETY: `offset` starts at 0 and only grows by 32, and `vectorized_len`
        // is at most `pixels.len()`, so `offset` is a byte index strictly inside
        // the slice and `add` stays within its allocation.
        let pointer = unsafe { pixels.as_mut_ptr().add(offset).cast() };
        // SAFETY: `offset` and `vectorized_len` are both multiples of 32 with
        // `offset < vectorized_len`, so `vectorized_len - offset >= 32`: the 32
        // bytes this reads are all inside `pixels`. The intrinsic permits
        // unaligned addresses, which a byte slice cannot promise anything about.
        let value = unsafe { _mm256_loadu_si256(pointer) };
        let value = _mm256_shuffle_epi8(value, shuffle);
        // SAFETY: writes back exactly the 32 in-bounds bytes the load above read,
        // through the `&mut [u8]` those bytes came from, so nothing else aliases
        // them for the duration. `_mm256_storeu_si256` is likewise unaligned.
        unsafe { _mm256_storeu_si256(pointer, value) };
        offset += 32;
    }

    bgra_to_rgba_scalar(&mut pixels[vectorized_len..]);
}

/// Which red/blue swizzle implementation this CPU should use.
///
/// BGRA and RGBA differ only by swapping red and blue, so every conversion in
/// either direction goes through this one enum. It exists as a value rather than
/// a check inside the conversion because callers convert a frame *row at a
/// time*: resolving the choice once per frame and carrying it into the row loop
/// keeps the CPU-feature probe out of a loop that runs thousands of times, and
/// the answer cannot change while the process lives.
///
/// The AVX2 variant is unrepresentable off x86, so no caller can ask for a path
/// the target does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelSwizzle {
    /// 32-byte `pshufb` blocks with a portable tail.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2,
    /// Four bytes at a time, everywhere.
    Portable,
}

impl PixelSwizzle {
    /// Pick the fastest implementation the running CPU supports.
    pub(crate) fn detect() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if std::arch::is_x86_feature_detected!("avx2") {
            return Self::Avx2;
        }
        Self::Portable
    }

    /// Copy `source` into `destination`, swapping each pixel's red and blue.
    ///
    /// Converts only whole pixels within the shorter of the two slices; a
    /// trailing partial pixel is left untouched, matching [`bgra_to_rgba`].
    pub(crate) fn convert(self, source: &[u8], destination: &mut [u8]) {
        let len = source.len().min(destination.len()) / 4 * 4;
        let source = &source[..len];
        let destination = &mut destination[..len];

        match self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            // SAFETY: `detect` yields `Avx2` only when the CPU reports AVX2.
            // Both slices were trimmed to one shared whole-pixel length.
            Self::Avx2 => unsafe { swap_red_blue_into_avx2(source, destination) },
            Self::Portable => swap_red_blue_into_portable(source, destination),
        }
    }

    /// Swap red and blue within `pixels`, in place.
    pub(crate) fn convert_in_place(self, pixels: &mut [u8]) {
        let complete_len = pixels.len() / 4 * 4;
        let pixels = &mut pixels[..complete_len];

        match self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            // SAFETY: as above; the helper stays inside the whole-pixel prefix.
            Self::Avx2 => unsafe { bgra_to_rgba_avx2(pixels) },
            Self::Portable => bgra_to_rgba_scalar(pixels),
        }
    }
}

#[inline]
fn swap_red_blue_into_portable(source: &[u8], destination: &mut [u8]) {
    for (source, destination) in source
        .as_chunks::<4>()
        .0
        .iter()
        .zip(destination.as_chunks_mut::<4>().0.iter_mut())
    {
        *destination = [source[2], source[1], source[0], source[3]];
    }
}

/// Copy `source` into `destination` with red and blue swapped, 32 bytes per
/// `pshufb`.
///
/// # Safety
///
/// Two preconditions, both discharged by [`PixelSwizzle::convert`]:
/// - the CPU must actually support AVX2;
/// - `destination.len()` must be at least `source.len()`. Every offset below is
///   derived from the *source* length, so a shorter destination would be written
///   past its end. `convert` trims both slices to one shared length first.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn swap_red_blue_into_avx2(source: &[u8], destination: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};

    // SAFETY: `RED_BLUE_SHUFFLE` is a 32-byte `[i8; 32]` const, so the read of
    // 32 bytes from its start is in bounds, and the intrinsic permits unaligned
    // loads - the array carries no 32-byte alignment guarantee.
    let shuffle = unsafe { _mm256_loadu_si256(RED_BLUE_SHUFFLE.as_ptr().cast()) };
    let vectorized_len = source.len() / 32 * 32;
    let mut offset = 0;
    while offset < vectorized_len {
        // SAFETY: `offset` starts at 0 and only grows by 32, and `vectorized_len`
        // is at most `source.len()`, so `offset` indexes strictly inside `source`.
        let source_pointer = unsafe { source.as_ptr().add(offset).cast() };
        // SAFETY: the same index, in bounds of `destination` too because this
        // function's contract requires `destination.len() >= source.len()`.
        let destination_pointer = unsafe { destination.as_mut_ptr().add(offset).cast() };
        // SAFETY: `offset` and `vectorized_len` are both multiples of 32 with
        // `offset < vectorized_len`, so `vectorized_len - offset >= 32` and all 32
        // bytes read lie inside `source`. Unaligned loads are permitted.
        let value = unsafe { _mm256_loadu_si256(source_pointer) };
        let value = _mm256_shuffle_epi8(value, shuffle);
        // SAFETY: the mirrored 32 bytes of `destination`, in bounds by the length
        // precondition above. `destination` is a `&mut` slice, so the write is
        // unaliased - and it cannot overlap `source`, which is a shared borrow of
        // a distinct slice.
        unsafe { _mm256_storeu_si256(destination_pointer, value) };
        offset += 32;
    }

    swap_red_blue_into_portable(
        &source[vectorized_len..],
        &mut destination[vectorized_len..],
    );
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

// Submodules split out of this file. The re-exports below keep every
// pre-split path (`crate::capture::…` / `rustshot::capture::…`) resolving
// unchanged, including the `benchmark_*` shims used by benches/benchmarks.rs.
mod bench_support;
mod platform;

// Consumed only through the library tree (benches import
// `rustshot::capture::benchmark_*`); unused in the binary tree by design.
#[allow(unused_imports)]
pub use bench_support::*;
use platform::capture_active_window_platform;
#[cfg(windows)]
pub(crate) use platform::capture_hwnd;
#[cfg(windows)]
pub(crate) use platform::pin_process_mta;

#[cfg(test)]
mod tests;
