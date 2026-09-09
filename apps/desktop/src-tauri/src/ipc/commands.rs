//! The least-privilege Tauri command surface (contracts/ipc.md §1).
//!
//! Every command here is the ENTIRE privileged API the webview may call. State-carrying commands take
//! a lowercase wire token and parse it at the boundary, so `set_desired_state`/`mirror_state` accept a
//! `SendableState` only — `booting`/`offline` can never be transmitted (FR-014/015). The dev-only
//! commands are compiled out of release builds via the `device-studio` feature (FR-028).

use tauri::State;

use crate::firmware::FirmwareStatus;
use crate::ipc::dto::{self, AppInfoDto, ConnectionStatusDto, DiagnosticEventDto};
use crate::runtime::state::{AppState, DeviceCommand};
use kivori_model::CompanionState;

/// Application/build info (all builds).
#[tauri::command]
pub fn get_app_info(app: State<'_, AppState>) -> AppInfoDto {
    dto::app_info(app.device_studio_enabled)
}

/// The current three-axis connection snapshot (initial UI sync; all builds).
#[tauri::command]
pub fn get_connection_status(app: State<'_, AppState>) -> ConnectionStatusDto {
    app.status_snapshot()
}

/// All six companion states as wire tokens (all builds).
#[tauri::command]
pub fn list_states() -> Vec<String> {
    CompanionState::ALL
        .iter()
        .map(|state| dto::companion_token(*state).to_string())
        .collect()
}

/// Sets the desired companion state (production path). Accepts a sendable token only.
///
/// # Errors
/// Returns an error string if `state` is not a sendable token, or the device runtime is unavailable.
#[tauri::command]
pub fn set_desired_state(app: State<'_, AppState>, state: String) -> Result<(), String> {
    if app.firmware_busy() {
        return Err(
            "Firmware update is in progress; wait for the device to reconnect.".to_string(),
        );
    }
    let desired =
        dto::sendable_from_token(&state).ok_or_else(|| format!("not a sendable state: {state}"))?;
    app.send_command(DeviceCommand::SetDesired(desired))
}

/// The fixed bundled firmware image and the native update workflow's safe status.
#[tauri::command]
pub fn get_firmware_status(app: State<'_, AppState>) -> FirmwareStatus {
    app.firmware_status_snapshot()
}

/// Queues a flash of this application's fixed bundled firmware on the currently verified device.
///
/// The webview supplies neither a port nor a path. The device thread validates connection ownership,
/// releases the serial link, programs the image, and verifies the same device after reconnecting.
#[tauri::command]
pub fn flash_firmware(app: State<'_, AppState>) -> Result<(), String> {
    app.queue_firmware_flash()
}

/// Recent safe diagnostics, newest last, capped at `limit` (all builds).
#[tauri::command]
pub fn get_diagnostics(app: State<'_, AppState>, limit: u16) -> Vec<DiagnosticEventDto> {
    app.diagnostics
        .recent(limit as usize)
        .into_iter()
        .map(|(at, diag)| dto::diagnostic_event(&diag, at))
        .collect()
}

/// Renders a Device Studio preview frame as raw RGBA8888 bytes (dev-only; efficient binary response).
///
/// # Errors
/// Returns an error string if `state` is not a companion token.
#[cfg(feature = "device-studio")]
#[tauri::command]
pub fn render_preview_frame(
    state: String,
    elapsed_ms: u32,
    animation: Option<crate::render::animation::AnimationTimeline>,
) -> Result<tauri::ipc::Response, String> {
    if let Some(animation) = animation {
        return Ok(tauri::ipc::Response::new(
            crate::render::render_animation_rgba(
                crate::render::bundled_blob(),
                &animation,
                elapsed_ms,
            )?,
        ));
    }
    let companion = dto::companion_from_token(&state)
        .ok_or_else(|| format!("unknown companion state: {state}"))?;
    Ok(tauri::ipc::Response::new(
        crate::render::render_preview_bundled(companion, elapsed_ms),
    ))
}

/// Mirrors the selected sendable state to the device (dev-only; identical to `set_desired_state`).
///
/// # Errors
/// Returns an error string if `state` is not a sendable token, or the device runtime is unavailable.
#[cfg(feature = "device-studio")]
#[tauri::command]
pub fn mirror_state(app: State<'_, AppState>, state: String) -> Result<(), String> {
    if app.firmware_busy() {
        return Err(
            "Firmware update is in progress; wait for the device to reconnect.".to_string(),
        );
    }
    let desired =
        dto::sendable_from_token(&state).ok_or_else(|| format!("not a sendable state: {state}"))?;
    app.send_command(DeviceCommand::SetDesired(desired))
}
