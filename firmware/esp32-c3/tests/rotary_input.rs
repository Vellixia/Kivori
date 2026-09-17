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

use kivori_firmware::input::gesture::{RotaryEvent, RotaryGesture};

const GESTURE_END_MS: u32 = 250;

#[test]
fn first_detent_opens_a_gesture_and_reports_the_detent() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    let (started, detent) = g.on_detent(Direction::Cw, 1_000);
    assert_eq!(started, Some(RotaryEvent::GestureStarted { gesture_id: 1 }));
    assert_eq!(
        detent,
        RotaryEvent::Detent {
            gesture_id: 1,
            direction: Direction::Cw
        }
    );
}

#[test]
fn detents_inside_the_window_stay_in_one_gesture() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    let (started, _) = g.on_detent(Direction::Cw, 1_000);
    assert!(started.is_some());

    // 249 ms later: still the same gesture, so no new GestureStarted.
    let (started, detent) = g.on_detent(Direction::Cw, 1_249);
    assert_eq!(started, None);
    assert_eq!(
        detent,
        RotaryEvent::Detent {
            gesture_id: 1,
            direction: Direction::Cw
        }
    );
}

#[test]
fn gesture_ends_after_the_inactivity_window() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    g.on_detent(Direction::Cw, 1_000);

    assert_eq!(
        g.poll(1_249),
        None,
        "must not end before the window elapses"
    );
    assert_eq!(
        g.poll(1_250),
        Some(RotaryEvent::GestureEnded { gesture_id: 1 })
    );
    assert_eq!(g.poll(1_500), None, "GestureEnded is emitted exactly once");
}

#[test]
fn a_detent_after_the_window_opens_a_new_gesture_id() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    g.on_detent(Direction::Cw, 1_000);
    assert_eq!(
        g.poll(1_250),
        Some(RotaryEvent::GestureEnded { gesture_id: 1 })
    );

    let (started, detent) = g.on_detent(Direction::Ccw, 2_000);
    assert_eq!(started, Some(RotaryEvent::GestureStarted { gesture_id: 2 }));
    assert_eq!(
        detent,
        RotaryEvent::Detent {
            gesture_id: 2,
            direction: Direction::Ccw
        }
    );
}

#[test]
fn reversal_within_a_gesture_does_not_split_the_gesture() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    g.on_detent(Direction::Cw, 1_000);
    let (started, detent) = g.on_detent(Direction::Ccw, 1_100);
    assert_eq!(started, None, "reversal is not a new gesture");
    assert_eq!(
        detent,
        RotaryEvent::Detent {
            gesture_id: 1,
            direction: Direction::Ccw
        }
    );
}

#[test]
fn reset_closes_the_gesture_silently_and_restarts_numbering() {
    let mut g = RotaryGesture::new(GESTURE_END_MS);
    g.on_detent(Direction::Cw, 1_000);
    g.reset();
    assert_eq!(g.poll(5_000), None, "a reset gesture emits no GestureEnded");

    let (started, _) = g.on_detent(Direction::Cw, 6_000);
    assert_eq!(started, Some(RotaryEvent::GestureStarted { gesture_id: 1 }));
}

use kivori_firmware::sim::ScriptedInput;
use kivori_model::input::InputLevels;

fn lv(a: bool, b: bool) -> InputLevels {
    InputLevels { a, b, sw: false }
}

#[test]
fn scripted_input_source_replays_levels_then_holds_the_last() {
    use kivori_firmware::ports::InputSource;

    let mut src = ScriptedInput::new(vec![lv(false, false), lv(false, true)]);
    assert_eq!(src.sample(), lv(false, false));
    assert_eq!(src.sample(), lv(false, true));
    // Exhausted scripts hold the final level rather than wrapping or panicking.
    assert_eq!(src.sample(), lv(false, true));
}

#[test]
fn a_full_cw_cycle_through_the_port_produces_started_detent_ended() {
    use kivori_firmware::sim::{drive_rotary, SeenInput};

    // Host-sim end-to-end: scripted levels -> validated detent -> emitted event stream.
    let levels = vec![
        lv(false, false),
        lv(false, true),
        lv(true, true),
        lv(true, false),
        lv(false, false),
    ];

    assert_eq!(
        drive_rotary(levels),
        vec![
            SeenInput::GestureStarted { gesture_id: 1 },
            SeenInput::Detent {
                gesture_id: 1,
                direction: Direction::Cw
            },
            SeenInput::GestureEnded { gesture_id: 1 },
        ]
    );
}

#[test]
fn a_reversal_stays_in_one_gesture_and_reports_both_directions() {
    use kivori_firmware::sim::{drive_rotary, SeenInput};

    let mut levels = vec![
        // one CW detent
        lv(false, false),
        lv(false, true),
        lv(true, true),
        lv(true, false),
        lv(false, false),
    ];
    // then one CCW detent, back the way it came
    levels.extend_from_slice(&[
        lv(true, false),
        lv(true, true),
        lv(false, true),
        lv(false, false),
    ]);

    assert_eq!(
        drive_rotary(levels),
        vec![
            SeenInput::GestureStarted { gesture_id: 1 },
            SeenInput::Detent {
                gesture_id: 1,
                direction: Direction::Cw
            },
            SeenInput::Detent {
                gesture_id: 1,
                direction: Direction::Ccw
            },
            SeenInput::GestureEnded { gesture_id: 1 },
        ]
    );
}

#[test]
fn the_dispatcher_records_the_accepted_session_nonce() {
    // Uses the existing host-sim handshake helper pattern from
    // `firmware/esp32-c3/tests/host_sim.rs`: drive Hello -> HelloAck -> Ready, then assert
    // the dispatcher retained the nonce it accepted.
    let d = kivori_firmware::sim::handshaken_dispatcher(0x1234_5678);
    assert_eq!(d.accepted_session(), Some(0x1234_5678));
}

// --- capability gating (controller decision 1): send_input_event must stay inert unless BOTH a
// session has been accepted AND the capability was negotiated. Each half is proven independently
// so a shortcut implementation (e.g. gating on session alone) cannot pass by accident.

use heapless::Vec as HVec;
use kivori_firmware::proto::{DeviceIdentity, Dispatcher};
use kivori_firmware::sim::{handshaken_dispatcher as handshaken, SimPipe};
use kivori_firmware::state::DeviceState;
use kivori_model::{Capabilities, ProtocolVersion};
use kivori_protocol::{
    decode_message, encode_message, ControlId, FirmwareVersion, Hello, InputKind, Message, Ready,
    MAX_FRAME, MAX_WIRE, PROTOCOL_MAJOR, PROTOCOL_MINOR,
};

fn gating_identity() -> DeviceIdentity {
    DeviceIdentity {
        device_id: [0xCD; 16],
        firmware_version: FirmwareVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        capabilities: Capabilities::PHYSICAL_INPUT_V1,
    }
}

fn gating_wire_version() -> ProtocolVersion {
    ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR)
}

/// Host -> device: frames `msg` with sequence `seq` onto the pipe (mirrors `tests/host_sim.rs`).
fn gating_host_write(pipe: &mut SimPipe, msg: &Message, seq: u16) {
    let mut wire: HVec<u8, MAX_WIRE> = HVec::new();
    encode_message(msg, gating_wire_version(), seq, &mut wire).expect("encode");
    pipe.host_send(&wire).expect("pipe has capacity");
}

/// Decodes every complete device -> host frame currently queued (mirrors `tests/host_sim.rs`).
fn gating_host_drain(pipe: &mut SimPipe) -> Vec<Message> {
    let bytes = pipe.host_recv();
    let mut messages = Vec::new();
    let mut scratch: HVec<u8, MAX_FRAME> = HVec::new();
    for packet in bytes.split(|&b| b == 0) {
        if packet.is_empty() {
            continue;
        }
        if let Ok((_, msg)) = decode_message(packet, &mut scratch, &[PROTOCOL_MAJOR]) {
            messages.push(msg);
        }
    }
    messages
}

#[test]
fn send_input_event_is_inert_without_a_negotiated_capability() {
    // Session accepted (Hello answered) but Ready never negotiated PHYSICAL_INPUT_V1: the
    // capability gate — not just the session gate — must be what stops emission.
    let mut pipe = SimPipe::new();
    let mut device = DeviceState::new();
    let mut dispatcher = Dispatcher::new(gating_identity());

    gating_host_write(
        &mut pipe,
        &Message::Hello(Hello {
            desktop_version: FirmwareVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            desktop_caps: Capabilities::NONE,
            nonce: 0xAAAA_BBBB,
        }),
        0,
    );
    dispatcher.poll(&mut pipe, &mut device, 0).expect("poll");
    assert_eq!(dispatcher.accepted_session(), Some(0xAAAA_BBBB));
    let _ = pipe.host_recv(); // discard the HelloAck; only the gate under test matters here

    let sent = dispatcher.send_input_event(&mut pipe, 1, InputKind::GestureStarted, 0);
    assert!(!sent, "an unnegotiated capability must stay inert");
    assert!(
        gating_host_drain(&mut pipe).is_empty(),
        "nothing may reach the wire"
    );
}

#[test]
fn send_input_event_is_inert_without_an_accepted_session() {
    // Ready negotiates the capability, but Hello/HelloAck never happened: no session nonce was
    // ever accepted, so emission must still stay inert.
    let mut pipe = SimPipe::new();
    let mut device = DeviceState::new();
    let mut dispatcher = Dispatcher::new(gating_identity());

    gating_host_write(
        &mut pipe,
        &Message::Ready(Ready {
            negotiated_minor: 0,
            negotiated_caps: Capabilities::PHYSICAL_INPUT_V1,
        }),
        0,
    );
    dispatcher.poll(&mut pipe, &mut device, 0).expect("poll");
    assert_eq!(dispatcher.accepted_session(), None);

    let sent = dispatcher.send_input_event(&mut pipe, 1, InputKind::GestureStarted, 0);
    assert!(!sent, "no accepted session must stay inert");
    assert!(gating_host_drain(&mut pipe).is_empty());
}

#[test]
fn send_input_event_emits_on_the_wire_once_negotiated_and_accepted() {
    let mut pipe = SimPipe::new();
    let mut dispatcher = handshaken(0x1111_2222);

    let sent = dispatcher.send_input_event(&mut pipe, 7, InputKind::Detent(Direction::Cw), 42);
    assert!(
        sent,
        "a negotiated capability and accepted session must emit"
    );

    match gating_host_drain(&mut pipe).as_slice() {
        [Message::InputEvent(event)] => {
            assert_eq!(event.session, 0x1111_2222);
            assert_eq!(event.gesture_id, 7);
            assert_eq!(event.control, ControlId::Rotary);
            assert_eq!(event.kind, InputKind::Detent(Direction::Cw));
            assert_eq!(event.device_ms, 42);
        }
        other => panic!("expected a single InputEvent, got {other:?}"),
    }
}

// --- session-boundary reset (controller decision 4): a gesture opened in one session must not
// be completable, or silently continued, in the next one. Exercised through the real `Runtime`
// (not just the decoder/gesture types directly), since the reset is wired in `Runtime::step`.

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_firmware::runtime::{Runtime, RuntimeConfig};
use kivori_firmware::sim::{CaptureDisplay, VirtualClock};
use kivori_protocol::{Bye, ByeReason};

/// One full CW detent cycle as a level script: 00 -> 01 -> 11 -> 10 -> 00.
fn cw_cycle_levels() -> Vec<InputLevels> {
    vec![
        lv(false, false),
        lv(false, true),
        lv(true, true),
        lv(true, false),
        lv(false, false),
    ]
}

#[test]
fn a_gesture_open_before_bye_cannot_be_silently_continued_after_reconnecting() {
    let mut runtime = Runtime::new(gating_identity(), RuntimeConfig::default());
    let clock = VirtualClock::new();
    let mut pipe = SimPipe::new();
    let mut display = CaptureDisplay::new();
    let blob_bytes = compile_default_blob();
    let blob = AssetBlob::parse(&blob_bytes).expect("valid blob");
    let mut idle = ScriptedInput::new(vec![lv(false, false)]);

    // Session A: handshake, negotiating PHYSICAL_INPUT_V1.
    gating_host_write(
        &mut pipe,
        &Message::Hello(Hello {
            desktop_version: FirmwareVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            desktop_caps: Capabilities::PHYSICAL_INPUT_V1,
            nonce: 0xA000_0001,
        }),
        0,
    );
    runtime.step(&clock, &mut pipe, &mut idle, &mut display, &blob);
    gating_host_write(
        &mut pipe,
        &Message::Ready(Ready {
            negotiated_minor: 0,
            negotiated_caps: Capabilities::PHYSICAL_INPUT_V1,
        }),
        1,
    );
    runtime.step(&clock, &mut pipe, &mut idle, &mut display, &blob);
    let _ = pipe.host_recv();

    // One full CW detent opens gesture 1 in session A. Well inside the 250ms window, so it stays
    // open (no `GestureEnded` yet) when the session closes.
    let mut cw = ScriptedInput::new(cw_cycle_levels());
    for _ in 0..5 {
        clock.advance(1);
        runtime.step(&clock, &mut pipe, &mut cw, &mut display, &blob);
    }
    let opened_in_a = gating_host_drain(&mut pipe).into_iter().any(|m| {
        matches!(
            m,
            Message::InputEvent(e) if e.session == 0xA000_0001
                && matches!(e.kind, InputKind::GestureStarted)
        )
    });
    assert!(
        opened_in_a,
        "the first detent must open a gesture in session A"
    );

    // `Bye` closes session A while the gesture is still open.
    gating_host_write(
        &mut pipe,
        &Message::Bye(Bye {
            reason: ByeReason::Shutdown,
        }),
        2,
    );
    runtime.step(&clock, &mut pipe, &mut idle, &mut display, &blob);
    let _ = pipe.host_recv();

    // Session B: a fresh handshake with a different nonce.
    gating_host_write(
        &mut pipe,
        &Message::Hello(Hello {
            desktop_version: FirmwareVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            desktop_caps: Capabilities::PHYSICAL_INPUT_V1,
            nonce: 0xB000_0002,
        }),
        3,
    );
    runtime.step(&clock, &mut pipe, &mut idle, &mut display, &blob);
    gating_host_write(
        &mut pipe,
        &Message::Ready(Ready {
            negotiated_minor: 0,
            negotiated_caps: Capabilities::PHYSICAL_INPUT_V1,
        }),
        4,
    );
    runtime.step(&clock, &mut pipe, &mut idle, &mut display, &blob);
    let _ = pipe.host_recv();

    // A second full CW detent in session B. Without the reset, `RotaryGesture` would still think
    // gesture 1 from session A is open and would silently continue it (no `GestureStarted`) — a
    // gesture from the old session leaking into the new one.
    let mut cw2 = ScriptedInput::new(cw_cycle_levels());
    for _ in 0..5 {
        clock.advance(1);
        runtime.step(&clock, &mut pipe, &mut cw2, &mut display, &blob);
    }
    let opened_fresh_in_b = gating_host_drain(&mut pipe).into_iter().any(|m| {
        matches!(
            m,
            Message::InputEvent(e) if e.session == 0xB000_0002
                && matches!(e.kind, InputKind::GestureStarted)
        )
    });
    assert!(
        opened_fresh_in_b,
        "a fresh GestureStarted must fire in session B; the old session's open gesture must not \
         silently continue across the boundary"
    );
}
