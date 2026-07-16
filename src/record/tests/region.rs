//! Tests for `validate_capture_region` (region-within-monitor bounds).

use crate::record::*;

#[test]
fn validate_capture_region_accepts_exact_screen_bounds() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_rejects_negative_origin() {
    let region = Region {
        physical_x: -1,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_rejects_region_outside_frame() {
    let region = Region {
        physical_x: 1800,
        physical_y: 900,
        physical_width: 200,
        physical_height: 200,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_rejects_zero_width() {
    let region = Region {
        physical_x: 10,
        physical_y: 10,
        physical_width: 0,
        physical_height: 100,
    };
    assert_eq!(
        validate_capture_region(region, 1920, 1080),
        Err("Recording region must have non-zero size".to_string())
    );
}

#[test]
fn validate_capture_region_rejects_zero_height() {
    let region = Region {
        physical_x: 10,
        physical_y: 10,
        physical_width: 100,
        physical_height: 0,
    };
    assert_eq!(
        validate_capture_region(region, 1920, 1080),
        Err("Recording region must have non-zero size".to_string())
    );
}

#[test]
fn validate_capture_region_accepts_offset_region_touching_frame_edge() {
    let region = Region {
        physical_x: 1820,
        physical_y: 980,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

// =========================================================================
// validate_capture_region — additional coverage
// =========================================================================

#[test]
fn validate_capture_region_region_at_origin_full_frame() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_one_pixel_past_right() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1921,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_one_pixel_past_bottom() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1081,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_offset_exactly_touching_edge() {
    let region = Region {
        physical_x: 100,
        physical_y: 200,
        physical_width: 1820,
        physical_height: 880,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_offset_one_past_edge() {
    let region = Region {
        physical_x: 100,
        physical_y: 200,
        physical_width: 1821,
        physical_height: 880,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_1x1_at_last_pixel() {
    let region = Region {
        physical_x: 1919,
        physical_y: 1079,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_negative_x_rejected() {
    let region = Region {
        physical_x: -1,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_negative_y_rejected() {
    let region = Region {
        physical_x: 0,
        physical_y: -1,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_zero_width_rejected() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_zero_height_rejected() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 0,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

// --- validate_capture_region additional edge cases ---

#[test]
fn validate_capture_region_1x1_at_origin() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_both_zero_dimensions() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 0,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_negative_x_and_y() {
    let region = Region {
        physical_x: -10,
        physical_y: -20,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_exact_frame_size() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_one_pixel_wider_than_frame() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1921,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_one_pixel_taller_than_frame() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1081,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_error_message_for_negative_coords() {
    let region = Region {
        physical_x: -1,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    let err = validate_capture_region(region, 1920, 1080).unwrap_err();
    assert!(err.contains("must stay within"), "error: {err}");
}

#[test]
fn validate_capture_region_error_message_for_zero_size() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 100,
    };
    let err = validate_capture_region(region, 1920, 1080).unwrap_err();
    assert!(err.contains("non-zero"), "error: {err}");
}

// -------------------------------------------------------------------------
// validate_capture_region additional tests
// -------------------------------------------------------------------------

#[test]
fn validate_capture_region_valid_full_frame() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_valid_small_region() {
    let region = Region {
        physical_x: 100,
        physical_y: 100,
        physical_width: 200,
        physical_height: 200,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_region_exceeds_width() {
    let region = Region {
        physical_x: 1800,
        physical_y: 0,
        physical_width: 200,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_region_exceeds_height() {
    let region = Region {
        physical_x: 0,
        physical_y: 1000,
        physical_width: 100,
        physical_height: 200,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_zero_dimensions_rejected() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());

    let region2 = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 0,
    };
    assert!(validate_capture_region(region2, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_negative_origin_rejected() {
    let region = Region {
        physical_x: -10,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

// -------------------------------------------------------------------------
// validate_capture_region more edge cases
// -------------------------------------------------------------------------

#[test]
fn validate_capture_region_negative_y_only_is_rejected() {
    let region = Region {
        physical_x: 0,
        physical_y: -1,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_1x1_at_origin_ok() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_right_edge_overflow() {
    let region = Region {
        physical_x: 1920,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_bottom_edge_overflow() {
    let region = Region {
        physical_x: 0,
        physical_y: 1080,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

// -------------------------------------------------------------------------
// validate_capture_region: both negative coordinates
// -------------------------------------------------------------------------

#[test]
fn validate_capture_region_both_coords_negative() {
    let region = Region {
        physical_x: -10,
        physical_y: -20,
        physical_width: 100,
        physical_height: 100,
    };
    let err = validate_capture_region(region, 1920, 1080).unwrap_err();
    assert!(err.contains("within the selected monitor"));
}

// -------------------------------------------------------------------------
// validate_capture_region — boundary touching
// -------------------------------------------------------------------------

#[test]
fn validate_capture_region_full_frame_exact_match() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_1x1_at_last_valid_position() {
    let region = Region {
        physical_x: 1919,
        physical_y: 1079,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

#[test]
fn validate_capture_region_1x1_past_right_edge() {
    let region = Region {
        physical_x: 1920,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_err());
}

#[test]
fn validate_capture_region_error_message_for_negative_origin() {
    let region = Region {
        physical_x: -1,
        physical_y: -1,
        physical_width: 10,
        physical_height: 10,
    };
    let err = validate_capture_region(region, 1920, 1080).unwrap_err();
    assert!(err.contains("within the selected monitor"));
}

#[test]
fn validate_capture_region_error_message_for_zero_dimensions() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 0,
    };
    let err = validate_capture_region(region, 1920, 1080).unwrap_err();
    assert!(err.contains("non-zero size"));
}
