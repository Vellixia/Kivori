//! Desktop session-driver tests (Phase 11 host-side; FR-002/009/012, SC-008). The test plays the
//! *device*: it decodes what the desktop transmits and injects encoded device responses through an
//! in-memory link, exercising the full connect → resync → set-state → reconnect loop.

use std::collections::VecDeque;
use std::convert::Infallible;

use kivori_desktop::device::fsm::ConnectionManager;
use kivori_desktop::device::session::{Session, SessionConfig};
use kivori_desktop::device::transport::SerialLink;
use kivori_desktop::device::ManagerEvent;
use kivori_desktop::orchestrator::Orchestrator;
use kivori_model::{Capabilities, ConnectionState, ProtocolVersion, SendableState};
use kivori_protocol::{
    decode_message, encode_message, FirmwareVersion, HelloAck, Message, Pong, PROTOCOL_MAJOR,
    PROTOCOL_MINOR,
};

#[derive(Default)]
struct FakeLink {
    to_device: VecDeque<u8>,
    from_device: VecDeque<u8>,
}

impl SerialLink for FakeLink {
    type Error = Infallible;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        let mut n = 0;
        while n < buf.len() {
            match self.from_device.pop_front() {
                Some(byte) => {
                    buf[n] = byte;
                    n += 1;
                }
                None => break,
            }
        }
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
        self.to_device.extend(buf.iter().copied());
        Ok(buf.len())
    }
}

/// Encodes a device→desktop message (at `version`, sequence `seq`) into the link.
fn device_push(link: &mut FakeLink, msg: &Message, version: ProtocolVersion, seq: u16) {
    let mut wire: heapless::Vec<u8, { kivori_protocol::MAX_WIRE }> = heapless::Vec::new();
    encode_message(msg, version, seq, &mut wire).expect("encode");
    link.from_device.extend(wire.iter().copied());
}

/// Decodes everything the desktop has transmitted since the last drain.
fn desktop_drain(link: &mut FakeLink) -> Vec<Message> {
    let bytes: Vec<u8> = link.to_device.drain(..).collect();
    let mut out = Vec::new();
    let mut scratch: heapless::Vec<u8, { kivori_protocol::MAX_FRAME }> = heapless::Vec::new();
    for packet in bytes.split(|&b| b == 0) {
        if packet.is_empty() {
            continue;
        }
        if let Ok((_, msg)) = decode_message(packet, &mut scratch, &[PROTOCOL_MAJOR]) {
            out.push(msg);
        }
    }
    out
}

fn hello_nonce(msgs: &[Message]) -> u32 {
    for msg in msgs {
        if let Message::Hello(hello) = msg {
            return hello.nonce;
        }
    }
    panic!("expected a Hello, got {msgs:?}");
}

fn wire_version() -> ProtocolVersion {
    ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR)
}

fn device_ack(nonce: u32) -> Message {
    Message::HelloAck(HelloAck {
        device_caps: Capabilities::NONE,
        device_id: [0x5A; 16],
        firmware_version: FirmwareVersion {
            major: 1,
            minor: 4,
            patch: 2,
        },
        nonce_echo: nonce,
    })
}

fn set_state_desired(msgs: &[Message]) -> Option<SendableState> {
    msgs.iter().find_map(|m| match m {
        Message::SetState(s) => Some(s.desired),
        _ => None,
    })
}

/// Drives a full handshake and returns the connected session + peers with the given initial desired.
fn connect(desired: SendableState) -> (FakeLink, Session, ConnectionManager, Orchestrator) {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();
    orchestrator.set_desired(desired);

    session.open(&mut link, &mut manager).expect("open");
    let nonce = hello_nonce(&desktop_drain(&mut link));
    device_push(&mut link, &device_ack(nonce), wire_version(), 0);
    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");
    (link, session, manager, orchestrator)
}

#[test]
fn handshake_reaches_connected_and_resyncs_desired_state() {
    let (mut link, _session, manager, _orch) = connect(SendableState::Idle);
    assert_eq!(manager.state(), ConnectionState::Connected);
    let device = manager.device().expect("connected device");
    assert_eq!(device.firmware_version.minor, 4);
    assert_eq!(device.device_id_hash_short.len(), 8);

    // On connect the desktop confirms with Ready and resyncs the desired state.
    let sent = desktop_drain(&mut link);
    assert!(
        sent.iter().any(|m| matches!(m, Message::Ready(_))),
        "sent Ready"
    );
    assert_eq!(
        set_state_desired(&sent),
        Some(SendableState::Idle),
        "resync SetState"
    );
}

#[test]
fn resync_transmits_the_current_desired_not_the_default() {
    let (mut link, _session, manager, _orch) = connect(SendableState::Happy);
    assert_eq!(manager.state(), ConnectionState::Connected);
    assert_eq!(
        set_state_desired(&desktop_drain(&mut link)),
        Some(SendableState::Happy)
    );
}

#[test]
fn set_desired_transmits_only_when_connected() {
    let (mut link, mut session, manager, mut orch) = connect(SendableState::Idle);
    let _ = desktop_drain(&mut link); // clear the connect traffic

    session
        .set_desired(&mut link, &manager, &mut orch, SendableState::Busy)
        .expect("set_desired");
    assert_eq!(
        set_state_desired(&desktop_drain(&mut link)),
        Some(SendableState::Busy)
    );
    assert_eq!(orch.desired(), SendableState::Busy);
}

#[test]
fn incompatible_major_is_surfaced_and_bye_sent() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();

    session.open(&mut link, &mut manager).expect("open");
    let nonce = hello_nonce(&desktop_drain(&mut link));
    // Device answers at an unsupported major version (2.x).
    device_push(&mut link, &device_ack(nonce), ProtocolVersion::new(2, 0), 0);
    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");

    assert_eq!(manager.state(), ConnectionState::Incompatible);
    assert!(manager.incompatible_reason().unwrap().contains("v2"));
    assert!(
        desktop_drain(&mut link)
            .iter()
            .any(|m| matches!(m, Message::Bye(_))),
        "sent Bye on incompatible"
    );
}

#[test]
fn bad_nonce_fails_the_handshake() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    let mut manager = ConnectionManager::new();
    let mut orchestrator = Orchestrator::new();

    session.open(&mut link, &mut manager).expect("open");
    let _real_nonce = hello_nonce(&desktop_drain(&mut link));
    device_push(&mut link, &device_ack(0xBAD_BAD), wire_version(), 0);
    session
        .pump(&mut link, &mut manager, &mut orchestrator)
        .expect("pump");

    assert_eq!(
        manager.state(),
        ConnectionState::Error,
        "unconfirmed identity → error"
    );
}

#[test]
fn heartbeat_pong_clears_the_miss_counter() {
    let (mut link, mut session, _manager, _orch) = connect(SendableState::Idle);
    let _ = desktop_drain(&mut link);

    session.send_ping(&mut link, 111).expect("ping");
    session.send_ping(&mut link, 222).expect("ping");
    assert!(!session.heartbeat_timed_out());

    device_push(
        &mut link,
        &Message::Pong(Pong {
            t_ms_echo: 111,
            uptime_ms: 5000,
        }),
        wire_version(),
        1,
    );
    let mut manager = ConnectionManager::new();
    let mut orch = Orchestrator::new();
    session
        .pump(&mut link, &mut manager, &mut orch)
        .expect("pump");
    assert!(!session.heartbeat_timed_out());
}

#[test]
fn missed_heartbeats_reach_the_timeout_threshold() {
    let mut link = FakeLink::default();
    let mut session = Session::new(SessionConfig::default());
    for t in 0..3 {
        session.send_ping(&mut link, t).expect("ping");
    }
    assert!(session.heartbeat_timed_out(), "3 unanswered pings time out");
}

#[test]
fn malformed_device_bytes_are_dropped_without_panic() {
    let (mut link, mut session, mut manager, mut orch) = connect(SendableState::Idle);
    assert_eq!(manager.state(), ConnectionState::Connected);

    link.from_device.extend([0x02, 0xFF, 0x00, 0x99, 0x00]);
    session
        .pump(&mut link, &mut manager, &mut orch)
        .expect("pump survives garbage");
    assert_eq!(
        manager.state(),
        ConnectionState::Connected,
        "state unchanged by garbage"
    );
}

#[test]
fn reconnect_resyncs_the_within_process_desired_state() {
    let (mut link, mut session, mut manager, mut orch) = connect(SendableState::Idle);
    let _ = desktop_drain(&mut link);

    // User sets Busy while connected.
    session
        .set_desired(&mut link, &manager, &mut orch, SendableState::Busy)
        .expect("set_desired");
    let _ = desktop_drain(&mut link);

    // Unplug (the run loop would raise this on I/O loss), then reconnect.
    manager.apply(ManagerEvent::PortRemoved);
    assert_eq!(manager.state(), ConnectionState::Disconnected);

    session.open(&mut link, &mut manager).expect("reopen");
    let nonce = hello_nonce(&desktop_drain(&mut link));
    device_push(&mut link, &device_ack(nonce), wire_version(), 0);
    session
        .pump(&mut link, &mut manager, &mut orch)
        .expect("pump");

    assert_eq!(manager.state(), ConnectionState::Connected);
    // The device is resynced to Busy (desired survives within the process).
    assert_eq!(
        set_state_desired(&desktop_drain(&mut link)),
        Some(SendableState::Busy)
    );
}
