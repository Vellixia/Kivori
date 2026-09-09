//! T049 — asset-backed golden frames: rasterize + compile the 6 layered mascot scenes, render each from
//! the compiled blob, and assert committed frame-hashes (SC-005) + that scenes are distinct.
//!
//! Regenerate the committed hashes deliberately if the scene art or renderer changes (Principle III).

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_framebuffer::TileBand;
use kivori_model::{CompanionState, Rect, Rgb565};
use kivori_renderer::{frame_hash, render_scene};

const DIM: u16 = 240;

// Committed golden frame-hashes (FNV-1a over RGB565 LE bytes); see manifest.toml.
const H_BOOTING: u64 = 0xC2C4_E9FF_E653_52DE;
const H_IDLE: u64 = 0xC939_3FBB_BE80_50BC;
const H_HAPPY: u64 = 0x934B_4C31_7C16_04B6;
const H_BUSY: u64 = 0x7B25_0F89_E9EB_9001;
const H_SLEEPING: u64 = 0x3EA7_9ECC_4864_18A4;
const H_OFFLINE: u64 = 0x520E_0C2B_C013_788A;

fn render_state_hash(state: CompanionState) -> u64 {
    let blob = compile_default_blob();
    let asset = AssetBlob::parse(&blob).unwrap();
    let scene = asset.scene(state).unwrap();
    let mut buf = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    let mut band = TileBand::new(Rect::new(0, 0, DIM, DIM), &mut buf).unwrap();
    render_scene(&asset, scene, 0, &mut band).unwrap();
    frame_hash(&buf)
}

#[test]
fn asset_scenes_render_to_committed_hashes() {
    let cases = [
        (CompanionState::Booting, H_BOOTING),
        (CompanionState::Idle, H_IDLE),
        (CompanionState::Happy, H_HAPPY),
        (CompanionState::Busy, H_BUSY),
        (CompanionState::Sleeping, H_SLEEPING),
        (CompanionState::Offline, H_OFFLINE),
    ];
    for (state, expected) in cases {
        assert_eq!(render_state_hash(state), expected, "{state:?}");
    }
}

#[test]
fn distinct_scenes_have_distinct_hashes() {
    let all: [u64; 6] = CompanionState::ALL.map(render_state_hash);
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "scenes {i} and {j} must render differently");
        }
    }
}

#[test]
fn motion_and_interrupted_transitions_match_reviewed_pixels() {
    use kivori_model::MascotAnimator;
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).unwrap();
    let mut animator = MascotAnimator::new(CompanionState::Idle, 0);
    animator.set_state(CompanionState::Happy, 100);
    animator.set_state(CompanionState::Sleeping, 250);
    for (ms, expected) in [
        (250, 0x4AEDD9C58E11139D),
        (400, 0xF637D5A0852B4B15),
        (600, 0xB3A3CD8859F2ECBE),
        (849, 0xA69EDF2A1101A0C2),
        (850, 0x658A82EFCB8BC423),
    ] {
        let mut pixels = vec![Rgb565::from_raw(0); 240 * 240];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut pixels).unwrap();
        kivori_renderer::render_pose(
            &blob,
            blob.scene(animator.target()).unwrap(),
            &animator.pose_at(ms),
            &mut band,
        )
        .unwrap();
        assert_eq!(
            frame_hash(&pixels),
            expected,
            "interrupted transition at {ms}"
        );
    }
    for (state, ms, expected) in [
        (CompanionState::Idle, 600, 0x41FB313647831A60),
        (CompanionState::Idle, 3600, 0x05A05B04B2C5FA93),
        (CompanionState::Happy, 300, 0xB8899A8FC6194A46),
        (CompanionState::Sleeping, 1200, 0x345F00A5DDAD5F0A),
    ] {
        let mut pixels = vec![Rgb565::from_raw(0); 240 * 240];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut pixels).unwrap();
        render_scene(&blob, blob.scene(state).unwrap(), ms, &mut band).unwrap();
        assert_eq!(frame_hash(&pixels), expected, "{state:?} at {ms}");
    }
}
