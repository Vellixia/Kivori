//! The device lifecycle FSM (data-model §1, FR-014).
//!
//! The device owns the `booting`/`offline` axis; the desktop owns the sendable states. This type
//! turns lifecycle events into the single companion state the device displays, and signals when that
//! state changed so the caller can emit a `StateReport`.

use kivori_model::{CompanionState, SendableState};

/// An event that may change the device's displayed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEvent {
    /// Boot / self-test finished; the device is ready to show content.
    BootComplete,
    /// A host session became active.
    LinkUp,
    /// The host session was lost (unplug, timeout, or `Bye`).
    LinkDown,
    /// The host commanded a sendable state.
    SetState(SendableState),
}

/// The device lifecycle state machine. Starts in [`CompanionState::Booting`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceState {
    current: CompanionState,
}

impl DeviceState {
    /// A freshly-powered device, showing the boot scene.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: CompanionState::Booting,
        }
    }

    /// The companion state the device is currently displaying.
    #[must_use]
    pub const fn current(&self) -> CompanionState {
        self.current
    }

    /// Applies `event`. Returns `Some(new_state)` iff the displayed state changed (the caller should
    /// then emit a `StateReport`); `None` if the event left the state unchanged.
    pub fn apply(&mut self, event: DeviceEvent) -> Option<CompanionState> {
        let next = match event {
            // Boot ends into `offline`: no host session exists yet, so nothing drives a sendable
            // state. A later host `SetState` moves it to idle/happy/busy/sleeping.
            DeviceEvent::BootComplete => {
                if matches!(self.current, CompanionState::Booting) {
                    CompanionState::Offline
                } else {
                    self.current
                }
            }
            // Losing the link always drops to the device-owned `offline` state.
            DeviceEvent::LinkDown => CompanionState::Offline,
            // A new link changes nothing on its own; the host follows with a `SetState`.
            DeviceEvent::LinkUp => self.current,
            DeviceEvent::SetState(desired) => desired.to_companion(),
        };
        if next != self.current {
            self.current = next;
            Some(next)
        } else {
            None
        }
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}
