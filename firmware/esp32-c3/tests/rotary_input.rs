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

#[test]
fn invalid_transition_discards_quarter_steps_banked_before_it() {
    // 00 -> 01 -> 11 banks two clockwise quarter-steps, then 11 -> 00 is an illegal
    // double-bit transition. Two more clockwise quarter-steps follow. If the banked
    // steps survived the discontinuity they would splice with the steps after it
    // (2 + 2 = 4) and wrongly complete a detent; invariant 42 / R-77 / R-82 require
    // that a detent only ever be emitted for a continuous, fully observed traversal.
    let mut d = QuadratureDecoder::new();
    let mut out = Vec::new();
    for &(a, b) in &[
        (false, false),
        (false, true),
        (true, true),
        (false, false), // illegal: 11 -> 00, both bits change
        (false, true),
        (true, true),
    ] {
        if let Some(dir) = d.update(a, b) {
            out.push(dir);
        }
    }
    assert_eq!(out, vec![]);
    assert_eq!(d.invalid_transitions(), 1);
}

#[test]
fn detent_completes_normally_immediately_after_an_invalid_transition() {
    // After the same illegal 11 -> 00 discontinuity used above, a full, continuous
    // clockwise cycle should still complete cleanly — an invalid transition must not
    // wedge the decoder against ever emitting again.
    let seq = [
        (false, false),
        (false, true),
        (true, true),
        (false, false), // illegal: 11 -> 00, both bits change
        (false, true),
        (true, true),
        (true, false),
        (false, false),
    ];
    assert_eq!(drive(&seq), vec![Direction::Cw]);
}

#[test]
fn reset_drops_both_phase_and_accumulator() {
    // Bank three of the four quarter-steps of a clockwise detent, then reset. If
    // `reset` failed to clear the accumulator, the very next quarter-step after the
    // reset would splice with the stale count and complete a detent one step early.
    let mut d = QuadratureDecoder::new();
    let mut out = Vec::new();

    for &(a, b) in &[
        (false, false), // establish phase 00
        (false, true),  // 01, acc = 1
        (true, true),   // 11, acc = 2
        (true, false),  // 10, acc = 3 (one shy of a detent)
    ] {
        if let Some(dir) = d.update(a, b) {
            out.push(dir);
        }
    }

    d.reset();

    // Re-establishes phase (must not be treated as a transition from the pre-reset
    // phase), then one more quarter-step. If the accumulator had leaked across reset
    // (stale 3 + 1), this would wrongly complete a detent here already.
    for &(a, b) in &[(false, false), (false, true)] {
        if let Some(dir) = d.update(a, b) {
            out.push(dir);
        }
    }
    assert_eq!(out, vec![], "reset must have dropped the stale accumulator");

    // The decoder must not be wedged: a genuine full cycle from here still completes.
    for &(a, b) in &[(true, true), (true, false), (false, false)] {
        if let Some(dir) = d.update(a, b) {
            out.push(dir);
        }
    }
    assert_eq!(out, vec![Direction::Cw]);
}
