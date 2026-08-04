//! The background device thread — the native owner of device communication.
//!
//! An OS thread (not a webview worker, not a Tauri async task tied to a window) drives the connection:
//! discover → open → pump the [`Session`] → apply desired-state commands → back off + reconnect on I/O
//! loss. It survives window close-to-hide, webview reload, webview loss, and frontend navigation
//! because nothing in its lifetime is tied to the window; it stops only when `cancel` is set (explicit
//! Quit / process shutdown). Snapshot changes are published to the shared cell and emitted as
//! `connection://status`; every lifecycle transition is also recorded as a redacted `SafeDiagnostic`
//! and emitted as `diagnostics://event` (T105). Real serial behaviour is validated manually (no
//! hardware in host CI).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::device::discovery::DEFAULT_ALLOWLIST;
use crate::device::fsm::{ConnectionManager, ManagerEvent};
use crate::device::reconnect::base_delay_ms;
use crate::device::serial::{first_candidate, SerialPortLink};
use crate::device::session::{Session, SessionConfig};
use crate::diagnostics::{lifecycle_diagnostic, DiagnosticsLog};
use crate::ipc::dto::{connection_status, ConnectionStatusDto};
use crate::ipc::events;
use crate::orchestrator::Orchestrator;
use crate::runtime::state::DeviceCommand;

const TICK: Duration = Duration::from_millis(50);

/// Spawns the background device thread and returns its join handle.
#[must_use]
pub fn spawn(
    app: AppHandle,
    status: Arc<Mutex<ConnectionStatusDto>>,
    diagnostics: Arc<DiagnosticsLog>,
    commands: Receiver<DeviceCommand>,
    cancel: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("kivori-device".to_string())
        .spawn(move || device_loop(app, status, diagnostics, commands, cancel))
        .expect("spawn kivori-device thread")
}

fn device_loop(
    app: AppHandle,
    status: Arc<Mutex<ConnectionStatusDto>>,
    diagnostics: Arc<DiagnosticsLog>,
    commands: Receiver<DeviceCommand>,
    cancel: Arc<AtomicBool>,
) {
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    let mut session = Session::new(SessionConfig::default());
    let mut link: Option<SerialPortLink> = None;
    let mut retry_at: Option<Instant> = None;
    let started = Instant::now();

    let mut last = connection_status(&manager, &orchestrator, session.reported());
    *status.lock().expect("status lock") = last.clone();
    events::emit_status(&app, &last);
    let mut previous_state = manager.state();
    record(&app, &diagnostics, &manager, started);

    while !cancel.load(Ordering::SeqCst) {
        // 1. Apply queued UI commands.
        while let Ok(command) = commands.try_recv() {
            match command {
                DeviceCommand::SetDesired(state) => match link.as_mut() {
                    Some(open_link) => {
                        let _ = session.set_desired(open_link, &manager, &mut orchestrator, state);
                    }
                    None => {
                        orchestrator.set_desired(state);
                    }
                },
                DeviceCommand::Refresh => {}
            }
        }

        // 2. Drive the link: discover + handshake when down (honoring backoff), pump when up.
        match link.as_mut() {
            None => {
                let ready = retry_at.is_none_or(|deadline| Instant::now() >= deadline);
                if ready {
                    retry_at = None;
                    if let Some(name) = first_candidate(DEFAULT_ALLOWLIST) {
                        if let Ok(mut opened) = SerialPortLink::open(&name) {
                            if session.open(&mut opened, &mut manager).is_ok() {
                                link = Some(opened);
                            }
                        }
                    }
                }
            }
            Some(open_link) => {
                if session
                    .pump(open_link, &mut manager, &mut orchestrator)
                    .is_err()
                {
                    // Recoverable I/O error: drop the link and schedule a bounded reconnect.
                    manager.apply(ManagerEvent::IoError);
                    link = None;
                    let backoff = base_delay_ms(manager.retry_count());
                    retry_at = Some(Instant::now() + Duration::from_millis(backoff));
                }
            }
        }

        // 3. Publish + emit the snapshot on change.
        let snapshot = connection_status(&manager, &orchestrator, session.reported());
        if snapshot != last {
            *status.lock().expect("status lock") = snapshot.clone();
            events::emit_status(&app, &snapshot);
            last = snapshot;
        }

        // 4. Record a redacted diagnostic for every lifecycle transition (connect, incompatible,
        //    disconnect, recoverable error, reconnect attempt) — T105.
        let current_state = manager.state();
        if current_state != previous_state {
            previous_state = current_state;
            record(&app, &diagnostics, &manager, started);
        }

        std::thread::sleep(TICK);
    }
}

/// Records the manager's current lifecycle state as a safe diagnostic and emits it to the webview.
fn record(
    app: &AppHandle,
    diagnostics: &DiagnosticsLog,
    manager: &ConnectionManager,
    started: Instant,
) {
    let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let (at, diag) = lifecycle_diagnostic(manager.state(), manager.retry_count(), elapsed_ms);
    let dto = crate::ipc::dto::diagnostic_event(&diag, at.clone());
    diagnostics.record(at, diag);
    events::emit_diagnostic(app, &dto);
}
