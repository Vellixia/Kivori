//! Session-only, typed activity history for the desktop UI.
//!
//! The log stores only closed event kinds and typed, allowlisted metadata. It has no persistence
//! path and does not accept caller-provided text, so UI-facing summaries are always generated from
//! trusted event data.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use kivori_model::ConnectionState;

static NEXT_ACTIVITY_ID: AtomicU64 = AtomicU64::new(1);

/// The closed set of native activity event kinds currently emitted by the desktop core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityEventKind {
    /// The device connection manager entered a new lifecycle state.
    ConnectionStateChanged,
}

/// Typed, optional metadata for an activity event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityMetadata {
    /// Allowlisted details recorded with a connection lifecycle transition.
    Connection {
        /// The new connection state.
        state: ConnectionState,
        /// Consecutive reconnect attempts at this transition.
        retry_count: u32,
        /// Monotonic elapsed time since the device task started.
        elapsed_ms: u32,
    },
}

/// One session activity record with a process-monotonic identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEvent {
    /// Process-monotonic event identifier.
    id: u64,
    /// Native-generated UTC ISO-8601 timestamp.
    at: String,
    /// Closed activity event kind.
    kind: ActivityEventKind,
    /// Optional typed, allowlisted details.
    metadata: Option<ActivityMetadata>,
    /// Native-generated summary from the event kind and allowlisted metadata.
    summary: String,
}

impl ActivityEvent {
    /// Process-monotonic event identifier.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Native-generated UTC ISO-8601 timestamp.
    #[must_use]
    pub fn at(&self) -> &str {
        &self.at
    }

    /// Closed activity event kind.
    #[must_use]
    pub const fn kind(&self) -> ActivityEventKind {
        self.kind
    }

    /// Optional typed, allowlisted details.
    #[must_use]
    pub const fn metadata(&self) -> Option<&ActivityMetadata> {
        self.metadata.as_ref()
    }

    /// Native-generated safe summary.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// A bounded, thread-safe, in-memory activity ring.
pub struct ActivityLog {
    entries: Mutex<VecDeque<ActivityEvent>>,
    capacity: usize,
}

impl ActivityLog {
    /// Creates a session-only activity ring holding at most `capacity` entries (minimum one).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    /// Adds one typed event and returns the stored record.
    pub fn record(
        &self,
        kind: ActivityEventKind,
        metadata: Option<ActivityMetadata>,
    ) -> ActivityEvent {
        let event = ActivityEvent {
            id: next_activity_id(),
            at: now_iso(),
            kind,
            summary: summary_for(kind, metadata.as_ref()),
            metadata,
        };
        let mut entries = self.entries.lock().expect("activity log lock");
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(event.clone());
        event
    }

    /// Returns up to `limit` recent records, ordered from oldest to newest.
    #[must_use]
    pub fn recent(&self, limit: usize) -> Vec<ActivityEvent> {
        let entries = self.entries.lock().expect("activity log lock");
        let start = entries.len().saturating_sub(limit);
        entries.iter().skip(start).cloned().collect()
    }
}

fn next_activity_id() -> u64 {
    NEXT_ACTIVITY_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .expect("activity event id exhausted")
}

fn summary_for(kind: ActivityEventKind, metadata: Option<&ActivityMetadata>) -> String {
    match (kind, metadata) {
        (
            ActivityEventKind::ConnectionStateChanged,
            Some(ActivityMetadata::Connection { state, .. }),
        ) => {
            format!("Connection changed to {}.", connection_token(*state))
        }
        (ActivityEventKind::ConnectionStateChanged, None) => {
            "Connection state changed.".to_string()
        }
    }
}

/// Lowercase token shared by safe summaries and the IPC projection.
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

/// The current UTC time as an ISO-8601 second-precision string (`YYYY-MM-DDTHH:MM:SSZ`).
#[must_use]
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}
