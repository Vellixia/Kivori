use kivori_desktop::render::animation::{AnimationEvent, AnimationTimeline, MascotActionEvent};
use kivori_model::{CompanionState, MascotAction, MascotAnimator, MascotPersonality};

#[test]
fn seeking_replays_the_same_interrupted_pose_as_live_device_events() {
    let timeline = AnimationTimeline {
        initial_state: "idle".into(),
        events: vec![
            AnimationEvent {
                at_ms: 100,
                state: "happy".into(),
            },
            AnimationEvent {
                at_ms: 250,
                state: "sleeping".into(),
            },
        ],
        action_events: vec![],
    };
    let mut live = MascotAnimator::new(CompanionState::Idle, 0);
    live.set_state(CompanionState::Happy, 100);
    assert_eq!(timeline.resolve(175).unwrap().1, live.pose_at(175));
    live.set_state(CompanionState::Sleeping, 250);
    for ms in [250, 251, 400, 849, 850, 4_001] {
        assert_eq!(timeline.resolve(ms).unwrap().1, live.pose_at(ms));
    }
    assert_eq!(timeline.resolve(175).unwrap().0, CompanionState::Happy);
}

#[test]
fn rejects_invalid_event_order_and_unknown_states() {
    let mut timeline = AnimationTimeline {
        initial_state: "idle".into(),
        events: vec![
            AnimationEvent {
                at_ms: 250,
                state: "happy".into(),
            },
            AnimationEvent {
                at_ms: 100,
                state: "busy".into(),
            },
        ],
        action_events: vec![],
    };
    assert!(timeline.validate().is_err());
    timeline.events.clear();
    timeline.initial_state = "unknown".into();
    assert!(timeline.validate().is_err());
    timeline.initial_state = "idle".into();
    timeline.events = vec![
        AnimationEvent {
            at_ms: 0,
            state: "happy".into()
        };
        257
    ];
    assert!(timeline.validate().is_err());
}

#[test]
fn seeking_replays_seeded_social_action_cues() {
    let timeline = AnimationTimeline {
        initial_state: "idle".into(),
        events: vec![],
        action_events: vec![MascotActionEvent {
            at_ms: 100,
            action: "tickle".into(),
            personality: "playful".into(),
            seed: 7,
        }],
    };
    let mut live = MascotAnimator::new(CompanionState::Idle, 0);
    live.trigger_action(MascotAction::Tickle, MascotPersonality::Playful, 7, 100);

    assert_eq!(timeline.resolve(700).unwrap().1, live.pose_at(700));
}
