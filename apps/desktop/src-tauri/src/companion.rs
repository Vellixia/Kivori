//! Desktop-owned mascot behavior policy: personality settings and deterministic self-play timing.

use kivori_model::{CompanionState, MascotAction, MascotPersonality};
use kivori_protocol::PlayMascotAction;

/// Schedules ambient social actions while leaving rendering and state semantics to shared/device code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionDirector {
    personality: MascotPersonality,
    self_play: bool,
    random_state: u32,
    next_due_ms: u32,
}

impl CompanionDirector {
    /// Creates a director and schedules its first ambient action after `now_ms`.
    #[must_use]
    pub fn new(personality: MascotPersonality, self_play: bool, seed: u32, now_ms: u32) -> Self {
        let mut director = Self {
            personality,
            self_play,
            random_state: seed,
            next_due_ms: now_ms,
        };
        director.schedule_after(now_ms);
        director
    }

    /// Current user-selected personality.
    #[must_use]
    pub const fn personality(&self) -> MascotPersonality {
        self.personality
    }

    /// Whether autonomous social actions are enabled.
    #[must_use]
    pub const fn self_play(&self) -> bool {
        self.self_play
    }

    /// Next scheduled device-runtime millisecond, exposed for deterministic orchestration/tests.
    #[must_use]
    pub const fn next_due_ms(&self) -> u32 {
        self.next_due_ms
    }

    /// Changes personality and starts a fresh cadence window.
    pub fn set_personality(&mut self, personality: MascotPersonality, now_ms: u32) {
        self.personality = personality;
        self.schedule_after(now_ms);
    }

    /// Enables or disables ambient actions and starts a fresh cadence window.
    pub fn set_self_play(&mut self, enabled: bool, now_ms: u32) {
        self.self_play = enabled;
        self.schedule_after(now_ms);
    }

    /// Returns one due autonomous action. Suppressed states consume and reschedule the opportunity.
    pub fn poll(&mut self, now_ms: u32, state: CompanionState) -> Option<PlayMascotAction> {
        if !self.self_play || now_ms < self.next_due_ms {
            return None;
        }
        if matches!(
            state,
            CompanionState::Booting
                | CompanionState::Busy
                | CompanionState::Sleeping
                | CompanionState::Offline
        ) {
            self.schedule_after(now_ms);
            return None;
        }
        let action = match self.next_random() % 5 {
            0 => MascotAction::Greet,
            1 => MascotAction::Pet,
            2 => MascotAction::Tickle,
            3 => MascotAction::Surprise,
            _ => MascotAction::Comfort,
        };
        let cue = self.cue(action);
        self.schedule_after(now_ms);
        Some(cue)
    }

    /// Builds one immediate user-requested action using current personality and a fresh seed.
    pub fn manual(&mut self, action: MascotAction) -> PlayMascotAction {
        self.cue(action)
    }

    fn cue(&mut self, action: MascotAction) -> PlayMascotAction {
        PlayMascotAction {
            action,
            personality: self.personality,
            seed: self.next_random(),
        }
    }

    fn schedule_after(&mut self, now_ms: u32) {
        let (minimum, maximum) = match self.personality {
            MascotPersonality::Cozy => (8_000, 16_000),
            MascotPersonality::Playful => (4_000, 8_000),
            MascotPersonality::Calm => (18_000, 30_000),
        };
        let span = maximum - minimum;
        let delay = minimum + self.next_random() % (span + 1);
        self.next_due_ms = now_ms.saturating_add(delay);
    }

    fn next_random(&mut self) -> u32 {
        self.random_state = self
            .random_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.random_state
    }
}
