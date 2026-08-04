//! T034 — malformed frames: arbitrary bytes never panic; the decoder recovers after a malformed
//! frame followed by a valid one (SC-008).

use heapless::Vec;
use kivori_model::{ProtocolVersion, SendableState};
use kivori_protocol::{decode_message, encode_message, Message, SetState, MAX_FRAME, MAX_WIRE};

#[test]
fn arbitrary_byte_streams_never_panic() {
    let mut scratch: Vec<u8, MAX_FRAME> = Vec::new();
    let mut seed: u32 = 0x1234_5678;
    for len in 0..400usize {
        let mut buf: Vec<u8, 600> = Vec::new();
        for _ in 0..len {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let _ = buf.push((seed >> 24) as u8);
        }
        // Must return a typed result (Err, or Ok for a coincidentally-valid frame) — never panic.
        let _ = decode_message(&buf, &mut scratch, &[1]);
    }
}

#[test]
fn recovers_after_a_malformed_then_valid_frame() {
    let mut scratch: Vec<u8, MAX_FRAME> = Vec::new();
    // Malformed COBS packet: code 0x05 claims 4 following bytes but only 2 are present.
    assert!(decode_message(&[0x05, 0xAA, 0xBB], &mut scratch, &[1]).is_err());

    // A subsequent valid packet decodes correctly — the decoder holds no poisoned state.
    let msg = Message::SetState(SetState {
        desired: SendableState::Happy,
        at_ms: None,
    });
    let mut wire: Vec<u8, MAX_WIRE> = Vec::new();
    encode_message(&msg, ProtocolVersion::new(1, 0), 1, &mut wire).unwrap();
    let packet = &wire[..wire.len() - 1];
    let (_header, decoded) = decode_message(packet, &mut scratch, &[1]).unwrap();
    assert_eq!(decoded, msg);
}
