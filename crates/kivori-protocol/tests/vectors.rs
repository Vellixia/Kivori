//! T035 — golden protocol vectors: deterministic encoding + committed canonical postcard payload
//! byte sequences (CRC/COBS validated by successful decode).

use heapless::Vec;
use kivori_model::{CompanionState, ProtocolVersion, SendableState};
use kivori_protocol::{
    decode_frame, encode_message, Message, Ping, SetState, StateReport, MAGIC, MAX_FRAME, MAX_WIRE,
};

fn encode(msg: &Message, seq: u16) -> Vec<u8, MAX_WIRE> {
    let mut w: Vec<u8, MAX_WIRE> = Vec::new();
    encode_message(msg, ProtocolVersion::new(1, 0), seq, &mut w).unwrap();
    w
}

#[test]
fn encoding_is_deterministic() {
    let msg = Message::Ping(Ping { t_ms: 1 });
    assert_eq!(encode(&msg, 1), encode(&msg, 1));
}

#[test]
fn magic_wire_bytes_are_stable() {
    assert_eq!(MAGIC.to_le_bytes(), [0x56, 0x4B]);
}

#[test]
fn canonical_postcard_payload_vectors() {
    // The payload bytes are the canonical postcard encoding (variant index varints + fields).
    let cases: [(Message, &[u8]); 3] = [
        // Message::Ping (variant 6) { t_ms: 1 } -> [0x06, 0x01]
        (Message::Ping(Ping { t_ms: 1 }), &[0x06, 0x01]),
        // Message::SetState (variant 4) { Idle (0), None (0) } -> [0x04, 0x00, 0x00]
        (
            Message::SetState(SetState {
                desired: SendableState::Idle,
                at_ms: None,
            }),
            &[0x04, 0x00, 0x00],
        ),
        // Message::StateReport (variant 5) { Booting (0), elapsed_ms 0 } -> [0x05, 0x00, 0x00]
        (
            Message::StateReport(StateReport {
                reported: CompanionState::Booting,
                elapsed_ms: 0,
            }),
            &[0x05, 0x00, 0x00],
        ),
    ];
    for (msg, expected_payload) in cases {
        let wire = encode(&msg, 3);
        assert_eq!(*wire.last().unwrap(), 0);
        let packet = &wire[..wire.len() - 1];
        let mut scratch: Vec<u8, MAX_FRAME> = Vec::new();
        let (header, payload) = decode_frame(packet, &mut scratch).unwrap();
        assert_eq!(header.version, ProtocolVersion::new(1, 0));
        assert_eq!(header.seq, 3);
        assert_eq!(payload, expected_payload, "canonical payload for {msg:?}");
    }
}
