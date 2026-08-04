//! The desktop orchestrator's desired-state ownership (data-model §4; clarified).
//!
//! The orchestrator is the single source of truth for the state the desktop wants the device to show.
//! It defaults to `Idle`, resets to `Idle` on every cold start (no persistence), and is re-transmitted
//! to the device on each (re)connect (FR-009 within-process restoration).

use kivori_model::SendableState;

/// Owns the desktop's desired companion state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orchestrator {
    desired: SendableState,
}

impl Orchestrator {
    /// A cold-start orchestrator: desired state is `Idle` (no persistence across launches).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            desired: SendableState::Idle,
        }
    }

    /// The current desired state.
    #[must_use]
    pub const fn desired(&self) -> SendableState {
        self.desired
    }

    /// Sets the desired state (the production path — a future integration calls the same method).
    /// Returns the new desired state.
    pub fn set_desired(&mut self, state: SendableState) -> SendableState {
        self.desired = state;
        self.desired
    }

    /// The state to (re)transmit to the device on (re)connect to resynchronize it (FR-009).
    #[must_use]
    pub const fn resync_state(&self) -> SendableState {
        self.desired
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}
