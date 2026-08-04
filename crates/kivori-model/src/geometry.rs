//! Integer pixel geometry primitives (data-model §6, §8). No floating point.

use serde::{Deserialize, Serialize};

/// A signed integer 2D point (pixels). Layer origins may be negative during animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Point {
    /// X coordinate.
    pub x: i16,
    /// Y coordinate.
    pub y: i16,
}

impl Point {
    /// The origin `(0, 0)`.
    pub const ORIGIN: Point = Point { x: 0, y: 0 };

    /// Creates a point.
    #[must_use]
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// An unsigned integer size (pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Size {
    /// Width.
    pub w: u16,
    /// Height.
    pub h: u16,
}

impl Size {
    /// Creates a size.
    #[must_use]
    pub const fn new(w: u16, h: u16) -> Self {
        Self { w, h }
    }

    /// Area in pixels.
    #[must_use]
    pub const fn area(self) -> u32 {
        self.w as u32 * self.h as u32
    }

    /// Returns `true` if either dimension is zero.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.w == 0 || self.h == 0
    }
}

/// An axis-aligned rectangle in unsigned pixel space (a screen region).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rect {
    /// Left edge.
    pub x: u16,
    /// Top edge.
    pub y: u16,
    /// Width.
    pub w: u16,
    /// Height.
    pub h: u16,
}

impl Rect {
    /// Creates a rectangle.
    #[must_use]
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    /// Exclusive right edge (`x + w`), computed in `u32` to avoid overflow.
    #[must_use]
    pub const fn right(self) -> u32 {
        self.x as u32 + self.w as u32
    }

    /// Exclusive bottom edge (`y + h`), computed in `u32` to avoid overflow.
    #[must_use]
    pub const fn bottom(self) -> u32 {
        self.y as u32 + self.h as u32
    }

    /// Area in pixels.
    #[must_use]
    pub const fn area(self) -> u32 {
        self.w as u32 * self.h as u32
    }

    /// Returns `true` if either dimension is zero.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.w == 0 || self.h == 0
    }
}
