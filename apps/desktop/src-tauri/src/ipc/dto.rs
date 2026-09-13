//! IPC DTOs and pure projections (contracts/ipc.md §4).
//!
//! These are the ONLY shapes that cross to the webview. They carry no raw device id, payload bytes,
//! serial handles, or paths. All enum tokens are the lowercase wire strings the frontend expects; the
//! projections are pure functions of the internal state, so they are unit-testable without Tauri.

use kivori_model::{
    CompanionState, ConnectionState, MascotAction, MascotPersonality, ProtocolVersion,
    SendableState,
};
use kivori_protocol::{MascotActionApplied, PROTOCOL_MAJOR, PROTOCOL_MINOR};
use serde::Serialize;

use crate::activity::{
    ActivityEvent, ActivityEventKind, ActivityMetadata, ActivityOutcome, ActivitySeverity,
    ActivitySource,
};
use crate::device::fsm::ConnectionManager;
use crate::orchestrator::Orchestrator;

/// Lowercase wire token for a connection state (ipc.md §4).
#[must_use]
pub fn connection_token(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Connecting => "connecting",
        ConnectionState::Connected => "connected",
        ConnectionState::Incompatible => "incompatible",
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Error => "error",
    }
}

/// Lowercase wire token for a companion state.
#[must_use]
pub fn companion_token(state: CompanionState) -> &'static str {
    match state {
        CompanionState::Booting => "booting",
        CompanionState::Idle => "idle",
        CompanionState::Happy => "happy",
        CompanionState::Busy => "busy",
        CompanionState::Sleeping => "sleeping",
        CompanionState::Offline => "offline",
    }
}

#[must_use]
pub fn mascot_action_token(action: MascotAction) -> &'static str {
    match action {
        MascotAction::Greet => "greet",
        MascotAction::Pet => "pet",
        MascotAction::Tickle => "tickle",
        MascotAction::Surprise => "surprise",
        MascotAction::Comfort => "comfort",
    }
}

#[must_use]
pub fn mascot_personality_token(personality: MascotPersonality) -> &'static str {
    match personality {
        MascotPersonality::Cozy => "cozy",
        MascotPersonality::Playful => "playful",
        MascotPersonality::Calm => "calm",
    }
}

/// Lowercase wire token for a sendable state.
#[must_use]
pub fn sendable_token(state: SendableState) -> &'static str {
    match state {
        SendableState::Idle => "idle",
        SendableState::Happy => "happy",
        SendableState::Busy => "busy",
        SendableState::Sleeping => "sleeping",
    }
}

/// `major.minor` protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProtocolVersionDto {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
}

impl From<ProtocolVersion> for ProtocolVersionDto {
    fn from(v: ProtocolVersion) -> Self {
        Self {
            major: v.major,
            minor: v.minor,
        }
    }
}

/// Application/build info (all builds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoDto {
    /// Desktop application version.
    pub app_version: String,
    /// Desktop protocol version.
    pub protocol_version: ProtocolVersionDto,
    /// Protocol major versions the desktop supports.
    pub supported_majors: Vec<u16>,
    /// Whether Device Studio is present in this build (false in release).
    pub device_studio_enabled: bool,
}

/// Safe connected-device summary (never the raw identity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoDto {
    /// Device firmware version (`major.minor.patch`).
    pub firmware_version: String,
    /// Device protocol version.
    pub protocol_version: ProtocolVersionDto,
    /// Short, non-reversible hash of the device identity.
    pub device_id_hash_short: String,
}

/// The three-axis connection snapshot surfaced to the UI (ipc.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusDto {
    /// Connection lifecycle state.
    pub connection: String,
    /// The desktop's desired sendable state (always present; default `idle`).
    pub desired: String,
    /// The device's reported state; `null` until the first `StateReport`.
    pub reported: Option<String>,
    /// Connected-device summary; present only when connected.
    pub device: Option<DeviceInfoDto>,
    /// Human-readable reason when `connection == "incompatible"`.
    pub incompatible_reason: Option<String>,
    /// Consecutive reconnect attempts.
    pub retry_count: u32,
    /// Within-process port-session identity. Changes whenever device uptime may reset.
    pub connection_generation: u32,
    /// Whether current device session supports transient mascot interactions.
    pub mascot_interaction: bool,
    /// Most recent correlated device acknowledgment for a social action in this session.
    pub mascot_action: Option<MascotActionAppliedDto>,
}

/// Acknowledged physical action cue, safe to replay in Device Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MascotActionAppliedDto {
    pub action: String,
    pub personality: String,
    pub seed: u32,
    pub applied_at_ms: u32,
}

/// One typed session activity event safe to send to the webview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEventDto {
    /// Process-monotonic event identifier.
    pub id: u64,
    /// Native-generated ISO-8601 timestamp.
    pub at: String,
    /// Closed event-kind token.
    #[serde(rename = "type")]
    pub event_type: ActivityEventTypeDto,
    /// Native-generated human-readable summary.
    pub summary: String,
    /// Closed severity token.
    pub severity: ActivitySeverityDto,
    /// Closed source token.
    pub source: ActivitySourceDto,
    /// Closed outcome token.
    pub outcome: ActivityOutcomeDto,
    /// Optional fixed-shape, allowlisted event details.
    pub metadata: Option<ActivityMetadataDto>,
}

/// Fixed-shape activity metadata; there is deliberately no arbitrary details map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityMetadataDto {
    /// Connection state for a lifecycle transition.
    pub connection: ActivityConnectionStateDto,
    /// Consecutive reconnect attempts.
    pub retry_count: u32,
    /// Monotonic elapsed-ms marker.
    pub elapsed_ms: u32,
}

/// Closed activity-event type token serialized to the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivityEventTypeDto {
    /// A device connection lifecycle state changed.
    ConnectionStateChanged,
    ActionRequested,
    ActionCompleted,
    ActionFailed,
    DeviceDiscovered,
    DeviceRejected,
    ProtocolMessageRejected,
    ProtocolFailed,
    FirmwareUpdateStarted,
    FirmwareUpdateCompleted,
    FirmwareUpdateFailed,
}

/// Closed connection-state token serialized in activity metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivityConnectionStateDto {
    Connecting,
    Connected,
    Incompatible,
    Disconnected,
    Error,
}

/// Closed severity token serialized to the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivitySeverityDto {
    Info,
    Warning,
    Error,
}

/// Closed source token serialized to the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivitySourceDto {
    Connection,
    Action,
    Device,
    Protocol,
    Firmware,
}

/// Closed outcome token serialized to the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityOutcomeDto {
    Observed,
    Started,
    Succeeded,
    Failed,
    Rejected,
}

/// Projects application info. `device_studio_enabled` reflects the compiled-in Device Studio feature.
#[must_use]
pub fn app_info(device_studio_enabled: bool) -> AppInfoDto {
    AppInfoDto {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR).into(),
        supported_majors: vec![PROTOCOL_MAJOR],
        device_studio_enabled,
    }
}

/// Projects the connection snapshot from the manager, orchestrator, and last reported state.
#[must_use]
pub fn connection_status(
    manager: &ConnectionManager,
    orchestrator: &Orchestrator,
    reported: Option<CompanionState>,
    mascot_interaction: bool,
    mascot_action: Option<MascotActionApplied>,
    connection_generation: u32,
) -> ConnectionStatusDto {
    let device = manager.device().map(|d| DeviceInfoDto {
        firmware_version: format!(
            "{}.{}.{}",
            d.firmware_version.major, d.firmware_version.minor, d.firmware_version.patch
        ),
        protocol_version: d.protocol_version.into(),
        device_id_hash_short: d.device_id_hash_short.clone(),
    });
    ConnectionStatusDto {
        connection: connection_token(manager.state()).to_string(),
        desired: sendable_token(orchestrator.desired()).to_string(),
        reported: reported.map(|r| companion_token(r).to_string()),
        device,
        incompatible_reason: manager.incompatible_reason().map(str::to_string),
        retry_count: manager.retry_count(),
        connection_generation,
        mascot_interaction: manager.state().can_drive_device() && mascot_interaction,
        mascot_action: mascot_action.map(mascot_action_applied),
    }
}

#[must_use]
pub fn mascot_action_applied(value: MascotActionApplied) -> MascotActionAppliedDto {
    MascotActionAppliedDto {
        action: mascot_action_token(value.action).to_string(),
        personality: mascot_personality_token(value.personality).to_string(),
        seed: value.seed,
        applied_at_ms: value.applied_at_ms,
    }
}

/// Projects a typed activity record to its wire DTO.
#[must_use]
pub fn activity_event(event: &ActivityEvent) -> ActivityEventDto {
    ActivityEventDto {
        id: event.id(),
        at: event.at().to_string(),
        event_type: activity_kind_token(event.kind()),
        summary: event.summary().to_string(),
        severity: activity_severity(event.severity()),
        source: activity_source(event.source()),
        outcome: activity_outcome(event.outcome()),
        metadata: event.metadata().map(activity_metadata),
    }
}

fn activity_kind_token(kind: ActivityEventKind) -> ActivityEventTypeDto {
    match kind {
        ActivityEventKind::ConnectionStateChanged => ActivityEventTypeDto::ConnectionStateChanged,
        ActivityEventKind::ActionRequested => ActivityEventTypeDto::ActionRequested,
        ActivityEventKind::ActionCompleted => ActivityEventTypeDto::ActionCompleted,
        ActivityEventKind::ActionFailed => ActivityEventTypeDto::ActionFailed,
        ActivityEventKind::DeviceDiscovered => ActivityEventTypeDto::DeviceDiscovered,
        ActivityEventKind::DeviceRejected => ActivityEventTypeDto::DeviceRejected,
        ActivityEventKind::ProtocolMessageRejected => ActivityEventTypeDto::ProtocolMessageRejected,
        ActivityEventKind::ProtocolFailed => ActivityEventTypeDto::ProtocolFailed,
        ActivityEventKind::FirmwareUpdateStarted => ActivityEventTypeDto::FirmwareUpdateStarted,
        ActivityEventKind::FirmwareUpdateCompleted => ActivityEventTypeDto::FirmwareUpdateCompleted,
        ActivityEventKind::FirmwareUpdateFailed => ActivityEventTypeDto::FirmwareUpdateFailed,
    }
}

fn activity_severity(severity: ActivitySeverity) -> ActivitySeverityDto {
    match severity {
        ActivitySeverity::Info => ActivitySeverityDto::Info,
        ActivitySeverity::Warning => ActivitySeverityDto::Warning,
        ActivitySeverity::Error => ActivitySeverityDto::Error,
    }
}

fn activity_source(source: ActivitySource) -> ActivitySourceDto {
    match source {
        ActivitySource::Connection => ActivitySourceDto::Connection,
        ActivitySource::Action => ActivitySourceDto::Action,
        ActivitySource::Device => ActivitySourceDto::Device,
        ActivitySource::Protocol => ActivitySourceDto::Protocol,
        ActivitySource::Firmware => ActivitySourceDto::Firmware,
    }
}

fn activity_outcome(outcome: ActivityOutcome) -> ActivityOutcomeDto {
    match outcome {
        ActivityOutcome::Observed => ActivityOutcomeDto::Observed,
        ActivityOutcome::Started => ActivityOutcomeDto::Started,
        ActivityOutcome::Succeeded => ActivityOutcomeDto::Succeeded,
        ActivityOutcome::Failed => ActivityOutcomeDto::Failed,
        ActivityOutcome::Rejected => ActivityOutcomeDto::Rejected,
    }
}

fn activity_metadata(metadata: &ActivityMetadata) -> ActivityMetadataDto {
    match metadata {
        ActivityMetadata::Connection {
            state,
            retry_count,
            elapsed_ms,
        } => ActivityMetadataDto {
            connection: activity_connection_state(*state),
            retry_count: *retry_count,
            elapsed_ms: *elapsed_ms,
        },
    }
}

fn activity_connection_state(state: ConnectionState) -> ActivityConnectionStateDto {
    match state {
        ConnectionState::Connecting => ActivityConnectionStateDto::Connecting,
        ConnectionState::Connected => ActivityConnectionStateDto::Connected,
        ConnectionState::Incompatible => ActivityConnectionStateDto::Incompatible,
        ConnectionState::Disconnected => ActivityConnectionStateDto::Disconnected,
        ConnectionState::Error => ActivityConnectionStateDto::Error,
    }
}

/// Parses a lowercase sendable token into a [`SendableState`]. Rejects `booting`/`offline` and any
/// unknown token, enforcing the sendable boundary at the IPC edge (FR-014/015).
#[must_use]
pub fn sendable_from_token(token: &str) -> Option<SendableState> {
    match token {
        "idle" => Some(SendableState::Idle),
        "happy" => Some(SendableState::Happy),
        "busy" => Some(SendableState::Busy),
        "sleeping" => Some(SendableState::Sleeping),
        _ => None,
    }
}

/// Parses a lowercase companion token into a [`CompanionState`] (all six; preview use only).
#[must_use]
pub fn companion_from_token(token: &str) -> Option<CompanionState> {
    match token {
        "booting" => Some(CompanionState::Booting),
        "idle" => Some(CompanionState::Idle),
        "happy" => Some(CompanionState::Happy),
        "busy" => Some(CompanionState::Busy),
        "sleeping" => Some(CompanionState::Sleeping),
        "offline" => Some(CompanionState::Offline),
        _ => None,
    }
}

/// Parses a lowercase direct social-action token.
#[must_use]
pub fn mascot_action_from_token(token: &str) -> Option<MascotAction> {
    match token {
        "greet" => Some(MascotAction::Greet),
        "pet" => Some(MascotAction::Pet),
        "tickle" => Some(MascotAction::Tickle),
        "surprise" => Some(MascotAction::Surprise),
        "comfort" => Some(MascotAction::Comfort),
        _ => None,
    }
}

/// Parses a lowercase mascot-personality token.
#[must_use]
pub fn mascot_personality_from_token(token: &str) -> Option<MascotPersonality> {
    match token {
        "cozy" => Some(MascotPersonality::Cozy),
        "playful" => Some(MascotPersonality::Playful),
        "calm" => Some(MascotPersonality::Calm),
        _ => None,
    }
}

/// The initial (pre-connection) snapshot: `disconnected` / desired `idle`, nothing reported.
#[must_use]
pub fn initial_status() -> ConnectionStatusDto {
    connection_status(
        &ConnectionManager::new(),
        &Orchestrator::new(),
        None,
        false,
        None,
        0,
    )
}
