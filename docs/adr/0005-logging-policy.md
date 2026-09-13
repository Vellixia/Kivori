# ADR-0005: Typed session-activity logging policy

- **Status**: Accepted
- **Date**: 2026-07-24
- **Relates to**: FR-031, FR-032, SC-010; constitution Principle VIII (least privilege, privacy)

## Context

The Kivori desktop core speaks a binary serial protocol to a physical device and surfaces typed session
activity to the user and local logs. Its raw material — payload bytes, opaque device identity, OS paths,
usernames, and tool output — is sensitive and must never leak into a webview or log file.

## Decision

1. **Typed activity allowlist, enforced structurally.** An activity record (`ActivityEventDto`) has
   native-issued ID/time/summary plus closed type, severity, source, and outcome tokens. Optional
   metadata is a fixed allowlist: connection state, safe diagnostic category/code, version/hash/capability
   summaries, semantic controls, and protocol counters. There is no arbitrary details map.
2. **Identity is hashed at the boundary.** The raw 16-byte `DeviceId` never leaves the transport layer;
   only a short, non-reversible hash may enter activity metadata.
3. **Raw payloads are dev-only and compiled out.** Any raw-payload output remains gated behind the
   `debug-payloads` Cargo feature, which is OFF by default and MUST NOT be enabled in shipped builds.
4. **One category vocabulary.** Activity diagnostic categories reuse the protocol's closed
   `ErrorCategory` vocabulary (`io`/`handshake`/`version`/`framing`/`checksum`/`timeout`/`busy`/`bad_payload`).

## Consequences

- Native redaction tests and frontend runtime-cast privacy tests enforce the fixed activity allowlist.
- The session Log subscribes before reading history, keeps the newest 256 event IDs, and never persists
  or uploads activity.
- Support loses raw-byte fidelity in release builds. This is the intended privacy-over-convenience trade-off.
- The native activity runtime, least-privilege Tauri capability allowlist, `get_activity_log` command,
  and `activity-log://event` subscription never bypass the fixed DTO.
