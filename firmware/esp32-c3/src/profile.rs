//! Board profiles: the one place a concrete panel + pin map may be named.
//!
//! # There is exactly one profile, and it is simulation-only
//!
//! [`wokwi`] describes `sim/wokwi/diagram-spi.json` and nothing else. **No physical Kivori profile exists**
//! — the panel controller, the SPI/CS/D/C/RST pin map, the visible-area offsets, the orientation, the colour
//! order, and the backlight polarity are all unconfirmed (`docs/validation-checklist.md` items 23-25). They
//! are deliberately absent rather than guessed, because a plausible-looking default is worse than a missing
//! one: it would compile, run, and quietly be wrong on real glass.
//!
//! When the hardware facts arrive, add a sibling `kivori_240` module here, gate it behind its own feature,
//! and leave this module untouched. Nothing outside this file may name a controller.

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
    /// The Kivori board's real assignment is **not decided** and must come from the hardware design.
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

#[cfg(feature = "physical-st7789")]
pub mod physical_st7789 {
    /// SPI clock.
    pub const SCK: u8 = 6;

    /// SPI MOSI.
    pub const MOSI: u8 = 7;

    /// Data/command.
    pub const DC: u8 = 2;

    /// Display reset.
    pub const RST: u8 = 3;

    /// Backlight.
    pub const BL: u8 = 8;
}