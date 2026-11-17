//! Clipboard image ownership shared by tray capture actions.

pub fn copy_image(
    owner: &mut Option<arboard::Clipboard>,
    image: &image::RgbaImage,
) -> Result<(), String> {
    let clipboard = match owner {
        Some(clipboard) => clipboard,
        None => owner.insert(
            arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?,
        ),
    };
    clipboard
        .set_image(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: std::borrow::Cow::Borrowed(image.as_raw()),
        })
        .map_err(|e| format!("Failed to copy to clipboard: {e}"))
}
