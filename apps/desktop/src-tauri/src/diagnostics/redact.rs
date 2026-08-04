//! Redaction helpers (ADR-0005): the closed category vocabulary, boundary identity hashing, and the
//! dev-only raw-payload escape hatch. Everything a diagnostic may contain is produced here or is a
//! primitive (length, sequence, counts) — never raw bytes, ids, paths, or secrets.

use crate::device::connection::hash_device_id_short;
use kivori_model::ConnectionState;
use kivori_protocol::{DeviceId, ErrorCategory, ProtoError};

/// Maps a local decode/encode error to its safe [`ErrorCategory`] — the only error information that is
/// ever logged. The offending bytes are never retained.
#[must_use]
pub fn category_of(error: &ProtoError) -> ErrorCategory {
    match error {
        ProtoError::Cobs
        | ProtoError::BadMagic
        | ProtoError::TooShort
        | ProtoError::LengthMismatch
        | ProtoError::PayloadTooLarge
        | ProtoError::BufferOverflow => ErrorCategory::Framing,
        ProtoError::BadCrc => ErrorCategory::Checksum,
        ProtoError::UnsupportedVersion => ErrorCategory::Version,
        ProtoError::Postcard => ErrorCategory::BadPayload,
    }
}

/// Maps a connection lifecycle state to the safe [`ErrorCategory`] recorded when it is entered.
/// `Connecting`/`Connected` are handshake-phase events; `Incompatible` is a version decision; a lost
/// or absent link is I/O.
#[must_use]
pub fn category_for_connection(state: ConnectionState) -> ErrorCategory {
    match state {
        ConnectionState::Connecting | ConnectionState::Connected => ErrorCategory::Handshake,
        ConnectionState::Incompatible => ErrorCategory::Version,
        ConnectionState::Disconnected | ConnectionState::Error => ErrorCategory::Io,
    }
}

/// Reduces a raw device identity to the short, non-reversible token that is the ONLY identity form
/// allowed in a diagnostic (ADR-0005 §2). The raw id never leaves the transport layer.
#[must_use]
pub fn redact_device_id(id: &DeviceId) -> String {
    hash_device_id_short(id)
}

/// DEV-ONLY raw-payload hex, for deep local debugging. Compiled out unless the `debug-payloads`
/// feature is enabled — which MUST never happen in a shipped build (ADR-0005 §3).
#[cfg(feature = "debug-payloads")]
#[must_use]
pub fn debug_payload_hex(payload: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(payload.len() * 2);
    for byte in payload {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
