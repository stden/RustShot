//! Edge-case tests: `bgra_to_rgba` corner inputs, pipeline ordering, and review-sweep coverage of format/path/screen helpers.

use crate::capture::*;
use std::cell::RefCell;

// --- bgra_to_rgba edge cases ---

#[test]
fn bgra_to_rgba_trailing_bytes_ignored() {
    // chunks_exact_mut skips trailing bytes that don't form a full 4-byte chunk
    let mut pixels = vec![255, 0, 0, 255, 10, 20]; // 4 + 2 trailing
    bgra_to_rgba(&mut pixels);
    // First chunk swapped
    assert_eq!(pixels[0], 0);
    assert_eq!(pixels[2], 255);
    // Trailing bytes untouched
    assert_eq!(pixels[4], 10);
    assert_eq!(pixels[5], 20);
}

#[test]
fn bgra_to_rgba_large_image_all_swapped() {
    // 100 pixels, all with B=10, G=20, R=30, A=40
    let mut pixels = [10, 20, 30, 40].repeat(100);
    bgra_to_rgba(&mut pixels);
    for chunk in pixels.chunks(4) {
        assert_eq!(chunk[0], 30); // was R, now in B position
        assert_eq!(chunk[1], 20); // G unchanged
        assert_eq!(chunk[2], 10); // was B, now in R position
        assert_eq!(chunk[3], 40); // A unchanged
    }
}

#[test]
fn bgra_to_rgba_matches_scalar_reference_for_unaligned_and_trailing_slices() {
    for offset in 0..32 {
        for len in 0..=129 {
            let mut storage: Vec<u8> = (0..offset + len + 8)
                .map(|index| ((index * 37 + 11) & 0xff) as u8)
                .collect();
            let pixels = &mut storage[offset..offset + len];
            let mut expected = pixels.to_vec();
            for pixel in expected.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }

            bgra_to_rgba(pixels);

            assert_eq!(
                pixels, expected,
                "conversion differed for byte offset {offset} and length {len}"
            );
        }
    }
}

// --- ensure_parent_directory error path ---

#[test]
fn ensure_parent_directory_root_path_is_ok() {
    // "/file.png" parent is "/" which exists
    assert!(ensure_parent_directory(Path::new("/file.png")).is_ok());
}

// --- capture_screenshot_with: ensure closure order ---

#[test]
fn capture_screenshot_with_calls_closures_in_order() {
    let order = RefCell::new(Vec::<&str>::new());

    let _ = capture_screenshot_with(
        1,
        None,
        || {
            order.borrow_mut().push("capture");
            Ok(())
        },
        Some(PathBuf::from("test.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| {
            order.borrow_mut().push("ensure_dir");
            Ok(())
        },
        |_, _, _| {
            order.borrow_mut().push("save");
            Ok(())
        },
    );

    assert_eq!(*order.borrow(), vec!["capture", "ensure_dir", "save"]);
}

#[test]
fn resolve_output_path_explicit_with_no_extension() {
    let explicit = PathBuf::from("no_ext");
    let result = resolve_output_path(Some(explicit.clone()), Format::Png);
    assert_eq!(result, explicit);
    assert_eq!(result.extension(), None);
}

// --- resolve_screen_index property tests ---

#[test]
fn resolve_screen_index_always_returns_requested_when_in_range() {
    for count in 1..=10usize {
        for idx in 0..count {
            assert_eq!(
                resolve_screen_index(count, Some(idx)).unwrap(),
                idx,
                "count={count}, idx={idx}"
            );
        }
    }
}

#[test]
fn resolve_screen_index_always_errors_when_out_of_range() {
    for count in 1..=5usize {
        for idx in count..count + 3 {
            assert!(
                resolve_screen_index(count, Some(idx)).is_err(),
                "count={count}, idx={idx} should be out of range"
            );
        }
    }
}
