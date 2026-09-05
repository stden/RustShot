//! Image format policy, independent of capture backends.

/// Supported output image formats.
///
/// This enum is the single source of truth for "which formats exist". Settings
/// store it directly rather than a `String`, so an unsupported format is not
/// representable once parsed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Png,
    Jpg,
    Bmp,
}

// The image-encoder method stays in the capture adapter, preserving the public
// API without making domain policy depend on the image library.
#[allow(clippy::multiple_inherent_impl)]
impl Format {
    /// Every supported format, in settings/UI order. The first entry is the
    /// default used as the fallback for unrecognised input.
    pub const ALL: [Format; 3] = [Format::Png, Format::Jpg, Format::Bmp];

    /// File extension string (without leading dot) for this format. This is
    /// also exactly how the format is written to `settings.json`.
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::Bmp => "bmp",
        }
    }

    /// Parse a format as stored in `settings.json`.
    ///
    /// Deliberately **case-sensitive** and forgiving: anything unrecognised
    /// falls back to PNG rather than failing, because `settings.json` is
    /// hand-editable and a bad value should not brick the app. Note the
    /// contrast with CLI output-extension parsing, which is
    /// case-insensitive because a user may well type `--output shot.PNG`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustshot::capture::Format;
    ///
    /// assert_eq!(Format::from_settings_str("jpg"), Format::Jpg);
    /// assert_eq!(Format::from_settings_str("webp"), Format::Png); // unknown → default
    /// assert_eq!(Format::from_settings_str("JPG"), Format::Png);  // case-sensitive
    /// ```
    pub fn from_settings_str(value: &str) -> Format {
        Format::ALL
            .into_iter()
            .find(|fmt| fmt.extension() == value)
            .unwrap_or(Format::Png)
    }
}
