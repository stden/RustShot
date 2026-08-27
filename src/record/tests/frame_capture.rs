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
    let mut output = vec![0xcc; 2 * 2 * 4];

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
fn padded_copies_into_a_reused_buffer_leave_no_stale_pixels() {
    // The encoder allocates one frame buffer and reuses it for every sample, so
    // each copy has to define every byte of the padded frame on its own — the
    // pad of a narrow region covers pixels an earlier, wider region wrote.
    let mut output = vec![0u8; 4 * 2 * 4];
    let wide = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 3,
        physical_height: 1,
    };
    copy_bgra_region_bottom_up(&[9; 12], 12, 3, 1, wide, 4, 2, &mut output).unwrap();
    assert_eq!(&output[16..28], &[9; 12], "the wide region is copied");

    let narrow = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 2,
        physical_height: 1,
    };
    copy_bgra_region_bottom_up(&[1; 8], 8, 2, 1, narrow, 4, 2, &mut output).unwrap();

    assert_eq!(&output[..16], &[0; 16], "the padding row stays cleared");
    assert_eq!(&output[16..24], &[1; 8], "the narrow region is copied");
    assert_eq!(
        &output[24..],
        &[0; 8],
        "the wider previous frame's pixels must not survive in the padding"
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
    let mut output = vec![0xcc; 2 * 2 * 4];

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

/// The sRGB table is indexed by `value * 4096.0 + 0.5` truncated rather than by
/// `round(value * 4096.0)`.
///
/// `f32::round` is round-half-away-from-zero, which baseline x86-64 has no
/// instruction for, so LLVM lowers it to a call into the CRT's `roundf` — three
/// calls per pixel, 25 million for a single 4K frame, and measurably the largest
/// cost in this loop. Adding the bias and truncating is the same operation for
/// the non-negative clamped values this receives, except within half an ULP of a
/// half-step, where the addition itself rounds and can select the next table
/// entry up. (Concretely: the largest `f32` below 0.5, scaled by the exact
/// power-of-two 4096, gives a sum that ties between `1 - 2^-24` and `1.0`, and
/// round-to-even picks `1.0`.)
///
/// So the bound is asserted against the exact sRGB transfer function rather than
/// against the previous implementation: two code values out of 255, which is
/// where a one-step index error (~0.8 code values across the toe, where the
/// curve is steepest) plus the table's own rounding (0.5) can land.
#[test]
fn scrgb_tone_mapping_stays_within_two_code_values_of_exact_srgb() {
    fn exact_srgb8(linear: f32) -> f32 {
        let srgb = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        srgb * 255.0
    }

    // Five samples per table step, so every step boundary is crossed.
    for step in 0..=20_000u32 {
        let level = step as f32 / 20_000.0;
        let mapped = tone_map_scrgb_pixel(level, level, level, 1_000.0);
        // Below diffuse white the curve is a flat 0.80 scale, so the linear
        // value reaching the table is known in closed form.
        let expected = exact_srgb8(level * 0.80);
        let actual = f32::from(mapped[0]);
        assert!(
            (actual - expected).abs() <= 2.0,
            "level {level}: table gave {actual}, exact sRGB is {expected}"
        );
        assert_eq!([mapped[0], mapped[0]], [mapped[1], mapped[2]]);
    }
}

/// Brightening the input must never darken the output.
///
/// Truncating a biased value is a floor of a monotonic function, so it preserves
/// the ordering that a rounding index gave. This sweeps from black up across
/// diffuse white and onto the highlight shoulder, which is where the curve
/// changes form and an off-by-one index would be most visible as a reversal.
#[test]
fn scrgb_tone_mapping_is_monotonic_across_the_highlight_shoulder() {
    let mut previous = 0u8;
    for step in 0..=30_000u32 {
        let level = step as f32 / 2_000.0;
        let mapped = tone_map_scrgb_pixel(level, level, level, 1_000.0);
        assert!(
            mapped[0] >= previous,
            "level {level} produced {} after {previous}",
            mapped[0]
        );
        previous = mapped[0];
    }
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

/// Exhaustive parity between the hardware and portable FP16 decoders.
///
/// The F16C path is only sound as an optimisation if it cannot move the tone
/// curve, and the claim in `HalfDecode::decode` is that the two agree bit-for-bit
/// on every finite value and both infinities. There are only 65536 half bit
/// patterns, so that claim is checkable outright rather than sampled.
///
/// NaN is compared as "both are NaN": `vcvtph2ps` quiets a signalling NaN, so the
/// payload bits legitimately differ. That is unobservable downstream because
/// `ToneCurve::map` sends every non-finite channel to zero.
#[test]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn hardware_and_portable_half_decode_agree_on_every_bit_pattern() {
    if HalfDecode::detect() != HalfDecode::F16c {
        // Not a silent skip: state it, so a green run on a machine without F16C
        // cannot be mistaken for the comparison having happened.
        eprintln!("SKIPPED: this CPU reports no F16C support, so there is nothing to compare");
        return;
    }

    // One batch's worth at a time, covering all 65536 patterns.
    let mut source = Vec::with_capacity(65536 * 2);
    for bits in 0..=u16::MAX {
        source.extend_from_slice(&bits.to_le_bytes());
    }
    let mut hardware = vec![0f32; 65536];
    let mut portable = vec![0f32; 65536];

    // SAFETY: `halves_to_f32_f16c` is `#[target_feature(enable = "avx,f16c")]`, so
    // it may only be called when the CPU has both. The early return above left
    // only the case where `HalfDecode::detect()` answered `F16c`, and it answers
    // that solely when `is_x86_feature_detected!` reports f16c *and* avx.
    unsafe { halves_to_f32_f16c(&source, &mut hardware) };
    halves_to_f32_portable(&source, &mut portable);

    for bits in 0..=u16::MAX {
        let index = bits as usize;
        let (h, p) = (hardware[index], portable[index]);
        if p.is_nan() {
            assert!(
                h.is_nan(),
                "half {bits:#06x}: portable produced NaN but hardware produced {h}"
            );
            continue;
        }
        assert_eq!(
            h.to_bits(),
            p.to_bits(),
            "half {bits:#06x}: hardware {h} ({:#010x}) != portable {p} ({:#010x})",
            h.to_bits(),
            p.to_bits()
        );
    }
}

/// The vectorised decode handles 8 halves per instruction and falls back to the
/// scalar loop for the remainder. Lengths that are not a multiple of 8 are the only
/// way to reach that tail, and a 4K row is a multiple of 8, so production traffic
/// alone would leave it unexercised.
#[test]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn hardware_half_decode_tail_matches_the_scalar_loop() {
    if HalfDecode::detect() != HalfDecode::F16c {
        eprintln!("SKIPPED: this CPU reports no F16C support");
        return;
    }

    // Values chosen to span normal, subnormal, zero, one and infinity.
    let pattern: [u16; 6] = [0x0000, 0x0001, 0x3c00, 0xc000, 0x7c00, 0x4900];
    for count in 0..=17usize {
        let mut source = Vec::with_capacity(count * 2);
        for index in 0..count {
            source.extend_from_slice(&pattern[index % pattern.len()].to_le_bytes());
        }
        let mut hardware = vec![0f32; count];
        let mut portable = vec![0f32; count];

        // SAFETY: same feature guard as above - the `detect() != F16c` early
        // return means avx and f16c are both present on this CPU. The short
        // `count`s deliberately exercised here are safe by the helper's own
        // contract: it converts `min(source.len() / 2, destination.len())`
        // values, and both buffers are sized from `count`.
        unsafe { halves_to_f32_f16c(&source, &mut hardware) };
        halves_to_f32_portable(&source, &mut portable);

        for index in 0..count {
            assert_eq!(
                hardware[index].to_bits(),
                portable[index].to_bits(),
                "count {count}, index {index}: tail decode diverged"
            );
        }
    }
}

/// Every decoder the running target can produce.
///
/// Mirrors `all_swizzles` in `capture::tests::platform_helpers` for the same
/// reason it exists there: naming the implementations in an enum is what keeps the
/// portable arm testable on a machine that has F16C. Without this, `detect()`
/// returns `F16c` on every x86 CI runner and the `Self::Portable` arm of
/// `HalfDecode::decode` is covered only by the arm64 macOS job -- so a broken
/// fallback would ship green on Windows and Linux.
fn all_half_decoders() -> Vec<HalfDecode> {
    let mut variants = vec![HalfDecode::Portable];
    let detected = HalfDecode::detect();
    // Not `cfg`-gated: off x86 `detect` is `Portable` and this pushes nothing.
    if detected != HalfDecode::Portable {
        variants.push(detected);
    }
    variants
}

/// Decoding must convert `min(source halves, destination len)` and touch nothing
/// past it, so a short buffer on either side cannot write out of bounds.
#[test]
fn half_decode_stops_at_the_shorter_of_the_two_buffers() {
    let source = [0x00, 0x3c, 0x00, 0x40, 0x00, 0x42]; // 1.0, 2.0, 3.0
    for decoder in all_half_decoders() {
        // Destination shorter than the source.
        let mut short = [-1f32; 2];
        decoder.decode(&source, &mut short);
        assert_eq!(short, [1.0, 2.0], "{decoder:?}");

        // Source shorter than the destination: the tail stays untouched.
        let mut long = [-1f32; 5];
        decoder.decode(&source[..4], &mut long);
        assert_eq!(long, [1.0, 2.0, -1.0, -1.0, -1.0], "{decoder:?}");

        // A trailing odd byte is not half a value.
        let mut odd = [-1f32; 4];
        decoder.decode(&source[..5], &mut odd);
        assert_eq!(odd, [1.0, 2.0, -1.0, -1.0], "{decoder:?}");
    }
}

/// The batch decode must not change what the frame conversion produces. A width
/// that straddles the batch size is where a batching bug would show up: the last
/// batch is partial, and the scratch array still holds the previous batch's values
/// past that point.
#[test]
fn batched_scrgb_conversion_matches_a_pixel_at_a_time_reference() {
    // Deliberately not a multiple of DECODE_BATCH_PIXELS.
    let width = DECODE_BATCH_PIXELS as u32 * 2 + 3;
    let height = 3u32;
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    };
    let pitch = width as usize * 8;

    // A gradient that reaches above 1.0 so the highlight shoulder is exercised.
    let mut source = vec![0u8; pitch * height as usize];
    for (index, pixel) in source.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let scale = index as u16;
        pixel[0..2].copy_from_slice(&(0x3000 + scale).to_le_bytes());
        pixel[2..4].copy_from_slice(&(0x3400 + scale).to_le_bytes());
        pixel[4..6].copy_from_slice(&(0x4200 + scale).to_le_bytes());
        pixel[6..8].copy_from_slice(&0x3c00u16.to_le_bytes());
    } // A gradient that reaches above 1.0 so the highlight shoulder is exercised.
    let mut source = vec![0u8; pitch * height as usize];
    for (index, pixel) in source.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let scale = index as u16;
        pixel[0..2].copy_from_slice(&(0x3000 + scale).to_le_bytes());
        pixel[2..4].copy_from_slice(&(0x3400 + scale).to_le_bytes());
        pixel[4..6].copy_from_slice(&(0x4200 + scale).to_le_bytes());
        pixel[6..8].copy_from_slice(&0x3c00u16.to_le_bytes());
    }

    // Non-finite and negative reds, placed both inside a full batch (5, 9) and
    // inside the final partial one (16, 17, 18). This is the only place
    // `HalfDecode::decode`'s NaN argument becomes checkable: the reference decodes
    // the signalling NaN 0x7c01 with `half_to_f32`, which keeps its payload, while
    // `vcvtph2ps` quiets it -- and the two must still produce the same byte,
    // because `ToneCurve::map` sends every non-finite channel to zero regardless of
    // payload. Every other test of that rule calls `tone_map_scrgb_pixel` directly
    // and so never reaches the decoder at all.
    for (pixel_index, bits) in [
        (5usize, 0x7c00u16), // +inf
        (9, 0xfc00),         // -inf
        (16, 0x7e00),        // quiet NaN
        (17, 0x7c01),        // signalling NaN
        (18, 0xbc00),        // -1.0, which clamps rather than wrapping
    ] {
        let pixel = &mut source.as_chunks_mut::<8>().0[pixel_index];
        pixel[0..2].copy_from_slice(&bits.to_le_bytes());
    }

    let mut actual = vec![0u8; width as usize * 4 * height as usize];
    copy_scrgb_region_bottom_up(
        &source,
        pitch,
        width,
        height,
        region,
        width,
        height,
        1_000.0,
        &mut actual,
    )
    .unwrap();

    // Reference: decode and map one pixel at a time, exactly as the loop did
    // before batching, including the bottom-up row flip.
    let curve_reference = |bytes: &[u8]| -> [u8; 3] {
        let red = half_to_f32(u16::from_le_bytes([bytes[0], bytes[1]]));
        let green = half_to_f32(u16::from_le_bytes([bytes[2], bytes[3]]));
        let blue = half_to_f32(u16::from_le_bytes([bytes[4], bytes[5]]));
        tone_map_scrgb_pixel(red, green, blue, 1_000.0)
    };
    let mut expected = vec![0u8; actual.len()];
    for row in 0..height as usize {
        for column in 0..width as usize {
            let [red, green, blue] = curve_reference(&source[row * pitch + column * 8..]);
            let destination = (height as usize - 1 - row) * width as usize * 4 + column * 4;
            expected[destination..destination + 4].copy_from_slice(&[blue, green, red, u8::MAX]);
        }
    }
    assert_eq!(
        actual, expected,
        "batched conversion diverged from reference"
    );
}
