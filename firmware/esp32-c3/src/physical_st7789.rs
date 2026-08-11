//! Physical ESP32-C3 + ST7789 240x240 Kivori runtime.

use core::convert::Infallible;

use embedded_hal::digital::{ErrorType, OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;

use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    peripherals::Peripherals,
    spi::{
        master::{Config, Spi},
        Mode,
    },
    time::Rate,
};

use kivori_assets::AssetBlob;
use kivori_model::Capabilities;
use kivori_protocol::FirmwareVersion;

use mipidsi::{
    interface::SpiInterface,
    models::ST7789,
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
    Builder,
};

use crate::{
    display::{MipidsiSink, PanelGeometry},
    proto::DeviceIdentity,
    runtime::{run, RuntimeConfig},
    transport::{TxBuffered, UsbJtagTransport},
};

const SPI_CLOCK_HZ: u32 = 20_000_000;
const SPI_BATCH_BYTES: usize = 512;

/// Runs Kivori on the physical ESP32-C3 + ST7789 hardware.
pub fn run_mode(
    peripherals: Peripherals,
    clock: crate::clock::EspClock,
    assets: &'static [u8],
) -> ! {
    esp_println::println!("KIVORI physical ST7789 runtime");

    //
    // USB transport
    //

    let serial = UsbJtagTransport::new(peripherals.USB_DEVICE);
    let mut transport = TxBuffered::new(serial);

    //
    // SPI
    //

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_hz(SPI_CLOCK_HZ))
            .with_mode(Mode::_3),
    )
    .expect("SPI config")
    .with_sck(peripherals.GPIO6)
    .with_mosi(peripherals.GPIO7);

    //
    // Display control pins
    //

    let dc = Output::new(
        peripherals.GPIO2,
        Level::High,
        OutputConfig::default(),
    );

    let rst = Output::new(
        peripherals.GPIO3,
        Level::High,
        OutputConfig::default(),
    );

    // Keep the backlight enabled.
    let _backlight = Output::new(
        peripherals.GPIO8,
        Level::High,
        OutputConfig::default(),
    );

    //
    // ST7789
    //

    let spi_dev = ExclusiveDevice::new(
        spi,
        NoChipSelect,
        Delay::new(),
    )
    .expect("SPI device");

    let mut interface_buffer = [0u8; SPI_BATCH_BYTES];

    let interface = SpiInterface::new(
        spi_dev,
        dc,
        &mut interface_buffer,
    );

    let mut display = Builder::new(ST7789, interface)
        .reset_pin(rst)
        .display_size(240, 240)
        .orientation(
            Orientation::new().rotate(Rotation::Deg90)
        )
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut Delay::new())
        .expect("ST7789 init");

    esp_println::println!("KIVORI display initialized");

    //
    // Wrap mipidsi display in Kivori's DisplaySink.
    //

    let geometry = PanelGeometry::new(
        240,
        240,
        0,
        0,
    );

    let mut display = MipidsiSink::new(
        display,
        geometry,
    );

    //
    // Parse Kivori's compiled assets.
    //

    let blob = AssetBlob::parse(assets)
        .expect("Kivori asset blob");

    esp_println::println!("KIVORI assets loaded");

    //
    // Device identity
    //

    let identity = DeviceIdentity {
        device_id: [
            0x4B, 0x49, 0x56, 0x4F,
            0x52, 0x49, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01,
        ],

        firmware_version: FirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },

        capabilities: Capabilities::NONE,
    };

    esp_println::println!("KIVORI runtime starting");

    //
    // This never returns.
    //

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