//! Replayable native preview timeline. Only semantic events cross IPC; Rust owns all pose math.
use kivori_model::{CompanionState, MascotAction, MascotAnimator, MascotPersonality, MascotPose};
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

/// One resolved transient mascot reaction at an absolute preview timestamp.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MascotActionEvent {
    /// Timestamp in integer milliseconds.
    pub at_ms: u32,
    /// Social action token.
    pub action: String,
    /// Temperament used to resolve motion strength.
    pub personality: String,
    /// Deterministic variation seed.
    pub seed: u32,
}

/// Desktop-only event history for exact backward seeking and interrupted transition replay.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTimeline {
    /// State at time zero.
    pub initial_state: String,
    /// Ordered selections; equal timestamps preserve selection order.
    pub events: Vec<AnimationEvent>,
    /// Ordered social cues replayed by the same animator as physical Kivori.
    #[serde(default)]
    pub action_events: Vec<MascotActionEvent>,
}

impl AnimationTimeline {
    /// Rejects unknown states and timestamps out of order.
    pub fn validate(&self) -> Result<(), String> {
        if self.events.len() + self.action_events.len() > 256 {
            return Err("restart preview after 256 timeline changes".into());
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
        last = 0;
        for event in &self.action_events {
            action(&event.action)?;
            personality(&event.personality)?;
            if event.at_ms < last {
                return Err("animation action events must be time-ordered".into());
            }
            last = event.at_ms;
        }
        Ok(())
    }

    /// Resolves through the same bounded controller used by firmware.
    pub fn resolve(&self, elapsed_ms: u32) -> Result<(CompanionState, MascotPose), String> {
        self.validate()?;
        let mut animator = MascotAnimator::new(token(&self.initial_state)?, 0);
        let mut states = self.events.iter().peekable();
        let mut actions = self.action_events.iter().peekable();
        loop {
            let next_state = states.peek().map(|event| event.at_ms);
            let next_action = actions.peek().map(|event| event.at_ms);
            let Some(next_at) = next_state.into_iter().chain(next_action).min() else {
                break;
            };
            if next_at > elapsed_ms {
                break;
            }
            if next_state == Some(next_at) {
                let event = states.next().expect("peeked state event");
                animator.set_state(token(&event.state)?, event.at_ms);
            }
            if next_action == Some(next_at) {
                let event = actions.next().expect("peeked action event");
                animator.trigger_action(
                    action(&event.action)?,
                    personality(&event.personality)?,
                    event.seed,
                    event.at_ms,
                );
            }
        }
        Ok((animator.target(), animator.pose_at(elapsed_ms)))
    }
}

fn action(value: &str) -> Result<MascotAction, String> {
    crate::ipc::dto::mascot_action_from_token(value).ok_or_else(|| "unknown mascot action".into())
}

fn personality(value: &str) -> Result<MascotPersonality, String> {
    crate::ipc::dto::mascot_personality_from_token(value)
        .ok_or_else(|| "unknown mascot personality".into())
}

fn token(state: &str) -> Result<CompanionState, String> {
    crate::ipc::dto::companion_from_token(state).ok_or_else(|| "unknown companion state".into())
}
