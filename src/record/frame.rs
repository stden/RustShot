//! Pure frame validation, cropping, padding, and pixel-order conversion.

use super::{validate_capture_region, Region};
use std::sync::OnceLock;

const SCRGB_REFERENCE_WHITE_NITS: f32 = 80.0;
const SDR_DIFFUSE_WHITE: f32 = 0.80;
const SDR_PEAK: f32 = 0.98;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FramePoolUpdate {
    Keep,
    Recreate,
}

pub(super) fn frame_pool_update(
    current_size: (u32, u32),
    content_size: (u32, u32),
    region: Region,
) -> Result<FramePoolUpdate, String> {
    validate_capture_region(region, content_size.0, content_size.1)?;
    if current_size == content_size {
        Ok(FramePoolUpdate::Keep)
    } else {
        Ok(FramePoolUpdate::Recreate)
    }
}

// Consumed by the cfg(windows) recording loop; compiled everywhere so the
// pure logic stays unit-tested on every platform.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy)]
enum SourcePixelOrder {
    Bgra,
    Rgba,
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(windows), allow(dead_code))]
fn copy_region_bottom_up(
    source: &[u8],
    source_row_pitch: usize,
    source_width: u32,
    source_height: u32,
    region: Region,
    output_width: u32,
    output_height: u32,
    output: &mut [u8],
    source_order: SourcePixelOrder,
) -> Result<(), String> {
    validate_capture_region(region, source_width, source_height)?;
    if output_width < region.physical_width || output_height < region.physical_height {
        return Err("Encoded frame is smaller than the recording region".to_string());
    }

    let source_row_bytes = (source_width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Captured frame row is too large".to_string())?;
    if source_row_pitch < source_row_bytes {
        return Err(format!(
            "Captured row pitch {source_row_pitch} is smaller than the {source_row_bytes}-byte row"
        ));
    }
    let source_required = source_row_pitch
        .checked_mul(source_height.saturating_sub(1) as usize)
        .and_then(|offset| offset.checked_add(source_row_bytes))
        .ok_or_else(|| "Captured frame buffer is too large".to_string())?;
    if source.len() < source_required {
        return Err(format!(
            "Captured frame has {} bytes but {source_required} are required",
            source.len()
        ));
    }

    let output_row_bytes = (output_width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Encoded frame row is too large".to_string())?;
    let output_required = output_row_bytes
        .checked_mul(output_height as usize)
        .ok_or_else(|| "Encoded frame buffer is too large".to_string())?;
    if output.len() < output_required {
        return Err(format!(
            "Encoded frame has {} bytes but {output_required} are required",
            output.len()
        ));
    }
    output[..output_required].fill(0);

    let source_x = region.physical_x as usize;
    let source_y = region.physical_y as usize;
    let copy_row_bytes = (region.physical_width as usize) * 4;
    for row in 0..region.physical_height as usize {
        let source_start = (source_y + row) * source_row_pitch + source_x * 4;
        let source_row = &source[source_start..source_start + copy_row_bytes];
        let destination_row = output_height as usize - 1 - row;
        let destination_start = destination_row * output_row_bytes;
        let destination = &mut output[destination_start..destination_start + copy_row_bytes];

        match source_order {
            SourcePixelOrder::Bgra => destination.copy_from_slice(source_row),
            SourcePixelOrder::Rgba => {
                for (rgba, bgra) in source_row
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(destination.as_chunks_mut::<4>().0.iter_mut())
                {
                    *bgra = [rgba[2], rgba[1], rgba[0], rgba[3]];
                }
            }
        }
    }

    Ok(())
}

/// Crop a pitched, top-down BGRA desktop frame into a bottom-up MF frame.
#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn copy_bgra_region_bottom_up(
    source: &[u8],
    source_row_pitch: usize,
    source_width: u32,
    source_height: u32,
    region: Region,
    output_width: u32,
    output_height: u32,
    output: &mut [u8],
) -> Result<(), String> {
    copy_region_bottom_up(
        source,
        source_row_pitch,
        source_width,
        source_height,
        region,
        output_width,
        output_height,
        output,
        SourcePixelOrder::Bgra,
    )
}

/// Convert a tightly packed top-down RGBA fallback frame into padded MF RGB32.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn copy_rgba_frame_bottom_up(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    output: &mut [u8],
) -> Result<(), String> {
    let source_row_pitch = (source_width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Captured frame row is too large".to_string())?;
    copy_region_bottom_up(
        source,
        source_row_pitch,
        source_width,
        source_height,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: source_width,
            physical_height: source_height,
        },
        output_width,
        output_height,
        output,
        SourcePixelOrder::Rgba,
    )
}

pub(super) fn half_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = (bits >> 10) & 0x1f;
    let fraction = u32::from(bits & 0x03ff);

    let float_bits = match exponent {
        0 if fraction == 0 => sign,
        0 => {
            let leading = fraction.leading_zeros() - 22;
            let normalized_fraction = (fraction << (leading + 1)) & 0x03ff;
            let float_exponent = 127 - 15 - leading;
            sign | (float_exponent << 23) | (normalized_fraction << 13)
        }
        0x1f => sign | 0x7f80_0000 | (fraction << 13),
        _ => sign | (u32::from(exponent) + 112) << 23 | (fraction << 13),
    };
    f32::from_bits(float_bits)
}

fn linear_to_srgb8(value: f32) -> u8 {
    static LUT: OnceLock<Box<[u8; 4097]>> = OnceLock::new();
    let lut = LUT.get_or_init(|| {
        let mut values = Box::new([0; 4097]);
        for (index, output) in values.iter_mut().enumerate() {
            let linear = index as f32 / 4096.0;
            let srgb = if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            };
            *output = (srgb * 255.0).round() as u8;
        }
        values
    });
    let index = (value.clamp(0.0, 1.0) * 4096.0).round() as usize;
    lut[index]
}

/// Convert one linear scRGB pixel to SDR sRGB without clipping HDR highlights.
///
/// scRGB uses 80 nits for diffuse white and permits values above 1.0. The
/// shoulder leaves headroom for those highlights and scales all channels by
/// one factor, preserving hue instead of clamping channels independently.
pub(super) fn tone_map_scrgb_pixel(
    red: f32,
    green: f32,
    blue: f32,
    peak_luminance_nits: f32,
) -> [u8; 3] {
    let red = if red.is_finite() { red.max(0.0) } else { 0.0 };
    let green = if green.is_finite() {
        green.max(0.0)
    } else {
        0.0
    };
    let blue = if blue.is_finite() { blue.max(0.0) } else { 0.0 };
    let level = red.max(green).max(blue);
    if level == 0.0 {
        return [0, 0, 0];
    }

    let source_peak = (peak_luminance_nits / SCRGB_REFERENCE_WHITE_NITS).max(1.01);
    let mapped_level = if level <= 1.0 {
        level * SDR_DIFFUSE_WHITE
    } else {
        let highlight = (level.ln() / source_peak.ln()).clamp(0.0, 1.0);
        SDR_DIFFUSE_WHITE + (SDR_PEAK - SDR_DIFFUSE_WHITE) * highlight
    };
    let scale = mapped_level / level;

    [
        linear_to_srgb8(red * scale),
        linear_to_srgb8(green * scale),
        linear_to_srgb8(blue * scale),
    ]
}

/// Crop pitched, top-down FP16 scRGB into tone-mapped bottom-up MF RGB32.
#[allow(clippy::too_many_arguments)]
pub(super) fn copy_scrgb_region_bottom_up(
    source: &[u8],
    source_row_pitch: usize,
    source_width: u32,
    source_height: u32,
    region: Region,
    output_width: u32,
    output_height: u32,
    peak_luminance_nits: f32,
    output: &mut [u8],
) -> Result<(), String> {
    validate_capture_region(region, source_width, source_height)?;
    if output_width < region.physical_width || output_height < region.physical_height {
        return Err("Encoded frame is smaller than the recording region".to_string());
    }

    let source_row_bytes = (source_width as usize)
        .checked_mul(8)
        .ok_or_else(|| "Captured FP16 frame row is too large".to_string())?;
    if source_row_pitch < source_row_bytes {
        return Err(format!(
            "Captured FP16 row pitch {source_row_pitch} is smaller than the {source_row_bytes}-byte row"
        ));
    }
    let source_required = source_row_pitch
        .checked_mul(source_height.saturating_sub(1) as usize)
        .and_then(|offset| offset.checked_add(source_row_bytes))
        .ok_or_else(|| "Captured FP16 frame buffer is too large".to_string())?;
    if source.len() < source_required {
        return Err(format!(
            "Captured FP16 frame has {} bytes but {source_required} are required",
            source.len()
        ));
    }

    let output_row_bytes = (output_width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Encoded frame row is too large".to_string())?;
    let output_required = output_row_bytes
        .checked_mul(output_height as usize)
        .ok_or_else(|| "Encoded frame buffer is too large".to_string())?;
    if output.len() < output_required {
        return Err(format!(
            "Encoded frame has {} bytes but {output_required} are required",
            output.len()
        ));
    }
    output[..output_required].fill(0);

    let source_x = region.physical_x as usize;
    let source_y = region.physical_y as usize;
    let copy_row_bytes = region.physical_width as usize * 8;
    for row in 0..region.physical_height as usize {
        let source_start = (source_y + row) * source_row_pitch + source_x * 8;
        let source_row = &source[source_start..source_start + copy_row_bytes];
        let destination_row = output_height as usize - 1 - row;
        let destination_start = destination_row * output_row_bytes;
        let destination =
            &mut output[destination_start..destination_start + region.physical_width as usize * 4];

        for (scrgb, bgra) in source_row
            .as_chunks::<8>()
            .0
            .iter()
            .zip(destination.as_chunks_mut::<4>().0.iter_mut())
        {
            let red = half_to_f32(u16::from_le_bytes([scrgb[0], scrgb[1]]));
            let green = half_to_f32(u16::from_le_bytes([scrgb[2], scrgb[3]]));
            let blue = half_to_f32(u16::from_le_bytes([scrgb[4], scrgb[5]]));
            let [red, green, blue] = tone_map_scrgb_pixel(red, green, blue, peak_luminance_nits);
            *bgra = [blue, green, red, u8::MAX];
        }
    }

    Ok(())
}
