//! T020 — deterministic timeline: exact known timestamps, no cumulative drift over long runs, and
//! per-scene frame selection with loops and boundaries (FR-019, SC-011, ADR-0003).

use kivori_model::timeline::{frame_step_ms, scene_frame, FrameRate, StudioTimeline};

#[test]
fn known_frame_indices_produce_exact_timestamps() {
    assert_eq!(frame_step_ms(0), 0);
    assert_eq!(frame_step_ms(1), 33); // (1000 + 15) / 30
    assert_eq!(frame_step_ms(2), 67); // (2000 + 15) / 30
    assert_eq!(frame_step_ms(3), 100); // (3000 + 15) / 30
    assert_eq!(frame_step_ms(30), 1000);
    assert_eq!(frame_step_ms(60), 2000);
    assert_eq!(frame_step_ms(-5), 0); // negative clamps to 0
}

#[test]
fn every_30_steps_advances_exactly_one_second() {
    // Drift-free: because time is derived (not accumulated), each 30-step block is exactly 1000 ms,
    // over a long run (100k seconds = 3M frames).
    let mut prev = frame_step_ms(0);
    for k in 1..=100_000i64 {
        let now = frame_step_ms(30 * k);
        assert_eq!(now - prev, 1000, "block {k}");
        prev = now;
    }
    assert_eq!(frame_step_ms(30 * 100_000), 100_000_000);
}

#[test]
fn stepping_matches_the_absolute_formula_both_directions() {
    let mut t = StudioTimeline::new();
    assert_eq!(t.elapsed_ms(), 0);
    for n in 1..=200_000i64 {
        t.step_forward();
        assert_eq!(t.elapsed_ms(), frame_step_ms(n), "forward n={n}");
    }
    for n in (0..200_000i64).rev() {
        t.step_back();
        assert_eq!(t.elapsed_ms(), frame_step_ms(n), "back n={n}");
    }
    assert_eq!(t.step_index(), 0);
    // step_back clamps at 0
    t.step_back();
    assert_eq!(t.step_index(), 0);
}

#[test]
fn frame_step_ms_saturates_on_huge_index() {
    assert_eq!(frame_step_ms(i64::MAX), u32::MAX);
}

#[test]
fn scene_frame_handles_loops_and_boundaries() {
    // 10 fps -> 100 ms per frame.
    let r = FrameRate::fps(10);
    // Static scene / degenerate frame counts.
    assert_eq!(scene_frame(500, r, 1), 0);
    assert_eq!(scene_frame(500, r, 0), 0);
    // 8-frame loop.
    assert_eq!(scene_frame(0, r, 8), 0);
    assert_eq!(scene_frame(99, r, 8), 0);
    assert_eq!(scene_frame(100, r, 8), 1);
    assert_eq!(scene_frame(799, r, 8), 7);
    // Wraps back to frame 0.
    assert_eq!(scene_frame(800, r, 8), 0);
    // Invalid rates -> frame 0.
    assert_eq!(scene_frame(1000, FrameRate::new(0, 1), 8), 0);
    assert_eq!(scene_frame(1000, FrameRate::new(10, 0), 8), 0);
    assert!(FrameRate::fps(12).is_valid());
    assert!(!FrameRate::new(0, 1).is_valid());
}
