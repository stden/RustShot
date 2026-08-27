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

/// Clear the pixels a bottom-up region copy will not write.
///
/// H.264 needs even dimensions, so a region with an odd width or height is
/// padded by a row and/or a column that the copy loop never touches. Only that
/// pad is cleared, not the whole frame: the encoder reuses one buffer for every
/// sample (see the `bgra_buffer` in [`super::encoder`]), and clearing all of it
/// per frame is an 8 MB memset at 1080p — 250 MB/s at 30 fps — to zero a few
/// kilobytes of padding whose contents nothing else writes.
///
/// `region_row_bytes` is how many bytes of each output row the copy fills, and
/// `region_height` how many rows it fills, counted from the bottom.
fn clear_frame_padding(
    output: &mut [u8],
    output_row_bytes: usize,
    output_height: usize,
    region_row_bytes: usize,
    region_height: usize,
) {
    let padding_rows = output_height.saturating_sub(region_height);
    output[..padding_rows * output_row_bytes].fill(0);

    if region_row_bytes >= output_row_bytes {
        return;
    }
    for row in padding_rows..output_height {
        let row_start = row * output_row_bytes;
        output[row_start + region_row_bytes..row_start + output_row_bytes].fill(0);
    }
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
    let source_x = region.physical_x as usize;
    let source_y = region.physical_y as usize;
    let copy_row_bytes = (region.physical_width as usize) * 4;
    clear_frame_padding(
        &mut output[..output_required],
        output_row_bytes,
        output_height as usize,
        copy_row_bytes,
        region.physical_height as usize,
    );
    // Resolved once for the frame, not once per row.
    let swizzle = crate::capture::PixelSwizzle::detect();
    for row in 0..region.physical_height as usize {
        let source_start = (source_y + row) * source_row_pitch + source_x * 4;
        let source_row = &source[source_start..source_start + copy_row_bytes];
        let destination_row = output_height as usize - 1 - row;
        let destination_start = destination_row * output_row_bytes;
        let destination = &mut output[destination_start..destination_start + copy_row_bytes];

        match source_order {
            SourcePixelOrder::Bgra => destination.copy_from_slice(source_row),
            SourcePixelOrder::Rgba => swizzle.convert(source_row, destination),
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

/// How many pixels one decode batch covers.
///
/// A stack array, not a per-frame scratch buffer: decoding a whole 4K row would
/// need 61 KB, and a monitor-sized allocation per frame is exactly what the
/// screenshot path was changed to stop doing. 8 pixels is 32 halves, which is four
/// `vcvtph2ps` issues, and the array stays small enough to sit in L1 alongside the
/// sRGB table.
pub(super) const DECODE_BATCH_PIXELS: usize = 8;

/// Which FP16 decode implementation this CPU should use.
///
/// Mirrors [`crate::capture::PixelSwizzle`] deliberately, for the same reason: the
/// value is resolved once per frame and carried into the row loop, so the
/// CPU-feature probe stays out of a loop that runs once per row and the answer
/// cannot change while the process lives.
///
/// The hardware variant is unrepresentable off x86, so no caller can ask for a
/// path the target does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HalfDecode {
    /// `vcvtph2ps`, eight halves per instruction, with a portable tail.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    F16c,
    /// One half at a time, everywhere.
    Portable,
}

impl HalfDecode {
    /// Pick the fastest implementation the running CPU supports.
    pub(crate) fn detect() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        // Both probed: the 256-bit load and store are AVX instructions, and only
        // the conversion itself is F16C. Every shipping CPU with F16C also has AVX,
        // but the `target_feature` block below is unsound if that is ever untrue.
        if std::arch::is_x86_feature_detected!("f16c") && std::arch::is_x86_feature_detected!("avx")
        {
            return Self::F16c;
        }
        Self::Portable
    }

    /// Decode little-endian halves from `source` into `destination`.
    ///
    /// Converts `min(source.len() / 2, destination.len())` values. Half-to-float is
    /// a widening conversion, so every result is exact and this cannot change the
    /// tone curve: `vcvtph2ps` and [`half_to_f32`] agree bit-for-bit on every
    /// finite value and on both infinities. They may differ in the payload bits of
    /// a NaN, which is unobservable here because [`ToneCurve::map`] maps every
    /// non-finite channel to zero regardless of payload.
    pub(super) fn decode(self, source: &[u8], destination: &mut [f32]) {
        match self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            // SAFETY: `detect` yields `F16c` only when the CPU reports both F16C
            // and AVX.
            Self::F16c => unsafe { halves_to_f32_f16c(source, destination) },
            Self::Portable => halves_to_f32_portable(source, destination),
        }
    }
}

#[inline]
pub(super) fn halves_to_f32_portable(source: &[u8], destination: &mut [f32]) {
    let count = (source.len() / 2).min(destination.len());
    for (index, output) in destination[..count].iter_mut().enumerate() {
        *output = half_to_f32(u16::from_le_bytes([
            source[index * 2],
            source[index * 2 + 1],
        ]));
    }
}

/// Decode little-endian halves from `source` into `destination`, eight per
/// `vcvtph2ps`.
///
/// # Safety
///
/// The CPU must actually support both F16C and AVX - the conversion is F16C, the
/// 128-bit load and 256-bit store are AVX. [`HalfDecode::detect`] probes both
/// before yielding [`HalfDecode::F16c`], and the recording tests call this
/// function only behind the same probe.
///
/// There is no length precondition: `count` below is clamped to what *both*
/// slices hold, exactly as in [`halves_to_f32_portable`], so any pair of lengths
/// is sound (and decodes the same values the portable path would).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx,f16c")]
pub(super) unsafe fn halves_to_f32_f16c(source: &[u8], destination: &mut [f32]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm256_cvtph_ps, _mm256_storeu_ps, _mm_loadu_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm256_cvtph_ps, _mm256_storeu_ps, _mm_loadu_si128};

    let count = (source.len() / 2).min(destination.len());
    let vectorized_len = count / 8 * 8;
    let mut offset = 0;
    while offset < vectorized_len {
        // SAFETY: `offset` and `vectorized_len` are both multiples of 8 with
        // `offset < vectorized_len`, so `offset + 8 <= vectorized_len <= count`,
        // and `count <= source.len() / 2`. The 16 bytes read from `offset * 2`
        // are therefore inside `source`. `_mm_loadu_si128` is unaligned, which is
        // required: a `&[u8]` promises no 16-byte alignment.
        let packed = unsafe { _mm_loadu_si128(source.as_ptr().add(offset * 2).cast()) };
        let widened = _mm256_cvtph_ps(packed);
        // SAFETY: `count` is also clamped by `destination.len()`, so the same
        // `offset + 8 <= count` bound leaves 8 writable floats from `offset`.
        // `_mm256_storeu_ps` is the unaligned store, and `destination` is a `&mut`
        // slice, so nothing else observes those floats mid-write.
        unsafe { _mm256_storeu_ps(destination.as_mut_ptr().add(offset), widened) };
        offset += 8;
    }

    // The tail is the same scalar decode, so a width that is not a multiple of 8
    // halves cannot take a different code path for its last few values.
    halves_to_f32_portable(
        &source[vectorized_len * 2..count * 2],
        &mut destination[vectorized_len..count],
    );
}

/// The shared linear-to-sRGB lookup table.
///
/// Fetched once per frame rather than once per channel: the `OnceLock` check is
/// an atomic load, and a 4K HDR frame converts 8.3 million pixels, so paying it
/// three times per pixel put ~25 million atomic loads in the tone-mapping loop.
fn srgb_lut() -> &'static [u8; 4097] {
    static LUT: OnceLock<Box<[u8; 4097]>> = OnceLock::new();
    LUT.get_or_init(|| {
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
    })
}

#[inline]
fn linear_to_srgb8_with(lut: &[u8; 4097], value: f32) -> u8 {
    let index = (value.clamp(0.0, 1.0) * 4096.0 + 0.5) as usize;
    lut[index]
}

/// Convert one linear scRGB pixel to SDR sRGB without clipping HDR highlights.
///
/// scRGB uses 80 nits for diffuse white and permits values above 1.0. The
/// shoulder leaves headroom for those highlights and scales all channels by
/// one factor, preserving hue instead of clamping channels independently.
/// The recording loop uses [`ToneCurve`] so it can hoist everything that depends
/// only on the monitor out of its per-pixel loop; this single-pixel form is what
/// the tests pin the tone curve against.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn tone_map_scrgb_pixel(
    red: f32,
    green: f32,
    blue: f32,
    peak_luminance_nits: f32,
) -> [u8; 3] {
    ToneCurve::new(srgb_lut(), peak_luminance_nits).map(red, green, blue)
}

/// The scRGB-to-SDR tone curve with its per-frame constants resolved once.
///
/// `peak_luminance_nits` describes the *monitor*, so everything derived from it
/// is identical for all 8.3 million pixels of a 4K frame. Holding those
/// derivations in a struct is what keeps them out of the loop: computing
/// `source_peak` cost a division and a `max` on every pixel, and its logarithm
/// cost a `ln` call on every highlight pixel, all to recompute the same two
/// numbers. `ln` is a libcall, not an instruction.
///
/// The stored logarithm is the *denominator*, kept as-is rather than
/// pre-reciprocated: `level.ln() / log_source_peak` is then bit-for-bit what the
/// pixel-at-a-time version computed, and the tone curve the tests pin does not
/// move by a last-bit rounding difference.
struct ToneCurve<'a> {
    lut: &'a [u8; 4097],
    /// `ln(source_peak)`, where `source_peak` is the monitor's peak expressed in
    /// multiples of scRGB reference white and floored just above 1.0 so the
    /// highlight shoulder can never divide by a logarithm of zero.
    log_source_peak: f32,
}

impl<'a> ToneCurve<'a> {
    fn new(lut: &'a [u8; 4097], peak_luminance_nits: f32) -> Self {
        let source_peak = (peak_luminance_nits / SCRGB_REFERENCE_WHITE_NITS).max(1.01);
        Self {
            lut,
            log_source_peak: source_peak.ln(),
        }
    }

    fn map(&self, red: f32, green: f32, blue: f32) -> [u8; 3] {
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

        let mapped_level = if level <= 1.0 {
            level * SDR_DIFFUSE_WHITE
        } else {
            let highlight = (level.ln() / self.log_source_peak).clamp(0.0, 1.0);
            SDR_DIFFUSE_WHITE + (SDR_PEAK - SDR_DIFFUSE_WHITE) * highlight
        };
        let scale = mapped_level / level;

        [
            linear_to_srgb8_with(self.lut, red * scale),
            linear_to_srgb8_with(self.lut, green * scale),
            linear_to_srgb8_with(self.lut, blue * scale),
        ]
    }
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
    let source_x = region.physical_x as usize;
    let source_y = region.physical_y as usize;
    let copy_row_bytes = region.physical_width as usize * 8;
    clear_frame_padding(
        &mut output[..output_required],
        output_row_bytes,
        output_height as usize,
        region.physical_width as usize * 4,
        region.physical_height as usize,
    );
    // Both resolved once for the whole frame, not once per channel per pixel.
    let curve = ToneCurve::new(srgb_lut(), peak_luminance_nits);
    let decoder = HalfDecode::detect();
    let mut decoded = [0f32; DECODE_BATCH_PIXELS * 4];
    for row in 0..region.physical_height as usize {
        let source_start = (source_y + row) * source_row_pitch + source_x * 8;
        let source_row = &source[source_start..source_start + copy_row_bytes];
        let destination_row = output_height as usize - 1 - row;
        let destination_start = destination_row * output_row_bytes;
        let destination =
            &mut output[destination_start..destination_start + region.physical_width as usize * 4];

        // Decode a batch of whole pixels, then tone-map them one at a time. The
        // decode is the only part that vectorises: the curve needs `ln` on
        // highlight pixels and three dependent table lookups per pixel, and an
        // approximated logarithm would move the curve the tests pin. Splitting the
        // loop this way takes the win that is exact and leaves the arithmetic
        // byte-for-byte as it was.
        //
        // Source and destination chunk at the same pixel count (8 bytes per pixel
        // against 4), so the final partial batch describes the same pixels in both.
        for (source_batch, destination_batch) in source_row
            .chunks(DECODE_BATCH_PIXELS * 8)
            .zip(destination.chunks_mut(DECODE_BATCH_PIXELS * 4))
        {
            let pixels = source_batch.len() / 8;
            let halves = pixels * 4;
            decoder.decode(&source_batch[..halves * 2], &mut decoded[..halves]);

            for (index, bgra) in destination_batch
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .take(pixels)
                .enumerate()
            {
                let channel = index * 4;
                let [red, green, blue] =
                    curve.map(decoded[channel], decoded[channel + 1], decoded[channel + 2]);
                *bgra = [blue, green, red, u8::MAX];
            }
        }
    }

    Ok(())
}
