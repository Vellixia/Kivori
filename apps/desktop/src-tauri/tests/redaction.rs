//! Redaction policy tests (T104; SC-010, FR-032; ADR-0005). Asserts every emitted diagnostic conforms
//! to the safe allowlist and that device identity appears only as a hash.

use kivori_desktop::diagnostics::redact::{category_of, redact_device_id};
use kivori_desktop::diagnostics::SafeDiagnostic;
use kivori_model::ConnectionState;
use kivori_protocol::{ErrorCategory, ProtoError};

#[test]
fn proto_errors_map_to_safe_categories() {
    assert_eq!(category_of(&ProtoError::Cobs), ErrorCategory::Framing);
    assert_eq!(category_of(&ProtoError::BadMagic), ErrorCategory::Framing);
    assert_eq!(category_of(&ProtoError::TooShort), ErrorCategory::Framing);
    assert_eq!(
        category_of(&ProtoError::LengthMismatch),
        ErrorCategory::Framing
    );
    assert_eq!(
        category_of(&ProtoError::PayloadTooLarge),
        ErrorCategory::Framing
    );
    assert_eq!(
        category_of(&ProtoError::BufferOverflow),
        ErrorCategory::Framing
    );
    assert_eq!(category_of(&ProtoError::BadCrc), ErrorCategory::Checksum);
    assert_eq!(
        category_of(&ProtoError::UnsupportedVersion),
        ErrorCategory::Version
    );
    assert_eq!(
        category_of(&ProtoError::Postcard),
        ErrorCategory::BadPayload
    );
}

#[test]
fn device_identity_is_only_ever_a_short_stable_hash() {
    let token = redact_device_id(&[0xAB; 16]);
    assert_eq!(token.len(), 8, "8 hex digits");
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()), "hex only");
    assert_eq!(token, redact_device_id(&[0xAB; 16]), "stable");
    assert_ne!(token, redact_device_id(&[0xAC; 16]), "distinct ids differ");
}

#[test]
fn safe_diagnostic_carries_only_allowlisted_fields() {
    let raw_id = [0x11; 16];
    let diag =
        SafeDiagnostic::from_proto_error(&ProtoError::BadCrc, ConnectionState::Connected, 2, 1234)
            .with_message_type("SetState")
            .with_payload_len(7)
            .with_seq(42)
            .with_device_id(&raw_id);

    assert_eq!(diag.category, ErrorCategory::Checksum);
    assert_eq!(diag.connection, ConnectionState::Connected);
    assert_eq!(diag.message_type, Some("SetState"));
    assert_eq!(diag.payload_len, Some(7));
    assert_eq!(diag.seq, Some(42));
    assert_eq!(diag.retry_count, 2);
    assert_eq!(diag.elapsed_ms, 1234);

    // Identity is present only as its hashed token — never the raw bytes.
    let token = diag.device_id_hash_short.expect("hashed id present");
    assert_eq!(token.len(), 8);
    assert_eq!(token, redact_device_id(&raw_id));
    assert_ne!(token, redact_device_id(&[0x22; 16]));
}

#[test]
fn a_minimal_diagnostic_defaults_to_no_optional_data() {
    let diag = SafeDiagnostic::new(ConnectionState::Error, ErrorCategory::Timeout, 3, 500);
    assert!(diag.message_type.is_none());
    assert!(diag.payload_len.is_none());
    assert!(diag.seq.is_none());
    assert!(diag.device_id_hash_short.is_none());
    assert_eq!(diag.retry_count, 3);
}
