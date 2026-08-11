//! Board profiles: the one place concrete panel and pin-map facts are recorded.
//!
//! Two profile classes currently exist:
//!
//! - [`wokwi`] — simulation-only wiring for the Wokwi test environment.
//! - [`physical_st7789`] — hardware-validated ESP32-C3 + ST7789 240x240 profile.
//!
//! Simulation values must never be treated as physical hardware evidence.
//! Physical profile changes require validation against real hardware.

/// The simulation-only profile for `sim/wokwi/diagram-spi.json`.
///
/// Belongs to the Wokwi diagram, not to any board. It is compiled only into simulation artifacts, and
/// `scripts/check-release-surface.sh` proves it is absent from a production firmware library.
#[cfg(any(feature = "wokwi-spi", feature = "wokwi-runtime"))]
pub mod wokwi {
    use crate::display::PanelGeometry;

    /// **Simulation-only** SPI pin profile matching `sim/wokwi/diagram-spi.json`.
    ///
    /// Evidence for the choice, not preference:
    ///
    /// * `wokwi-cli lint` reports the valid pins of `board-esp32-c3-devkitm-1` as
    ///   `0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 18, 19, RST, RX, TX` (plus power/ground). GPIO 11-17 and 20-21
    ///   are not exposed by that part at all.
    /// * `18`/`19` are excluded: they carry USB Serial/JTAG on this SoC, which the runtime itself uses.
    /// * `2`, `8`, `9` are excluded: ESP32-C3 strapping pins.
    ///
    /// These pins belong only to the Wokwi simulation and are unrelated to the
    /// validated physical Kivori ST7789 pin map.
    pub struct WokwiSpiPins;

    impl WokwiSpiPins {
        /// SPI clock (`esp:4` → `lcd:SCK`).
        pub const SCK: u8 = 4;
        /// SPI data out (`esp:5` → `lcd:MOSI`).
        pub const MOSI: u8 = 5;
        /// Chip select (`esp:6` → `lcd:CS`).
        pub const CS: u8 = 6;
        /// Data/command select (`esp:7` → `lcd:D/C`).
        pub const DC: u8 = 7;
        /// Panel reset (`esp:10` → `lcd:RST`).
        pub const RST: u8 = 10;
    }

    /// Geometry of the simulated panel: the full 240x240 window at offset `(0, 0)`.
    ///
    /// `(0, 0)` is correct **for the Wokwi part**; it is not a measurement of any physical module, and must
    /// not be copied into a hardware profile.
    #[must_use]
    pub const fn geometry() -> PanelGeometry {
        PanelGeometry::new(240, 240, 0, 0)
    }
}

/// Verified physical ESP32-C3 + ST7789 240x240 hardware profile.
#[cfg(feature = "physical-st7789")]
pub mod physical_st7789 {
    use crate::display::PanelGeometry;
    use esp_hal::{gpio::Level, spi::Mode};
    use mipidsi::{
        models::ST7789,
        options::{ColorInversion, ColorOrder, Orientation, Rotation},
    };

    /// SPI clock pin.
    pub const SCK: u8 = 6;

    /// SPI MOSI pin.
    pub const MOSI: u8 = 7;

    /// Display data/command pin.
    pub const DC: u8 = 2;

    /// Display reset pin.
    pub const RST: u8 = 3;

    /// Display backlight pin.
    pub const BL: u8 = 8;

    /// This physical panel does not use chip select.
    pub const CS: Option<u8> = None;

    /// Verified SPI clock.
    pub const SPI_CLOCK_HZ: u32 = 20_000_000;

    /// Visible panel width.
    pub const WIDTH: u16 = 240;

    /// Visible panel height.
    pub const HEIGHT: u16 = 240;

    /// Verified visible-area X offset.
    pub const OFFSET_X: u16 = 0;

    /// Verified visible-area Y offset.
    pub const OFFSET_Y: u16 = 0;

    /// Backlight is enabled by driving the pin high.
    pub const BACKLIGHT_ACTIVE_HIGH: bool = true;

    /// Verified physical display geometry.
    #[must_use]
    pub const fn geometry() -> PanelGeometry {
        PanelGeometry::new(WIDTH, HEIGHT, OFFSET_X, OFFSET_Y)
    }

    /// Verified SPI mode.
    #[must_use]
    pub const fn spi_mode() -> Mode {
        Mode::_3
    }

    /// Verified physical panel controller.
    #[must_use]
    pub const fn panel_model() -> ST7789 {
        ST7789
    }

    /// Verified panel orientation.
    #[must_use]
    pub fn orientation() -> Orientation {
        Orientation::new().rotate(Rotation::Deg90)
    }

    /// Verified panel color order.
    #[must_use]
    pub const fn color_order() -> ColorOrder {
        ColorOrder::Rgb
    }

    /// Verified panel color inversion.
    #[must_use]
    pub const fn color_inversion() -> ColorInversion {
        ColorInversion::Inverted
    }

    /// Backlight output level.
    #[must_use]
    pub const fn backlight_level() -> Level {
        if BACKLIGHT_ACTIVE_HIGH {
            Level::High
        } else {
            Level::Low
        }
    }
}
