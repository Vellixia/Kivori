//! Bounded reconnect backoff (FR-008, FR-010).
//!
//! Backoff is exponential from 250 ms, capped at 5 s, with symmetric jitter to avoid thundering-herd
//! reconnection. The schedule is a pure function of the attempt index (jitter is injected by the
//! caller), so tests are deterministic. Within-process desired-state restoration on reconnect (FR-009)
//! is owned by [`crate::orchestrator::Orchestrator`].

use core::time::Duration;

/// Base (first) backoff delay, in milliseconds.
pub const BASE_MS: u64 = 250;
/// Maximum backoff delay, in milliseconds.
pub const MAX_MS: u64 = 5_000;

/// The un-jittered backoff delay for a zero-based `attempt`: `250ms * 2^attempt`, capped at 5 s.
///
/// `attempt >= 5` is already at the cap (`250 * 2^5 = 8000 > 5000`), so it returns [`MAX_MS`] directly
/// — no shift overflow for large counts.
#[must_use]
pub const fn base_delay_ms(attempt: u32) -> u64 {
    if attempt >= 5 {
        MAX_MS
    } else {
        BASE_MS << attempt
    }
}

/// Applies symmetric ±25% jitter to `base_ms`. `jitter_permille` in `0..=1000` maps linearly onto
/// `[-25%, +25%]` (500 = no change), so the caller injects randomness while the math stays testable.
#[must_use]
pub const fn with_jitter(base_ms: u64, jitter_permille: u16) -> u64 {
    let permille = if jitter_permille > 1000 {
        1000
    } else {
        jitter_permille as u64
    };
    let span = (base_ms / 4) as i64; // 25% of the base delay
    let offset = span * (permille as i64 - 500) / 500; // in [-span, +span]
    let jittered = base_ms as i64 + offset;
    if jittered < 0 {
        0
    } else {
        jittered as u64
    }
}

/// The reconnect backoff sequencer. Tracks the attempt index; reset on a successful connect.
#[derive(Debug, Clone, Copy, Default)]
pub struct Backoff {
    attempt: u32,
}

impl Backoff {
    /// A backoff at the first attempt.
    #[must_use]
    pub const fn new() -> Self {
        Self { attempt: 0 }
    }

    /// The current attempt index (zero-based).
    #[must_use]
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns the (un-jittered) delay for the current attempt and advances to the next.
    pub fn next_delay(&mut self) -> Duration {
        let ms = base_delay_ms(self.attempt);
        self.attempt = self.attempt.saturating_add(1);
        Duration::from_millis(ms)
    }

    /// Resets to the first attempt (called on a successful connect).
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}
