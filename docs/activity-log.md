# Typed session activity log

Kivori records local session activity without leaking sensitive data. The accepted policy is
[ADR-0005](./adr/0005-logging-policy.md); this is the operational summary.

## Typed allowlist

Every activity record sent to the webview is an `ActivityEventDto`. The native runtime owns the event
ID, ISO-8601 time, summary, closed `type`, `severity`, `source`, and `outcome` tokens. Its optional
`metadata` has a fixed shape: connection state, retry/elapsed markers, safe diagnostic category/code,
firmware and protocol versions, short device hash, capability mask, state/personality/action controls,
protocol counters, and reported state. There is no arbitrary details map.

The log never carries raw payload bytes, raw device identity, serial port or filesystem paths,
usernames, credentials, secrets, flasher output, or raw error text. The UI manually reads only the
fixed metadata fields; it never enumerates metadata keys.

## Session delivery

The webview requests history with `get_activity_log(limit)` and listens on `activity-log://event`.
It subscribes before requesting history, deduplicates by numeric event ID with the live copy winning,
sorts ascending by ID, and retains the newest 256 records in the session-only view. It does not persist
or upload activity.

## Identity and raw payloads

The raw device ID never leaves the transport boundary. A short non-reversible hash is the only device
correlation value allowed in activity metadata. Raw payload output remains a dev-only, compiled-out
path and must not be enabled in shipped builds.

## Enforcement

- Native activity DTO and redaction tests constrain the producing runtime.
- Frontend IPC tests pin the command/event names.
- Log view tests cover lifecycle ordering, 256-entry retention, filters, safe runtime-cast handling,
  cleanup, and accessibility.
