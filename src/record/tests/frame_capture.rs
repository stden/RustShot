//! Regression tests for persistent native recording-frame conversion.

use crate::record::*;

#[test]
fn bgra_region_copy_respects_source_pitch_and_flips_rows() {
    let source = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 99, 99, 99, 99, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 88, 88, 88, 88,
    ];
    let region = Region {
        physical_x: 1,
        physical_y: 0,
        physical_width: 2,
        physical_height: 2,
    };
    let mut output = vec![0; 2 * 2 * 4];

    copy_bgra_region_bottom_up(&source, 16, 3, 2, region, 2, 2, &mut output).unwrap();

    assert_eq!(
        output,
        vec![25, 26, 27, 28, 29, 30, 31, 32, 5, 6, 7, 8, 9, 10, 11, 12,]
    );
}

#[test]
fn bgra_region_copy_zero_pads_even_output_dimensions() {
    let source = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 3,
        physical_height: 1,
    };
    let mut output = vec![255; 4 * 2 * 4];

    copy_bgra_region_bottom_up(&source, 12, 3, 1, region, 4, 2, &mut output).unwrap();

    assert_eq!(&output[..16], &[0; 16]);
    assert_eq!(
        &output[16..],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0]
    );
}

#[test]
fn rgba_fallback_copy_swaps_channels_and_zero_pads() {
    let source = vec![10, 20, 30, 40, 50, 60, 70, 80];
    let mut output = vec![255; 2 * 2 * 4];

    copy_rgba_frame_bottom_up(&source, 2, 1, 2, 2, &mut output).unwrap();

    assert_eq!(&output[..8], &[0; 8]);
    assert_eq!(&output[8..], &[30, 20, 10, 40, 70, 60, 50, 80]);
}

#[test]
fn native_frame_copy_rejects_invalid_geometry_and_short_buffers() {
    let region = Region {
        physical_x: 1,
        physical_y: 0,
        physical_width: 2,
        physical_height: 1,
    };
    let mut output = vec![0; 8];

    assert!(copy_bgra_region_bottom_up(&[0; 12], 12, 2, 1, region, 2, 1, &mut output).is_err());
    assert!(copy_bgra_region_bottom_up(
        &[0; 8],
        4,
        2,
        1,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2,
            physical_height: 1,
        },
        2,
        1,
        &mut output
    )
    .is_err());
    assert!(copy_rgba_frame_bottom_up(&[0; 8], 2, 1, 2, 2, &mut [0; 8]).is_err());
}

#[test]
fn frame_pool_resize_is_recreated_only_when_the_region_still_fits() {
    let region = Region {
        physical_x: 10,
        physical_y: 20,
        physical_width: 100,
        physical_height: 80,
    };

    assert_eq!(
        frame_pool_update((1920, 1080), (1920, 1080), region).unwrap(),
        FramePoolUpdate::Keep
    );
    assert_eq!(
        frame_pool_update((1920, 1080), (2560, 1440), region).unwrap(),
        FramePoolUpdate::Recreate
    );
    assert!(frame_pool_update((1920, 1080), (80, 60), region).is_err());
}

#[test]
fn half_float_conversion_handles_normal_subnormal_and_special_values() {
    assert_eq!(half_to_f32(0x0000), 0.0);
    assert_eq!(half_to_f32(0x3c00), 1.0);
    assert_eq!(half_to_f32(0xc000), -2.0);
    assert!(half_to_f32(0x0001) > 0.0);
    assert!(half_to_f32(0x7c00).is_infinite());
    assert!(half_to_f32(0x7e00).is_nan());
}

#[test]
fn scrgb_copy_tone_maps_hdr_preserves_hue_and_flips_rows() {
    fn pixel(r: u16, g: u16, b: u16) -> Vec<u8> {
        [r, g, b, 0x3c00]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    let mut source = Vec::new();
    source.extend(pixel(0x3c00, 0x0000, 0x0000));
    source.extend(pixel(0x0000, 0x3c00, 0x0000));
    source.extend([0xaa; 8]);
    source.extend(pixel(0x0000, 0x0000, 0x3c00));
    source.extend(pixel(0x4400, 0x4400, 0x4400));
    source.extend([0xbb; 8]);
    let mut output = vec![0; 2 * 2 * 4];

    copy_scrgb_region_bottom_up(
        &source,
        24,
        2,
        2,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2,
            physical_height: 2,
        },
        2,
        2,
        1_000.0,
        &mut output,
    )
    .unwrap();

    assert!(output[0] > output[1] && output[0] > output[2]);
    assert_eq!(output[3], u8::MAX);
    assert!(output[4] >= output[0]);
    assert_eq!(output[7], u8::MAX);
    assert!(output[10] > output[8] && output[10] > output[9]);
    assert!(output[13] > output[12] && output[13] > output[14]);
}

#[test]
fn scrgb_tone_mapping_keeps_highlights_brighter_without_clipping_to_white() {
    let diffuse_white = tone_map_scrgb_pixel(1.0, 1.0, 1.0, 1_000.0);
    let highlight = tone_map_scrgb_pixel(4.0, 4.0, 4.0, 1_000.0);
    let saturated_red = tone_map_scrgb_pixel(4.0, 0.0, 0.0, 1_000.0);

    assert!(highlight[0] > diffuse_white[0]);
    assert!(highlight[0] < u8::MAX);
    assert!(saturated_red[0] > 0);
    assert_eq!(saturated_red[1], 0);
    assert_eq!(saturated_red[2], 0);
}
