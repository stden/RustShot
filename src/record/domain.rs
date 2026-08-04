//! Validated value types used by the recording pipeline.

use super::{checked_rgba_len, checked_round_to_even};
use crate::selection::Region;
use std::fmt;

/// An even, non-zero H.264 output width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Width(u32);

impl TryFrom<u32> for Width {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err("Recording width must be greater than 0".to_string());
        }
        Ok(Self(checked_round_to_even(value)?))
    }
}

impl Width {
    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

/// An even, non-zero H.264 output height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Height(u32);

impl TryFrom<u32> for Height {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err("Recording height must be greater than 0".to_string());
        }
        Ok(Self(checked_round_to_even(value)?))
    }
}

impl Height {
    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

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

/// A Media Foundation timestamp measured in 100-nanosecond units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HnsTimestamp(i64);

impl HnsTimestamp {
    pub(crate) const fn get(self) -> i64 {
        self.0
    }
}

/// A Media Foundation duration measured in 100-nanosecond units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HnsDuration(i64);

impl HnsDuration {
    pub(crate) const fn get(self) -> i64 {
        self.0
    }
}

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
