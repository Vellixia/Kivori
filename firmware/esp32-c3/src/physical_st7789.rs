//! Physical ESP32-C3 + ST7789 240x240 Kivori runtime.
//!
//! All verified physical display facts live in
//! [`crate::profile::physical_st7789`].
//!
//! `esp-hal` exposes GPIOs as typed fields (`peripherals.GPIO6`, etc.), so the
//! concrete field selections remain here. Compile-time assertions below ensure
//! they cannot silently drift from the physical profile.

use core::convert::Infallible;

use embedded_hal::digital::{ErrorType, OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;

use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    peripherals::Peripherals,
    spi::master::{Config, Spi},
    time::Rate,
};

use kivori_assets::AssetBlob;
use kivori_model::Capabilities;
use kivori_protocol::FirmwareVersion;

use mipidsi::{interface::SpiInterface, Builder};

use crate::{
    display::MipidsiSink,
    profile::physical_st7789 as hw,
    proto::DeviceIdentity,
    runtime::{run, RuntimeConfig},
    transport::{TxBuffered, UsbJtagTransport},
};

/// Number of bytes used by `mipidsi` for batching SPI display writes.
const SPI_BATCH_BYTES: usize = 512;

/// Keep the typed `esp-hal` GPIO field selections below synchronized with the
/// numeric hardware profile.
///
/// If somebody changes a pin in `profile.rs` without updating this file, the
/// physical firmware build fails instead of silently using the wrong wiring.
const _: () = {
    assert!(hw::SCK == 6);
    assert!(hw::MOSI == 7);
    assert!(hw::DC == 2);
    assert!(hw::RST == 3);
    assert!(hw::BL == 8);
    assert!(hw::CS.is_none());
};

/// Runs Kivori on the verified physical ESP32-C3 + ST7789 hardware.
///
/// This function owns the USB Serial/JTAG transport, SPI peripheral, display
/// control pins, backlight, parsed asset blob, and production runtime. It never
/// returns.
pub fn run_mode(
    peripherals: Peripherals,
    clock: crate::clock::EspClock,
    assets: &'static [u8],
) -> ! {
    esp_println::println!("KIVORI physical ST7789 runtime");

    // -------------------------------------------------------------------------
    // USB Serial/JTAG transport
    // -------------------------------------------------------------------------

    let serial = UsbJtagTransport::new(peripherals.USB_DEVICE);
    let mut transport = TxBuffered::new(serial);

    // -------------------------------------------------------------------------
    // SPI2
    //
    // Verified physical wiring:
    //   SCK  -> GPIO6
    //   MOSI -> GPIO7
    //
    // No MISO is needed because the display path is write-only.
    // -------------------------------------------------------------------------

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_hz(hw::SPI_CLOCK_HZ))
            .with_mode(hw::spi_mode()),
    )
    .expect("SPI config")
    .with_sck(peripherals.GPIO6)
    .with_mosi(peripherals.GPIO7);

    // -------------------------------------------------------------------------
    // Display control pins
    //
    // Verified physical wiring:
    //   DC  -> GPIO2
    //   RST -> GPIO3
    //   BL  -> GPIO8
    // -------------------------------------------------------------------------

    let dc = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());

    let rst = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());

    // Keep the verified backlight enabled for the lifetime of the runtime.
    let _backlight = Output::new(
        peripherals.GPIO8,
        hw::backlight_level(),
        OutputConfig::default(),
    );

    // -------------------------------------------------------------------------
    // SPI device
    //
    // The verified physical ST7789 module does not use a CS line, so
    // `NoChipSelect` satisfies the `SpiDevice` abstraction without toggling
    // another GPIO.
    // -------------------------------------------------------------------------

    let spi_dev = ExclusiveDevice::new(spi, NoChipSelect, Delay::new()).expect("SPI device");

    let mut interface_buffer = [0u8; SPI_BATCH_BYTES];

    let interface = SpiInterface::new(spi_dev, dc, &mut interface_buffer);

    // -------------------------------------------------------------------------
    // ST7789 initialization
    //
    // Controller, geometry, orientation, RGB/BGR order, inversion, SPI mode,
    // and offsets all come from the verified physical profile.
    // -------------------------------------------------------------------------

    let display = Builder::new(hw::panel_model(), interface)
        .reset_pin(rst)
        .display_size(hw::WIDTH, hw::HEIGHT)
        .display_offset(hw::OFFSET_X, hw::OFFSET_Y)
        .orientation(hw::orientation())
        .color_order(hw::color_order())
        .invert_colors(hw::color_inversion())
        .init(&mut Delay::new())
        .expect("ST7789 init");

    esp_println::println!("KIVORI display initialized");

    // -------------------------------------------------------------------------
    // Kivori display sink
    //
    // Converts the initialized mipidsi display into Kivori's generic
    // DisplaySink. The renderer will send change-driven 240x40 RGB565 tiles.
    // -------------------------------------------------------------------------

    let mut display = MipidsiSink::new(display, hw::geometry());

    // -------------------------------------------------------------------------
    // Compiled Kivori assets
    // -------------------------------------------------------------------------

    let blob = AssetBlob::parse(assets).expect("Kivori asset blob");

    esp_println::println!("KIVORI assets loaded");

    // -------------------------------------------------------------------------
    // Device identity
    // -------------------------------------------------------------------------

    let identity = DeviceIdentity {
        device_id: [
            0x4B, 0x49, 0x56, 0x4F, 0x52, 0x49, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ],

        firmware_version: FirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },

        capabilities: Capabilities::NONE,
    };

    esp_println::println!("KIVORI runtime starting");

    // -------------------------------------------------------------------------
    // Production runtime
    //
    // Handles:
    // - USB protocol
    // - Hello / handshake
    // - SetState
    // - StateReport
    // - heartbeat
    // - lifecycle
    // - shared renderer
    // - change-driven display updates
    //
    // Never returns.
    // -------------------------------------------------------------------------

    run(
        identity,
        RuntimeConfig::default(),
        &clock,
        &mut transport,
        &mut display,
        &blob,
        |_tick, _transport| {},
    );
}

/// Dummy chip-select pin for the verified physical panel, which has no CS line.
struct NoChipSelect;

impl ErrorType for NoChipSelect {
    type Error = Infallible;
}

impl OutputPin for NoChipSelect {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
