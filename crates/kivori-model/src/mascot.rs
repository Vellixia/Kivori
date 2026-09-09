//! Deterministic mascot motion and state-transition resolution.
//!
//! The controller is deliberately small and allocation-free so desktop preview and firmware can
//! resolve the same pose from the same integer timestamp. It owns no sprites or rendering state.

use crate::{CompanionState, ElapsedMs, Point};

/// Standard duration for changing between expressive mascot states.
pub const MASCOT_TRANSITION_MS: u32 = 350;
/// Duration for entering the slower sleeping state.
pub const MASCOT_SLEEP_TRANSITION_MS: u32 = 600;
/// Fixed-point identity scale (`1.0`) in Q8 units.
pub const SCALE_Q8_ONE: u16 = 256;
/// Bottom-centre pivot shared by the compiled body and face layers.
pub const MASCOT_ANCHOR: Point = Point::new(120, 208);

/// A fully-resolved mascot pose suitable for direct composition.
///
/// Body coordinates use Q8 pixels to keep eased motion continuous until the renderer converts them
/// to output pixels. `state_weights` are indexed by [`CompanionState::index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MascotPose {
    /// Translation from the canonical artwork position, in Q8 pixels.
    pub body_offset_q8: (i32, i32),
    /// Uniform body scale in Q8 units.
    pub body_scale_q8: u16,
    /// Whole-mascot opacity (`0..=255`).
    pub opacity: u8,
    /// Vertical eye scale in Q8 units; blinking squashes this value.
    pub eyes_scale_y_q8: u16,
    /// Facial-expression blend weights, in companion-state declaration order.
    pub state_weights: [u8; 6],
}

impl MascotPose {
    /// The neutral opaque pose with one fully selected face expression.
    #[must_use]
    pub const fn for_state(state: CompanionState) -> Self {
        let mut state_weights = [0; 6];
        state_weights[state.index()] = u8::MAX;
        Self {
            body_offset_q8: (0, 0),
            body_scale_q8: SCALE_Q8_ONE,
            opacity: u8::MAX,
            eyes_scale_y_q8: SCALE_Q8_ONE,
            state_weights,
        }
    }
}

/// Stateful controller which preserves the resolved pose when its target state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MascotAnimator {
    target: CompanionState,
    transition_started_at: ElapsedMs,
    transition_duration_ms: u32,
    from: MascotPose,
}

// Hard firmware budget: the controller and resolved pose never become framebuffer-sized.
const _: () = assert!(
    core::mem::size_of::<MascotAnimator>() + core::mem::size_of::<MascotPose>() <= 8 * 1024
);

impl MascotAnimator {
    /// Starts an animator already settled in `state` at `now_ms`.
    #[must_use]
    pub const fn new(state: CompanionState, now_ms: ElapsedMs) -> Self {
        Self {
            target: state,
            transition_started_at: now_ms,
            transition_duration_ms: 0,
            from: MascotPose::for_state(state),
        }
    }

    /// The state toward which this animator is moving.
    #[must_use]
    pub const fn target(&self) -> CompanionState {
        self.target
    }

    /// Retargets the animator while retaining its exact currently-resolved pose.
    pub fn set_state(&mut self, state: CompanionState, now_ms: ElapsedMs) {
        if state == self.target {
            return;
        }
        let current = self.pose_at(now_ms);
        self.target = state;
        self.transition_started_at = now_ms;
        self.transition_duration_ms = if state == CompanionState::Sleeping {
            MASCOT_SLEEP_TRANSITION_MS
        } else {
            MASCOT_TRANSITION_MS
        };
        self.from = current;
    }

    /// Resolves the target loop and any in-flight eased transition at `now_ms`.
    #[must_use]
    pub fn pose_at(&self, now_ms: ElapsedMs) -> MascotPose {
        let target = target_pose(
            self.target,
            now_ms,
            now_ms.saturating_sub(self.transition_started_at),
        );
        if self.transition_duration_ms == 0 {
            return target;
        }
        let elapsed = now_ms.saturating_sub(self.transition_started_at);
        if elapsed >= self.transition_duration_ms {
            return target;
        }
        let mut pose = blend_pose(
            self.from,
            target,
            smoothstep_q8(elapsed, self.transition_duration_ms),
        );
        pose.eyes_scale_y_q8 = pose
            .eyes_scale_y_q8
            .min(transition_blink_scale(elapsed, self.transition_duration_ms));
        pose
    }
}

impl CompanionState {
    /// Stable zero-based index used by mascot expression weights.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Booting => 0,
            Self::Idle => 1,
            Self::Happy => 2,
            Self::Busy => 3,
            Self::Sleeping => 4,
            Self::Offline => 5,
        }
    }
}

fn target_pose(state: CompanionState, now_ms: ElapsedMs, state_age_ms: ElapsedMs) -> MascotPose {
    let mut pose = MascotPose::for_state(state);
    let (x, y, scale) = match state {
        CompanionState::Booting => (
            0,
            0,
            248 + (u32::from(smoothstep_q8(state_age_ms.min(800), 800)) * 8 / 256) as u16,
        ),
        CompanionState::Idle => (0, 0, SCALE_Q8_ONE),
        CompanionState::Happy => {
            let bounce = positive_wave(now_ms, 900, 1_536);
            let scale = positive_wave(now_ms, 900, 4);
            (0, -bounce, (SCALE_Q8_ONE as i32 + scale) as u16)
        }
        CompanionState::Busy => {
            let sway = centred_wave(now_ms, 1_600, 768);
            (sway, 0, SCALE_Q8_ONE)
        }
        CompanionState::Sleeping => (0, 0, SCALE_Q8_ONE),
        CompanionState::Offline => (0, i32::from(SCALE_Q8_ONE), SCALE_Q8_ONE),
    };
    pose.body_offset_q8 = (x, y);
    pose.body_scale_q8 = scale;
    pose.eyes_scale_y_q8 = match state {
        CompanionState::Sleeping => SCALE_Q8_ONE,
        CompanionState::Booting => {
            let open = smoothstep_q8(state_age_ms.min(300), 300);
            48 + (u32::from(SCALE_Q8_ONE - 48) * u32::from(open) / 256) as u16
        }
        CompanionState::Idle
        | CompanionState::Happy
        | CompanionState::Busy
        | CompanionState::Offline => blink_scale(now_ms),
    };
    pose
}

fn centred_wave(now_ms: ElapsedMs, period_ms: u32, amplitude: i32) -> i32 {
    let phase = now_ms % period_ms;
    let half = period_ms / 2;
    if phase <= half {
        -amplitude + (i32::from(smoothstep_q8(phase, half)) * amplitude * 2 / 256)
    } else {
        amplitude - (i32::from(smoothstep_q8(phase - half, half)) * amplitude * 2 / 256)
    }
}

fn positive_wave(now_ms: ElapsedMs, period_ms: u32, amplitude: i32) -> i32 {
    let phase = now_ms % period_ms;
    let half = period_ms / 2;
    if phase <= half {
        i32::from(smoothstep_q8(phase, half)) * amplitude / 256
    } else {
        i32::from(smoothstep_q8(period_ms - phase, half)) * amplitude / 256
    }
}

fn blink_scale(now_ms: ElapsedMs) -> u16 {
    let phase = now_ms % 4_000;
    let close = if (3_560..=3_640).contains(&phase) {
        (phase - 3_560) * 2
    } else if (3_640..=3_720).contains(&phase) {
        (3_720 - phase) * 2
    } else {
        0
    };
    SCALE_Q8_ONE - close.min(208) as u16
}

fn transition_blink_scale(elapsed_ms: ElapsedMs, duration_ms: ElapsedMs) -> u16 {
    let midpoint = duration_ms / 2;
    let distance = elapsed_ms.abs_diff(midpoint);
    let half_window = (duration_ms / 4).max(1);
    if distance >= half_window {
        SCALE_Q8_ONE
    } else {
        48 + (u32::from(SCALE_Q8_ONE - 48) * distance / half_window) as u16
    }
}

fn smoothstep_q8(elapsed_ms: u32, duration_ms: u32) -> u16 {
    let x = (u64::from(elapsed_ms) * 256 / u64::from(duration_ms)) as u32;
    let x = x.min(256);
    ((3 * x * x * 256 - 2 * x * x * x) / (256 * 256)) as u16
}

fn lerp_i32(from: i32, to: i32, progress_q8: u16) -> i32 {
    from + ((i64::from(to - from) * i64::from(progress_q8)) / 256) as i32
}

fn lerp_u16(from: u16, to: u16, progress_q8: u16) -> u16 {
    lerp_i32(i32::from(from), i32::from(to), progress_q8) as u16
}

fn lerp_u8(from: u8, to: u8, progress_q8: u16) -> u8 {
    lerp_i32(i32::from(from), i32::from(to), progress_q8) as u8
}

fn blend_pose(from: MascotPose, to: MascotPose, progress_q8: u16) -> MascotPose {
    let state_weights = if progress_q8 < SCALE_Q8_ONE / 2 {
        from.state_weights
    } else {
        to.state_weights
    };
    MascotPose {
        body_offset_q8: (
            lerp_i32(from.body_offset_q8.0, to.body_offset_q8.0, progress_q8),
            lerp_i32(from.body_offset_q8.1, to.body_offset_q8.1, progress_q8),
        ),
        body_scale_q8: lerp_u16(from.body_scale_q8, to.body_scale_q8, progress_q8),
        opacity: lerp_u8(from.opacity, to.opacity, progress_q8),
        eyes_scale_y_q8: lerp_u16(from.eyes_scale_y_q8, to.eyes_scale_y_q8, progress_q8),
        state_weights,
    }
}
