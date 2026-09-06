//! Shared, platform-independent pixel conversion, validation, and frame preparation.

use crate::domain::Region;

pub(crate) mod frame;

/// Convert BGRA pixel data to RGBA in place.
///
/// Algorithm:
/// 1. Interpret the byte slice as a sequence of 4-byte pixels.
/// 2. For each complete pixel, swap byte 0 (blue) with byte 2 (red).
/// 3. Leave green and alpha unchanged.
/// 4. Ignore any trailing incomplete chunk to avoid out-of-bounds work.
///
/// The operation is in-place and allocation-free, which keeps the hot path
/// cheap for full-screen captures.
///
/// The x86/x86-64 implementation uses AVX2 when the CPU supports it and falls
/// back to the portable scalar loop everywhere else. The runtime feature check
/// keeps the binary compatible with older processors.
///
/// Possible improvement:
/// - Validate producer pixel format up front if future capture backends can
///   return layouts other than BGRA.
///
/// # Examples
///
/// ```
/// use rustshot::capture::bgra_to_rgba;
///
/// let mut pixels = vec![0u8, 100, 200, 255]; // B=0, G=100, R=200, A=255
/// bgra_to_rgba(&mut pixels);
/// assert_eq!(pixels, vec![200, 100, 0, 255]); // R=200, G=100, B=0, A=255
/// ```
pub fn bgra_to_rgba(pixels: &mut [u8]) {
    PixelSwizzle::detect().convert_in_place(pixels);
}

#[inline]
fn bgra_to_rgba_scalar(pixels: &mut [u8]) {
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2); // swap B and R
    }
}

/// AVX2 byte-shuffle control that swaps red and blue inside each 4-byte pixel,
/// duplicated across both 128-bit lanes. Shared by the in-place and copying
/// converters so the two cannot drift apart.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const RED_BLUE_SHUFFLE: [i8; 32] = [
    2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14, 13, 12, 15, 2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11, 14,
    13, 12, 15,
];

/// Swap red and blue inside `pixels` in place, 32 bytes per `pshufb`.
///
/// # Safety
///
/// The CPU must actually support AVX2. That is the whole of this function's
/// contract - it has no requirement on `pixels`, because the length arithmetic
/// below derives every offset from `pixels.len()` and the scalar tail finishes
/// whatever the 32-byte loop could not reach. [`PixelSwizzle::detect`] is what
/// establishes the feature for the only caller.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn bgra_to_rgba_avx2(pixels: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};

    // SAFETY: `RED_BLUE_SHUFFLE` is a 32-byte `[i8; 32]` const, so the read of
    // 32 bytes from its start is in bounds, and the intrinsic permits unaligned
    // loads - the array carries no 32-byte alignment guarantee.
    let shuffle = unsafe { _mm256_loadu_si256(RED_BLUE_SHUFFLE.as_ptr().cast()) };
    let vectorized_len = pixels.len() / 32 * 32;
    let mut offset = 0;
    while offset < vectorized_len {
        // SAFETY: `offset` starts at 0 and only grows by 32, and `vectorized_len`
        // is at most `pixels.len()`, so `offset` is a byte index strictly inside
        // the slice and `add` stays within its allocation.
        let pointer = unsafe { pixels.as_mut_ptr().add(offset).cast() };
        // SAFETY: `offset` and `vectorized_len` are both multiples of 32 with
        // `offset < vectorized_len`, so `vectorized_len - offset >= 32`: the 32
        // bytes this reads are all inside `pixels`. The intrinsic permits
        // unaligned addresses, which a byte slice cannot promise anything about.
        let value = unsafe { _mm256_loadu_si256(pointer) };
        let value = _mm256_shuffle_epi8(value, shuffle);
        // SAFETY: writes back exactly the 32 in-bounds bytes the load above read,
        // through the `&mut [u8]` those bytes came from, so nothing else aliases
        // them for the duration. `_mm256_storeu_si256` is likewise unaligned.
        unsafe { _mm256_storeu_si256(pointer, value) };
        offset += 32;
    }

    bgra_to_rgba_scalar(&mut pixels[vectorized_len..]);
}

/// Which red/blue swizzle implementation this CPU should use.
///
/// BGRA and RGBA differ only by swapping red and blue, so every conversion in
/// either direction goes through this one enum. It exists as a value rather than
/// a check inside the conversion because callers convert a frame *row at a
/// time*: resolving the choice once per frame and carrying it into the row loop
/// keeps the CPU-feature probe out of a loop that runs thousands of times, and
/// the answer cannot change while the process lives.
///
/// The AVX2 variant is unrepresentable off x86, so no caller can ask for a path
/// the target does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelSwizzle {
    /// 32-byte `pshufb` blocks with a portable tail.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2,
    /// Four bytes at a time, everywhere.
    Portable,
}

impl PixelSwizzle {
    /// Pick the fastest implementation the running CPU supports.
    pub(crate) fn detect() -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if std::arch::is_x86_feature_detected!("avx2") {
            return Self::Avx2;
        }
        Self::Portable
    }

    /// Copy `source` into `destination`, swapping each pixel's red and blue.
    ///
    /// Converts only whole pixels within the shorter of the two slices; a
    /// trailing partial pixel is left untouched, matching [`bgra_to_rgba`].
    pub(crate) fn convert(self, source: &[u8], destination: &mut [u8]) {
        let len = source.len().min(destination.len()) / 4 * 4;
        let source = &source[..len];
        let destination = &mut destination[..len];

        match self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            // SAFETY: `detect` yields `Avx2` only when the CPU reports AVX2.
            // Both slices were trimmed to one shared whole-pixel length.
            Self::Avx2 => unsafe { swap_red_blue_into_avx2(source, destination) },
            Self::Portable => swap_red_blue_into_portable(source, destination),
        }
    }

    /// Swap red and blue within `pixels`, in place.
    pub(crate) fn convert_in_place(self, pixels: &mut [u8]) {
        let complete_len = pixels.len() / 4 * 4;
        let pixels = &mut pixels[..complete_len];

        match self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            // SAFETY: as above; the helper stays inside the whole-pixel prefix.
            Self::Avx2 => unsafe { bgra_to_rgba_avx2(pixels) },
            Self::Portable => bgra_to_rgba_scalar(pixels),
        }
    }
}

#[inline]
fn swap_red_blue_into_portable(source: &[u8], destination: &mut [u8]) {
    for (source, destination) in source
        .as_chunks::<4>()
        .0
        .iter()
        .zip(destination.as_chunks_mut::<4>().0.iter_mut())
    {
        *destination = [source[2], source[1], source[0], source[3]];
    }
}

/// Copy `source` into `destination` with red and blue swapped, 32 bytes per
/// `pshufb`.
///
/// # Safety
///
/// Two preconditions, both discharged by [`PixelSwizzle::convert`]:
/// - the CPU must actually support AVX2;
/// - `destination.len()` must be at least `source.len()`. Every offset below is
///   derived from the *source* length, so a shorter destination would be written
///   past its end. `convert` trims both slices to one shared length first.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn swap_red_blue_into_avx2(source: &[u8], destination: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};

    // SAFETY: `RED_BLUE_SHUFFLE` is a 32-byte `[i8; 32]` const, so the read of
    // 32 bytes from its start is in bounds, and the intrinsic permits unaligned
    // loads - the array carries no 32-byte alignment guarantee.
    let shuffle = unsafe { _mm256_loadu_si256(RED_BLUE_SHUFFLE.as_ptr().cast()) };
    let vectorized_len = source.len() / 32 * 32;
    let mut offset = 0;
    while offset < vectorized_len {
        // SAFETY: `offset` starts at 0 and only grows by 32, and `vectorized_len`
        // is at most `source.len()`, so `offset` indexes strictly inside `source`.
        let source_pointer = unsafe { source.as_ptr().add(offset).cast() };
        // SAFETY: the same index, in bounds of `destination` too because this
        // function's contract requires `destination.len() >= source.len()`.
        let destination_pointer = unsafe { destination.as_mut_ptr().add(offset).cast() };
        // SAFETY: `offset` and `vectorized_len` are both multiples of 32 with
        // `offset < vectorized_len`, so `vectorized_len - offset >= 32` and all 32
        // bytes read lie inside `source`. Unaligned loads are permitted.
        let value = unsafe { _mm256_loadu_si256(source_pointer) };
        let value = _mm256_shuffle_epi8(value, shuffle);
        // SAFETY: the mirrored 32 bytes of `destination`, in bounds by the length
        // precondition above. `destination` is a `&mut` slice, so the write is
        // unaliased - and it cannot overlap `source`, which is a shared borrow of
        // a distinct slice.
        unsafe { _mm256_storeu_si256(destination_pointer, value) };
        offset += 32;
    }

    swap_red_blue_into_portable(
        &source[vectorized_len..],
        &mut destination[vectorized_len..],
    );
}

pub(crate) fn checked_rgba_len(width: u32, height: u32) -> Result<usize, String> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Pixel buffer dimensions are too large".to_string())
}

pub(crate) fn validate_capture_region(
    region: Region,
    frame_width: u32,
    frame_height: u32,
) -> Result<(), String> {
    // Validate in three stages: origin, non-zero size, then far edge. Splitting
    // the checks this way keeps the error messages simple and predictable.
    if region.physical_x < 0 || region.physical_y < 0 {
        return Err("Recording region must stay within the selected monitor".to_string());
    }

    if region.physical_width == 0 || region.physical_height == 0 {
        return Err("Recording region must have non-zero size".to_string());
    }

    let region_right = i64::from(region.physical_x) + i64::from(region.physical_width);
    let region_bottom = i64::from(region.physical_y) + i64::from(region.physical_height);

    if region_right > i64::from(frame_width) || region_bottom > i64::from(frame_height) {
        return Err("Recording region must stay within the selected monitor".to_string());
    }

    Ok(())
}

/// Convert RGBA (top-down) to BGRA bottom-up (WMF RGB32 format).
///
/// Allocates the destination and delegates to [`rgba_to_bgra_bottom_up_into`],
/// which flips and swizzles each row in one pass through the crate-private
/// `PixelSwizzle` (plain backticks, not a link: it is `pub(crate)`, and rustdoc
/// runs with `-D warnings`).
///
/// No production path calls this. Every caller that matters supplies its own
/// buffer - the screenshot worker converts in place via
/// [`rgba_to_bgra_bottom_up_in_place`], and the recorder crops through
/// `frame::copy_bgra_region_bottom_up` - so this allocating form survives for
/// benchmarks, the BDD scenarios and doc examples. Prefer either of the other two
/// in new code.
///
/// Possible improvements:
/// - Nothing outstanding. This doc used to describe a two-pass "memcpy rows, then
///   swap R/B in place" algorithm and list "fuse the row flip and channel swap"
///   plus "reuse a caller-provided scratch buffer" as future work. All three had
///   already happened: the pass is fused, `_into` takes the caller's buffer and
///   `_in_place` needs no second one. Stale "possible improvements" are how
///   `round_to_even`'s doc came to promise a contract the code did not honour.
///
/// # Examples
///
/// ```
/// use rustshot::record::rgba_to_bgra_bottom_up;
///
/// // 2x1 image: pixel₀ = (R=10, G=20, B=30, A=255)
/// let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
/// let bgra = rgba_to_bgra_bottom_up(&rgba, 2, 1).unwrap();
/// // Single row, so bottom-up is the same order; R↔B swapped:
/// assert_eq!(bgra, vec![30, 20, 10, 255, 60, 50, 40, 255]);
/// ```
pub fn rgba_to_bgra_bottom_up(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let required = checked_rgba_len(width, height)?;
    let mut out = vec![0u8; required];
    rgba_to_bgra_bottom_up_into(rgba, &mut out, width, height)?;
    Ok(out)
}

/// Optimized RGBA to BGRA bottom-up conversion.
///
/// Writes the converted pixels into the provided `out` buffer.
/// Returns an error when either buffer is shorter than `width * height * 4`
/// or when the dimension calculation overflows.
///
/// No production path calls this any more -- `rgba_to_bgra_bottom_up_in_place` is what
/// the screenshot worker uses -- and it is `pub` and benchmarked anyway for three
/// reasons that are each easy to miss while deleting it. Two `recording.feature`
/// scenarios drive it through a BDD step in `tests/bdd.rs`, so it is specified
/// behaviour rather than spare code. `pixels.rs`'s
/// `rgba_to_bgra_bottom_up_in_place_matches_the_two_buffer_conversion` uses it as the
/// ORACLE the in-place form is checked against, so removing it removes the only thing
/// proving the two agree byte for byte. And its `PixelSwizzle::convert` kernel is a
/// different function from `convert_in_place`'s (`swap_red_blue_into_avx2` versus
/// `bgra_to_rgba_avx2`), live in `capture::platform::rgba_from_bgra_rows` and in the
/// GDI recording fallback, which is why the performance gate still benchmarks this
/// signature: it has no other way to reach that kernel.
pub fn rgba_to_bgra_bottom_up_into(
    rgba: &[u8],
    out: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let required = checked_rgba_len(width, height)?;
    if rgba.len() < required {
        return Err(format!(
            "RGBA input buffer is too short: expected {required} bytes, got {}",
            rgba.len()
        ));
    }
    if out.len() < required {
        return Err(format!(
            "BGRA output buffer is too short: expected {required} bytes, got {}",
            out.len()
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Pixel row is too large".to_string())?;
    let height = height as usize;
    // Resolved once for the frame, not once per row.
    let swizzle = PixelSwizzle::detect();

    for y in 0..height {
        let src_start = y * row_bytes;
        let dst_start = (height - 1 - y) * row_bytes;
        let src_row = &rgba[src_start..src_start + row_bytes];
        let dst_row = &mut out[dst_start..dst_start + row_bytes];

        swizzle.convert(src_row, dst_row);
    }
    Ok(())
}

/// The same flip-and-swizzle as [`rgba_to_bgra_bottom_up_into`], performed inside
/// a single buffer.
///
/// The flip pairs row `y` with row `height - 1 - y` and the swizzle only trades
/// red for blue, so the transform is its own inverse and needs no destination:
/// swapping a row pair and swizzling both halves converts both rows at once. That
/// is also why the screenshot path may run it in the opposite direction --
/// bottom-up BGRA in, top-down RGBA out -- and get byte-identical results.
///
/// Deleting the destination is the point. `capture_monitor_image_on_worker` used
/// to hand [`rgba_to_bgra_bottom_up_into`] a fresh `vec![0; width * height * 4]`,
/// ~33 MB for one 4K monitor, which the conversion then overwrote byte for byte.
/// The zero-fill was pure waste, and first-touching those ~8000 fresh pages costs
/// milliseconds of the hotkey-to-overlay budget this module keeps under 100 ms.
/// Peak footprint per screenshot halves too, from two monitor-sized buffers to one.
///
/// An odd `height` leaves the middle row without a partner, so it is swizzled
/// exactly once, on its own. Swizzling it twice is the same as skipping it -- the
/// swizzle is an involution -- and either mistake leaves a single scanline in the
/// middle of an otherwise perfect screenshot with red and blue crossed, which no
/// check on buffer length or corner pixels would notice.
pub fn rgba_to_bgra_bottom_up_in_place(
    pixels: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let required = checked_rgba_len(width, height)?;
    if pixels.len() < required {
        return Err(format!(
            "Pixel buffer is too short: expected {required} bytes, got {}",
            pixels.len()
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Pixel row is too large".to_string())?;
    let height = height as usize;
    // Resolved once for the frame, not once per row.
    let swizzle = PixelSwizzle::detect();

    // Split into three disjoint pieces so the borrow checker can see that a row
    // and its partner never alias: the top half, the unpaired middle row, and the
    // bottom half walked backwards. `middle` is empty for an even height, which is
    // what keeps the odd case from needing a branch that could count it twice.
    let pairs = height / 2;
    let (top, rest) = pixels[..required].split_at_mut(pairs * row_bytes);
    let (middle, bottom) = rest.split_at_mut((height % 2) * row_bytes);

    // Indexed rather than `chunks_exact_mut`: a zero-width frame makes `row_bytes`
    // zero, and a zero chunk size panics, where the two-buffer form converts such
    // a frame to nothing at all.
    for y in 0..pairs {
        let top_row = &mut top[y * row_bytes..(y + 1) * row_bytes];
        let bottom_row = &mut bottom[(pairs - 1 - y) * row_bytes..(pairs - y) * row_bytes];
        // Move the bytes first, then swizzle each row where it has landed.
        // `PixelSwizzle::convert` cannot do this in one pass without scratch
        // space: writing either flipped row would destroy its partner's source.
        top_row.swap_with_slice(bottom_row);
        swizzle.convert_in_place(top_row);
        swizzle.convert_in_place(bottom_row);
    }
    // Empty for an even height; the frame's only row when `height` is 1.
    swizzle.convert_in_place(middle);
    Ok(())
}
