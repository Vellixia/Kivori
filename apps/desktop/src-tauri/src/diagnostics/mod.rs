//! Safe diagnostics (FR-031/032; ADR-0005).
//!
//! [`SafeDiagnostic`] is the allowlist made a type: it can only hold connection state, a category, a
//! message *kind* name, a payload *length*, a sequence number, a retry count, an elapsed-ms marker,
//! and a *hashed* device id. There is no field for raw payload bytes, a raw id, a path, or a secret,
//! so redaction is a structural guarantee rather than a review rule. The `tracing` subscriber that
//! emits these and the `get_diagnostics` IPC command are wired on top of this in the runtime phase.

pub mod redact;

use kivori_model::ConnectionState;
use kivori_protocol::{DeviceId, ErrorCategory, ProtoError};

/// A diagnostic that is safe to log and to surface to the UI (the ADR-0005 allowlist as a type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeDiagnostic {
    /// The connection state when the diagnostic was recorded.
    pub connection: ConnectionState,
    /// The safe error/diagnostic category (the closed set shared with the wire).
    pub category: ErrorCategory,
    /// The message *kind* name (e.g. `"SetState"`) — never its contents.
    pub message_type: Option<&'static str>,
    /// The payload *length* in bytes — never the bytes themselves.
    pub payload_len: Option<u16>,
    /// The frame sequence number, if applicable.
    pub seq: Option<u16>,
    /// Consecutive reconnect attempts at the time of the event.
    pub retry_count: u32,
    /// A monotonic elapsed-ms marker.
    pub elapsed_ms: u32,
    /// A short, non-reversible hash of the device identity — never the raw id.
    pub device_id_hash_short: Option<String>,
}

impl SafeDiagnostic {
    /// Creates a minimal safe diagnostic; enrich it with the `with_*` builders.
    #[must_use]
    pub fn new(
        connection: ConnectionState,
        category: ErrorCategory,
        retry_count: u32,
        elapsed_ms: u32,
    ) -> Self {
        Self {
            connection,
            category,
            message_type: None,
            payload_len: None,
            seq: None,
            retry_count,
            elapsed_ms,
            device_id_hash_short: None,
        }
    }

    /// Builds a safe diagnostic from a protocol error (category only — never the offending bytes).
    #[must_use]
    pub fn from_proto_error(
        error: &ProtoError,
        connection: ConnectionState,
        retry_count: u32,
        elapsed_ms: u32,
    ) -> Self {
        Self::new(
            connection,
            redact::category_of(error),
            retry_count,
            elapsed_ms,
        )
    }

    /// Records the message *kind* name.
    #[must_use]
    pub fn with_message_type(mut self, kind: &'static str) -> Self {
        self.message_type = Some(kind);
        self
    }

    /// Records the payload *length* (never the payload).
    #[must_use]
    pub fn with_payload_len(mut self, len: u16) -> Self {
        self.payload_len = Some(len);
        self
    }

    /// Records the frame sequence number.
    #[must_use]
    pub fn with_seq(mut self, seq: u16) -> Self {
        self.seq = Some(seq);
        self
    }

    /// Records the device identity as its hashed token (the raw id is consumed, never stored).
    #[must_use]
    pub fn with_device_id(mut self, id: &DeviceId) -> Self {
        self.device_id_hash_short = Some(redact::redact_device_id(id));
        self
    }
}

/// A bounded, thread-safe ring of safe diagnostics (each with a wall-clock stamp) backing
/// `get_diagnostics`. Only [`SafeDiagnostic`] values are stored, so the ADR-0005 allowlist holds by
/// construction. Populating it from the connection lifecycle is the `tracing`-layer work (T101).
pub struct DiagnosticsLog {
    entries: std::sync::Mutex<std::collections::VecDeque<(String, SafeDiagnostic)>>,
    capacity: usize,
}

impl DiagnosticsLog {
    /// Creates a log holding at most `capacity` recent entries (clamped to at least 1).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: std::sync::Mutex::new(std::collections::VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    /// Records a safe diagnostic stamped with the ISO-8601 time `at`, evicting the oldest if full.
    pub fn record(&self, at: String, diagnostic: SafeDiagnostic) {
        let mut entries = self.entries.lock().expect("diagnostics lock");
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back((at, diagnostic));
    }

    /// Returns up to `limit` of the most recent entries, oldest-first.
    #[must_use]
    pub fn recent(&self, limit: usize) -> Vec<(String, SafeDiagnostic)> {
        let entries = self.entries.lock().expect("diagnostics lock");
        let start = entries.len().saturating_sub(limit);
        entries.iter().skip(start).cloned().collect()
    }
}

/// Builds the stamped, redacted diagnostic recorded when the connection enters `state` (T105).
///
/// Carries only allowlisted fields: the state, its safe category, the retry count, and an elapsed-ms
/// marker. There is no raw payload, identity, or path anywhere on this path (ADR-0005).
#[must_use]
pub fn lifecycle_diagnostic(
    state: ConnectionState,
    retry_count: u32,
    elapsed_ms: u32,
) -> (String, SafeDiagnostic) {
    let diagnostic = SafeDiagnostic::new(
        state,
        redact::category_for_connection(state),
        retry_count,
        elapsed_ms,
    );
    (now_iso(), diagnostic)
}

/// The current UTC time as an ISO-8601 second-precision string (`YYYY-MM-DDTHH:MM:SSZ`). Used to
/// stamp diagnostics without pulling a date-time dependency.
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

/// Converts days-since-1970-01-01 to `(year, month, day)` (Howard Hinnant's civil-from-days, UTC).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if month <= 2 { year + 1 } else { year }, month, day)
}
