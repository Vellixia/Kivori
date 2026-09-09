//! Firmware flashing state-machine coverage without hardware or a real `espflash` process.

use kivori_desktop::firmware::{FirmwarePhase, FlashWorkflow, ResumeTarget};
use kivori_desktop::runtime::state::{AppState, DeviceCommand};
use kivori_desktop::{diagnostics::DiagnosticsLog, ipc::dto};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

struct FakeFlasher {
    result: Result<(), &'static str>,
}

impl FakeFlasher {
    fn run(&self) -> Result<(), &'static str> {
        self.result
    }
}

#[test]
fn rejects_disconnected_and_duplicate_flash_requests() {
    let mut flash = FlashWorkflow::new(true, 512);
    assert!(flash
        .request(false, Some("COM7"), Some("deadbeef"))
        .is_err());
    assert_eq!(flash.status().phase, FirmwarePhase::Idle);

    assert_eq!(
        flash.request(true, Some("COM7"), Some("deadbeef")).unwrap(),
        "COM7"
    );
    assert_eq!(flash.status().phase, FirmwarePhase::Preparing);
    assert!(flash.request(true, Some("COM7"), Some("deadbeef")).is_err());
}

#[test]
fn failed_flash_resumes_normal_connection_work() {
    let mut flash = FlashWorkflow::new(true, 512);
    flash.request(true, Some("COM7"), Some("deadbeef")).unwrap();
    let fake = FakeFlasher {
        result: Err("Firmware flashing timed out."),
    };

    assert_eq!(flash.finish(fake.run()), ResumeTarget::Discovery);
    assert_eq!(flash.status().phase, FirmwarePhase::Failed);
    assert_eq!(flash.status().message, "Firmware flashing timed out.");
}

#[test]
fn app_state_reserves_flash_before_queuing_a_second_request() {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = Arc::clone(&cancel);
    let thread = std::thread::spawn(move || {
        while !thread_cancel.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
    });
    let mut connected = dto::initial_status();
    connected.connection = "connected".to_string();
    let state = AppState::new(
        false,
        Arc::new(Mutex::new(connected)),
        Arc::new(DiagnosticsLog::new(8)),
        tx,
        Arc::clone(&cancel),
        thread,
    );

    state.firmware_status.lock().unwrap().available = false;
    assert!(state.queue_firmware_flash().is_err());
    assert!(rx.try_recv().is_err());
    state.firmware_status.lock().unwrap().available = true;
    assert!(state.queue_firmware_flash().is_ok());
    assert_eq!(rx.recv().unwrap(), DeviceCommand::FlashFirmware);
    assert!(state.queue_firmware_flash().is_err());
    assert_eq!(
        state.firmware_status_snapshot().phase,
        FirmwarePhase::Preparing
    );
    state.shutdown();
}

#[test]
fn successful_flash_waits_for_handshake_on_the_same_port() {
    let mut flash = FlashWorkflow::new(true, 512);
    flash.request(true, Some("COM7"), Some("deadbeef")).unwrap();
    let fake = FakeFlasher { result: Ok(()) };

    assert_eq!(
        flash.finish(fake.run()),
        ResumeTarget::SamePort("COM7".to_string())
    );
    assert_eq!(flash.status().phase, FirmwarePhase::Reconnecting);
    assert!(!flash.handshake("COM8", "deadbeef", true));
    assert_eq!(flash.status().phase, FirmwarePhase::Reconnecting);
    assert!(!flash.handshake("COM7", "not-same", true));
    assert!(flash.handshake("COM7", "deadbeef", true));
    assert_eq!(flash.status().phase, FirmwarePhase::Succeeded);
}

#[test]
fn reconnect_deadline_fails_even_without_a_handshake() {
    let mut flash = FlashWorkflow::new(true, 512);
    flash.request(true, Some("COM7"), Some("deadbeef")).unwrap();
    assert_eq!(
        flash.finish(Ok::<(), &str>(())),
        ResumeTarget::SamePort("COM7".to_string())
    );

    // This is the state reached when the port opens but remains silent until the bounded deadline.
    flash.reconnect_timed_out();
    assert_eq!(flash.status().phase, FirmwarePhase::Failed);
    assert!(flash.status().message.contains("could not be verified"));
}
