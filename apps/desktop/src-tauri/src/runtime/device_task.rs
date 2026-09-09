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
use crate::firmware::{self, FirmwareStatus, FlashWorkflow, ResumeTarget};
use crate::ipc::dto::{connection_status, ConnectionStatusDto};
use crate::ipc::events;
use crate::orchestrator::Orchestrator;
use crate::runtime::state::DeviceCommand;

const TICK: Duration = Duration::from_millis(50);

/// The longest a newly opened serial port may remain in `Connecting` without a valid `HelloAck`.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// How often an established session is checked for liveness.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

/// Timer policy owned by the device task. It uses elapsed durations rather than `Instant` so the
/// transitions remain deterministic in host tests.
#[derive(Debug, Default)]
pub struct ConnectionDeadlines {
    handshake_deadline: Option<Duration>,
    next_heartbeat: Option<Duration>,
}

impl ConnectionDeadlines {
    /// Starts with no open link or scheduled work.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            handshake_deadline: None,
            next_heartbeat: None,
        }
    }

    /// Starts a new bounded handshake after the serial port opens.
    pub fn on_port_opened(&mut self, now: Duration) {
        self.handshake_deadline = Some(now + HANDSHAKE_TIMEOUT);
        self.next_heartbeat = None;
    }

    /// Replaces the handshake deadline with the first heartbeat deadline.
    pub fn on_handshake_complete(&mut self, now: Duration) {
        self.handshake_deadline = None;
        self.next_heartbeat = Some(now + HEARTBEAT_INTERVAL);
    }

    /// Schedules the next heartbeat after a successful send.
    pub fn on_heartbeat_sent(&mut self, now: Duration) {
        self.next_heartbeat = Some(now + HEARTBEAT_INTERVAL);
    }

    /// Clears timers tied to a serial link that was closed or lost.
    pub fn on_link_lost(&mut self) {
        self.handshake_deadline = None;
        self.next_heartbeat = None;
    }

    #[must_use]
    pub fn handshake_timed_out(&self, now: Duration) -> bool {
        self.handshake_deadline
            .is_some_and(|deadline| now >= deadline)
    }

    #[must_use]
    pub fn heartbeat_due(&self, now: Duration) -> bool {
        self.next_heartbeat.is_some_and(|deadline| now >= deadline)
    }

    #[must_use]
    fn awaiting_handshake(&self) -> bool {
        self.handshake_deadline.is_some()
    }
}

/// Spawns the background device thread and returns its join handle.
#[must_use]
pub fn spawn(
    app: AppHandle,
    status: Arc<Mutex<ConnectionStatusDto>>,
    diagnostics: Arc<DiagnosticsLog>,
    firmware_status: Arc<Mutex<FirmwareStatus>>,
    commands: Receiver<DeviceCommand>,
    cancel: Arc<AtomicBool>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("kivori-device".to_string())
        .spawn(move || device_loop(app, status, diagnostics, firmware_status, commands, cancel))
        .expect("spawn kivori-device thread")
}

fn device_loop(
    app: AppHandle,
    status: Arc<Mutex<ConnectionStatusDto>>,
    diagnostics: Arc<DiagnosticsLog>,
    firmware_status: Arc<Mutex<FirmwareStatus>>,
    commands: Receiver<DeviceCommand>,
    cancel: Arc<AtomicBool>,
) {
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    let mut session = Session::new(SessionConfig::default());
    let mut link: Option<SerialPortLink> = None;
    let mut connected_port: Option<String> = None;
    let mut retry_at: Option<Instant> = None;
    let mut reconnect_deadline: Option<Instant> = None;
    let mut deadlines = ConnectionDeadlines::default();
    let mut flash = FlashWorkflow::new(
        firmware::bundled_image_available(),
        firmware::BUNDLED_FIRMWARE.len() as u64,
    );
    publish_firmware_status(&firmware_status, &flash);
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
                DeviceCommand::SetDesired(_) if flash.is_busy() => {
                    // The public command rejects new changes while busy; discard anything queued just
                    // before the device thread reserved the serial session.
                }
                DeviceCommand::SetDesired(state) => {
                    let write_failed = match link.as_mut() {
                        Some(open_link) => session
                            .set_desired(open_link, &manager, &mut orchestrator, state)
                            .is_err(),
                        None => {
                            orchestrator.set_desired(state);
                            false
                        }
                    };
                    if write_failed {
                        recover_link(
                            &mut manager,
                            ManagerEvent::IoError,
                            &mut link,
                            &mut connected_port,
                            &mut retry_at,
                            &mut deadlines,
                            &flash,
                        );
                    }
                }
                DeviceCommand::Refresh => {}
                DeviceCommand::FlashFirmware => {
                    let requested = flash.request(
                        manager.state().can_drive_device(),
                        connected_port.as_deref(),
                        manager
                            .device()
                            .map(|device| device.device_id_hash_short.as_str()),
                    );
                    let Ok(port) = requested else {
                        if !flash.is_busy() {
                            flash.fail_preparation();
                        }
                        publish_firmware_status(&firmware_status, &flash);
                        continue;
                    };
                    publish_firmware_status(&firmware_status, &flash);

                    // This drop closes the serial handle before `espflash` opens the same port.
                    link = None;
                    connected_port = None;
                    deadlines.on_link_lost();
                    manager.apply(ManagerEvent::PortRemoved);
                    flash.mark_flashing();
                    publish_firmware_status(&firmware_status, &flash);

                    let resume = flash.finish(firmware::flash_bundled(&port, &cancel));
                    publish_firmware_status(&firmware_status, &flash);
                    retry_at = None;
                    match resume {
                        ResumeTarget::Discovery => reconnect_deadline = None,
                        ResumeTarget::SamePort(_) => {
                            reconnect_deadline = Some(Instant::now() + firmware::RECONNECT_TIMEOUT);
                        }
                    }
                }
            }
        }

        // 2. Drive the link: discover + handshake when down (honoring backoff), pump when up.
        // The deadline applies even if the selected port opened but never sends a valid HelloAck.
        if reconnect_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            link = None;
            connected_port = None;
            deadlines.on_link_lost();
            manager.apply(ManagerEvent::PortRemoved);
            flash.reconnect_timed_out();
            publish_firmware_status(&firmware_status, &flash);
            reconnect_deadline = None;
            retry_at = None;
        }
        match link.as_mut() {
            None => {
                let ready = retry_at.is_none_or(|deadline| Instant::now() >= deadline);
                if ready {
                    retry_at = None;
                    if manager.state() == kivori_model::ConnectionState::Error {
                        manager.apply(ManagerEvent::BackoffElapsed);
                    }
                    let candidate = match flash.status().phase {
                        crate::firmware::FirmwarePhase::Reconnecting => {
                            flash_target(&flash).map(str::to_string)
                        }
                        _ => first_candidate(DEFAULT_ALLOWLIST),
                    };
                    if let Some(name) = candidate {
                        if let Ok(mut opened) = SerialPortLink::open(&name) {
                            if session.open(&mut opened, &mut manager).is_ok() {
                                link = Some(opened);
                                connected_port = Some(name);
                                deadlines.on_port_opened(started.elapsed());
                            }
                        }
                    }
                }
            }
            Some(open_link) => {
                let pump_failed = session
                    .pump(open_link, &mut manager, &mut orchestrator)
                    .is_err();
                let now = started.elapsed();
                let event = if pump_failed {
                    Some(ManagerEvent::IoError)
                } else if manager.state() == kivori_model::ConnectionState::Connecting
                    && deadlines.handshake_timed_out(now)
                {
                    Some(ManagerEvent::HandshakeTimeout)
                } else if manager.state().can_drive_device() {
                    if deadlines.awaiting_handshake() {
                        deadlines.on_handshake_complete(now);
                    }
                    if deadlines.heartbeat_due(now) {
                        if session.send_ping(open_link, elapsed_ms(now)).is_err() {
                            Some(ManagerEvent::IoError)
                        } else {
                            deadlines.on_heartbeat_sent(now);
                            session
                                .heartbeat_timed_out()
                                .then_some(ManagerEvent::HeartbeatTimeout)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(event) = event {
                    recover_link(
                        &mut manager,
                        event,
                        &mut link,
                        &mut connected_port,
                        &mut retry_at,
                        &mut deadlines,
                        &flash,
                    );
                }
            }
        }

        if flash.status().phase == crate::firmware::FirmwarePhase::Reconnecting
            && manager.state().can_drive_device()
        {
            if let (Some(port), Some(device)) = (connected_port.as_deref(), manager.device()) {
                if flash.handshake(port, &device.device_id_hash_short, true) {
                    publish_firmware_status(&firmware_status, &flash);
                    reconnect_deadline = None;
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

fn elapsed_ms(elapsed: Duration) -> u32 {
    u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX)
}

fn recover_link(
    manager: &mut ConnectionManager,
    event: ManagerEvent,
    link: &mut Option<SerialPortLink>,
    connected_port: &mut Option<String>,
    retry_at: &mut Option<Instant>,
    deadlines: &mut ConnectionDeadlines,
    flash: &FlashWorkflow,
) {
    manager.apply(event);
    *link = None;
    *connected_port = None;
    deadlines.on_link_lost();
    if flash.status().phase == crate::firmware::FirmwarePhase::Reconnecting {
        *retry_at = None;
    } else {
        let backoff = base_delay_ms(manager.retry_count());
        *retry_at = Some(Instant::now() + Duration::from_millis(backoff));
    }
}

fn publish_firmware_status(status: &Mutex<FirmwareStatus>, workflow: &FlashWorkflow) {
    *status.lock().expect("firmware status lock") = workflow.status().clone();
}

fn flash_target(workflow: &FlashWorkflow) -> Option<&str> {
    // The workflow's status deliberately hides the port from IPC. The device task needs it to reopen
    // exactly the same native-owned port after programming.
    workflow.target_port()
}

/// Records the manager's current lifecycle state as a safe diagnostic and emits it to the webview.
fn record(
    app: &AppHandle,
    diagnostics: &DiagnosticsLog,
    manager: &ConnectionManager,
    started: Instant,
) {
    tracing::info!(connection = ?manager.state(), retries = manager.retry_count(), "device connection changed");
    let elapsed_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let (at, diag) = lifecycle_diagnostic(manager.state(), manager.retry_count(), elapsed_ms);
    let dto = crate::ipc::dto::diagnostic_event(&diag, at.clone());
    diagnostics.record(at, diag);
    events::emit_diagnostic(app, &dto);
}
