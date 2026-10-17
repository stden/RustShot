use image::RgbaImage;
use std::io::{Read, Write};

#[cfg(target_os = "linux")]
pub const OUTPUT_ENV: &str = "RUSTSHOT_SELECTION_OUTPUT";
const MAX_IMAGE_BYTES: usize = 512 * 1024 * 1024;
pub type ImageResult = Result<Option<RgbaImage>, String>;

fn image_byte_count(width: u32, height: u32) -> Result<usize, String> {
    let size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|&bytes| bytes > 0 && bytes <= MAX_IMAGE_BYTES)
        .ok_or_else(|| "Selected image dimensions exceed the transfer limit".to_string())?;
    Ok(size)
}

pub fn write_image(mut writer: impl Write, image: &RgbaImage) -> Result<(), String> {
    let size = image_byte_count(image.width(), image.height())?;
    writer
        .write_all(&image.width().to_le_bytes())
        .and_then(|_| writer.write_all(&image.height().to_le_bytes()))
        .and_then(|_| writer.write_all(&image.as_raw()[..size]))
        .and_then(|_| writer.flush())
        .map_err(|e| format!("Failed to send selected image: {e}"))
}

fn allocate_pixel_buffer(size: usize) -> Result<Vec<u8>, String> {
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(size)
        .map_err(|e| format!("Failed to allocate selected image: {e}"))?;
    pixels.resize(size, 0);
    Ok(pixels)
}

pub fn read_image(mut reader: impl Read) -> ImageResult {
    let mut header = [0u8; 8];
    match reader.read_exact(&mut header[..1]) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(format!("Failed to receive selected image: {e}")),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|e| format!("Incomplete selected image header: {e}"))?;
    let width = u32::from_le_bytes(header[..4].try_into().unwrap());
    let height = u32::from_le_bytes(header[4..].try_into().unwrap());
    let mut pixels = image_byte_count(width, height).and_then(allocate_pixel_buffer)?;
    reader
        .read_exact(&mut pixels)
        .map_err(|e| format!("Incomplete selected image pixels: {e}"))?;
    Ok(RgbaImage::from_raw(width, height, pixels))
}

#[cfg(target_os = "linux")]
pub fn copy_image(owner: &mut Option<arboard::Clipboard>, image: RgbaImage) -> Result<(), String> {
    let clipboard = match owner {
        Some(clipboard) => clipboard,
        None => owner.insert(
            arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?,
        ),
    };
    let data = arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Owned(image.into_raw()),
    };
    clipboard
        .set_image(data)
        .map_err(|e| format!("Failed to copy to clipboard: {e}"))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn selected_image_survives_transfer() {
        let mut image = RgbaImage::new(2, 1);
        image.put_pixel(0, 0, Rgba([1, 2, 3, 255]));
        image.put_pixel(1, 0, Rgba([4, 5, 6, 128]));
        let mut output = Vec::new();

        write_image(&mut output, &image).unwrap();

        assert_eq!(read_image(output.as_slice()).unwrap(), Some(image));
    }

    #[test]
    fn cancelled_selection_has_no_image() {
        assert_eq!(read_image(std::io::empty()).unwrap(), None);
    }

    #[test]
    fn incomplete_header_and_pixels_are_rejected() {
        assert!(read_image([1u8].as_slice()).is_err());
        let mut output = Vec::new();
        write_image(&mut output, &RgbaImage::new(2, 2)).unwrap();
        output.pop();
        assert!(read_image(output.as_slice()).is_err());
    }

    #[test]
    fn unreasonable_dimensions_are_rejected_before_allocating() {
        for (width, height) in [(0u32, 1u32), (u32::MAX, u32::MAX), (16384, 16384)] {
            let header = [width.to_le_bytes(), height.to_le_bytes()].concat();
            assert!(read_image(header.as_slice()).is_err());
        }
        assert!(allocate_pixel_buffer(usize::MAX).is_err());
    }

    #[test]
    fn image_transfer_propagates_write_errors() {
        struct FailedWriter;
        impl Write for FailedWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("pipe closed"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        assert!(write_image(FailedWriter, &RgbaImage::new(0, 0)).is_err());
        assert!(write_image(FailedWriter, &RgbaImage::new(1, 1)).is_err());
    }

    #[test]
    fn image_transfer_propagates_non_eof_read_error() {
        enum TestReader<'a> {
            Slice(&'a [u8]),
            BrokenPipe,
        }
        impl Read for TestReader<'_> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                match self {
                    Self::Slice(slice) => slice.read(buf),
                    Self::BrokenPipe => Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "pipe broken",
                    )),
                }
            }
        }

        let err = read_image(TestReader::BrokenPipe).unwrap_err();
        assert!(err.contains("Failed to receive selected image"));
        assert_eq!(read_image(TestReader::Slice(&[])).unwrap(), None);
        assert!(read_image(TestReader::Slice(&[1])).is_err());
        let zero_header = [0u32.to_le_bytes(), 1u32.to_le_bytes()].concat();
        assert!(read_image(TestReader::Slice(&zero_header)).is_err());
        let mut valid = Vec::new();
        write_image(&mut valid, &RgbaImage::new(1, 1)).unwrap();
        assert!(read_image(TestReader::Slice(&valid)).unwrap().is_some());
        valid.pop();
        assert!(read_image(TestReader::Slice(&valid)).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires an X11 display; run under xvfb-run"]
    fn tray_retains_clipboard_between_captures() {
        let mut owner = None;
        let first = RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]));
        copy_image(&mut owner, first).unwrap();
        assert!(owner.is_some());
        let mut reader = arboard::Clipboard::new().unwrap();
        assert_eq!(reader.get_image().unwrap().bytes.as_ref(), &[1, 2, 3, 255]);

        let second = RgbaImage::from_pixel(1, 1, Rgba([4, 5, 6, 255]));
        copy_image(&mut owner, second).unwrap();
        assert_eq!(reader.get_image().unwrap().bytes.as_ref(), &[4, 5, 6, 255]);
    }
}
