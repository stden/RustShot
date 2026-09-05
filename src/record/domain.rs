//! Validated value types used by the recording pipeline.

use crate::domain::Region;
use crate::pixels::checked_rgba_len;
use std::fmt;

/// Declare an even, non-zero H.264 output dimension.
///
/// The two axes differ only by name and by the noun their rejection message
/// uses, so they are declared rather than written twice. That keeps the zero
/// check and the round-to-even rule in one place: the failure this shape guards
/// against is one axis quietly losing the rounding H.264 requires while the
/// other keeps it.
macro_rules! even_dimension {
    ($(#[$attr:meta])* $name:ident, $noun:literal) => {
        $(#[$attr])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub(crate) struct $name(u32);

        impl TryFrom<u32> for $name {
            type Error = String;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                if value == 0 {
                    return Err(
                        concat!("Recording ", $noun, " must be greater than 0").to_string()
                    );
                }
                Ok(Self(checked_round_to_even(value)?))
            }
        }

        impl $name {
            pub(crate) const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

even_dimension!(
    /// An even, non-zero H.264 output width.
    Width,
    "width"
);
even_dimension!(
    /// An even, non-zero H.264 output height.
    Height,
    "height"
);

/// Even, non-zero H.264 output dimensions with a checked RGBA buffer length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RecordingDimensions {
    width: Width,
    height: Height,
    rgba_len: usize,
}

impl TryFrom<Region> for RecordingDimensions {
    type Error = String;

    fn try_from(region: Region) -> Result<Self, Self::Error> {
        if region.physical_width == 0 || region.physical_height == 0 {
            return Err("Recording region must have non-zero size".to_string());
        }

        Self::new(
            Width::try_from(region.physical_width)?,
            Height::try_from(region.physical_height)?,
        )
    }
}

impl RecordingDimensions {
    pub(crate) fn new(width: Width, height: Height) -> Result<Self, String> {
        let rgba_len = checked_rgba_len(width.get(), height.get())?;
        Ok(Self {
            width,
            height,
            rgba_len,
        })
    }

    pub(crate) const fn width(self) -> Width {
        self.width
    }

    pub(crate) const fn height(self) -> Height {
        self.height
    }

    pub(crate) const fn rgba_len(self) -> usize {
        self.rgba_len
    }
}

/// A non-negative index in the encoded frame timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FrameIndex(i64);

impl FrameIndex {
    pub(crate) const ZERO: Self = Self(0);

    #[cfg(test)]
    pub(crate) fn new(value: u64) -> Self {
        Self(i64::try_from(value).unwrap_or(i64::MAX))
    }

    pub(crate) fn saturating_from_u128(value: u128) -> Self {
        Self(i64::try_from(value).unwrap_or(i64::MAX))
    }

    pub(crate) const fn get(self) -> i64 {
        self.0
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub(crate) fn deadline_multiplier(self) -> u32 {
        u32::try_from(self.0).unwrap_or(u32::MAX)
    }
}

impl fmt::Display for FrameIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Declare a Media Foundation 100-nanosecond quantity.
///
/// A timestamp and a duration are both bare `i64` HNS counts, and passing one
/// where the other belongs produces a video with wrong frame timing and no error
/// anywhere. Separate types make that a compile error; declaring them keeps the
/// pair symmetric.
macro_rules! hns_quantity {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(crate) struct $name(i64);

        impl $name {
            pub(crate) const fn get(self) -> i64 {
                self.0
            }
        }
    };
}

hns_quantity!(
    /// A Media Foundation timestamp measured in 100-nanosecond units.
    HnsTimestamp
);
hns_quantity!(
    /// A Media Foundation duration measured in 100-nanosecond units.
    HnsDuration
);

/// Timestamp and duration for one encoded frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameTiming {
    timestamp: HnsTimestamp,
    duration: HnsDuration,
}

impl FrameTiming {
    pub(crate) const fn new(timestamp_hns: i64, duration_hns: i64) -> Self {
        Self {
            timestamp: HnsTimestamp(timestamp_hns),
            duration: HnsDuration(duration_hns),
        }
    }

    pub(crate) const fn timestamp(self) -> HnsTimestamp {
        self.timestamp
    }

    pub(crate) const fn duration(self) -> HnsDuration {
        self.duration
    }
}

/// Encoded video bitrate in bits per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Bitrate(u32);

impl Bitrate {
    pub(crate) const fn new(bits_per_second: u32) -> Self {
        Self(bits_per_second)
    }

    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// Round a dimension up to the nearest even number, saturating at `u32::MAX - 1`.
///
/// H.264 encoders require even width and height. Odd values round up by 1, with
/// exactly one exception: `u32::MAX`.
///
/// Algorithm:
/// 1. Add 1 to the input, saturating.
/// 2. Clear the least-significant bit.
/// 3. Even values remain unchanged; odd values below `u32::MAX` round up to the
///    next even, and `u32::MAX` saturates one below itself (see below).
///
/// This bit trick is branch-free and cheap, which is useful in sizing code
/// that may run repeatedly during capture setup.
///
/// # The `u32::MAX` exception
///
/// `u32::MAX` is odd and the next even value, 2^32, does not fit in a `u32`, so
/// step 1 saturates back to `u32::MAX` and step 2 yields `u32::MAX - 1` — one
/// *below* the input. This function therefore does **not** guarantee
/// `result >= value`, and no infallible `u32 -> u32` function can: the remaining
/// options at that input are returning the odd `u32::MAX` (not even, so useless
/// to the encoder) or panicking.
///
/// Use [`checked_round_to_even`] wherever `result >= value` must hold. It rejects
/// `u32::MAX` with an error instead of handing back a smaller size, and it is the
/// form the recording pipeline uses — every recorded dimension is built through
/// `RecordingDimensions`, which rounds with the checked helper, so the saturating
/// case is unreachable in production. The infallible form stays public for the
/// branch-free benchmark and for callers whose input is already bounded.
///
/// Possible improvements:
/// - Nothing outstanding. The two ideas previously listed here — "switch to
///   `saturating_add`" and "return a checked error instead" — had both already
///   been implemented (the add saturates; [`checked_round_to_even`] exists), and
///   leaving them standing as future work is how this doc came to describe a
///   plain `+ 1` that rounds every input up.
///
/// # Examples
///
/// ```
/// use rustshot::record::round_to_even;
///
/// assert_eq!(round_to_even(1920), 1920); // already even
/// assert_eq!(round_to_even(1921), 1922); // rounded up
/// assert_eq!(round_to_even(0), 0);
/// // The single input that comes back SMALLER: 2^32 is not representable.
/// assert_eq!(round_to_even(u32::MAX), u32::MAX - 1);
/// ```
#[allow(dead_code)]
pub fn round_to_even(value: u32) -> u32 {
    value.saturating_add(1) & !1
}

/// Round a dimension up to the nearest even number, or reject it.
///
/// The guaranteed form of [`round_to_even`]: every `Ok` value is even and
/// `>= value`. `u32::MAX` is the only rejected input, because the even value it
/// would need is 2^32; returning `u32::MAX - 1` there — which is what the
/// infallible helper does — would encode a frame one row or column short of the
/// request with nothing reported. Failing at the boundary keeps that impossible
/// geometry visible.
pub fn checked_round_to_even(value: u32) -> Result<u32, String> {
    if value == u32::MAX {
        return Err("Recording dimension is too large to round to an even value".to_string());
    }
    Ok(round_to_even(value))
}
