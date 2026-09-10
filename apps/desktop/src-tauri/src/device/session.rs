//! The desktop protocol session driver (Phase 11 host-testable core).
//!
//! `Session` is the synchronous, [`SerialLink`]-driven heart of the eventual async run loop: it opens
//! a session (`Hello`), verifies the device's `HelloAck`, drives the [`ConnectionManager`] from wire
//! events, resynchronizes the orchestrator's desired state on connect (FR-009), and answers/emits the
//! heartbeat. It performs no async or timing itself — the caller owns the clock and the task — so it is
//! fully host-testable against an in-memory link (and, in the E2E harness, against the real firmware
//! dispatcher). Malformed inbound frames are dropped without panicking (SC-008).

use crate::device::connection::{build_hello, summarize};
use crate::device::fsm::{ConnectionManager, ManagerEvent};
use crate::device::heartbeat::HeartbeatMonitor;
use crate::device::transport::SerialLink;
use crate::orchestrator::Orchestrator;
use kivori_model::{
    Capabilities, CompanionState, MascotAction, MascotPersonality, ProtocolVersion, SendableState,
};
use kivori_protocol::{
    decode_frame, decode_message, encode_message, evaluate_hello_ack, Bye, ByeReason,
    FirmwareVersion, HandshakeOutcome, Hello, MascotActionApplied, Message, Ping, PlayMascotAction,
    ProtoError, SeqClass, SequenceTracker, SetState, MAX_FRAME, MAX_WIRE, PROTOCOL_MAJOR,
    PROTOCOL_MINOR,
};

/// Static session parameters (the desktop's advertised identity + compatibility).
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// The desktop application version advertised in `Hello`.
    pub app_version: FirmwareVersion,
    /// The desktop's protocol version (carried in every frame header).
    pub protocol_version: ProtocolVersion,
    /// Capabilities the desktop advertises.
    pub capabilities: Capabilities,
    /// Protocol major versions the desktop supports.
    pub supported_majors: Vec<u16>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            app_version: FirmwareVersion {
                major: 0,
                minor: 0,
                patch: 0,
            },
            protocol_version: ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR),
            capabilities: Capabilities::MASCOT_INTERACTION,
            supported_majors: vec![PROTOCOL_MAJOR],
        }
    }
}

/// A session failure. Malformed inbound frames are handled internally (dropped), so only a transport
/// error is surfaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError<E> {
    /// The serial link read or write failed.
    Transport(E),
}

/// The desktop side of a device session.
pub struct Session {
    config: SessionConfig,
    tx_seq: u16,
    inbound: SequenceTracker,
    rx: Vec<u8>,
    sent_hello: Option<Hello>,
    next_nonce: u32,
    heartbeat: HeartbeatMonitor,
    reported: Option<CompanionState>,
    negotiated_caps: Capabilities,
    last_mascot_action_applied: Option<MascotActionApplied>,
    connection_generation: u32,
}

impl Session {
    /// Creates a session with the given configuration (nothing is sent until [`Session::open`]).
    #[must_use]
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            tx_seq: 0,
            inbound: SequenceTracker::new(),
            rx: Vec::new(),
            sent_hello: None,
            next_nonce: 1,
            heartbeat: HeartbeatMonitor::default(),
            reported: None,
            negotiated_caps: Capabilities::NONE,
            last_mascot_action_applied: None,
            connection_generation: 0,
        }
    }

    /// Opens a session on a freshly-connected port: marks the manager `Connecting` and sends `Hello`.
    ///
    /// # Errors
    /// [`SessionError::Transport`] if the write fails.
    pub fn open<L: SerialLink>(
        &mut self,
        link: &mut L,
        manager: &mut ConnectionManager,
    ) -> Result<(), SessionError<L::Error>> {
        manager.apply(ManagerEvent::PortOpened);
        self.connection_generation = self.connection_generation.wrapping_add(1).max(1);
        self.rx.clear();
        self.inbound = SequenceTracker::new();
        self.heartbeat = HeartbeatMonitor::default();
        self.reported = None;
        self.negotiated_caps = Capabilities::NONE;
        self.last_mascot_action_applied = None;
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.wrapping_add(1);
        let hello = build_hello(self.config.app_version, self.config.capabilities, nonce);
        self.sent_hello = Some(hello);
        self.send(link, &Message::Hello(hello))
    }

    /// The device's most recently reported companion state (`None` until the first `StateReport`).
    #[must_use]
    pub fn reported(&self) -> Option<CompanionState> {
        self.reported
    }

    /// Whether both peers negotiated transient mascot interactions for this connection.
    #[must_use]
    pub fn supports_mascot_interaction(&self) -> bool {
        self.negotiated_caps
            .contains(Capabilities::MASCOT_INTERACTION)
    }

    /// Most recent device acknowledgment for a social action in this connection.
    #[must_use]
    pub const fn last_mascot_action_applied(&self) -> Option<MascotActionApplied> {
        self.last_mascot_action_applied
    }

    /// Monotonic identity for the current within-process port session.
    #[must_use]
    pub const fn connection_generation(&self) -> u32 {
        self.connection_generation
    }

    /// Reads and handles all currently-available inbound frames, driving `manager`/`orchestrator` and
    /// auto-responding (`Ready` + a resync `SetState` on connect; heartbeat bookkeeping on `Pong`).
    ///
    /// # Errors
    /// [`SessionError::Transport`] if a read/write fails.
    pub fn pump<L: SerialLink>(
        &mut self,
        link: &mut L,
        manager: &mut ConnectionManager,
        orchestrator: &mut Orchestrator,
    ) -> Result<(), SessionError<L::Error>> {
        self.fill_rx(link)?;
        while let Some(pos) = self.rx.iter().position(|&b| b == 0) {
            let packet: Vec<u8> = self.rx[..pos].to_vec();
            self.rx.drain(..=pos);
            if !packet.is_empty() && packet.len() <= MAX_WIRE {
                self.handle(&packet, link, manager, orchestrator)?;
            }
        }
        Ok(())
    }

    /// Sets the desired companion state and, if the link is `Connected`, transmits it (FR-012).
    ///
    /// # Errors
    /// [`SessionError::Transport`] if the write fails.
    pub fn set_desired<L: SerialLink>(
        &mut self,
        link: &mut L,
        manager: &ConnectionManager,
        orchestrator: &mut Orchestrator,
        state: SendableState,
    ) -> Result<(), SessionError<L::Error>> {
        orchestrator.set_desired(state);
        if manager.state().can_drive_device() {
            self.transmit_set_state(link, state)?;
        }
        Ok(())
    }

    /// Sends one transient social reaction when the connection negotiated support.
    ///
    /// Returns `Ok(false)` without writing when disconnected or paired with older firmware.
    pub fn play_mascot_action<L: SerialLink>(
        &mut self,
        link: &mut L,
        manager: &ConnectionManager,
        action: MascotAction,
        personality: MascotPersonality,
        seed: u32,
    ) -> Result<bool, SessionError<L::Error>> {
        if !manager.state().can_drive_device() || !self.supports_mascot_interaction() {
            return Ok(false);
        }
        self.send(
            link,
            &Message::PlayMascotAction(PlayMascotAction {
                action,
                personality,
                seed,
            }),
        )?;
        Ok(true)
    }

    /// Sends a heartbeat `Ping` and records it as pending (see [`Session::heartbeat_timed_out`]).
    ///
    /// # Errors
    /// [`SessionError::Transport`] if the write fails.
    pub fn send_ping<L: SerialLink>(
        &mut self,
        link: &mut L,
        t_ms: u32,
    ) -> Result<(), SessionError<L::Error>> {
        self.heartbeat.on_ping_sent();
        self.send(link, &Message::Ping(Ping { t_ms }))
    }

    /// Whether the heartbeat has missed its threshold (the caller then raises `HeartbeatTimeout`).
    #[must_use]
    pub fn heartbeat_timed_out(&self) -> bool {
        self.heartbeat.timed_out()
    }

    fn handle<L: SerialLink>(
        &mut self,
        packet: &[u8],
        link: &mut L,
        manager: &mut ConnectionManager,
        orchestrator: &mut Orchestrator,
    ) -> Result<(), SessionError<L::Error>> {
        let mut scratch: heapless::Vec<u8, MAX_FRAME> = heapless::Vec::new();
        match decode_message(packet, &mut scratch, &self.config.supported_majors) {
            Ok((header, message)) => {
                if matches!(self.inbound.classify(header.seq), SeqClass::Duplicate) {
                    return Ok(());
                }
                self.handle_message(header.version, message, link, manager, orchestrator)?;
            }
            Err(ProtoError::UnsupportedVersion) => {
                // A supported-major gate rejected the frame. Read just the header to learn the
                // device's major and surface incompatibility (the payload layout is not trusted).
                let mut header_scratch: heapless::Vec<u8, MAX_FRAME> = heapless::Vec::new();
                if let Ok((header, _payload)) = decode_frame(packet, &mut header_scratch) {
                    self.inbound.classify(header.seq);
                    if manager.apply(ManagerEvent::HandshakeIncompatible {
                        device_major: header.version.major,
                    }) {
                        self.send(
                            link,
                            &Message::Bye(Bye {
                                reason: ByeReason::IncompatibleVersion,
                            }),
                        )?;
                    }
                }
            }
            Err(_) => {} // malformed — drop, never panic (SC-008)
        }
        Ok(())
    }

    fn handle_message<L: SerialLink>(
        &mut self,
        device_version: ProtocolVersion,
        message: Message,
        link: &mut L,
        manager: &mut ConnectionManager,
        orchestrator: &mut Orchestrator,
    ) -> Result<(), SessionError<L::Error>> {
        match message {
            Message::HelloAck(ack) => {
                let Some(sent) = self.sent_hello else {
                    return Ok(()); // unsolicited HelloAck — ignore
                };
                match evaluate_hello_ack(
                    &sent,
                    &ack,
                    device_version,
                    self.config.protocol_version,
                    &self.config.supported_majors,
                ) {
                    HandshakeOutcome::Compatible(ready) => {
                        manager.apply(ManagerEvent::HandshakeOk(summarize(&ack, device_version)));
                        self.negotiated_caps = ready.negotiated_caps;
                        self.send(link, &Message::Ready(ready))?;
                        // Resynchronize the device to our desired state on (re)connect (FR-009).
                        let desired = orchestrator.resync_state();
                        self.transmit_set_state(link, desired)?;
                    }
                    HandshakeOutcome::Incompatible { device_major } => {
                        if manager.apply(ManagerEvent::HandshakeIncompatible { device_major }) {
                            self.send(
                                link,
                                &Message::Bye(Bye {
                                    reason: ByeReason::IncompatibleVersion,
                                }),
                            )?;
                        }
                    }
                    HandshakeOutcome::BadNonce => {
                        // Identity not confirmed — treat as a failed handshake.
                        manager.apply(ManagerEvent::HandshakeTimeout);
                    }
                }
            }
            Message::Pong(_) => self.heartbeat.on_pong(),
            Message::StateReport(report) => self.reported = Some(report.reported),
            Message::MascotActionApplied(applied) => {
                self.last_mascot_action_applied = Some(applied);
            }
            // `Ready` and the remaining device→desktop kinds are observed by the UI layer, not here.
            _ => {}
        }
        Ok(())
    }

    fn transmit_set_state<L: SerialLink>(
        &mut self,
        link: &mut L,
        desired: SendableState,
    ) -> Result<(), SessionError<L::Error>> {
        self.send(
            link,
            &Message::SetState(SetState {
                desired,
                at_ms: None,
            }),
        )
    }

    fn send<L: SerialLink>(
        &mut self,
        link: &mut L,
        message: &Message,
    ) -> Result<(), SessionError<L::Error>> {
        let mut wire: heapless::Vec<u8, MAX_WIRE> = heapless::Vec::new();
        // Encoding only fails on an over-capacity payload, which our fixed messages never hit.
        if encode_message(
            message,
            self.config.protocol_version,
            self.tx_seq,
            &mut wire,
        )
        .is_ok()
        {
            self.tx_seq = self.tx_seq.wrapping_add(1);
            let mut sent = 0;
            while sent < wire.len() {
                let n = link.write(&wire[sent..]).map_err(SessionError::Transport)?;
                if n == 0 {
                    break; // link full; best-effort (the real adapter buffers)
                }
                sent += n;
            }
        }
        Ok(())
    }

    fn fill_rx<L: SerialLink>(&mut self, link: &mut L) -> Result<(), SessionError<L::Error>> {
        let mut chunk = [0u8; 256];
        loop {
            let n = link.read(&mut chunk).map_err(SessionError::Transport)?;
            if n == 0 {
                return Ok(());
            }
            self.rx.extend_from_slice(&chunk[..n]);
        }
    }
}
