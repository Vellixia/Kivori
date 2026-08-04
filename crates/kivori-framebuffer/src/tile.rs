//! A [`TileBand`]: a mutable RGB565 view over a rectangular region backed by caller-provided storage.

use kivori_model::{Rect, Rgb565};

/// A mutable RGB565 pixel band covering `rect`, backed by a caller-provided slice of exactly
/// `rect.w * rect.h` pixels (row-major). Coordinates passed to accessors are absolute (display)
/// coordinates; out-of-band coordinates are ignored on write and return `None` on read.
pub struct TileBand<'a> {
    rect: Rect,
    pixels: &'a mut [Rgb565],
}

impl<'a> TileBand<'a> {
    /// Creates a band over `pixels`, or `None` if `pixels.len() != rect.w * rect.h`.
    #[must_use]
    pub fn new(rect: Rect, pixels: &'a mut [Rgb565]) -> Option<Self> {
        if pixels.len() == rect.area() as usize {
            Some(Self { rect, pixels })
        } else {
            None
        }
    }

    /// The band's region in absolute display coordinates.
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// The band's pixels (row-major, `rect.w * rect.h`).
    #[must_use]
    pub fn pixels(&self) -> &[Rgb565] {
        self.pixels
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.rect.x || y < self.rect.y {
            return None;
        }
        let lx = x - self.rect.x;
        let ly = y - self.rect.y;
        if lx >= self.rect.w || ly >= self.rect.h {
            return None;
        }
        Some(ly as usize * self.rect.w as usize + lx as usize)
    }

    /// Sets the pixel at absolute `(x, y)`. Ignored if `(x, y)` is outside the band.
    pub fn set(&mut self, x: u16, y: u16, color: Rgb565) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color;
        }
    }

    /// Gets the pixel at absolute `(x, y)`, or `None` if outside the band.
    #[must_use]
    pub fn get(&self, x: u16, y: u16) -> Option<Rgb565> {
        self.index(x, y).map(|i| self.pixels[i])
    }

    /// Fills the entire band with a single color.
    pub fn fill(&mut self, color: Rgb565) {
        for p in self.pixels.iter_mut() {
            *p = color;
        }
    }
}
