//! Companion state axes (data-model §1; constitution Principle VI, FR-012/FR-014/FR-015).
//!
//! There are six semantic [`CompanionState`]s. The desktop may only command the four
//! [`SendableState`]s; `Booting` and `Offline` are device-originated ([`LifecycleState`]) and are not
//! representable as a `SendableState`, so the production send boundary is enforced by the type system.

use serde::{Deserialize, Serialize};

/// A semantic companion state — the visual *meaning*, never pixels. There are exactly six.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompanionState {
    /// Device-originated: shown at power-on, before a desktop session drives the device.
    Booting,
    /// Calm idle presence (desktop-sendable).
    Idle,
    /// Happy reaction (desktop-sendable).
    Happy,
    /// Busy / working (desktop-sendable).
    Busy,
    /// Sleeping / low-key (desktop-sendable).
    Sleeping,
    /// Device-originated: shown when powered but not driven by a desktop session.
    Offline,
}

impl CompanionState {
    /// All six companion states, in declaration order.
    pub const ALL: [CompanionState; 6] = [
        CompanionState::Booting,
        CompanionState::Idle,
        CompanionState::Happy,
        CompanionState::Busy,
        CompanionState::Sleeping,
        CompanionState::Offline,
    ];

    /// Returns `true` if this state is device-originated (`Booting`/`Offline`) and therefore never
    /// sent by the desktop.
    #[must_use]
    pub const fn is_device_originated(self) -> bool {
        matches!(self, CompanionState::Booting | CompanionState::Offline)
    }

    /// Returns the desktop-sendable equivalent, or `None` for device-originated states.
    #[must_use]
    pub const fn as_sendable(self) -> Option<SendableState> {
        match self {
            CompanionState::Idle => Some(SendableState::Idle),
            CompanionState::Happy => Some(SendableState::Happy),
            CompanionState::Busy => Some(SendableState::Busy),
            CompanionState::Sleeping => Some(SendableState::Sleeping),
            CompanionState::Booting | CompanionState::Offline => None,
        }
    }
}

/// A companion state the desktop is allowed to command over the wire (production boundary).
///
/// `Booting` and `Offline` are intentionally **not** representable here (FR-012/FR-014/FR-015), so a
/// `SetState` command can only ever carry a value the device is allowed to be driven into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SendableState {
    /// Calm idle presence. This is the cold-start default desired state (data-model §4).
    #[default]
    Idle,
    /// Happy reaction.
    Happy,
    /// Busy / working.
    Busy,
    /// Sleeping / low-key.
    Sleeping,
}

impl SendableState {
    /// All four sendable states, in declaration order.
    pub const ALL: [SendableState; 4] = [
        SendableState::Idle,
        SendableState::Happy,
        SendableState::Busy,
        SendableState::Sleeping,
    ];

    /// Widens this sendable state to its [`CompanionState`].
    #[must_use]
    pub const fn to_companion(self) -> CompanionState {
        match self {
            SendableState::Idle => CompanionState::Idle,
            SendableState::Happy => CompanionState::Happy,
            SendableState::Busy => CompanionState::Busy,
            SendableState::Sleeping => CompanionState::Sleeping,
        }
    }
}

impl From<SendableState> for CompanionState {
    fn from(s: SendableState) -> Self {
        s.to_companion()
    }
}

/// Error returned when a device-originated [`CompanionState`] is converted to a [`SendableState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotSendable(
    /// The device-originated state that cannot be sent.
    pub CompanionState,
);

impl TryFrom<CompanionState> for SendableState {
    type Error = NotSendable;

    fn try_from(c: CompanionState) -> Result<Self, Self::Error> {
        match c.as_sendable() {
            Some(s) => Ok(s),
            None => Err(NotSendable(c)),
        }
    }
}

/// Device-originated lifecycle states — owned by the device, never sent by the desktop (FR-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleState {
    /// Shown at power-on.
    Booting,
    /// Shown when powered but not driven by a desktop session.
    Offline,
}

impl LifecycleState {
    /// Widens this lifecycle state to its [`CompanionState`].
    #[must_use]
    pub const fn to_companion(self) -> CompanionState {
        match self {
            LifecycleState::Booting => CompanionState::Booting,
            LifecycleState::Offline => CompanionState::Offline,
        }
    }
}
