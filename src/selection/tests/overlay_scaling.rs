//! Coordinate-mapping tests for DPI-scaled selection overlays.

use super::*;

#[test]
fn overlay_pixel_mapping_scales_into_the_captured_image() {
    assert_eq!(map_overlay_pixel_to_image(0, 1280, 1920), Some(0));
    assert_eq!(map_overlay_pixel_to_image(640, 1280, 1920), Some(960));
    assert_eq!(map_overlay_pixel_to_image(1279, 1280, 1920), Some(1919));
}

#[test]
fn overlay_pixel_mapping_samples_pixel_centres() {
    assert_eq!(map_overlay_pixel_to_image(0, 3, 5), Some(0));
    assert_eq!(map_overlay_pixel_to_image(1, 3, 5), Some(2));
    assert_eq!(map_overlay_pixel_to_image(2, 3, 5), Some(4));
}

#[test]
fn overlay_pixel_mapping_rejects_empty_dimensions() {
    assert_eq!(map_overlay_pixel_to_image(0, 0, 1920), None);
    assert_eq!(map_overlay_pixel_to_image(0, 1280, 0), None);
}

#[test]
fn overlay_pixel_map_cache_reuses_mappings_until_dimensions_change() {
    let mut cache = OverlayPixelMapCache::default();

    assert!(cache.update((3, 2), (5, 4)));
    assert_eq!(cache.x, [0, 2, 4]);
    assert_eq!(cache.y, [1, 3]);

    assert!(!cache.update((3, 2), (5, 4)));
    assert!(cache.update((2, 3), (5, 7)));
    assert_eq!(cache.x, [1, 3]);
    assert_eq!(cache.y, [1, 3, 5]);
}

#[test]
fn overlay_decoration_computes_border_and_crosshair_once() {
    let border = OverlayDecoration::new(Some((10, 20)), Some((30, 40)), 100, 100);
    assert_eq!(border.color_at(10, 20), Some((0, 200, 255)));
    assert_eq!(border.color_at(20, 30), None);

    let crosshair = OverlayDecoration::new(None, Some((5, 6)), 100, 100);
    assert_eq!(crosshair.color_at(5, 20), Some((255, 0, 0)));
    assert_eq!(crosshair.color_at(20, 6), Some((255, 0, 0)));
    assert_eq!(crosshair.color_at(20, 20), None);
}

#[test]
fn overlay_decoration_new_clamps_negative_origin_to_zero() {
    // Border: a drag that starts off-screen (negative coords) clamps to (0, 0)
    let border = OverlayDecoration::new(Some((-10, -20)), Some((30, 40)), 100, 100);
    assert_eq!(
        border,
        OverlayDecoration::Border {
            x1: 0,
            y1: 0,
            x2: 30,
            y2: 40
        }
    );

    // Crosshair: a negative cursor position clamps to (0, 0) as well
    let crosshair = OverlayDecoration::new(None, Some((-5, -7)), 100, 100);
    assert_eq!(crosshair, OverlayDecoration::Crosshair { x: 0, y: 0 });
}

#[test]
fn overlay_decoration_new_zero_dimensions_returns_none_variant() {
    assert_eq!(
        OverlayDecoration::new(Some((10, 10)), Some((20, 20)), 0, 100),
        OverlayDecoration::None
    );
    assert_eq!(
        OverlayDecoration::new(Some((10, 10)), Some((20, 20)), 100, 0),
        OverlayDecoration::None
    );
    // The None variant never paints anything
    assert_eq!(OverlayDecoration::None.color_at(0, 0), None);
}

#[test]
fn overlay_decoration_new_missing_endpoint_returns_none_variant() {
    // A drag start without a current position (cursor left the window)
    // draws nothing, as does the complete absence of positions.
    assert_eq!(
        OverlayDecoration::new(Some((10, 10)), None, 100, 100),
        OverlayDecoration::None
    );
    assert_eq!(
        OverlayDecoration::new(None, None, 100, 100),
        OverlayDecoration::None
    );
}

#[test]
fn overlay_decoration_border_halo_is_one_pixel_thick() {
    let border = OverlayDecoration::new(Some((10, 20)), Some((30, 40)), 100, 100);

    // One pixel outside each border edge is still painted (the halo)…
    assert_eq!(border.color_at(9, 30), Some((0, 200, 255))); // left
    assert_eq!(border.color_at(31, 30), Some((0, 200, 255))); // right
    assert_eq!(border.color_at(20, 19), Some((0, 200, 255))); // top
    assert_eq!(border.color_at(20, 41), Some((0, 200, 255))); // bottom

    // …one pixel further out is not
    assert_eq!(border.color_at(8, 30), None);
    assert_eq!(border.color_at(32, 30), None);
    assert_eq!(border.color_at(20, 18), None);
    assert_eq!(border.color_at(20, 42), None);

    // and the interior of the rectangle stays unpainted
    assert_eq!(border.color_at(20, 30), None);
}

#[test]
fn overlay_drag_mapping_scales_both_axes() {
    let region = map_overlay_drag_to_image((100, 50), (1100, 650), (1200, 800), (2400, 1600))
        .expect("scaled drag should produce a region");

    assert_eq!(
        region,
        Region {
            physical_x: 200,
            physical_y: 100,
            physical_width: 2000,
            physical_height: 1200,
        }
    );
}

#[test]
fn overlay_drag_mapping_clamps_to_image_bounds() {
    let region = map_overlay_drag_to_image((-100, -50), (1400, 900), (1200, 800), (2400, 1600))
        .expect("clamped drag should produce a region");

    assert_eq!(
        region,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2400,
            physical_height: 1600,
        }
    );
}

#[test]
fn overlay_drag_mapping_rejects_empty_overlay() {
    assert!(map_overlay_drag_to_image((0, 0), (10, 10), (0, 100), (200, 200)).is_none());
}

#[test]
fn overlay_drag_mapping_normalizes_reverse_drags() {
    let forward = map_overlay_drag_to_image((100, 50), (1100, 650), (1200, 800), (2400, 1600));
    let reverse = map_overlay_drag_to_image((1100, 650), (100, 50), (1200, 800), (2400, 1600));

    assert_eq!(forward, reverse);
}

#[test]
fn target_overlay_monitor_matches_physical_origin_and_capture_size() {
    let monitors = [((0, 0), (1920, 1080)), ((1920, 0), (2560, 1440))];

    assert_eq!(
        target_overlay_monitor_index(monitors, (1920, 0), (2560, 1440)),
        Some(1)
    );
}
