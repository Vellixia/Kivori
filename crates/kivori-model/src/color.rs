//! RGB565 canonical runtime color (data-model §6). Deterministic integer conversions only.

use serde::{Deserialize, Serialize};

/// A 16-bit RGB565 color (5 bits red, 6 bits green, 5 bits blue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgb565(u16);

impl Rgb565 {
    /// Black (`0x0000`).
    pub const BLACK: Rgb565 = Rgb565(0x0000);
    /// White (`0xFFFF`).
    pub const WHITE: Rgb565 = Rgb565(0xFFFF);

    /// Creates a color from a raw RGB565 value.
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Rgb565(raw)
    }

    /// Returns the raw RGB565 value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Converts 8-bit-per-channel RGB into RGB565 by truncating the low bits (deterministic).
    #[must_use]
    pub const fn from_rgb888(r: u8, g: u8, b: u8) -> Self {
        let r5 = (r as u16 >> 3) & 0x1F;
        let g6 = (g as u16 >> 2) & 0x3F;
        let b5 = (b as u16 >> 3) & 0x1F;
        Rgb565((r5 << 11) | (g6 << 5) | b5)
    }

    /// Expands to 8-bit-per-channel RGB via bit-replication (deterministic).
    #[must_use]
    pub const fn to_rgb888(self) -> (u8, u8, u8) {
        let r5 = (self.0 >> 11) & 0x1F;
        let g6 = (self.0 >> 5) & 0x3F;
        let b5 = self.0 & 0x1F;
        let r = ((r5 << 3) | (r5 >> 2)) as u8;
        let g = ((g6 << 2) | (g6 >> 4)) as u8;
        let b = ((b5 << 3) | (b5 >> 2)) as u8;
        (r, g, b)
    }
}
