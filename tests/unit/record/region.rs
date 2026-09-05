//! Tests for `validate_capture_region` (region-within-monitor bounds).

use crate::record::*;

// -------------------------------------------------------------------------
// Accepted regions
// -------------------------------------------------------------------------

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
fn validate_capture_region_accepts_offset_region_touching_frame_edge() {
    let region = Region {
        physical_x: 1820,
        physical_y: 980,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

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
fn validate_capture_region_1x1_at_last_pixel() {
    let region = Region {
        physical_x: 1919,
        physical_y: 1079,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(validate_capture_region(region, 1920, 1080).is_ok());
}

// -------------------------------------------------------------------------
// Negative origin
// -------------------------------------------------------------------------

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

// -------------------------------------------------------------------------
// Zero-size regions
// -------------------------------------------------------------------------

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

// -------------------------------------------------------------------------
// Regions extending past the frame
// -------------------------------------------------------------------------

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
