//! Pure regression tests for the platform capture boundary.

use crate::capture::platform::{
    frame_crop_within_window, gdi_bgrx_to_rgba, receive_capture_result, resolve_window_bounds,
    rgba_from_bgra_rows, validate_gdi_scan_lines, WindowBounds,
};
use std::cell::Cell;
use std::sync::mpsc;
use std::time::Duration;

// --- frame_crop_within_window: PrintWindow output -> extended-frame crop ---

#[test]
fn frame_crop_offsets_are_relative_to_the_window_rect() {
    // Window rect includes 7px invisible resize borders on the sides/bottom;
    // the DWM extended frame excludes them.
    let window = WindowBounds::new(100, 100, 900, 700);
    let frame = WindowBounds::new(107, 100, 893, 693);

    let (x, y, w, h) = frame_crop_within_window(window, frame).unwrap();

    assert_eq!((x, y), (7, 0));
    assert_eq!((w, h), (786, 593));
}

#[test]
fn frame_crop_of_identical_rects_is_the_full_window() {
    let window = WindowBounds::new(-200, 50, 600, 650);

    let (x, y, w, h) = frame_crop_within_window(window, window).unwrap();

    assert_eq!((x, y), (0, 0));
    assert_eq!((w, h), (800, 600));
}

#[test]
fn frame_crop_clamps_a_frame_that_leaks_outside_the_window() {
    // DWM occasionally reports frame bounds a pixel outside the window rect;
    // the crop must stay within the rendered surface instead of underflowing.
    let window = WindowBounds::new(10, 10, 110, 110);
    let frame = WindowBounds::new(8, 9, 112, 111);

    let (x, y, w, h) = frame_crop_within_window(window, frame).unwrap();

    assert_eq!((x, y), (0, 0));
    assert_eq!((w, h), (100, 100));
}

#[test]
fn frame_crop_rejects_a_frame_fully_outside_the_window() {
    let window = WindowBounds::new(0, 0, 100, 100);
    let frame = WindowBounds::new(200, 200, 300, 300);

    let error = frame_crop_within_window(window, frame).unwrap_err();

    assert!(error.contains("outside"));
}

#[test]
fn gdi_bgrx_conversion_makes_every_pixel_opaque() {
    let mut pixels = vec![
        10, 20, 30, 0, // BGRX with the unused byte cleared
        40, 50, 60, 17, // the unused byte must not leak as alpha
    ];

    gdi_bgrx_to_rgba(&mut pixels);

    assert_eq!(pixels, vec![30, 20, 10, 255, 60, 50, 40, 255]);
}

#[test]
fn gdi_scan_line_validation_requires_the_full_image() {
    assert!(validate_gdi_scan_lines(1080, 1080).is_ok());

    for actual in [0, 1, 1079] {
        let error = validate_gdi_scan_lines(actual, 1080).unwrap_err();
        assert!(error.contains(&actual.to_string()));
        assert!(error.contains("1080"));
    }
}

#[test]
fn window_bounds_prefer_extended_frame_without_fallback() {
    let fallback_called = Cell::new(false);
    let expected = WindowBounds::new(10, 20, 310, 220);

    let actual = resolve_window_bounds(
        || Ok(expected),
        || {
            fallback_called.set(true);
            Err("fallback must not run".to_string())
        },
    )
    .unwrap();

    assert_eq!(actual, expected);
    assert!(!fallback_called.get());
}

#[test]
fn window_bounds_fall_back_when_extended_frame_fails() {
    let expected = WindowBounds::new(-200, 50, 600, 650);

    let actual =
        resolve_window_bounds(|| Err("DWM unavailable".to_string()), || Ok(expected)).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(actual.dimensions().unwrap(), (800, 600));
}

#[test]
fn window_bounds_reject_invalid_fallback_dimensions() {
    let error = resolve_window_bounds(
        || Err("DWM unavailable".to_string()),
        || Ok(WindowBounds::new(10, 10, 10, 20)),
    )
    .unwrap_err();

    assert!(error.contains("zero or negative"));
}

#[test]
fn bgra_row_copy_ignores_gpu_padding_and_preserves_alpha() {
    let mapped = [
        10, 20, 30, 40, 50, 60, 70, 80, 0xAA, 0xAA, 0xAA, 0xAA, // row 1
        90, 100, 110, 120, 130, 140, 150, 160, 0xBB, 0xBB, 0xBB, 0xBB, // row 2
    ];

    let rgba = rgba_from_bgra_rows(&mapped, 12, 2, 2).unwrap();

    assert_eq!(
        rgba,
        vec![30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120, 150, 140, 130, 160,]
    );
}

#[test]
fn bgra_row_copy_rejects_short_pitch_or_buffer() {
    let short_pitch = rgba_from_bgra_rows(&[0; 16], 7, 2, 2).unwrap_err();
    assert!(short_pitch.contains("row pitch"));

    let short_buffer = rgba_from_bgra_rows(&[0; 15], 8, 2, 2).unwrap_err();
    assert!(short_buffer.contains("buffer"));
}

#[test]
fn bgra_row_copy_rejects_zero_dimensions() {
    assert!(rgba_from_bgra_rows(&[], 0, 0, 1).is_err());
    assert!(rgba_from_bgra_rows(&[], 0, 1, 0).is_err());
}

#[test]
fn capture_worker_result_is_returned_before_the_deadline() {
    let (sender, receiver) = mpsc::channel();
    sender.send(Ok(42)).unwrap();

    assert_eq!(
        receive_capture_result(receiver, Duration::from_millis(50), "GDI").unwrap(),
        42
    );
}

#[test]
fn capture_worker_timeout_does_not_wait_for_the_worker_to_finish() {
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    let (release_sender, release_receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        release_receiver.recv().unwrap();
        let _ = sender.send(Ok(()));
    });

    let error = receive_capture_result(receiver, Duration::from_millis(20), "GDI").unwrap_err();

    assert!(error.contains("timed out"));
    release_sender.send(()).unwrap();
    worker.join().unwrap();
}

#[test]
fn capture_worker_disconnect_is_reported() {
    let (sender, receiver) = mpsc::channel::<Result<(), String>>();
    drop(sender);

    let error = receive_capture_result(receiver, Duration::from_millis(50), "GDI").unwrap_err();

    assert!(error.contains("stopped without returning a result"));
}

/// `SendableHwnd` is what lets a window handle cross into the capture worker
/// thread without an `unsafe impl Send`, so the value must survive the trip to
/// `isize` and back unchanged — including the high-bit patterns a 64-bit handle
/// can carry.
#[cfg(windows)]
#[test]
fn sendable_hwnd_round_trips_every_handle_pattern() {
    use crate::capture::platform::SendableHwnd;
    use windows::Win32::Foundation::HWND;

    for raw in [0isize, 1, 0x0001_0044, -1, isize::MAX, isize::MIN] {
        let original = HWND(raw as *mut core::ffi::c_void);
        let restored = SendableHwnd::from(original).hwnd();
        assert_eq!(restored.0 as isize, raw);
        assert_eq!(restored, original);
    }
}

// --- PixelSwizzle: the shared RGBA/BGRA converter ---

/// Independent reference: the obvious per-pixel swap, kept deliberately naive.
fn reference_swap_red_blue(source: &[u8]) -> Vec<u8> {
    source
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
        .collect()
}

/// Every variant the running target can produce.
///
/// Naming the implementations in an enum is what makes this possible: the
/// portable path stays testable on a machine that has AVX2, so CI cannot pass
/// while the fallback is silently broken for older CPUs.
fn all_swizzles() -> Vec<crate::capture::PixelSwizzle> {
    use crate::capture::PixelSwizzle;

    let mut variants = vec![PixelSwizzle::Portable];
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if std::arch::is_x86_feature_detected!("avx2") {
        variants.push(PixelSwizzle::Avx2);
    }
    variants
}

#[test]
fn every_swizzle_matches_the_reference_across_vector_boundaries() {
    // 32-byte AVX2 blocks mean the interesting lengths are those either side of
    // a block boundary, so every tail length is exercised here.
    for swizzle in all_swizzles() {
        for pixels in 0..40usize {
            let source: Vec<u8> = (0..pixels * 4).map(|index| (index % 251) as u8).collect();
            let mut destination = vec![0u8; source.len()];
            swizzle.convert(&source, &mut destination);
            assert_eq!(
                destination,
                reference_swap_red_blue(&source),
                "{swizzle:?} mismatch at {pixels} pixels"
            );
        }
    }
}

#[test]
fn detected_swizzle_agrees_with_the_portable_one() {
    use crate::capture::PixelSwizzle;

    // The whole point of the AVX2 path is that it is indistinguishable from the
    // fallback, so a 4K-sized buffer must convert identically under both.
    let source: Vec<u8> = (0..4096u32).map(|index| (index % 253) as u8).collect();
    let mut detected = vec![0u8; source.len()];
    let mut portable = vec![0u8; source.len()];

    PixelSwizzle::detect().convert(&source, &mut detected);
    PixelSwizzle::Portable.convert(&source, &mut portable);

    assert_eq!(detected, portable);
}

#[test]
fn in_place_swizzle_agrees_with_the_copying_one() {
    for swizzle in all_swizzles() {
        let source: Vec<u8> = (0..2048u32).map(|index| (index % 249) as u8).collect();
        let mut copied = vec![0u8; source.len()];
        swizzle.convert(&source, &mut copied);

        let mut in_place = source.clone();
        swizzle.convert_in_place(&mut in_place);

        assert_eq!(in_place, copied, "{swizzle:?} in-place diverged");
    }
}

#[test]
fn in_place_swizzle_agrees_with_the_reference_across_vector_boundaries() {
    // The 2048-byte case above is an exact multiple of 32, so the in-place AVX2
    // path's scalar tail never runs there, and the only other caller in the tree
    // -- `rgba_to_bgra_bottom_up_in_place` over in `record` -- was handing it rows
    // of at most 28 bytes, which are all tail and no block. Between them the two
    // left one branch of `bgra_to_rgba_avx2` unreached: a tail that is skipped
    // *because* the block loop ran. A mutant that ran the tail only when
    // `vectorized_len == 0` passed the whole suite; this sweep fails it at 9
    // pixels. Lengths 0..40 cover none, one and two whole 32-byte blocks with
    // every tail width in between, matching what
    // `every_swizzle_matches_the_reference_across_vector_boundaries` already does
    // for the copying form.
    //
    // Compared against the naive reference rather than against `convert`: both
    // vectorized paths share `RED_BLUE_SHUFFLE`, so a broken control array would
    // cancel out if they were only ever compared with each other.
    for swizzle in all_swizzles() {
        for pixels in 0..40usize {
            let source: Vec<u8> = (0..pixels * 4).map(|index| (index % 247) as u8).collect();

            let mut in_place = source.clone();
            swizzle.convert_in_place(&mut in_place);

            assert_eq!(
                in_place,
                reference_swap_red_blue(&source),
                "{swizzle:?} in-place mismatch at {pixels} pixels"
            );
        }
    }
}

#[test]
fn swizzle_is_its_own_inverse() {
    for swizzle in all_swizzles() {
        let source: Vec<u8> = (0..1024u32).map(|index| (index % 256) as u8).collect();
        let mut once = vec![0u8; source.len()];
        let mut twice = vec![0u8; source.len()];

        swizzle.convert(&source, &mut once);
        swizzle.convert(&once, &mut twice);

        assert_eq!(twice, source, "{swizzle:?} is not an involution");
    }
}

#[test]
fn swizzle_converts_only_whole_shared_pixels() {
    for swizzle in all_swizzles() {
        // A trailing partial pixel is left alone, matching `bgra_to_rgba`.
        let source = [1, 2, 3, 4, 9, 9, 9];
        let mut destination = [0u8; 7];
        swizzle.convert(&source, &mut destination);
        assert_eq!(destination, [3, 2, 1, 4, 0, 0, 0]);

        // A destination shorter than the source stops at its own last whole
        // pixel rather than panicking.
        let mut short = [0u8; 5];
        swizzle.convert(&source, &mut short);
        assert_eq!(short, [3, 2, 1, 4, 0]);

        // And a source shorter than the destination leaves the rest untouched.
        let mut long = [7u8; 12];
        swizzle.convert(&[1, 2, 3, 4], &mut long);
        assert_eq!(long, [3, 2, 1, 4, 7, 7, 7, 7, 7, 7, 7, 7]);
    }
}

// --- receive_worker_message: bounded waits for non-Result payloads ---

#[test]
fn worker_message_is_returned_untouched_before_the_deadline() {
    use crate::capture::platform::receive_worker_message;

    // The payload is not a `Result<_, String>`, which is the whole reason this
    // exists: the WGC worker reports Unsupported-versus-Failed in its own enum
    // and that distinction has to survive the wait.
    let (sender, receiver) = mpsc::channel();
    sender.send(Err::<u8, &str>("unsupported")).unwrap();

    let message = receive_worker_message(receiver, Duration::from_millis(50), "WGC").unwrap();
    assert_eq!(message, Err("unsupported"));
}

#[test]
fn worker_message_timeout_names_the_backend_and_the_budget() {
    use crate::capture::platform::receive_worker_message;

    let (sender, receiver) = mpsc::channel::<u8>();
    let (release_sender, release_receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        // Stands in for a driver call that never returns: the wait must expire
        // without waiting for this thread, which is why the caller must not join.
        release_receiver.recv().unwrap();
        let _ = sender.send(1);
    });

    let error = receive_worker_message(receiver, Duration::from_millis(20), "WGC").unwrap_err();

    assert!(error.contains("WGC"), "unexpected message: {error}");
    assert!(error.contains("20 ms"), "unexpected message: {error}");
    release_sender.send(()).unwrap();
    worker.join().unwrap();
}

#[test]
fn worker_message_reports_a_worker_that_died_silently() {
    use crate::capture::platform::receive_worker_message;

    let (sender, receiver) = mpsc::channel::<u8>();
    drop(sender);

    let error = receive_worker_message(receiver, Duration::from_millis(50), "WGC").unwrap_err();

    assert!(error.contains("stopped without returning a result"));
}
