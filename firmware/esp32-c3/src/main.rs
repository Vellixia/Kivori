#![no_std]
#![no_main]

//! Kivori ESP32-C3 firmware — entry point.
//!
//! Boots the esp-hal runtime, then selects a mode:
//!
//! * `wokwi-runtime` — the **production** run loop ([`kivori_firmware::runtime::run`]) driving real
//!   peripherals, with the simulation-only board profile. Same runtime the physical firmware calls.
//! * `wokwi-serial` — EXTERNAL serial test: Wokwi injects real wire frames into the real USB Serial/JTAG
//!   receive/transmit loop.
//! * `wokwi-spi` — GENERIC SPI/RGB565 tile-transfer probe over the real T072 adapter. Never a controller
//!   validation.
//! * `wokwi` — INTERNAL on-target self-test: the device core driven from inside the firmware.
//! * `embedded` only — boots, brings up the clock and USB Serial/JTAG, and then stops short of the render
//!   loop **because no physical panel profile exists**: the controller, pin map, and offsets are
//!   unconfirmed (`docs/validation-checklist.md` items 23-25). Nothing is faked to fill the gap; add a
//!   profile to `kivori_firmware::profile` once the hardware facts are known.
//!
//! Building this binary requires the `embedded` feature (see `Cargo.toml` `required-features`).

#[cfg(feature = "embedded")]
use esp_backtrace as _;

/// The canonical asset blob, compiled at build time (see `build.rs`). The device never parses source art.
#[cfg(feature = "embedded")]
static ASSETS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kivori.assets"));

/// Firmware entry point. `esp_hal::main` installs the vector table; it never returns.
#[cfg(feature = "embedded")]
#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let clock = kivori_firmware::bsp::clock();

    // PRODUCTION RUNTIME under simulation: the same `runtime::run` the physical firmware calls, wired to
    // real peripherals through the simulation-only profile (T074 + T131 boundary).
    #[cfg(feature = "wokwi-runtime")]
    {
        kivori_firmware::wokwi_runtime::run_mode(peripherals, clock, ASSETS);
    }

    // EXTERNAL serial mode owns the USB Serial/JTAG peripheral and never returns.
    #[cfg(all(feature = "wokwi-serial", not(feature = "wokwi-runtime")))]
    {
        use kivori_firmware::ports::Clock;
        let mut io = kivori_firmware::bsp::serial(peripherals.USB_DEVICE);
        kivori_firmware::external::run(&mut io, || clock.now_ms());
    }

    // GENERIC SPI probe mode owns SPI2 + the display GPIOs.
    #[cfg(all(
        feature = "wokwi-spi",
        not(feature = "wokwi-serial"),
        not(feature = "wokwi-runtime")
    ))]
    {
        let _passed = kivori_firmware::spi_probe::run(peripherals);
        let _ = &clock;
    }

    #[cfg(all(
        feature = "wokwi",
        not(feature = "wokwi-serial"),
        not(feature = "wokwi-spi"),
        not(feature = "wokwi-runtime")
    ))]
    {
        // INTERNAL on-target integration self-test: drives the real device core and asserts over serial.
        let _passed = kivori_firmware::selftest::run(&clock);
        let _ = peripherals;
    }

    #[cfg(not(any(
        feature = "wokwi",
        feature = "wokwi-serial",
        feature = "wokwi-spi",
        feature = "wokwi-runtime"
    )))]
    {
        // The transport and clock adapters are real and brought up here; the render loop is not entered,
        // because entering it would require choosing a panel controller, pin map, and offsets that nobody
        // has measured. Guessing them would compile and run and be wrong on real glass.
        use kivori_firmware::ports::Clock;
        let mut serial = kivori_firmware::bsp::serial(peripherals.USB_DEVICE);
        let boot_ms = clock.now_ms();
        esp_println::println!(
            "KIVORI boot: clock+serial up at {boot_ms}ms; render loop not entered — no confirmed panel \
             profile (controller, pin map, offsets). See docs/validation-checklist.md items 23-25."
        );
        let _ = (&mut serial, ASSETS);
    }

    #[cfg(not(any(feature = "wokwi-serial", feature = "wokwi-runtime")))]
    loop {
        // The self-test/probe has printed its verdict, or the production profile is pending.
        esp_hal::delay::Delay::new().delay_millis(1000);
    }
}
