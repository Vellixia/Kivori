#![cfg(feature = "host-sim")]

use kivori_firmware::input::quadrature::QuadratureDecoder;
use kivori_model::input::Direction;

/// Drive the decoder through a sequence of (a, b) levels, collecting emitted detents.
fn drive(seq: &[(bool, bool)]) -> Vec<Direction> {
    let mut d = QuadratureDecoder::new();
    let mut out = Vec::new();
    for &(a, b) in seq {
        if let Some(dir) = d.update(a, b) {
            out.push(dir);
        }
    }
    out
}

/// One full clockwise detent is the four-phase cycle 00 -> 01 -> 11 -> 10 -> 00.
const CW_CYCLE: [(bool, bool); 5] = [
    (false, false),
    (false, true),
    (true, true),
    (true, false),
    (false, false),
];

/// Counter-clockwise is the same cycle traversed in reverse.
const CCW_CYCLE: [(bool, bool); 5] = [
    (false, false),
    (true, false),
    (true, true),
    (false, true),
    (false, false),
];

#[test]
fn full_clockwise_cycle_emits_exactly_one_cw_detent() {
    assert_eq!(drive(&CW_CYCLE), vec![Direction::Cw]);
}

#[test]
fn full_counter_clockwise_cycle_emits_exactly_one_ccw_detent() {
    assert_eq!(drive(&CCW_CYCLE), vec![Direction::Ccw]);
}

#[test]
fn partial_motion_that_returns_emits_no_detent() {
    // Bounce walks to an adjacent Gray state and comes back without completing a detent.
    let seq = [
        (false, false),
        (false, true),
        (false, false),
        (false, true),
        (false, false),
    ];
    assert_eq!(drive(&seq), vec![]);
}

#[test]
fn repeated_identical_samples_emit_nothing() {
    let seq = [(false, false); 8];
    assert_eq!(drive(&seq), vec![]);
}

#[test]
fn illegal_double_bit_transition_is_counted_and_emits_no_detent() {
    let mut d = QuadratureDecoder::new();
    assert_eq!(d.update(false, false), None);
    // 00 -> 11 changes both bits at once: electrically impossible for a real detent.
    assert_eq!(d.update(true, true), None);
    assert_eq!(d.invalid_transitions(), 1);
}

#[test]
fn three_consecutive_cw_cycles_emit_three_cw_detents() {
    let mut seq = Vec::new();
    for _ in 0..3 {
        seq.extend_from_slice(&CW_CYCLE[1..]);
    }
    let mut full = vec![(false, false)];
    full.extend(seq);
    assert_eq!(
        drive(&full),
        vec![Direction::Cw, Direction::Cw, Direction::Cw]
    );
}

#[test]
fn reversal_mid_cycle_does_not_emit_a_detent() {
    // Advance two quarter-steps clockwise, then retreat to the start. No detent completed.
    let seq = [
        (false, false),
        (false, true),
        (true, true),
        (false, true),
        (false, false),
    ];
    assert_eq!(drive(&seq), vec![]);
}
