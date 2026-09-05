//! Tests for JPEG pre-conversion, save errors, platform behavior, and Unicode paths.

use crate::capture::*;
use crate::test_support::TestDir;
use std::io;

// =========================================================================
// save_image_optimized
// =========================================================================

#[test]
fn save_image_optimized_jpg_matches_rgb_preconversion() {
    use image::codecs::jpeg::JpegEncoder;
    use std::io::BufWriter;

    let img = image::RgbaImage::from_fn(17, 9, |x, y| {
        image::Rgba([
            (x * 13) as u8,
            (y * 29) as u8,
            ((x + y) * 7) as u8,
            ((x * 11 + y * 17) % 256) as u8,
        ])
    });
    // One TestDir shared by both files this test writes.
    let dir = TestDir::new("rustshot_test_jpg_rgb_preconversion");
    let actual_path = dir.file("actual.jpg");
    let expected_path = dir.file("expected.jpg");

    save_image_optimized(&img, &actual_path, Format::Jpg).unwrap();

    let raw: Vec<u8> = img
        .pixels()
        .flat_map(|pixel| pixel.0[..3].iter().copied())
        .collect();
    let rgb = image::RgbImage::from_raw(img.width(), img.height(), raw).unwrap();
    let writer = BufWriter::new(std::fs::File::create(&expected_path).unwrap());
    rgb.write_with_encoder(JpegEncoder::new_with_quality(writer, 90))
        .unwrap();

    assert_eq!(
        std::fs::read(&actual_path).unwrap(),
        std::fs::read(&expected_path).unwrap()
    );
}

#[test]
fn save_image_optimized_invalid_path() {
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
    let path = std::path::Path::new("/nonexistent_dir_xyz/impossible/test.png");

    let result = save_image_optimized(&img, path, Format::Png);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to create file"));
}

// =========================================================================
// save_image_to_writer - a late I/O failure must never be swallowed
// =========================================================================

/// Accepts every byte, then fails on `flush`.
///
/// `BufWriter::drop` discards the result of its final flush and
/// `png::Writer::drop` closes the file with `let _ = self.write_iend()`, so
/// before the explicit flush this sink's failure vanished and a truncated
/// screenshot was reported as saved.
struct FlushFailsWriter;

impl Write for FlushFailsWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "no space left on device",
        ))
    }
}

/// Fails on the first byte it is asked to write, and reports a clean `flush`.
///
/// Behind a `BufWriter` this is the real shape of a volume filling up: the
/// encoder only ever touches the buffer and therefore succeeds, and the device
/// error appears solely when the buffer is emptied.
struct WriteFailsWriter;

impl Write for WriteFailsWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "no space left on device",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 4x4 keeps every format's output well under the 8 KiB `BufWriter` capacity.
/// That is load-bearing for the buffered test below: a larger image would spill
/// mid-encode and let the encoder report the error, hiding the flush path.
fn tiny_image() -> image::RgbaImage {
    image::RgbaImage::from_pixel(4, 4, image::Rgba([9, 8, 7, 255]))
}

#[test]
fn save_image_to_writer_reports_flush_failure_for_every_format() {
    let img = tiny_image();

    for format in Format::ALL {
        let error = match save_image_to_writer(&img, FlushFailsWriter, format) {
            Ok(()) => panic!("{format:?} reported success for a sink whose flush fails"),
            Err(error) => error,
        };
        assert!(
            error.contains("no space left on device"),
            "{format:?} must name the underlying cause, got {error:?}"
        );
    }
}

#[test]
fn save_image_to_writer_reports_buffered_write_failure_for_every_format() {
    let img = tiny_image();

    for format in Format::ALL {
        // Same layering as the production path: nothing reaches the sink until
        // the buffer is flushed, so this fails exactly where a full disk does.
        let writer = io::BufWriter::new(WriteFailsWriter);
        let error = match save_image_to_writer(&img, writer, format) {
            Ok(()) => panic!("{format:?} reported success for a full-disk sink"),
            Err(error) => error,
        };
        assert!(
            error.contains("no space left on device"),
            "{format:?} must name the underlying cause, got {error:?}"
        );
    }
}

#[test]
fn save_image_to_writer_matches_the_bytes_written_to_a_file() {
    // Guards the extraction itself: both entry points must keep encoding with
    // the settings the benchmarks were tuned on, byte for byte.
    let dir = TestDir::new("rustshot_test_writer_matches_file");
    let img = image::RgbaImage::from_fn(6, 5, |x, y| {
        image::Rgba([(x * 40) as u8, (y * 50) as u8, ((x + y) * 20) as u8, 255])
    });

    for format in Format::ALL {
        let path = dir.file(&format!("shot.{}", format.extension()));
        save_image_optimized(&img, &path, format).unwrap();

        let mut in_memory: Vec<u8> = Vec::new();
        save_image_to_writer(&img, &mut in_memory, format).unwrap();

        assert_eq!(
            std::fs::read(&path).unwrap(),
            in_memory,
            "{format:?} must encode identically through both entry points"
        );
    }
}

// -------------------------------------------------------------------------
// capture_active_window platform tests
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn capture_active_window_in_dir_returns_unsupported_on_linux() {
    let dir = TestDir::new("rustshot_test_active_window_unsupported");
    let result = capture_active_window_in_dir(None, Format::Png, dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

// -------------------------------------------------------------------------
// ensure_parent_directory additional tests
// -------------------------------------------------------------------------

#[test]
fn ensure_parent_directory_unicode_path() {
    // The unicode-named directory itself must NOT pre-exist: the point of
    // the test is that ensure_parent_directory creates it. TestDir only
    // creates the (ASCII-named) root; the unicode child stays missing.
    let root = TestDir::new("rustshot_test_unicode");
    let dir = root.file("тест_юникод");
    let path = dir.join("file.png");
    assert!(ensure_parent_directory(&path).is_ok());
    assert!(dir.exists());
}
