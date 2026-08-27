//! `#[doc(hidden)]` shims exposing capture internals to `benches/benchmarks.rs`.

use super::*;
use std::cell::RefCell;
use std::path::Path;

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

/// Capture a monitor through the same backend as the production screenshot path.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_capture_monitor_image(screen: &Screen) -> Result<image::RgbaImage, String> {
    #[cfg(windows)]
    {
        crate::record::capture::capture_monitor_image(screen.display_info.raw_handle.0)
    }

    #[cfg(not(windows))]
    {
        let captured = screen
            .capture()
            .map_err(|error| format!("Failed to capture screen: {error}"))?;
        image::RgbaImage::from_raw(captured.width(), captured.height(), captured.into_raw())
            .ok_or_else(|| "Failed to construct the captured image".to_string())
    }
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

/// Run the dependency-injected screenshot pipeline and return its stage trace.
///
/// This keeps executable specifications independent of display hardware while
/// exercising the same short-circuit control flow as production capture.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_capture_pipeline(
    failing_stage: &str,
    error: &str,
) -> (Result<PathBuf, String>, Vec<String>) {
    let stages = RefCell::new(Vec::new());
    let result = capture_screenshot_with(
        1,
        Some(0),
        || {
            stages.borrow_mut().push("capture".to_string());
            if failing_stage == "capture" {
                Err(error.to_string())
            } else {
                Ok(())
            }
        },
        Some(PathBuf::from("bdd-shot.png")),
        Format::Png,
        Path::new("."),
        |_| {
            stages.borrow_mut().push("directory".to_string());
            if failing_stage == "directory" {
                Err(error.to_string())
            } else {
                Ok(())
            }
        },
        |_, _, _| {
            stages.borrow_mut().push("encode".to_string());
            if failing_stage == "encode" {
                Err(error.to_string())
            } else {
                Ok(())
            }
        },
    );
    (result, stages.into_inner())
}
