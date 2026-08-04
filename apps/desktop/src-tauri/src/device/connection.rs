//! Handshake verification and the safe connected-device summary (FR-002, FR-004; ADR-0005).
//!
//! Identity is authoritative only via the handshake. The raw device id is never exposed above the
//! transport layer — it is hashed to a short, non-reversible token for the UI and logs.

use kivori_model::{Capabilities, ProtocolVersion};
use kivori_protocol::{
    evaluate_hello_ack, DeviceId, FirmwareVersion, HandshakeOutcome, Hello, HelloAck,
};

/// A safe summary of a connected device (never the raw identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDevice {
    /// Device firmware version.
    pub firmware_version: FirmwareVersion,
    /// Device protocol version (taken from the frame header).
    pub protocol_version: ProtocolVersion,
    /// Short, non-reversible hash of the device identity.
    pub device_id_hash_short: String,
}

/// Builds the desktop's opening `Hello`.
#[must_use]
pub fn build_hello(app_version: FirmwareVersion, caps: Capabilities, nonce: u32) -> Hello {
    Hello {
        desktop_version: app_version,
        desktop_caps: caps,
        nonce,
    }
}

/// Verifies a device's `HelloAck` against the `Hello` we sent (delegates to the shared protocol
/// evaluation). Identity is confirmed only when the nonce echoes and the major version is supported.
#[must_use]
pub fn verify_handshake(
    sent: &Hello,
    ack: &HelloAck,
    device_version: ProtocolVersion,
    desktop_version: ProtocolVersion,
    supported_majors: &[u16],
) -> HandshakeOutcome {
    evaluate_hello_ack(sent, ack, device_version, desktop_version, supported_majors)
}

/// Builds the safe [`ConnectedDevice`] summary from a verified `HelloAck` and its header version.
#[must_use]
pub fn summarize(ack: &HelloAck, device_version: ProtocolVersion) -> ConnectedDevice {
    ConnectedDevice {
        firmware_version: ack.firmware_version,
        protocol_version: device_version,
        device_id_hash_short: hash_device_id_short(&ack.device_id),
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Hashes a raw device id to a short, non-reversible hex token (FNV-1a; the high 32 bits as 8 hex
/// digits). This is the ONLY device-identity representation allowed outside the transport layer
/// (ADR-0005 redaction; the raw id never reaches a log field, DTO, or the UI).
#[must_use]
pub fn hash_device_id_short(id: &DeviceId) -> String {
    let mut h = FNV_OFFSET;
    for &byte in id {
        h ^= u64::from(byte);
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:08x}", (h >> 32) as u32)
}
