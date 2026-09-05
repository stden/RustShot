//! Validated recording rate shared by settings and recording.

/// Recording frames per second.
///
/// A newtype rather than a bare `u32` so the valid range is carried by the type:
/// once you hold an `Fps`, it is in `1..=Fps::MAX`. That makes an out-of-range
/// FPS unrepresentable in settings instead of something every consumer has
/// to re-check.
///
/// Two constructors, for the two different trust levels:
/// - [`Fps::new`] — strict. Rejects `0` and anything above [`Fps::MAX`]. Used at
///   the boundary, e.g. validating a recording request or dialog input.
/// - [`Fps::from_settings`] — forgiving. Falls back to [`Fps::DEFAULT`] for any
///   invalid value, because `settings.json` is hand-editable and a bad number
///   should not brick the app. This mirrors how an unknown image format falls
///   back to PNG.
///
/// # Examples
///
/// ```
/// use rustshot::settings::Fps;
///
/// assert_eq!(Fps::new(60).unwrap().get(), 60);
/// assert!(Fps::new(0).is_err());
/// assert!(Fps::new(300).is_err());
///
/// // Forgiving path: invalid values become the default rather than an error.
/// assert_eq!(Fps::from_settings(0), Fps::DEFAULT);
/// assert_eq!(Fps::from_settings(300), Fps::DEFAULT);
/// assert_eq!(Fps::from_settings(60).get(), 60);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fps(u32);

impl Fps {
    /// Maximum FPS a recording may request.
    pub const MAX: u32 = 240;

    /// The default recording rate, used whenever a value is missing or invalid.
    pub const DEFAULT: Fps = Fps(30);

    /// Strict constructor: the only way to build an `Fps` from untrusted input.
    pub fn new(value: u32) -> Result<Self, String> {
        if value == 0 {
            return Err("Recording FPS must be greater than 0".to_string());
        }
        if value > Self::MAX {
            return Err(format!("Recording FPS must be {} or less", Self::MAX));
        }
        Ok(Self(value))
    }

    /// Forgiving constructor for hand-edited `settings.json`: invalid values
    /// become [`Fps::DEFAULT`] rather than an error.
    pub fn from_settings(value: u32) -> Self {
        Self::new(value).unwrap_or(Self::DEFAULT)
    }

    /// The underlying rate. Guaranteed to be in `1..=Fps::MAX`.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for Fps {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TryFrom<u32> for Fps {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Fps> for u32 {
    fn from(fps: Fps) -> Self {
        fps.get()
    }
}

impl std::fmt::Display for Fps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
