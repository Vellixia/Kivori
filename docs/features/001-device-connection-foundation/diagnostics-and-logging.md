# Diagnostics & logging

How Feature 001 records diagnostics without leaking sensitive data. The durable policy is [ADR-0005](../../adr/0005-logging-policy.md); this file is the operational summary. Requirements: FR-031, FR-032, SC-010.

## The one rule

A diagnostic — a log event **or** a `DiagnosticEventDto` sent to the webview — may carry **only** allowlisted, safe fields. This is enforced by a type, not a review rule: `SafeDiagnostic` ([`diagnostics/mod.rs`](../../../apps/desktop/src-tauri/src/diagnostics/mod.rs)) has no field that can hold a raw payload, raw device id, path, username, or token.

| Allowed field | Never |
|---|---|
| connection state | raw payload bytes |
| error/diagnostic category | raw 16-byte device id |
| message kind name | OS paths / usernames |
| payload length | tokens / secrets |
| sequence number | frame contents |
| retry count, elapsed-ms marker | unnecessary wall-clock data |
| short hash of device identity | |

## Identity is hashed at the boundary

The raw `DeviceId` does not leave the transport/session boundary for ordinary diagnostics. `hash_device_id_short` reduces it to a short token before it reaches log fields, DTOs, or the UI.

This Feature 001 hash is diagnostic/UI correlation only. It is **not** a durable multi-device database identity and must not be confused with future product `DeviceId`/`HardwareId` work described in technical research.

## Category vocabulary

Diagnostic categories reuse the protocol's `ErrorCategory`, keeping wire, logs, and DTO vocabulary aligned. `redact::category_of` maps local protocol errors to the safe closed set without retaining offending bytes.

## Raw payloads: dev-only, compiled out

Any path that renders raw payload bytes is gated behind the `debug-payloads` Cargo feature, which is off by default and must not be enabled in a shipped build. A release binary therefore contains no ordinary raw-payload logging path.

## What CI enforces

- [`tests/redaction.rs`](../../../apps/desktop/src-tauri/tests/redaction.rs) validates the safe diagnostic surface and error-category mapping.
- The default build exposes no raw-payload diagnostic path.
- The Tauri capability boundary prevents the webview from receiving raw serial/fs/shell/network authority through this feature.

## Emitting diagnostics

The runtime integration layer must construct `SafeDiagnostic` values. Adding a field capable of carrying sensitive data requires changing the safe diagnostic type itself and its tests; integration code must not bypass that boundary.
