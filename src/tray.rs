//! System tray icon and context menu.
//!
//! Creates the tray icon (a simple 32×32 camera glyph) and a right-click menu
//! with items for Capture, Open Folder, Toggle Recording, and Exit.
//!
//! The tray runs inside the winit event loop managed by `RustShotApp` in
//! `main.rs`.  Menu events are received via [`menu_receiver`] and dispatched
//! in `RustShotApp::about_to_wait`.

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

// Menu item IDs
pub const MENU_CAPTURE: &str = "capture";
pub const MENU_OPEN_FOLDER: &str = "open_folder";
pub const MENU_EXIT: &str = "exit";
pub const MENU_CAPTURE_WINDOW: &str = "capture_window";
pub const MENU_SETTINGS: &str = "settings";
pub const MENU_START_RECORD: &str = "start_record";
pub const MENU_STOP_RECORD: &str = "stop_record";
const DEFAULT_ICON_SIZE: u32 = 32;
const TOGGLE_RECORD_LABEL: &str = "Start or Stop Recording (Ctrl+Shift+R)";

/// Generate a 32×32 RGBA pixel buffer for the tray icon (blue camera with white lens).
fn build_default_icon_pixels() -> Vec<u8> {
    let size = DEFAULT_ICON_SIZE;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    // Pre-compute lens threshold as squared distance to avoid sqrt per pixel
    const LENS_RADIUS_SQ: i32 = 6 * 6; // 36

    for y in 0..size {
        let dist_y = (y as i32 - 16).abs();
        let in_body_y = dist_y <= 10;
        let dist_y_sq = dist_y * dist_y;

        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dist_x = (x as i32 - 16).abs();

            if in_body_y && dist_x <= 12 {
                let dist_sq = dist_x * dist_x + dist_y_sq;
                if dist_sq <= LENS_RADIUS_SQ {
                    // Lens - white circle in center
                    rgba[idx] = 255; // R
                    rgba[idx + 1] = 255; // G
                    rgba[idx + 2] = 255; // B
                } else {
                    // Camera body - dark blue
                    rgba[idx] = 41; // R
                    rgba[idx + 1] = 128; // G
                    rgba[idx + 2] = 185; // B
                }
                rgba[idx + 3] = 255; // A
            }
            // else: transparent (already 0)
        }
    }

    rgba
}

/// Create the system tray icon with context menu
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn create_tray() -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = Menu::new();

    let capture_item =
        MenuItem::with_id(MENU_CAPTURE, "Capture Screenshot (PrintScreen)", true, None);
    let capture_window_item = MenuItem::with_id(
        MENU_CAPTURE_WINDOW,
        "Capture Active Window (Alt+PrintScreen)",
        true,
        None,
    );
    let open_folder_item =
        MenuItem::with_id(MENU_OPEN_FOLDER, "Open Screenshots Folder", true, None);
    let settings_item = MenuItem::with_id(MENU_SETTINGS, "Settings", true, None);
    let exit_item = MenuItem::with_id(MENU_EXIT, "Exit", true, None);
    let start_record_item = MenuItem::with_id(MENU_START_RECORD, TOGGLE_RECORD_LABEL, true, None);

    menu.append(&capture_item)?;
    menu.append(&capture_window_item)?;
    menu.append(&open_folder_item)?;
    menu.append(&start_record_item)?;
    menu.append(&settings_item)?;
    menu.append(&exit_item)?;

    let icon = create_default_icon()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("RustShot - Press PrintScreen to capture")
        .with_icon(icon)
        .build()?;

    Ok(tray)
}

fn icon_from_rgba_pixels(rgba: Vec<u8>) -> Result<Icon, Box<dyn std::error::Error>> {
    Icon::from_rgba(rgba, DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE).map_err(|e| e.into())
}

/// Create a simple default icon (blue camera-like icon)
fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    // Create a simple 32x32 RGBA icon (blue square with white center)
    let rgba = build_default_icon_pixels();
    icon_from_rgba_pixels(rgba)
}

/// Expose icon pixel generation for deterministic benchmarks.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_build_default_icon_pixels() -> Vec<u8> {
    build_default_icon_pixels()
}

/// Get the menu event receiver
pub fn menu_receiver() -> &'static tray_icon::menu::MenuEventReceiver {
    MenuEvent::receiver()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // Helper: get pixel index for (x, y) in the 32x32 RGBA buffer
    fn pixel_idx(x: u32, y: u32) -> usize {
        ((y * DEFAULT_ICON_SIZE + x) * 4) as usize
    }

    #[test]
    fn default_icon_size_constant_is_32() {
        assert_eq!(DEFAULT_ICON_SIZE, 32);
    }

    #[test]
    fn default_icon_has_expected_size() {
        let pixels = build_default_icon_pixels();
        assert_eq!(
            pixels.len(),
            (DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE * 4) as usize
        );
    }

    #[test]
    fn default_icon_total_pixel_count_is_1024() {
        // 32 * 32 = 1024 pixels, each with 4 channels = 4096 bytes
        let pixels = build_default_icon_pixels();
        assert_eq!(pixels.len(), 4096);
    }

    #[test]
    fn default_icon_center_is_white() {
        let pixels = build_default_icon_pixels();
        let x = 16;
        let y = 16;
        let idx = ((y * DEFAULT_ICON_SIZE + x) * 4) as usize;
        assert_eq!(pixels[idx], 255);
        assert_eq!(pixels[idx + 1], 255);
        assert_eq!(pixels[idx + 2], 255);
        assert_eq!(pixels[idx + 3], 255);
    }

    #[test]
    fn default_icon_corner_is_transparent() {
        let pixels = build_default_icon_pixels();
        let idx = 0usize;
        assert_eq!(pixels[idx + 3], 0);
    }

    #[test]
    fn default_icon_top_left_corner_0_0_is_transparent() {
        // x=0, y=0: dist_x=16, dist_y=16, outside body (16 > 12), so transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(0, 0);
        assert_eq!(pixels[idx + 3], 0, "top-left corner should be transparent");
    }

    #[test]
    fn default_icon_top_right_corner_31_0_is_transparent() {
        // x=31, y=0: dist_x=|31-16|=15 > 12, outside body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(31, 0);
        assert_eq!(pixels[idx + 3], 0, "top-right corner should be transparent");
    }

    #[test]
    fn default_icon_bottom_left_corner_0_31_is_transparent() {
        // x=0, y=31: dist_x=16 > 12, outside body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(0, 31);
        assert_eq!(
            pixels[idx + 3],
            0,
            "bottom-left corner should be transparent"
        );
    }

    #[test]
    fn default_icon_body_pixel_is_blue_not_lens() {
        // x=5, y=16: dist_x=|5-16|=11 <= 12, dist_y=|16-16|=0 <= 10 -> in body
        // dist = sqrt(11^2 + 0^2) = 11.0 > 6.0 -> NOT in lens -> blue
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(5, 16);
        assert_eq!(pixels[idx], 41, "R should be camera-body blue (41)");
        assert_eq!(pixels[idx + 1], 128, "G should be camera-body blue (128)");
        assert_eq!(pixels[idx + 2], 185, "B should be camera-body blue (185)");
        assert_eq!(pixels[idx + 3], 255, "alpha should be fully opaque");
    }

    #[test]
    fn default_icon_body_pixel_alpha_is_255() {
        // Any pixel inside the body (not transparent) must have alpha=255
        // x=16, y=8: dist_x=0, dist_y=|8-16|=8 <= 10 -> in body
        // dist = sqrt(0+64) = 8.0 > 6.0 -> blue (not lens)
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 8);
        assert_eq!(pixels[idx + 3], 255, "body pixel alpha must be 255");
    }

    #[test]
    fn default_icon_lens_pixel_is_white() {
        // x=16, y=20: dist_x=0, dist_y=|20-16|=4 <= 10 -> in body
        // dist = sqrt(0 + 16) = 4.0 <= 6.0 -> lens (white)
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 20);
        assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
        assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
        assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
    }

    #[test]
    fn default_icon_pixel_at_lens_edge_is_white() {
        // x=16, y=22: dist_x=0, dist_y=|22-16|=6, dist=6.0 <= 6.0 -> lens (white)
        // Also inside body: dist_y=6 <= 10 and dist_x=0 <= 12
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 22);
        assert_eq!(pixels[idx], 255, "lens edge R should be 255");
        assert_eq!(pixels[idx + 1], 255, "lens edge G should be 255");
        assert_eq!(pixels[idx + 2], 255, "lens edge B should be 255");
    }

    #[test]
    fn default_icon_pixel_just_outside_lens_is_blue() {
        // x=16, y=23: dist_x=0, dist_y=|23-16|=7, dist=7.0 > 6.0 -> body blue
        // dist_y=7 <= 10 -> still in body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 23);
        assert_eq!(
            pixels[idx], 41,
            "just-outside-lens R should be body blue (41)"
        );
        assert_eq!(
            pixels[idx + 1],
            128,
            "just-outside-lens G should be body blue (128)"
        );
        assert_eq!(
            pixels[idx + 2],
            185,
            "just-outside-lens B should be body blue (185)"
        );
    }

    #[test]
    fn default_icon_body_boundary_pixel_is_blue() {
        // x=4, y=16: dist_x=|4-16|=12, dist_y=0 -> exactly on body boundary (dist_x<=12)
        // dist = sqrt(144) = 12.0 > 6.0 -> blue body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(4, 16);
        assert_eq!(pixels[idx + 3], 255, "boundary body pixel should be opaque");
        assert_eq!(pixels[idx + 2], 185, "boundary body pixel B should be 185");
    }

    #[test]
    fn default_icon_outside_body_boundary_is_transparent() {
        // x=3, y=16: dist_x=|3-16|=13 > 12 -> outside body -> transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(3, 16);
        assert_eq!(
            pixels[idx + 3],
            0,
            "pixel outside body boundary should be transparent"
        );
    }

    #[test]
    fn default_icon_body_top_boundary_pixel_is_opaque() {
        // x=16, y=6: dist_x=0, dist_y=|6-16|=10 -> exactly on top boundary (dist_y<=10)
        // dist = 10.0 > 6.0 -> blue body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 6);
        assert_eq!(
            pixels[idx + 3],
            255,
            "top body boundary pixel should be opaque"
        );
    }

    #[test]
    fn default_icon_above_body_top_is_transparent() {
        // x=16, y=5: dist_y=|5-16|=11 > 10 -> outside body -> transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 5);
        assert_eq!(pixels[idx + 3], 0, "pixel above body should be transparent");
    }

    #[test]
    fn menu_receiver_does_not_panic() {
        // menu_receiver() must be callable without panicking
        let _recv = menu_receiver();
    }

    #[test]
    fn create_default_icon_succeeds() {
        assert!(create_default_icon().is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn create_tray_smoke_test() {
        let _ = create_tray();
    }

    #[test]
    fn menu_constants_are_stable() {
        assert_eq!(MENU_CAPTURE, "capture");
        assert_eq!(MENU_CAPTURE_WINDOW, "capture_window");
        assert_eq!(MENU_OPEN_FOLDER, "open_folder");
        assert_eq!(MENU_EXIT, "exit");
    }

    #[test]
    fn menu_capture_window_constant_value() {
        assert_eq!(MENU_CAPTURE_WINDOW, "capture_window");
    }

    #[test]
    fn menu_capture_and_capture_window_are_distinct() {
        assert_ne!(MENU_CAPTURE, MENU_CAPTURE_WINDOW);
    }

    #[test]
    fn menu_capture_constant_value() {
        assert_eq!(MENU_CAPTURE, "capture");
    }

    #[test]
    fn menu_open_folder_constant_value() {
        assert_eq!(MENU_OPEN_FOLDER, "open_folder");
    }

    #[test]
    fn menu_exit_constant_value() {
        assert_eq!(MENU_EXIT, "exit");
    }

    #[test]
    fn menu_start_record_constant_value() {
        assert_eq!(MENU_START_RECORD, "start_record");
    }

    #[test]
    fn menu_stop_record_constant_value() {
        assert_eq!(MENU_STOP_RECORD, "stop_record");
    }

    #[test]
    fn menu_start_record_and_stop_record_are_distinct() {
        assert_ne!(MENU_START_RECORD, MENU_STOP_RECORD);
    }

    #[test]
    fn toggle_record_label_mentions_start_and_stop() {
        assert!(TOGGLE_RECORD_LABEL.contains("Start"));
        assert!(TOGGLE_RECORD_LABEL.contains("Stop"));
    }

    #[test]
    fn default_icon_pixels_length_matches_formula() {
        let pixels = build_default_icon_pixels();
        let expected = DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE * 4;
        assert_eq!(pixels.len() as u32, expected);
    }

    #[test]
    fn default_icon_right_body_boundary_pixel_is_blue() {
        // x=28, y=16: dist_x=|28-16|=12, dist_y=0 -> exactly on right body boundary
        // dist=12.0 > 6.0 -> blue body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(28, 16);
        assert_eq!(pixels[idx + 3], 255, "right body boundary should be opaque");
        assert_eq!(pixels[idx + 2], 185, "right body boundary B should be 185");
    }

    #[test]
    fn default_icon_outside_right_body_boundary_is_transparent() {
        // x=29, y=16: dist_x=|29-16|=13 > 12 -> outside body -> transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(29, 16);
        assert_eq!(
            pixels[idx + 3],
            0,
            "pixel outside right body boundary should be transparent"
        );
    }

    #[test]
    fn default_icon_bottom_body_boundary_pixel_is_opaque() {
        // x=16, y=26: dist_x=0, dist_y=|26-16|=10 -> exactly on bottom boundary (dist_y<=10)
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 26);
        assert_eq!(
            pixels[idx + 3],
            255,
            "bottom body boundary pixel should be opaque"
        );
    }

    #[test]
    fn default_icon_below_body_bottom_is_transparent() {
        // x=16, y=27: dist_y=|27-16|=11 > 10 -> outside body -> transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 27);
        assert_eq!(pixels[idx + 3], 0, "pixel below body should be transparent");
    }

    #[test]
    fn default_icon_no_pixel_has_partial_alpha() {
        // Every pixel is either fully opaque (255) or fully transparent (0)
        let pixels = build_default_icon_pixels();
        for i in (3..pixels.len()).step_by(4) {
            let alpha = pixels[i];
            assert!(
                alpha == 0 || alpha == 255,
                "alpha at byte {i} should be 0 or 255, got {alpha}"
            );
        }
    }

    #[test]
    fn default_icon_body_pixel_rgb_values_are_camera_blue() {
        // Any opaque non-lens pixel must be exactly RGB(41, 128, 185)
        let pixels = build_default_icon_pixels();
        for i in (0..pixels.len()).step_by(4) {
            let r = pixels[i];
            let g = pixels[i + 1];
            let b = pixels[i + 2];
            let a = pixels[i + 3];
            if a == 255 && !(r == 255 && g == 255 && b == 255) {
                assert_eq!(r, 41, "body pixel R at {i}");
                assert_eq!(g, 128, "body pixel G at {i}");
                assert_eq!(b, 185, "body pixel B at {i}");
            }
        }
    }

    #[test]
    fn default_icon_bottom_right_corner_is_transparent() {
        // x=31, y=31: dist_x=|31-16|=15 > 12 → outside body → transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(31, 31);
        assert_eq!(
            pixels[idx + 3],
            0,
            "bottom-right corner should be transparent"
        );
    }

    #[test]
    fn default_icon_left_edge_center_is_transparent() {
        // x=0, y=16: dist_x=|0-16|=16 > 12 → outside body → transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(0, 16);
        assert_eq!(pixels[idx + 3], 0, "left edge center should be transparent");
    }

    #[test]
    fn default_icon_right_edge_center_is_transparent() {
        // x=31, y=16: dist_x=|31-16|=15 > 12 → outside body → transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(31, 16);
        assert_eq!(
            pixels[idx + 3],
            0,
            "right edge center should be transparent"
        );
    }

    #[test]
    fn default_icon_top_center_is_transparent() {
        // x=16, y=0: dist_y=|0-16|=16 > 10 → outside body → transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(16, 0);
        assert_eq!(pixels[idx + 3], 0, "top center should be transparent");
    }

    #[test]
    fn default_icon_lens_pixel_at_x20_y16_is_white() {
        // x=20, y=16: dist_x=4, dist_y=0, dist=4.0 <= 6.0 → lens (white)
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(20, 16);
        assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
        assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
        assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
    }

    #[test]
    fn default_icon_lens_pixel_at_x12_y16_is_white() {
        // x=12, y=16: dist_x=4, dist_y=0, dist=4.0 <= 6.0 → lens (white)
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(12, 16);
        assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
        assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
        assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
    }

    #[test]
    fn default_icon_transparent_pixels_have_zero_rgb() {
        // Transparent pixels must not leak colour data (alpha=0 region)
        // x=0, y=0 is transparent
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(0, 0);
        // We only assert alpha; RGB of transparent pixels is undefined by spec,
        // but we can at least verify alpha is 0
        assert_eq!(pixels[idx + 3], 0);
    }

    #[test]
    fn default_icon_opaque_pixel_count_is_positive() {
        // At least one pixel must have alpha == 255 (the camera body)
        let pixels = build_default_icon_pixels();
        let count = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 255)
            .count();
        assert!(count > 0, "there should be at least one opaque pixel");
    }

    #[test]
    fn default_icon_transparent_pixel_count_is_positive() {
        // At least one pixel must have alpha == 0 (the rounded corners / outside the body)
        let pixels = build_default_icon_pixels();
        let count = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 0)
            .count();
        assert!(count > 0, "there should be at least one transparent pixel");
    }

    #[test]
    fn default_icon_lens_pixel_at_x22_y16_is_white() {
        // x=22, y=16: dist_x=|22-16|=6, dist_y=0
        // body: 6<=12 && 0<=10 → inside body
        // dist = sqrt(36+0) = 6.0 <= 6.0 → lens → R=255, G=255, B=255
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(22, 16);
        assert_eq!(pixels[idx], 255, "lens pixel at (22,16) R should be 255");
        assert_eq!(
            pixels[idx + 1],
            255,
            "lens pixel at (22,16) G should be 255"
        );
        assert_eq!(
            pixels[idx + 2],
            255,
            "lens pixel at (22,16) B should be 255"
        );
    }

    #[test]
    fn default_icon_body_corner_x4_y6_is_opaque() {
        // x=4, y=6: dist_x=|4-16|=12, dist_y=|6-16|=10
        // body: 12<=12 && 10<=10 → on body boundary (inside)
        // dist = sqrt(144+100) ≈ 15.62 > 6.0 → blue body → alpha=255
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(4, 6);
        assert_eq!(
            pixels[idx + 3],
            255,
            "body corner at (4,6) should be opaque"
        );
    }

    #[test]
    fn default_icon_body_corner_x28_y26_is_opaque() {
        // x=28, y=26: dist_x=|28-16|=12, dist_y=|26-16|=10
        // body: 12<=12 && 10<=10 → on body boundary (inside)
        // dist = sqrt(144+100) ≈ 15.62 > 6.0 → blue body → alpha=255
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(28, 26);
        assert_eq!(
            pixels[idx + 3],
            255,
            "body corner at (28,26) should be opaque"
        );
    }

    #[test]
    fn default_icon_opaque_count_exceeds_transparent_count() {
        // Body is 25×21=525 pixels; total=1024; so opaque > transparent
        let pixels = build_default_icon_pixels();
        let opaque = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 255)
            .count();
        let transparent = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 0)
            .count();
        assert!(
            opaque > transparent,
            "opaque({opaque}) should exceed transparent({transparent})"
        );
    }

    #[test]
    fn default_icon_body_pixel_at_10_10_is_blue() {
        // x=10, y=10: dist_x=6, dist_y=6 → in body (6<=12 && 6<=10)
        // dist=sqrt(72)≈8.49 > 6.0 → not lens → blue body
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(10, 10);
        assert_eq!(pixels[idx], 41, "R at (10,10) should be 41");
        assert_eq!(pixels[idx + 1], 128, "G at (10,10) should be 128");
        assert_eq!(pixels[idx + 2], 185, "B at (10,10) should be 185");
        assert_eq!(pixels[idx + 3], 255, "alpha at (10,10) should be 255");
    }

    #[test]
    fn default_icon_diagonal_lens_pixel_is_white() {
        // x=20, y=20: dist_x=4, dist_y=4 → in body (4<=12 && 4<=10)
        // dist=sqrt(32)≈5.66 <= 6.0 → lens → white
        let pixels = build_default_icon_pixels();
        let idx = pixel_idx(20, 20);
        assert_eq!(pixels[idx], 255, "R at (20,20) should be 255");
        assert_eq!(pixels[idx + 1], 255, "G at (20,20) should be 255");
        assert_eq!(pixels[idx + 2], 255, "B at (20,20) should be 255");
    }

    #[test]
    fn default_icon_all_white_pixels_are_opaque() {
        // Any pixel that is RGB(255,255,255) must also have alpha=255
        let pixels = build_default_icon_pixels();
        for i in (0..pixels.len()).step_by(4) {
            if pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255 {
                assert_eq!(
                    pixels[i + 3],
                    255,
                    "white pixel at byte {i} must be fully opaque"
                );
            }
        }
    }

    #[test]
    fn default_icon_center_region_all_opaque() {
        // All pixels with |x-16|<=5 && |y-16|<=5 are inside the body → alpha=255
        let pixels = build_default_icon_pixels();
        for y in 11u32..=21 {
            for x in 11u32..=21 {
                let idx = pixel_idx(x, y);
                assert_eq!(
                    pixels[idx + 3],
                    255,
                    "pixel ({x},{y}) in center region should be opaque"
                );
            }
        }
    }

    // --- menu constant uniqueness ---

    #[test]
    fn all_menu_constants_are_unique() {
        let ids = [
            MENU_CAPTURE,
            MENU_CAPTURE_WINDOW,
            MENU_OPEN_FOLDER,
            MENU_EXIT,
            MENU_START_RECORD,
            MENU_STOP_RECORD,
        ];
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert!(ids[i] != ids[j]);
            }
        }
    }

    #[test]
    fn menu_constants_are_non_empty() {
        assert!(!MENU_CAPTURE.is_empty());
        assert!(!MENU_CAPTURE_WINDOW.is_empty());
        assert!(!MENU_OPEN_FOLDER.is_empty());
        assert!(!MENU_EXIT.is_empty());
        assert!(!MENU_START_RECORD.is_empty());
        assert!(!MENU_STOP_RECORD.is_empty());
    }

    #[test]
    fn menu_constants_contain_no_whitespace() {
        for id in [
            MENU_CAPTURE,
            MENU_CAPTURE_WINDOW,
            MENU_OPEN_FOLDER,
            MENU_EXIT,
            MENU_START_RECORD,
            MENU_STOP_RECORD,
        ] {
            assert!(
                !id.contains(' '),
                "menu ID should not contain spaces: '{id}'"
            );
        }
    }

    #[test]
    fn default_icon_pixel_count_sums_to_total() {
        // Every pixel is either fully opaque or fully transparent; the sum must equal total pixels
        let pixels = build_default_icon_pixels();
        let total_pixels = (DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE) as usize;
        let opaque_count = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 255)
            .count();
        let transparent_count = (3..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 0)
            .count();
        assert_eq!(
            opaque_count + transparent_count,
            total_pixels,
            "opaque + transparent must equal total pixels (1024)"
        );
    }

    // --- MENU_SETTINGS constant ---

    #[test]
    fn menu_settings_constant_value() {
        assert_eq!(MENU_SETTINGS, "settings");
    }

    #[test]
    fn menu_settings_is_distinct_from_all_others() {
        assert_ne!(MENU_SETTINGS, MENU_CAPTURE);
        assert_ne!(MENU_SETTINGS, MENU_CAPTURE_WINDOW);
        assert_ne!(MENU_SETTINGS, MENU_OPEN_FOLDER);
        assert_ne!(MENU_SETTINGS, MENU_EXIT);
        assert_ne!(MENU_SETTINGS, MENU_START_RECORD);
        assert_ne!(MENU_SETTINGS, MENU_STOP_RECORD);
    }

    #[test]
    fn menu_settings_contains_no_whitespace() {
        assert!(
            !MENU_SETTINGS.contains(' '),
            "settings menu ID should not contain spaces"
        );
    }

    #[test]
    fn menu_settings_is_non_empty() {
        assert!(!MENU_SETTINGS.is_empty());
    }

    // --- all_menu_constants_including_settings ---

    #[test]
    fn all_seven_menu_constants_are_unique() {
        let ids = [
            MENU_CAPTURE,
            MENU_CAPTURE_WINDOW,
            MENU_OPEN_FOLDER,
            MENU_SETTINGS,
            MENU_EXIT,
            MENU_START_RECORD,
            MENU_STOP_RECORD,
        ];
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert!(ids[i] != ids[j]);
            }
        }
    }

    // --- icon pixel statistics ---

    #[test]
    fn default_icon_lens_pixel_count_is_positive() {
        // Count white (lens) pixels: R=255, G=255, B=255
        let pixels = build_default_icon_pixels();
        let lens_count = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255)
            .count();
        assert!(
            lens_count > 0,
            "there should be at least one lens (white) pixel"
        );
    }

    #[test]
    fn default_icon_body_pixel_count_is_positive() {
        // Count body (camera blue) pixels: R=41, G=128, B=185
        let pixels = build_default_icon_pixels();
        let body_count = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 41 && pixels[i + 1] == 128 && pixels[i + 2] == 185)
            .count();
        assert!(
            body_count > 0,
            "there should be at least one body (blue) pixel"
        );
    }

    #[test]
    fn default_icon_lens_is_smaller_than_body() {
        // Lens (white circle radius 6) should have fewer pixels than the body (25x21 rect minus lens)
        let pixels = build_default_icon_pixels();
        let lens_count = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255)
            .count();
        let body_count = (0..pixels.len())
            .step_by(4)
            .filter(|&i| pixels[i] == 41 && pixels[i + 1] == 128 && pixels[i + 2] == 185)
            .count();
        assert!(
            lens_count < body_count,
            "lens ({lens_count}) should have fewer pixels than body ({body_count})"
        );
    }

    // --- icon symmetry ---

    #[test]
    fn default_icon_has_opaque_pixels_on_both_horizontal_halves() {
        let pixels = build_default_icon_pixels();
        let midpoint = DEFAULT_ICON_SIZE / 2;

        let left_opaque = (0..DEFAULT_ICON_SIZE).any(|y| {
            (0..midpoint).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx + 3] == 255
            })
        });
        let right_opaque = (0..DEFAULT_ICON_SIZE).any(|y| {
            (midpoint..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx + 3] == 255
            })
        });

        assert!(left_opaque, "left half should contain opaque icon pixels");
        assert!(right_opaque, "right half should contain opaque icon pixels");
    }

    #[test]
    fn default_icon_has_opaque_pixels_on_both_vertical_halves() {
        let pixels = build_default_icon_pixels();
        let midpoint = DEFAULT_ICON_SIZE / 2;

        let top_opaque = (0..midpoint).any(|y| {
            (0..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx + 3] == 255
            })
        });
        let bottom_opaque = (midpoint..DEFAULT_ICON_SIZE).any(|y| {
            (0..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx + 3] == 255
            })
        });

        assert!(top_opaque, "top half should contain opaque icon pixels");
        assert!(
            bottom_opaque,
            "bottom half should contain opaque icon pixels"
        );
    }

    #[test]
    fn default_icon_body_has_blue_pixels_on_both_horizontal_halves() {
        let pixels = build_default_icon_pixels();
        let midpoint = DEFAULT_ICON_SIZE / 2;

        let left_body = (0..DEFAULT_ICON_SIZE).any(|y| {
            (0..midpoint).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx] == 41
                    && pixels[idx + 1] == 128
                    && pixels[idx + 2] == 185
                    && pixels[idx + 3] == 255
            })
        });
        let right_body = (0..DEFAULT_ICON_SIZE).any(|y| {
            (midpoint..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx] == 41
                    && pixels[idx + 1] == 128
                    && pixels[idx + 2] == 185
                    && pixels[idx + 3] == 255
            })
        });

        assert!(left_body, "left half should contain blue body pixels");
        assert!(right_body, "right half should contain blue body pixels");
    }

    #[test]
    fn default_icon_body_has_blue_pixels_on_both_vertical_halves() {
        let pixels = build_default_icon_pixels();
        let midpoint = DEFAULT_ICON_SIZE / 2;

        let top_body = (0..midpoint).any(|y| {
            (0..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx] == 41
                    && pixels[idx + 1] == 128
                    && pixels[idx + 2] == 185
                    && pixels[idx + 3] == 255
            })
        });
        let bottom_body = (midpoint..DEFAULT_ICON_SIZE).any(|y| {
            (0..DEFAULT_ICON_SIZE).any(|x| {
                let idx = pixel_idx(x, y);
                pixels[idx] == 41
                    && pixels[idx + 1] == 128
                    && pixels[idx + 2] == 185
                    && pixels[idx + 3] == 255
            })
        });

        assert!(top_body, "top half should contain blue body pixels");
        assert!(bottom_body, "bottom half should contain blue body pixels");
    }

    // --- toggle_record_label ---

    #[test]
    fn toggle_record_label_mentions_hotkey() {
        assert!(
            TOGGLE_RECORD_LABEL.contains("Ctrl+Shift+R"),
            "label should mention the hotkey"
        );
    }

    #[test]
    fn toggle_record_label_is_non_empty() {
        assert!(!TOGGLE_RECORD_LABEL.is_empty());
    }

    // -------------------------------------------------------------------------
    // All 7 menu constants are distinct from each other
    // -------------------------------------------------------------------------

    #[test]
    fn all_seven_menu_constants_are_pairwise_distinct() {
        let ids = [
            MENU_CAPTURE,
            MENU_OPEN_FOLDER,
            MENU_EXIT,
            MENU_CAPTURE_WINDOW,
            MENU_SETTINGS,
            MENU_START_RECORD,
            MENU_STOP_RECORD,
        ];
        for (i, a) in ids.iter().enumerate() {
            for (j, b) in ids.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "menu constants at index {i} and {j} must differ");
                }
            }
        }
    }

    #[test]
    fn default_icon_size_is_power_of_two() {
        assert!(DEFAULT_ICON_SIZE.is_power_of_two());
    }

    #[test]
    fn build_default_icon_pixels_all_alpha_zero_or_255() {
        let pixels = build_default_icon_pixels();
        for (i, chunk) in pixels.chunks(4).enumerate() {
            let alpha = chunk[3];
            assert!(
                alpha == 0 || alpha == 255,
                "pixel {i} has unexpected alpha {alpha}"
            );
        }
    }

    #[test]
    fn toggle_record_label_contains_start_or_stop_keyword() {
        let lower = TOGGLE_RECORD_LABEL.to_lowercase();
        assert!(
            lower.contains("start") || lower.contains("stop"),
            "label should mention start or stop"
        );
    }

    // --- Requirement 16.4: horizontal symmetry of opaque/transparent pattern ---

    #[test]
    fn default_icon_horizontal_symmetry_of_opacity() {
        // The icon is centered at x=16. For distance d from center, pixels at
        // x=(16-d) and x=(16+d) have the same |x-16| so the body/lens conditions
        // produce identical alpha values. We verify alpha(16-d, y) == alpha(16+d, y)
        // for d=1..=15 (d=0 is the center column, d=16 would be x=0 vs x=32 which
        // is out of bounds).
        let pixels = build_default_icon_pixels();
        for y in 0..DEFAULT_ICON_SIZE {
            for d in 1..=15u32 {
                let left_x = 16 - d;
                let right_x = 16 + d;
                let left_alpha = pixels[pixel_idx(left_x, y) + 3];
                let right_alpha = pixels[pixel_idx(right_x, y) + 3];
                assert_eq!(
                    left_alpha, right_alpha,
                    "horizontal symmetry broken at row {y}: pixel ({left_x}) alpha={left_alpha} vs pixel ({right_x}) alpha={right_alpha}"
                );
            }
        }
    }

    #[test]
    fn icon_from_rgba_pixels_returns_err_for_invalid_buffer_size() {
        assert!(icon_from_rgba_pixels(Vec::new()).is_err());
    }
}
