//! Host-only end-to-end harness (tests only).
//!
//! This crate has no runtime code; the value is in `tests/`, which bridges the desktop session driver
//! (`kivori-desktop`) to the firmware protocol dispatcher (`kivori-firmware`) over an in-memory byte
//! pipe and asserts the full connect → set-state → reconnect loop with both real state machines
//! running together — no Tauri, no serial port, no hardware.
