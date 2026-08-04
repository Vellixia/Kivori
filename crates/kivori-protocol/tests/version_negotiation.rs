//! T032 — version negotiation: incompatible major, compatible minor difference, bad nonce.

use kivori_model::{Capabilities, ProtocolVersion};
use kivori_protocol::{
    evaluate_hello_ack, FirmwareVersion, HandshakeOutcome, Hello, HelloAck, Ready,
};

fn hello(nonce: u32, caps: Capabilities) -> Hello {
    Hello {
        desktop_version: FirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        desktop_caps: caps,
        nonce,
    }
}

fn ack(nonce_echo: u32, caps: Capabilities) -> HelloAck {
    HelloAck {
        device_caps: caps,
        device_id: [0u8; 16],
        firmware_version: FirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        nonce_echo,
    }
}

#[test]
fn incompatible_major_is_reported() {
    let out = evaluate_hello_ack(
        &hello(1, Capabilities::NONE),
        &ack(1, Capabilities::NONE),
        ProtocolVersion::new(2, 0),
        ProtocolVersion::new(1, 3),
        &[1],
    );
    assert_eq!(out, HandshakeOutcome::Incompatible { device_major: 2 });
}

#[test]
fn compatible_minor_difference_negotiates_min_and_caps() {
    let out = evaluate_hello_ack(
        &hello(9, Capabilities::from_bits(0b110)),
        &ack(9, Capabilities::from_bits(0b011)),
        ProtocolVersion::new(1, 5), // device minor 5
        ProtocolVersion::new(1, 2), // desktop minor 2
        &[1],
    );
    assert_eq!(
        out,
        HandshakeOutcome::Compatible(Ready {
            negotiated_minor: 2,
            negotiated_caps: Capabilities::from_bits(0b010),
        })
    );
}

#[test]
fn wrong_nonce_is_rejected() {
    let out = evaluate_hello_ack(
        &hello(1, Capabilities::NONE),
        &ack(999, Capabilities::NONE),
        ProtocolVersion::new(1, 0),
        ProtocolVersion::new(1, 0),
        &[1],
    );
    assert_eq!(out, HandshakeOutcome::BadNonce);
}
