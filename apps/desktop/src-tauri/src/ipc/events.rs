//! Redacted native → webview events (contracts/ipc.md §2). Payloads are the safe DTOs only — never
//! raw serial bytes or hardware identifiers. All the semantic changes the UI needs (connection /
//! compatible / incompatible / disconnect / reconnect / desired / reported) are surfaced through the
//! single three-axis `connection://status` event; diagnostics (incl. recoverable errors) through
//! `diagnostics://event`.

use tauri::{AppHandle, Emitter};

use crate::ipc::dto::{ConnectionStatusDto, DiagnosticEventDto};

/// Event name for connection-snapshot changes.
pub const CONNECTION_STATUS: &str = "connection://status";
/// Event name for safe diagnostic events.
pub const DIAGNOSTICS_EVENT: &str = "diagnostics://event";

/// Broadcasts the latest connection snapshot to the webview.
pub fn emit_status(app: &AppHandle, status: &ConnectionStatusDto) {
    let _ = app.emit(CONNECTION_STATUS, status);
}

/// Broadcasts a safe diagnostic event to the webview.
pub fn emit_diagnostic(app: &AppHandle, diagnostic: &DiagnosticEventDto) {
    let _ = app.emit(DIAGNOSTICS_EVENT, diagnostic);
}
