//! Redacted native → webview events (contracts/ipc.md §2). Payloads are the safe DTOs only — never
//! raw serial bytes or hardware identifiers. All the semantic changes the UI needs (connection /
//! compatible / incompatible / disconnect / reconnect / desired / reported) are surfaced through the
//! single three-axis `connection://status` event; typed session activity (including lifecycle changes)
//! through `activity-log://event`.

use tauri::{AppHandle, Emitter};

use crate::ipc::dto::{ActivityEventDto, ConnectionStatusDto};

/// Event name for connection-snapshot changes.
pub const CONNECTION_STATUS: &str = "connection://status";
/// Event name for typed session activity records.
pub const ACTIVITY_LOG_EVENT: &str = "activity-log://event";

/// Broadcasts the latest connection snapshot to the webview.
pub fn emit_status(app: &AppHandle, status: &ConnectionStatusDto) {
    let _ = app.emit(CONNECTION_STATUS, status);
}

/// Broadcasts a typed activity record to the webview.
pub fn emit_activity_log(app: &AppHandle, event: &ActivityEventDto) {
    let _ = app.emit(ACTIVITY_LOG_EVENT, event);
}
