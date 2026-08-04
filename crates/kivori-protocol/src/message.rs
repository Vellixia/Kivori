//! Wire message set (contracts/protocol.md §3). The top-level [`Message`] is an **append-only** enum;
//! the `postcard` variant index is the wire tag.

use crate::error::{ByeReason, ErrorCategory};
use kivori_model::{Capabilities, CompanionState, SendableState};
use serde::{Deserialize, Serialize};

/// A handshake nonce the device must echo to prove liveness/identity.
pub type Nonce = u32;

/// An opaque device identity. Never logged raw — hashed before it reaches any log field.
pub type DeviceId = [u8; 16];

/// A firmware or desktop application version (`major.minor.patch`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FirmwareVersion {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Patch version.
    pub patch: u16,
}

/// `Hello` — desktop → device: opens the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Desktop application version.
    pub desktop_version: FirmwareVersion,
    /// Capabilities the desktop advertises.
    pub desktop_caps: Capabilities,
    /// A fresh nonce the device must echo.
    pub nonce: Nonce,
}

/// `HelloAck` — device → desktop: answers the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAck {
    /// Capabilities the device advertises.
    pub device_caps: Capabilities,
    /// Opaque device identity.
    pub device_id: DeviceId,
    /// Device firmware version.
    pub firmware_version: FirmwareVersion,
    /// Echo of the `Hello` nonce.
    pub nonce_echo: Nonce,
}

/// `Ready` — desktop → device: confirms the negotiated session parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ready {
    /// Negotiated minor version (`min` of both peers).
    pub negotiated_minor: u16,
    /// Negotiated capability set (intersection of both peers).
    pub negotiated_caps: Capabilities,
}

/// `Bye` — either side: closes the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bye {
    /// Why the session is closing.
    pub reason: ByeReason,
}

/// `SetState` — desktop → device: command a sendable companion state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetState {
    /// The desired sendable state (production boundary — never `booting`/`offline`).
    pub desired: SendableState,
    /// Optional canonical timeline position (ms) at which to apply the state.
    pub at_ms: Option<u32>,
}

/// `StateReport` — device → desktop: the device's current companion state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateReport {
    /// The state the device is currently showing (any of the six).
    pub reported: CompanionState,
    /// The device's current canonical elapsed time (ms).
    pub elapsed_ms: u32,
}

/// `Ping` — desktop → device: heartbeat request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    /// Desktop timestamp (ms), echoed in the `Pong`.
    pub t_ms: u32,
}

/// `Pong` — device → desktop: heartbeat reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    /// Echo of the `Ping` timestamp.
    pub t_ms_echo: u32,
    /// Device uptime (ms).
    pub uptime_ms: u32,
}

/// `Health` — device → desktop: safe device health diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    /// Free heap/SRAM in bytes (a safe diagnostic).
    pub free_bytes: u32,
}

/// `Diagnostic` — device → desktop: a safe, categorized diagnostic (no payload contents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Diagnostic category.
    pub category: ErrorCategory,
    /// A category-specific code.
    pub code: u16,
}

/// `Error` — either side: a categorized protocol error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorReport {
    /// Error category.
    pub category: ErrorCategory,
    /// A category-specific code.
    pub code: u16,
}

/// The top-level wire message.
///
/// **Append-only**: new variants are added at the end within a major version — the `postcard`
/// variant index is the wire tag, so reordering or removing variants is a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    /// Desktop → device: opens the handshake.
    Hello(Hello),
    /// Device → desktop: answers the handshake.
    HelloAck(HelloAck),
    /// Desktop → device: confirms the negotiated session.
    Ready(Ready),
    /// Either side: closes the session.
    Bye(Bye),
    /// Desktop → device: command a companion state.
    SetState(SetState),
    /// Device → desktop: reports the current companion state.
    StateReport(StateReport),
    /// Desktop → device: heartbeat request.
    Ping(Ping),
    /// Device → desktop: heartbeat reply.
    Pong(Pong),
    /// Device → desktop: health report.
    Health(Health),
    /// Device → desktop: a safe diagnostic.
    Diagnostic(Diagnostic),
    /// Either side: a categorized protocol error.
    Error(ErrorReport),
}
