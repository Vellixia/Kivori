//! T033 — capability negotiation: the negotiated set is the intersection; a capability only one
//! side advertises is dropped.

use kivori_model::Capabilities;
use kivori_protocol::negotiate;

#[test]
fn negotiated_caps_are_the_intersection() {
    let (minor, caps) = negotiate(
        3,
        1,
        Capabilities::from_bits(0b1011),
        Capabilities::from_bits(0b0110),
    );
    assert_eq!(minor, 1);
    assert_eq!(caps, Capabilities::from_bits(0b0010));
}

#[test]
fn capability_only_one_side_advertises_is_dropped() {
    let (_minor, caps) = negotiate(
        0,
        0,
        Capabilities::from_bits(0b001), // desktop only
        Capabilities::from_bits(0b100), // device only
    );
    assert!(!caps.contains(Capabilities::from_bits(0b001)));
    assert!(!caps.contains(Capabilities::from_bits(0b100)));
    assert!(caps.is_empty());
}

#[test]
fn full_overlap_is_preserved() {
    let both = Capabilities::from_bits(0b1010);
    let (_minor, caps) = negotiate(2, 2, both, both);
    assert_eq!(caps, both);
}
