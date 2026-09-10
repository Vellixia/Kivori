//! Host-testable runtime coverage (T123): DTO projections + redaction, the sendable production
//! boundary, the diagnostics ring, and AppState construction/shutdown. The Tauri window/tray behaviour
//! and the serial device loop are exercised manually (no GUI / no hardware in host CI).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kivori_desktop::device::connection::ConnectedDevice;
use kivori_desktop::device::fsm::{ConnectionManager, ManagerEvent};
use kivori_desktop::diagnostics::{DiagnosticsLog, SafeDiagnostic};
use kivori_desktop::ipc::dto;
use kivori_desktop::orchestrator::Orchestrator;
use kivori_desktop::runtime::state::{AppState, DeviceCommand};
use kivori_model::{
    CompanionState, ConnectionState, MascotAction, MascotPersonality, ProtocolVersion,
    SendableState,
};
use kivori_protocol::{ErrorCategory, FirmwareVersion};

fn connected_device() -> ConnectedDevice {
    ConnectedDevice {
        firmware_version: FirmwareVersion {
            major: 1,
            minor: 4,
            patch: 2,
        },
        protocol_version: ProtocolVersion::new(1, 0),
        device_id_hash_short: "deadbeef".to_string(),
    }
}

#[test]
fn initial_status_is_disconnected_idle() {
    let dto = dto::initial_status();
    assert_eq!(dto.connection, "disconnected");
    assert_eq!(dto.desired, "idle");
    assert_eq!(dto.reported, None);
    assert!(dto.device.is_none());
    assert_eq!(dto.retry_count, 0);
    assert!(!dto.mascot_interaction);
}

#[test]
fn connected_status_projects_all_three_axes() {
    let mut manager = ConnectionManager::new();
    manager.apply(ManagerEvent::PortOpened);
    manager.apply(ManagerEvent::HandshakeOk(connected_device()));
    let mut orchestrator = Orchestrator::new();
    orchestrator.set_desired(SendableState::Busy);

    let dto = dto::connection_status(
        &manager,
        &orchestrator,
        Some(CompanionState::Happy),
        true,
        None,
        7,
    );
    assert_eq!(dto.connection, "connected");
    assert_eq!(dto.desired, "busy");
    assert_eq!(dto.reported.as_deref(), Some("happy"));
    assert!(dto.mascot_interaction);
    assert_eq!(dto.connection_generation, 7);
    let device = dto.device.expect("device present when connected");
    assert_eq!(device.firmware_version, "1.4.2");
    assert_eq!(device.device_id_hash_short, "deadbeef");
    assert_eq!(device.protocol_version.major, 1);
}

#[test]
fn incompatible_status_carries_reason() {
    let mut manager = ConnectionManager::new();
    manager.apply(ManagerEvent::PortOpened);
    manager.apply(ManagerEvent::HandshakeIncompatible { device_major: 2 });
    let dto = dto::connection_status(&manager, &Orchestrator::new(), None, false, None, 1);
    assert_eq!(dto.connection, "incompatible");
    assert!(dto.incompatible_reason.unwrap().contains("v2"));
    assert!(dto.device.is_none());
}

#[test]
fn app_info_reports_supported_protocol_and_studio_flag() {
    let dev = dto::app_info(true);
    assert!(dev.device_studio_enabled);
    assert_eq!(dev.supported_majors, vec![1]);
    assert_eq!(dev.protocol_version.major, 1);
    assert!(!dto::app_info(false).device_studio_enabled);
}

#[test]
fn sendable_boundary_rejects_device_originated_and_unknown() {
    // The production state boundary accepts only the four sendable tokens (FR-014/015).
    assert_eq!(dto::sendable_from_token("idle"), Some(SendableState::Idle));
    assert_eq!(dto::sendable_from_token("busy"), Some(SendableState::Busy));
    assert_eq!(dto::sendable_from_token("booting"), None);
    assert_eq!(dto::sendable_from_token("offline"), None);
    assert_eq!(dto::sendable_from_token("nonsense"), None);
    // Companion parsing (preview only) accepts all six.
    assert_eq!(
        dto::companion_from_token("booting"),
        Some(CompanionState::Booting)
    );
    assert_eq!(
        dto::companion_from_token("offline"),
        Some(CompanionState::Offline)
    );
    assert_eq!(dto::companion_from_token("nope"), None);
    assert_eq!(
        dto::mascot_action_from_token("tickle"),
        Some(MascotAction::Tickle)
    );
    assert_eq!(dto::mascot_action_from_token("unknown"), None);
    assert_eq!(
        dto::mascot_personality_from_token("calm"),
        Some(MascotPersonality::Calm)
    );
    assert_eq!(dto::mascot_personality_from_token("unknown"), None);
}

#[test]
fn diagnostic_event_projection_is_redacted() {
    let diag = SafeDiagnostic::new(ConnectionState::Error, ErrorCategory::Timeout, 2, 750)
        .with_message_type("Ping")
        .with_seq(7)
        .with_device_id(&[0xAB; 16]);
    let dto = dto::diagnostic_event(&diag, "2026-07-24T00:00:00Z".to_string());
    assert_eq!(dto.at, "2026-07-24T00:00:00Z");
    assert_eq!(dto.connection, "error");
    assert_eq!(dto.category, "timeout");
    assert_eq!(dto.message_type.as_deref(), Some("Ping"));
    assert_eq!(dto.seq, Some(7));
    // Identity is present only as its short hash.
    assert_eq!(dto.device_id_hash_short.as_deref().map(str::len), Some(8));
}

#[test]
fn diagnostics_log_keeps_recent_within_capacity() {
    let log = DiagnosticsLog::new(3);
    for retry in 0..5u32 {
        log.record(
            format!("t{retry}"),
            SafeDiagnostic::new(ConnectionState::Error, ErrorCategory::Io, retry, retry * 10),
        );
    }
    let recent = log.recent(10);
    assert_eq!(recent.len(), 3, "capacity caps the ring");
    assert_eq!(recent.first().unwrap().0, "t2", "oldest surviving entry");
    assert_eq!(recent.last().unwrap().0, "t4", "newest entry last");
    assert_eq!(log.recent(1).len(), 1, "limit honoured");
}

/// Builds an AppState around a stand-in device thread that honours the cancellation contract.
fn app_state_with_dummy_thread() -> (AppState, Arc<AtomicBool>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel::<DeviceCommand>();
    let thread_cancel = Arc::clone(&cancel);
    let thread = std::thread::spawn(move || {
        while !thread_cancel.load(Ordering::SeqCst) {
            let _ = rx.recv_timeout(Duration::from_millis(5));
        }
    });
    let status = Arc::new(Mutex::new(dto::initial_status()));
    let diagnostics = Arc::new(DiagnosticsLog::new(8));
    let state = AppState::new(true, status, diagnostics, tx, Arc::clone(&cancel), thread);
    (state, cancel)
}

#[test]
fn app_state_relays_commands_then_shuts_down_the_thread() {
    let (state, cancel) = app_state_with_dummy_thread();
    assert_eq!(state.status_snapshot().connection, "disconnected");
    state
        .send_command(DeviceCommand::SetDesired(SendableState::Busy))
        .expect("command relayed while running");

    state.shutdown();
    assert!(
        cancel.load(Ordering::SeqCst),
        "shutdown signalled cancellation"
    );
    // The thread has joined and dropped the receiver, so further sends fail cleanly.
    assert!(
        state.send_command(DeviceCommand::Refresh).is_err(),
        "commands fail once the device thread has stopped"
    );
}
