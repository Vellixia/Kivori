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
const H_BOOTING: u64 = 0x544C_376E_D423_44DB;
const H_IDLE: u64 = 0xCB11_A0C7_D4D3_86DD;
const H_HAPPY: u64 = 0x591C_B0F2_43C4_16F5;
const H_BUSY: u64 = 0xB8F3_7A0F_D0C6_64A0;
const H_SLEEPING: u64 = 0x0004_ECC2_EAF9_0106;
const H_OFFLINE: u64 = 0x02F8_A62D_E61D_0219;

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
        (250, 0xDEA2144A0D9DB8A9),
        (400, 0x384DCA480E73B979),
        (600, 0xE9F2C948FAD057B2),
        (849, 0x59CEE62CDB2B60DE),
        (850, 0x0004ECC2EAF90106),
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
        (CompanionState::Idle, 600, 0xCB11A0C7D4D386DD),
        (CompanionState::Idle, 3600, 0xCB11A0C7D4D386DD),
        (CompanionState::Idle, 21227, 0xBFAB00184BEE720D),
        (CompanionState::Idle, 21377, 0x4CC447392E7519D9),
        (CompanionState::Idle, 21457, 0xD3DA9DA7BFE2B1E1),
        (CompanionState::Idle, 22277, 0xE543BE06DC786445),
        (CompanionState::Happy, 300, 0x6611A78887D32EBA),
        (CompanionState::Sleeping, 1200, 0x0004ECC2EAF90106),
    ] {
        let mut pixels = vec![Rgb565::from_raw(0); 240 * 240];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut pixels).unwrap();
        render_scene(&blob, blob.scene(state).unwrap(), ms, &mut band).unwrap();
        assert_eq!(frame_hash(&pixels), expected, "{state:?} at {ms}");
    }
}

#[test]
fn social_reactions_render_distinct_faces_from_the_same_semantic_state() {
    use kivori_model::{MascotExpression, MascotPose};

    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).unwrap();
    let scene = blob.scene(CompanionState::Idle).unwrap();
    let render = |pose| {
        let mut pixels = vec![Rgb565::from_raw(0); 240 * 240];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut pixels).unwrap();
        kivori_renderer::render_pose(&blob, scene, &pose, &mut band).unwrap();
        frame_hash(&pixels)
    };

    let idle_pose = MascotPose::for_state(CompanionState::Idle);
    let idle = render(idle_pose);
    let mut affectionate_pose = idle_pose;
    affectionate_pose.expression = MascotExpression::Affectionate;
    let affectionate = render(affectionate_pose);
    let mut surprised_pose = idle_pose;
    surprised_pose.expression = MascotExpression::Surprised;
    let surprised = render(surprised_pose);

    assert_ne!(affectionate, idle);
    assert_ne!(surprised, idle);
    assert_ne!(surprised, affectionate);
}
