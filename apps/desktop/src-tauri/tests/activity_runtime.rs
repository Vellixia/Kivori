//! Runtime activity producers: host-side coverage without Tauri or serial hardware.

use std::collections::VecDeque;
use std::convert::Infallible;

use kivori_desktop::activity::ActivityEventKind;
use kivori_desktop::device::fsm::ConnectionManager;
use kivori_desktop::device::session::{Session, SessionConfig};
use kivori_desktop::device::transport::SerialLink;
use kivori_desktop::orchestrator::Orchestrator;
use kivori_model::{CompanionState, ProtocolVersion};
use kivori_protocol::{
    encode_message, Diagnostic, ErrorCategory, MascotActionApplied, Message, PROTOCOL_MAJOR,
    PROTOCOL_MINOR,
};

#[derive(Default)]
struct FakeLink {
    incoming: VecDeque<u8>,
    outgoing: VecDeque<u8>,
}

impl SerialLink for FakeLink {
    type Error = Infallible;

    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        let mut count = 0;
        while count < bytes.len() {
            let Some(byte) = self.incoming.pop_front() else {
                break;
            };
            bytes[count] = byte;
            count += 1;
        }
        Ok(count)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.outgoing.extend(bytes.iter().copied());
        Ok(bytes.len())
    }
}

fn push(link: &mut FakeLink, message: Message, sequence: u16) {
    let mut wire = heapless::Vec::new();
    encode_message(
        &message,
        ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR),
        sequence,
        &mut wire,
    )
    .expect("test message encodes");
    link.incoming.extend(wire);
}

#[test]
fn opening_a_session_queues_handshake_activity() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();

    session.open(&mut link, &mut manager).expect("open");

    let kinds: Vec<_> = session
        .drain_activity()
        .into_iter()
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            ActivityEventKind::ConnectionOpened,
            ActivityEventKind::HandshakeStarted
        ]
    );
}

#[test]
fn safe_device_diagnostics_and_errors_are_queued_in_wire_order() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Framing,
            code: 1,
        }),
        0,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Checksum,
            code: 2,
        }),
        1,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Version,
            code: 3,
        }),
        2,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::BadPayload,
            code: 4,
        }),
        3,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Framing,
            code: 5,
        }),
        4,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Io,
            code: 6,
        }),
        5,
    );
    push(
        &mut link,
        Message::Diagnostic(Diagnostic {
            category: ErrorCategory::Io,
            code: 7,
        }),
        6,
    );
    push(
        &mut link,
        Message::Error(kivori_protocol::ErrorReport {
            category: ErrorCategory::Busy,
            code: 9,
        }),
        7,
    );

    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");

    assert_eq!(
        session
            .drain_activity()
            .into_iter()
            .map(|entry| entry.kind)
            .collect::<Vec<_>>(),
        [
            ActivityEventKind::DeviceDiagnosticFraming,
            ActivityEventKind::DeviceDiagnosticChecksum,
            ActivityEventKind::DeviceDiagnosticVersion,
            ActivityEventKind::DeviceDiagnosticPayload,
            ActivityEventKind::DeviceSequenceGap,
            ActivityEventKind::DeviceDisplayFault,
            ActivityEventKind::DeviceLinkLost,
            ActivityEventKind::DeviceBusy,
        ]
    );
}

#[test]
fn malformed_and_gapped_frames_are_reported_but_duplicates_and_healthy_traffic_are_suppressed() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    link.incoming.extend([0x02, 0xff, 0x00]);
    push(
        &mut link,
        Message::Pong(kivori_protocol::Pong {
            t_ms_echo: 1,
            uptime_ms: 2,
        }),
        3,
    );
    push(
        &mut link,
        Message::Health(kivori_protocol::Health { free_bytes: 1024 }),
        5,
    );
    // The duplicate must repeat the most recently accepted sequence. Reusing 3 here would be a
    // forward sequence gap after 5, not a duplicate under the protocol's tracker.
    push(
        &mut link,
        Message::Health(kivori_protocol::Health { free_bytes: 1024 }),
        5,
    );

    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");

    assert_eq!(
        session
            .drain_activity()
            .into_iter()
            .map(|entry| entry.kind)
            .collect::<Vec<_>>(),
        [
            ActivityEventKind::ProtocolMalformedFrame,
            ActivityEventKind::ProtocolSequenceGap
        ]
    );
}

#[test]
fn action_acknowledgement_and_changed_state_are_queued_in_wire_order() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    push(
        &mut link,
        Message::MascotActionApplied(MascotActionApplied {
            action: kivori_model::MascotAction::Pet,
            personality: kivori_model::MascotPersonality::Cozy,
            seed: 7,
            applied_at_ms: 120,
        }),
        0,
    );
    push(
        &mut link,
        Message::StateReport(kivori_protocol::StateReport {
            reported: CompanionState::Happy,
            elapsed_ms: 121,
        }),
        1,
    );
    push(
        &mut link,
        Message::StateReport(kivori_protocol::StateReport {
            reported: CompanionState::Happy,
            elapsed_ms: 122,
        }),
        2,
    );

    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");

    assert_eq!(
        session
            .drain_activity()
            .into_iter()
            .map(|entry| entry.kind)
            .collect::<Vec<_>>(),
        [
            ActivityEventKind::SocialActionApplied,
            ActivityEventKind::DeviceStateObserved
        ]
    );
}
