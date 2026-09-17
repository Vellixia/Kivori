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
