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
const H_BOOTING: u64 = 0x34B9_01C3_D653_084F;
const H_IDLE: u64 = 0xD689_F565_156C_9FDF;
const H_HAPPY: u64 = 0xD824_8D66_109F_2B97;
const H_BUSY: u64 = 0xBE78_0292_ED3A_368A;
const H_SLEEPING: u64 = 0x4889_1A98_878D_E69C;
const H_OFFLINE: u64 = 0xECDF_2010_2BC3_988B;

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
        (250, 0x730E59EE2D8AF793),
        (400, 0xA407F4827CA49773),
        (600, 0xA894915ECC9B60C8),
        (849, 0x1329280B5B3AE26C),
        (850, 0x48891A98878DE69C),
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
        (CompanionState::Idle, 600, 0xD689F565156C9FDF),
        (CompanionState::Idle, 3600, 0xD689F565156C9FDF),
        (CompanionState::Idle, 21227, 0xFA5A60E47649B33F),
        (CompanionState::Idle, 21377, 0xC3967DD096D64BA3),
        (CompanionState::Idle, 21457, 0x2B85F0CE4D13C5DB),
        (CompanionState::Idle, 22277, 0x252A6BBE5DDFCDE7),
        (CompanionState::Happy, 300, 0x1BB6318767D9F314),
        (CompanionState::Sleeping, 1200, 0x48891A98878DE69C),
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
