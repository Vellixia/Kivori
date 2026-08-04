//! The monotonic device clock (T073).
//!
//! Backed by esp-hal's system timer, so it is the same implementation on real hardware and in the
//! Wokwi simulator — only the surrounding board bring-up differs. Time is exposed as canonical integer
//! milliseconds ([`crate::ports::Clock`]), which is what the deterministic renderer consumes (ADR-0003).

use crate::ports::Clock;
use kivori_model::ElapsedMs;

/// Monotonic millisecond clock over the ESP32-C3 system timer.
#[derive(Debug, Clone, Copy, Default)]
pub struct EspClock;

impl EspClock {
    /// A clock reading the system timer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Clock for EspClock {
    fn now_ms(&self) -> ElapsedMs {
        let micros = esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_micros();
        // Saturating: the device is not expected to run for ~49 days, but wrapping time would break
        // the monotonicity the render loop relies on.
        u32::try_from(micros / 1000).unwrap_or(u32::MAX)
    }
}
