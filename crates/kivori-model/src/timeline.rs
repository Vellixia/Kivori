//! Deterministic, integer-only time primitives (ADR-0003, data-model §5, FR-019, SC-011).
//!
//! The canonical rendering timebase is integer elapsed milliseconds. Device Studio's nominal 30 FPS
//! step is *derived* from an integer step index — never accumulated in floating point — so stepping
//! has no cumulative drift.

use serde::{Deserialize, Serialize};

/// Canonical elapsed time, in integer milliseconds.
pub type ElapsedMs = u32;

/// Nominal Device Studio preview rate, in frames per second.
pub const PREVIEW_FPS: u32 = 30;

/// Converts an integer step index (at the nominal 30 FPS preview rate) to canonical elapsed
/// milliseconds using `timestamp_ms(n) = (n * 1000 + 15) / 30` (round-to-nearest via `+15 = 30/2`).
///
/// The result is a pure function of `n`, so repeated stepping never accumulates floating-point drift
/// (every 30 steps advances exactly 1000 ms). Negative indices clamp to `0`; the computation is done
/// in `i128` and saturates at [`u32::MAX`].
#[must_use]
pub const fn frame_step_ms(step_index: i64) -> ElapsedMs {
    if step_index <= 0 {
        return 0;
    }
    let ms = (step_index as i128 * 1000 + 15) / PREVIEW_FPS as i128;
    if ms > u32::MAX as i128 {
        u32::MAX
    } else {
        ms as u32
    }
}

/// A per-scene animation frame rate as an exact integer ratio (`num / den` fps). No floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    /// Numerator (frames).
    pub num: u16,
    /// Denominator (seconds).
    pub den: u16,
}

impl FrameRate {
    /// Creates a frame rate from an explicit `num / den` ratio.
    #[must_use]
    pub const fn new(num: u16, den: u16) -> Self {
        Self { num, den }
    }

    /// A whole-number frame rate (`fps / 1`).
    #[must_use]
    pub const fn fps(fps: u16) -> Self {
        Self { num: fps, den: 1 }
    }

    /// Returns `true` if the rate is valid (both parts non-zero).
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.num != 0 && self.den != 0
    }
}

/// Selects the active frame of a looping animation at a given canonical elapsed time.
///
/// Returns `0` for a static scene (`frame_count <= 1`) or an invalid rate. Otherwise the frame index
/// advances as `floor(elapsed_ms * num / (1000 * den))` and wraps modulo `frame_count`, so loops and
/// boundaries are handled deterministically with integer math only.
#[must_use]
pub const fn scene_frame(elapsed_ms: ElapsedMs, rate: FrameRate, frame_count: u16) -> u16 {
    if frame_count <= 1 || rate.num == 0 || rate.den == 0 {
        return 0;
    }
    let numerator = elapsed_ms as u64 * rate.num as u64;
    let denominator = 1000u64 * rate.den as u64;
    let index = numerator / denominator;
    (index % frame_count as u64) as u16
}

/// The Device Studio timeline: an integer step index plus a play flag.
///
/// This models only the drift-free integer stepping. Wall-clock "play" advancement (mapping real
/// elapsed time to steps) is layered in the desktop Device Studio; this type stays `no_std` and
/// deterministic. The frontend store mirrors `step_index` and obtains frames from the shared renderer
/// via IPC (constitution Principle II).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StudioTimeline {
    step_index: i64,
    playing: bool,
}

impl StudioTimeline {
    /// A timeline at step `0`, paused.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            step_index: 0,
            playing: false,
        }
    }

    /// The current step index.
    #[must_use]
    pub const fn step_index(self) -> i64 {
        self.step_index
    }

    /// Whether the preview is playing.
    #[must_use]
    pub const fn is_playing(self) -> bool {
        self.playing
    }

    /// The canonical elapsed time for the current step.
    #[must_use]
    pub const fn elapsed_ms(self) -> ElapsedMs {
        frame_step_ms(self.step_index)
    }

    /// Advances one step (saturating at [`i64::MAX`]).
    pub fn step_forward(&mut self) {
        self.step_index = self.step_index.saturating_add(1);
    }

    /// Retreats one step, clamped at `0`.
    pub fn step_back(&mut self) {
        if self.step_index > 0 {
            self.step_index -= 1;
        }
    }

    /// Seeks to an absolute step index (negative values read as elapsed time `0`).
    pub fn seek(&mut self, step_index: i64) {
        self.step_index = step_index;
    }

    /// Sets the play flag.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }
}
