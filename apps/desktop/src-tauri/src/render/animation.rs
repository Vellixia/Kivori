//! Replayable native preview timeline. Only semantic events cross IPC; Rust owns all pose math.
use kivori_model::{CompanionState, MascotAnimator, MascotPose};
use serde::Deserialize;

/// One state selection at an absolute preview timestamp.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationEvent {
    /// Timestamp in integer milliseconds.
    pub at_ms: u32,
    /// Existing companion-state token.
    pub state: String,
}

/// Desktop-only event history for exact backward seeking and interrupted transition replay.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTimeline {
    /// State at time zero.
    pub initial_state: String,
    /// Ordered selections; equal timestamps preserve selection order.
    pub events: Vec<AnimationEvent>,
}

impl AnimationTimeline {
    /// Rejects unknown states and timestamps out of order.
    pub fn validate(&self) -> Result<(), String> {
        if self.events.len() > 256 {
            return Err("restart preview after 256 state changes".into());
        }
        token(&self.initial_state)?;
        let mut last = 0;
        for event in &self.events {
            token(&event.state)?;
            if event.at_ms < last {
                return Err("animation events must be time-ordered".into());
            }
            last = event.at_ms;
        }
        Ok(())
    }

    /// Resolves through the same bounded controller used by firmware.
    pub fn resolve(&self, elapsed_ms: u32) -> Result<(CompanionState, MascotPose), String> {
        self.validate()?;
        let mut animator = MascotAnimator::new(token(&self.initial_state)?, 0);
        for event in &self.events {
            if event.at_ms > elapsed_ms {
                break;
            }
            animator.set_state(token(&event.state)?, event.at_ms);
        }
        Ok((animator.target(), animator.pose_at(elapsed_ms)))
    }
}

fn token(state: &str) -> Result<CompanionState, String> {
    crate::ipc::dto::companion_from_token(state).ok_or_else(|| "unknown companion state".into())
}
