//! Coordinate-space value types shared by capture, selection, and recording.

const INVALID_SCALE_FACTOR: &str = "Display scale factor must be positive";

/// A point measured in logical, DPI-independent desktop coordinates.
///
/// This type is intentionally distinct from [`PhysicalPoint`], so coordinate
/// spaces cannot be mixed without an explicit scale conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogicalPoint((i32, i32));

impl LogicalPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self((x, y))
    }

    pub const fn x(self) -> i32 {
        self.0 .0
    }

    pub const fn y(self) -> i32 {
        self.0 .1
    }

    /// Convert this logical point into physical pixels.
    pub fn to_physical(self, scale_factor: ScaleFactor) -> PhysicalPoint {
        PhysicalPoint::new(
            scale_factor.scale_coordinate(self.x()),
            scale_factor.scale_coordinate(self.y()),
        )
    }
}

impl From<(i32, i32)> for LogicalPoint {
    fn from(value: (i32, i32)) -> Self {
        Self(value)
    }
}

impl From<LogicalPoint> for (i32, i32) {
    fn from(value: LogicalPoint) -> Self {
        value.0
    }
}

/// A point measured in physical framebuffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhysicalPoint((i32, i32));

impl PhysicalPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self((x, y))
    }

    pub const fn x(self) -> i32 {
        self.0 .0
    }

    pub const fn y(self) -> i32 {
        self.0 .1
    }
}

impl From<(i32, i32)> for PhysicalPoint {
    fn from(value: (i32, i32)) -> Self {
        Self(value)
    }
}

impl From<PhysicalPoint> for (i32, i32) {
    fn from(value: PhysicalPoint) -> Self {
        value.0
    }
}

/// A width and height measured in logical, DPI-independent pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogicalSize((u32, u32));

impl LogicalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self((width, height))
    }

    pub const fn width(self) -> u32 {
        self.0 .0
    }

    pub const fn height(self) -> u32 {
        self.0 .1
    }

    /// Convert this logical size into physical pixels, rounding each extent up.
    pub fn to_physical(self, scale_factor: ScaleFactor) -> Result<PhysicalSize, String> {
        Ok(PhysicalSize::new(
            scale_factor.scale_extent(self.width())?,
            scale_factor.scale_extent(self.height())?,
        ))
    }
}

impl From<(u32, u32)> for LogicalSize {
    fn from(value: (u32, u32)) -> Self {
        Self(value)
    }
}

impl From<LogicalSize> for (u32, u32) {
    fn from(value: LogicalSize) -> Self {
        value.0
    }
}

/// A width and height measured in physical framebuffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhysicalSize((u32, u32));

impl PhysicalSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self((width, height))
    }

    pub const fn width(self) -> u32 {
        self.0 .0
    }

    pub const fn height(self) -> u32 {
        self.0 .1
    }
}

impl From<(u32, u32)> for PhysicalSize {
    fn from(value: (u32, u32)) -> Self {
        Self(value)
    }
}

impl From<PhysicalSize> for (u32, u32) {
    fn from(value: PhysicalSize) -> Self {
        value.0
    }
}

/// A validated logical-to-physical display scale factor.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ScaleFactor(f32);

impl TryFrom<f32> for ScaleFactor {
    type Error = String;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || value <= 0.0 {
            return Err(INVALID_SCALE_FACTOR.to_string());
        }
        Ok(Self(value))
    }
}

impl ScaleFactor {
    pub const ONE: Self = Self(1.0);

    pub const fn get(self) -> f32 {
        self.0
    }

    fn scale_coordinate(self, value: i32) -> i32 {
        ((value as f32) * self.0).round() as i32
    }

    pub(crate) fn scale_extent(self, value: u32) -> Result<u32, String> {
        let scaled = (value as f64) * (self.0 as f64);
        if !(0.0..=(u32::MAX as f64)).contains(&scaled) {
            return Err("Physical screen size is out of range".to_string());
        }
        Ok(scaled.ceil() as u32)
    }
}

/// A rectangular selection region measured in physical framebuffer pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub physical_x: i32,
    pub physical_y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
}

impl Region {
    /// Construct a selection region from explicitly physical coordinates.
    pub const fn from_physical(origin: PhysicalPoint, size: PhysicalSize) -> Self {
        Self {
            physical_x: origin.x(),
            physical_y: origin.y(),
            physical_width: size.width(),
            physical_height: size.height(),
        }
    }

    pub const fn origin(self) -> PhysicalPoint {
        PhysicalPoint::new(self.physical_x, self.physical_y)
    }

    pub const fn size(self) -> PhysicalSize {
        PhysicalSize::new(self.physical_width, self.physical_height)
    }
}
