# Diagnostics & logging

How Kivori records diagnostics without leaking sensitive data. The policy is [ADR-0005](./adr/0005-logging-policy.md);
this is the operational summary. Requirements: FR-031, FR-032, SC-010; constitution Principle VIII.

## The one rule

A diagnostic — a log event **or** a `DiagnosticEventDto` sent to the webview — may carry **only** the
allowlisted, safe fields. This is enforced by a type, not a review rule: `SafeDiagnostic`
([`diagnostics/mod.rs`](../apps/desktop/src-tauri/src/diagnostics/mod.rs)) has no field that can hold a
raw payload, a raw device id, a path, a username, or a token.

| Allowed field | Never |
|---------------|-------|
| connection state | raw payload bytes |
| error/diagnostic category | raw 16-byte device id |
| message *kind* name (e.g. `"SetState"`) | OS paths / usernames |
| payload *length* (bytes count) | tokens / secrets |
| sequence number | frame contents |
| retry count, elapsed-ms marker | wall-clock beyond the emit timestamp |
| short **hash** of the device identity | |

## Identity is hashed at the boundary

The raw `DeviceId` never leaves the transport layer. `hash_device_id_short` (FNV-1a → 8 hex digits)
reduces it to a short, non-reversible token before it reaches any log field, DTO, or the UI. The same
token is used everywhere a device must be named, so logs and the UI correlate without exposing identity.

## Category vocabulary

Diagnostic categories reuse the protocol's `ErrorCategory` — `io`, `handshake`, `version`, `framing`,
`checksum`, `timeout`, `busy`, `bad_payload` — so the wire, the logs, and the DTO share one closed set.
`redact::category_of` maps every local `ProtoError` to one of these (never the offending bytes).

## Raw payloads: dev-only, compiled out

Any path that renders raw payload bytes (hex) is gated behind the `debug-payloads` Cargo feature, which
is **off by default and must never be enabled in a shipped build**. A release binary therefore contains
no raw-payload logging path at all — the redaction guarantee is structural, not configuration-dependent.

## What CI enforces

- [`tests/redaction.rs`](../apps/desktop/src-tauri/tests/redaction.rs): every `SafeDiagnostic` carries
  only allowlisted fields; identity appears only as its hash; `ProtoError`→category mapping is total.
- The default (non-`debug-payloads`) build exposes no raw-payload path.
- The least-privilege Tauri capability allowlist (no fs/shell/http; only the defined commands) and the
  release-omits-dev-commands assertion are validated in the runtime-integration layer, on top of this
  policy — they consume `SafeDiagnostic` and never bypass it.

## Emitting diagnostics (integration layer)

The `tracing` subscriber and the `get_diagnostics` IPC command are built in the runtime-integration
layer. They must construct `SafeDiagnostic` values — there is no supported path that logs an error, a
frame, or an identity directly. To add a new diagnostic, extend the safe constructors; adding a field
that carries sensitive data requires changing `SafeDiagnostic` itself, which the redaction test guards.
