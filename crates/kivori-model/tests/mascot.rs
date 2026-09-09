use kivori_model::{CompanionState, MascotAnimator, MASCOT_TRANSITION_MS};

#[test]
fn interrupted_transition_starts_from_the_resolved_pose() {
    let mut animator = MascotAnimator::new(CompanionState::Idle, 0);
    animator.set_state(CompanionState::Happy, 1_000);
    let before_interrupt = animator.pose_at(1_175);

    animator.set_state(CompanionState::Busy, 1_175);
    assert_eq!(animator.pose_at(1_175), before_interrupt);
}

#[test]
fn expression_swap_is_hidden_by_a_blink_instead_of_overlaying_faces() {
    let mut animator = MascotAnimator::new(CompanionState::Idle, 0);
    animator.set_state(CompanionState::Happy, 1_000);

    let start = animator.pose_at(1_000);
    let before_swap = animator.pose_at(1_000 + MASCOT_TRANSITION_MS / 2 - 1);
    let halfway = animator.pose_at(1_000 + MASCOT_TRANSITION_MS / 2);
    let end = animator.pose_at(1_000 + MASCOT_TRANSITION_MS);

    assert_eq!(start.state_weights[CompanionState::Idle.index()], 255);
    assert_eq!(before_swap.state_weights[CompanionState::Idle.index()], 255);
    assert_eq!(halfway.state_weights[CompanionState::Happy.index()], 255);
    assert_eq!(
        halfway
            .state_weights
            .iter()
            .filter(|weight| **weight > 0)
            .count(),
        1,
        "two facial sprites must never be visible together"
    );
    assert!(
        halfway.eyes_scale_y_q8 <= 48,
        "the eyes close while the expression changes"
    );
    assert_eq!(end.state_weights[CompanionState::Happy.index()], 255);
}

#[test]
fn sleeping_transition_is_slower_than_the_standard_transition() {
    let mut animator = MascotAnimator::new(CompanionState::Idle, 0);
    animator.set_state(CompanionState::Sleeping, 500);

    let standard_end = animator.pose_at(500 + MASCOT_TRANSITION_MS);
    assert!(standard_end.eyes_scale_y_q8 < 256);

    let settled = animator.pose_at(1_100);
    assert_eq!(settled.state_weights[CompanionState::Sleeping.index()], 255);
    assert_eq!(settled.eyes_scale_y_q8, 256);
}

#[test]
fn idle_loop_keeps_the_silhouette_stable_and_blinks() {
    let animator = MascotAnimator::new(CompanionState::Idle, 0);
    let resting = animator.pose_at(0);
    let later = animator.pose_at(900);
    let blink = animator.pose_at(3_600);

    assert_eq!(resting.body_offset_q8, later.body_offset_q8);
    assert_eq!(resting.body_scale_q8, later.body_scale_q8);
    assert!(blink.eyes_scale_y_q8 < resting.eyes_scale_y_q8);
}
