# ADR-0005: Sensitive-data logging policy

- **Status**: Accepted
- **Date**: 2026-07-24
- **Relates to**: FR-031, FR-032, SC-010; constitution Principle VIII (least privilege, privacy)

## Context

The Kivori desktop core speaks a binary serial protocol to a physical device and surfaces connection
diagnostics to the user and to logs. Diagnostics are valuable for support, but their raw material —
payload bytes, the device's opaque identity, OS paths, usernames — is sensitive and must never leak
into logs or the webview. There is no cloud/telemetry sink (offline-first), but local log files and the
diagnostics UI are still distribution surfaces. We need one enforceable rule for what a diagnostic may
contain, strong enough that a future contributor cannot accidentally widen it.

## Decision

1. **Safe-diagnostics allowlist, enforced structurally.** A diagnostic (log event or
   `DiagnosticEventDto`) may carry ONLY: connection state, an error/diagnostic category, the message
   *kind* name (never contents), the payload *length* (never bytes), the sequence number, the retry
   count, an elapsed-ms marker, and a short *hash* of the device identity. This is enforced by the
   `SafeDiagnostic` type, which has no field capable of holding raw payload bytes, a raw device id, a
   path, a username, or a token. Redaction is therefore a property of the type, not a review rule.
2. **Identity is hashed at the boundary.** The raw 16-byte `DeviceId` never leaves the transport
   layer. It is reduced to a short, non-reversible token (`hash_device_id_short`: FNV-1a → 8 hex
   digits) before it reaches any log field, DTO, or the UI.
3. **Raw payloads are dev-only and compiled out.** Any code path that logs raw payload bytes is gated
   behind the `debug-payloads` Cargo feature, which is OFF by default and MUST NOT be enabled in a
   shipped build. Release binaries contain no raw-payload logging path at all.
4. **One category vocabulary.** Diagnostic categories reuse the protocol's `ErrorCategory`
   (`io`/`handshake`/`version`/`framing`/`checksum`/`timeout`/`busy`/`bad_payload`), so the wire, the
   logs, and the DTO share one closed set.

## Consequences

- A redaction test (SC-010) drives representative errors and asserts every emitted diagnostic conforms
  to the allowlist, and that a default (non-`debug-payloads`) build exposes no raw-payload path.
- Support loses raw-byte fidelity in release builds; a developer reproducing an issue enables
  `debug-payloads` locally. This is the intended trade-off — privacy over convenience.
- Adding a field that carries sensitive data is impossible without changing `SafeDiagnostic`, which the
  redaction test and code review guard.
- The `tracing` subscriber that emits these events, the least-privilege Tauri capability allowlist, and
  the `get_diagnostics` command + diagnostics view are wired on top of this policy in the runtime
  integration; they consume `SafeDiagnostic` and never bypass it.
