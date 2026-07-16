//! `#[doc(hidden)]` shims exposing capture internals to `benches/benchmarks.rs`.

use super::*;
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
