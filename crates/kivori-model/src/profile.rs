//! Device display profile: resolution, color format, panel controller, and tile geometry
//! (data-model §7, FR-020). The Device Studio preview and the firmware use the SAME profile for a
//! given build (constitution Principle II).

use serde::{Deserialize, Serialize};

/// Runtime color format. Only RGB565 is supported (canonical runtime color).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ColorFormat {
    /// 16-bit RGB565.
    #[default]
    Rgb565,
}

/// Supported 240x240 SPI display controllers (chosen per hardware; see research R-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelController {
    /// GC9A01 (round).
    Gc9a01,
    /// ST7789.
    St7789,
}

/// Tile/scanline band geometry used for change-driven rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileConfig {
    /// Tile width in pixels.
    pub tile_w: u16,
    /// Tile height in pixels.
    pub tile_h: u16,
}

/// A description of the target display and how it is tiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Display width in pixels.
    pub width: u16,
    /// Display height in pixels.
    pub height: u16,
    /// Runtime color format.
    pub color: ColorFormat,
    /// Panel controller.
    pub controller: PanelController,
    /// Tile geometry.
    pub tile: TileConfig,
}

/// Reason a [`DeviceProfile`] is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    /// A display or tile dimension was zero.
    ZeroDimension,
    /// The display size is not an exact multiple of the tile size.
    TilesDoNotTileExactly,
}

impl DeviceProfile {
    /// The provisional Kivori 240x240 profile. The panel controller is provisional pending hardware
    /// validation (task T118); the display is tiled as full-width 240x40 scanline bands.
    pub const KIVORI_240: DeviceProfile = DeviceProfile {
        width: 240,
        height: 240,
        color: ColorFormat::Rgb565,
        controller: PanelController::St7789,
        tile: TileConfig {
            tile_w: 240,
            tile_h: 40,
        },
    };

    /// Validates that dimensions are non-zero and that the display tiles exactly.
    ///
    /// # Errors
    /// [`ProfileError::ZeroDimension`] if any dimension is zero; [`ProfileError::TilesDoNotTileExactly`]
    /// if the display is not an exact multiple of the tile size.
    pub fn validate(self) -> Result<(), ProfileError> {
        if self.width == 0 || self.height == 0 || self.tile.tile_w == 0 || self.tile.tile_h == 0 {
            return Err(ProfileError::ZeroDimension);
        }
        if !self.width.is_multiple_of(self.tile.tile_w)
            || !self.height.is_multiple_of(self.tile.tile_h)
        {
            return Err(ProfileError::TilesDoNotTileExactly);
        }
        Ok(())
    }

    /// Number of tiles that cover the display, or `0` if the tile size is degenerate.
    #[must_use]
    pub const fn tile_count(self) -> u32 {
        if self.tile.tile_w == 0 || self.tile.tile_h == 0 {
            return 0;
        }
        let cols = (self.width / self.tile.tile_w) as u32;
        let rows = (self.height / self.tile.tile_h) as u32;
        cols * rows
    }
}
