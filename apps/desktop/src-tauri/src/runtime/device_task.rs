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

use crate::action::gesture_value::GestureValue;
use crate::device::discovery::DEFAULT_ALLOWLIST;
use crate::device::fsm::{ConnectionManager, ManagerEvent};
use crate::device::reconnect::base_delay_ms;
use crate::device::serial::{first_candidate, SerialPortLink};
use crate::device::session::{Session, SessionConfig};
use crate::diagnostics::{lifecycle_diagnostic, DiagnosticsLog, SafeDiagnostic};
use crate::input::InputIngress;
use crate::ipc::dto::{connection_status, ConnectionStatusDto};
use crate::ipc::events;
use crate::orchestrator::Orchestrator;
use crate::platform;
use crate::presentation::{PresentationResolver, ProductSnapshot};
use crate::runtime::state::DeviceCommand;
use kivori_model::{Capabilities, ConnectionState};
use kivori_protocol::ErrorCategory;

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
    let mut ingress = InputIngress::new();
    let mut gesture_value = GestureValue::new();
    // The initial nonce is a placeholder: `begin_session` (below, on every entry to `Connected`)
    // rebinds it — and restarts `revision` at 0 — before any real `Presentation` is ever resolved.
    let mut resolver = PresentationResolver::new(0);
    // Windows has a real backend (`platform::windows`, Task 12); every other target falls back to
    // the honest "not implemented yet" backend so this crate always compiles. Kept as a concrete
    // (non-`dyn`) type here, not boxed, so the Windows branch can still reach the
    // Windows-only `try_recv_change` below.
    #[cfg(windows)]
    let backend = platform::windows::WindowsVolumeBackend::new();
    #[cfg(not(windows))]
    let backend = platform::unimplemented::UnimplementedVolumeBackend::new(std::env::consts::OS);
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

        // 1.5. Drain backend-originated volume changes (the Windows flyout, a media key, another
        // app, or a default-endpoint switch). Every `change.percent` here is read straight from
        // the Core Audio change-notification payload on the owning `kivori-audio` thread — never
        // assumed, requested, or cached — so routing it through `on_external_change` /
        // `on_endpoint_rebind` (which report `Confirmed` unconditionally) still upholds invariant
        // 3: Confirmed only ever follows an actual backend read.
        #[cfg(windows)]
        while let Some(change) = backend.try_recv_change() {
            let update = match change.origin {
                platform::windows::ChangeOrigin::External => {
                    gesture_value.on_external_change(change.percent)
                }
                platform::windows::ChangeOrigin::EndpointRebind => {
                    gesture_value.on_endpoint_rebind(change.percent)
                }
                // Kivori's own write, echoed back; `set()` already confirmed it synchronously via
                // its own read-back, so it must not be routed as a change (would double-report).
                platform::windows::ChangeOrigin::Kivori => None,
            };
            if let Some(update) = update {
                if let Some(open_link) = link.as_mut() {
                    if session
                        .negotiated_caps()
                        .contains(Capabilities::PRESENTATION_V1)
                    {
                        let presentation = resolver.resolve(&ProductSnapshot::with_value(update));
                        let _ = session.send_presentation(open_link, presentation);
                    }
                }
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
                let pump_result = session.pump(open_link, &mut manager, &mut orchestrator);

                // Route every decoded `InputEvent` through ingress -> gesture value ->
                // presentation, regardless of whether `pump` itself later reports a transport
                // error below (frames it already decoded this call are still real input).
                for event in session.take_input_events() {
                    match ingress.accept(&event) {
                        Ok(Some(input)) => {
                            // `on_input` only ever reports `Confirmed` from an OS read the
                            // backend itself performed (gesture end, read-back); never from a
                            // requested/assumed value (invariant 3).
                            if let Some(update) = gesture_value.on_input(input, &backend) {
                                if session
                                    .negotiated_caps()
                                    .contains(Capabilities::PRESENTATION_V1)
                                {
                                    let presentation =
                                        resolver.resolve(&ProductSnapshot::with_value(update));
                                    let _ = session.send_presentation(open_link, presentation);
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(_reason) => {
                            // Rejected input is dropped for execution and recorded as a safe
                            // `bad_payload` diagnostic (ADR-0005 allowlist: no raw payload, no raw
                            // device id, no paths/usernames).
                            record_rejected_input(&app, &diagnostics, &manager, started);
                        }
                    }
                }

                if pump_result.is_err() {
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

        // 4. On every connection-state transition: (re)scope input/presentation session state and
        //    record a redacted diagnostic (connect, incompatible, disconnect, recoverable error,
        //    reconnect attempt) — T105.
        //
        //    This is keyed off `manager.state()`, NOT `Session::current_session()`: `Session`
        //    structurally cannot observe `ManagerEvent::HeartbeatTimeout`/`PortRemoved`, which are
        //    applied to `manager` here, outside `Session` — the connection state is the only place
        //    that sees every exit from `Connected`.
        let current_state = manager.state();
        if current_state != previous_state {
            if current_state == ConnectionState::Connected {
                if let Some(nonce) = session.current_session() {
                    ingress.begin_session(nonce);
                    resolver.begin_session(nonce);
                }
            } else if previous_state == ConnectionState::Connected {
                ingress.end_session();
            }
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

/// Records a rejected `InputEvent` as a safe `bad_payload` diagnostic and emits it to the webview.
/// The event itself is never logged — only its safe category (ADR-0005 allowlist).
fn record_rejected_input(
    app: &AppHandle,
    diagnostics: &DiagnosticsLog,
    manager: &ConnectionManager,
    started: Instant,
) {
    let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let diag = SafeDiagnostic::new(
        manager.state(),
        ErrorCategory::BadPayload,
        manager.retry_count(),
        elapsed_ms,
    )
    .with_message_type("InputEvent");
    let at = crate::diagnostics::now_iso();
    let dto = crate::ipc::dto::diagnostic_event(&diag, at.clone());
    diagnostics.record(at, diag);
    events::emit_diagnostic(app, &dto);
}
