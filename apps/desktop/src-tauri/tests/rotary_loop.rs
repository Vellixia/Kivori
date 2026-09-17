use kivori_desktop::input::{InputIngress, LogicalInput, RejectReason};
use kivori_model::input::Direction;
use kivori_protocol::message::{ControlId, InputEvent, InputKind};

fn ev(session: u32, gesture_id: u16, kind: InputKind) -> InputEvent {
    InputEvent {
        session,
        gesture_id,
        control: ControlId::Rotary,
        kind,
        device_ms: 0,
    }
}

#[test]
fn a_started_gesture_admits_its_detents() {
    let mut ingress = InputIngress::new();
    ingress.begin_session(0xAAAA);

    assert_eq!(
        ingress.accept(&ev(0xAAAA, 1, InputKind::GestureStarted)),
        Ok(Some(LogicalInput::GestureStarted { gesture_id: 1 }))
    );
    assert_eq!(
        ingress.accept(&ev(0xAAAA, 1, InputKind::Detent(Direction::Cw))),
        Ok(Some(LogicalInput::Detent {
            gesture_id: 1,
            direction: Direction::Cw
        }))
    );
    assert_eq!(
        ingress.accept(&ev(0xAAAA, 1, InputKind::GestureEnded)),
        Ok(Some(LogicalInput::GestureEnded { gesture_id: 1 }))
    );
}

#[test]
fn a_detent_for_a_gesture_that_never_started_is_rejected() {
    let mut ingress = InputIngress::new();
    ingress.begin_session(0xAAAA);

    assert_eq!(
        ingress.accept(&ev(0xAAAA, 7, InputKind::Detent(Direction::Cw))),
        Err(RejectReason::UnknownGesture)
    );
}

/// THE ADVERSARIAL CASE that removed the epoch-free design.
/// A COMPLETE stale pair — GestureStarted AND Detent — replayed after a reconnect.
#[test]
fn a_complete_stale_gesture_pair_after_reconnect_executes_nothing() {
    let mut ingress = InputIngress::new();

    ingress.begin_session(0x1111);
    ingress
        .accept(&ev(0x1111, 1, InputKind::GestureStarted))
        .unwrap();

    // Link drops, a new session is established with a different nonce.
    ingress.end_session();
    ingress.begin_session(0x2222);

    // Both halves of the old gesture arrive, in order, from the stale buffer.
    assert_eq!(
        ingress.accept(&ev(0x1111, 1, InputKind::GestureStarted)),
        Err(RejectReason::StaleSession)
    );
    assert_eq!(
        ingress.accept(&ev(0x1111, 1, InputKind::Detent(Direction::Cw))),
        Err(RejectReason::StaleSession)
    );
}

#[test]
fn gesture_ids_reused_by_a_new_session_do_not_inherit_old_state() {
    let mut ingress = InputIngress::new();

    ingress.begin_session(0x1111);
    ingress
        .accept(&ev(0x1111, 1, InputKind::GestureStarted))
        .unwrap();

    ingress.end_session();
    ingress.begin_session(0x2222);

    // Same gesture id, new session: the open-gesture set was cleared, so a bare
    // detent has no start to attach to.
    assert_eq!(
        ingress.accept(&ev(0x2222, 1, InputKind::Detent(Direction::Cw))),
        Err(RejectReason::UnknownGesture)
    );
}

#[test]
fn input_outside_a_session_is_rejected() {
    let mut ingress = InputIngress::new();
    assert_eq!(
        ingress.accept(&ev(0x1111, 1, InputKind::GestureStarted)),
        Err(RejectReason::NoSession)
    );
}

#[test]
fn a_gesture_cannot_continue_after_it_ended() {
    let mut ingress = InputIngress::new();
    ingress.begin_session(0xAAAA);
    ingress
        .accept(&ev(0xAAAA, 1, InputKind::GestureStarted))
        .unwrap();
    ingress
        .accept(&ev(0xAAAA, 1, InputKind::GestureEnded))
        .unwrap();

    assert_eq!(
        ingress.accept(&ev(0xAAAA, 1, InputKind::Detent(Direction::Cw))),
        Err(RejectReason::UnknownGesture)
    );
}

use kivori_desktop::action::volume::{apply_step, BASE_STEP_PERCENT};
use kivori_desktop::action::{resolve_binding, ActionId, Outcome};
use kivori_desktop::platform::{ActionAvailability, FakeVolumeBackend};

#[test]
fn the_global_rotary_binding_resolves_to_master_volume() {
    assert_eq!(
        resolve_binding(ControlId::Rotary),
        Some(ActionId::MasterVolume)
    );
}

#[test]
fn one_detent_moves_exactly_the_base_step() {
    assert_eq!(
        apply_step(50, Direction::Cw),
        (50 + BASE_STEP_PERCENT, false)
    );
    assert_eq!(
        apply_step(50, Direction::Ccw),
        (50 - BASE_STEP_PERCENT, false)
    );
}

#[test]
fn values_clamp_at_both_bounds_and_flag_the_boundary() {
    assert_eq!(apply_step(100, Direction::Cw), (100, true));
    assert_eq!(apply_step(0, Direction::Ccw), (0, true));
    // Approaching the bound from inside one step lands exactly on it, not past it.
    assert_eq!(apply_step(99, Direction::Cw), (100, false));
    assert_eq!(apply_step(1, Direction::Ccw), (0, false));
}

#[test]
fn the_first_reverse_detent_leaves_the_boundary_immediately() {
    let (at_max, boundary) = apply_step(100, Direction::Cw);
    assert!(boundary);
    assert_eq!(
        apply_step(at_max, Direction::Ccw),
        (100 - BASE_STEP_PERCENT, false)
    );
}

#[test]
fn a_successful_set_with_readback_is_state_confirmed() {
    let backend = FakeVolumeBackend::new(40);
    let outcome = kivori_desktop::action::execute_volume(&backend, 60);
    assert_eq!(outcome, Outcome::StateConfirmed { volume_percent: 60 });
}

#[test]
fn state_confirmed_reports_the_observed_value_not_the_requested_one() {
    let backend = FakeVolumeBackend::quantised(40, 5);
    let outcome = kivori_desktop::action::execute_volume(&backend, 62);
    assert_eq!(
        outcome,
        Outcome::StateConfirmed { volume_percent: 60 },
        "confirmation must carry OS truth, never the request"
    );
}

#[test]
fn a_write_without_readback_is_unverified_never_confirmed() {
    let backend = FakeVolumeBackend::unreadable_after_write(30);
    assert_eq!(
        kivori_desktop::action::execute_volume(&backend, 40),
        Outcome::TriggeredUnverified
    );
}

#[test]
fn an_unavailable_backend_fails_rather_than_silently_succeeding() {
    let backend = FakeVolumeBackend::with_availability(ActionAvailability::RuntimeUnavailable {
        reason: "no default render endpoint".to_string(),
    });
    assert!(matches!(
        kivori_desktop::action::execute_volume(&backend, 40),
        Outcome::Failed { .. }
    ));
}

#[test]
fn an_unimplemented_backend_does_not_attempt_execution() {
    let backend = kivori_desktop::platform::unimplemented::UnimplementedVolumeBackend::new("macos");
    assert!(matches!(
        kivori_desktop::action::execute_volume(&backend, 40),
        Outcome::Failed { .. }
    ));
}
