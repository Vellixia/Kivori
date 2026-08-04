//! Heartbeat liveness policy (FR-007).
//!
//! The run loop sends a `Ping` each interval and calls [`HeartbeatMonitor::on_ping_sent`]; each
//! `Pong` resets the miss counter. When the unanswered count reaches the threshold, the connection is
//! declared timed out. Pure and timer-free — the run loop owns the clock.

/// Default number of consecutive missed replies before a heartbeat timeout.
pub const DEFAULT_MISS_THRESHOLD: u32 = 3;

/// Tracks consecutive unanswered heartbeats.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatMonitor {
    misses: u32,
    threshold: u32,
}

impl HeartbeatMonitor {
    /// A monitor with the given consecutive-miss threshold (clamped to at least 1).
    #[must_use]
    pub const fn new(threshold: u32) -> Self {
        Self {
            misses: 0,
            threshold: if threshold == 0 { 1 } else { threshold },
        }
    }

    /// Records that a `Ping` was sent (a not-yet-answered heartbeat).
    pub fn on_ping_sent(&mut self) {
        self.misses = self.misses.saturating_add(1);
    }

    /// Records a `Pong`, clearing the miss counter.
    pub fn on_pong(&mut self) {
        self.misses = 0;
    }

    /// The current consecutive-miss count.
    #[must_use]
    pub const fn misses(&self) -> u32 {
        self.misses
    }

    /// Whether the miss count has reached the threshold (the caller then raises `HeartbeatTimeout`).
    #[must_use]
    pub const fn timed_out(&self) -> bool {
        self.misses >= self.threshold
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new(DEFAULT_MISS_THRESHOLD)
    }
}
