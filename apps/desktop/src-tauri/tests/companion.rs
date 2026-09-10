use kivori_desktop::companion::CompanionDirector;
use kivori_model::{CompanionState, MascotAction, MascotPersonality};

#[test]
fn identical_seed_produces_identical_self_play_schedule_and_action() {
    let mut first = CompanionDirector::new(MascotPersonality::Cozy, true, 123, 1_000);
    let mut second = CompanionDirector::new(MascotPersonality::Cozy, true, 123, 1_000);

    assert_eq!(first.next_due_ms(), second.next_due_ms());
    let due = first.next_due_ms();
    assert_eq!(
        first.poll(due, CompanionState::Idle),
        second.poll(due, CompanionState::Idle)
    );
}

#[test]
fn personality_controls_self_play_cadence() {
    let cozy = CompanionDirector::new(MascotPersonality::Cozy, true, 1, 5_000);
    let playful = CompanionDirector::new(MascotPersonality::Playful, true, 1, 5_000);
    let calm = CompanionDirector::new(MascotPersonality::Calm, true, 1, 5_000);

    assert!((13_000..=21_000).contains(&cozy.next_due_ms()));
    assert!((9_000..=13_000).contains(&playful.next_due_ms()));
    assert!((23_000..=35_000).contains(&calm.next_due_ms()));
}

#[test]
fn busy_and_sleeping_suppress_due_self_play_instead_of_queueing_it() {
    let mut director = CompanionDirector::new(MascotPersonality::Playful, true, 8, 0);
    let first_due = director.next_due_ms();

    assert_eq!(director.poll(first_due, CompanionState::Busy), None);
    assert!(director.next_due_ms() > first_due);
    let second_due = director.next_due_ms();
    assert_eq!(director.poll(second_due, CompanionState::Sleeping), None);
    assert!(director.next_due_ms() > second_due);
}

#[test]
fn manual_cue_uses_current_personality_even_when_self_play_is_disabled() {
    let mut director = CompanionDirector::new(MascotPersonality::Calm, false, 44, 0);

    assert_eq!(director.poll(u32::MAX, CompanionState::Idle), None);
    let cue = director.manual(MascotAction::Comfort);
    assert_eq!(cue.action, MascotAction::Comfort);
    assert_eq!(cue.personality, MascotPersonality::Calm);
}
