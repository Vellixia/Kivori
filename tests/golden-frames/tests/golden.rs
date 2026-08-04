//! T041 — golden-frame tests: computable known pixels + committed frame-hash goldens (manifest.toml).

use kivori_golden_frames::{render_full, TestScene, DIM};
use kivori_model::Rgb565;
use kivori_renderer::frame_hash;

fn px(frame: &[Rgb565], x: u16, y: u16) -> Rgb565 {
    frame[y as usize * DIM as usize + x as usize]
}

#[test]
fn known_pixels_at_frame_0() {
    // 0 ms -> frame 0 -> square covers x in [0,40), y in [40,80).
    let f = render_full(&TestScene, 0);
    assert_eq!(px(&f, 0, 40), TestScene::FG);
    assert_eq!(px(&f, 39, 79), TestScene::FG);
    assert_eq!(px(&f, 0, 0), TestScene::BG);
    assert_eq!(px(&f, 50, 60), TestScene::BG);
    assert_eq!(px(&f, 39, 80), TestScene::BG); // y=80 is outside [40,80)
}

#[test]
fn known_pixels_at_frame_1() {
    // 100 ms -> frame 1 -> square covers x in [20,60).
    let f = render_full(&TestScene, 100);
    assert_eq!(px(&f, 50, 60), TestScene::FG);
    assert_eq!(px(&f, 59, 60), TestScene::FG);
    assert_eq!(px(&f, 10, 60), TestScene::BG);
}

// Committed golden frame-hashes (FNV-1a over RGB565 LE bytes); see manifest.toml.
const GOLDEN_MS_0: u64 = 0x911BC4B3FD4537A5;
const GOLDEN_MS_100: u64 = 0x55251F6C652105A5;

#[test]
fn golden_frame_hashes_match_manifest() {
    assert_eq!(frame_hash(&render_full(&TestScene, 0)), GOLDEN_MS_0, "ms=0");
    assert_eq!(
        frame_hash(&render_full(&TestScene, 100)),
        GOLDEN_MS_100,
        "ms=100"
    );
    // 800 ms -> frame 8 -> loops to frame 0, so the frame (and hash) equal ms=0.
    assert_eq!(
        frame_hash(&render_full(&TestScene, 800)),
        GOLDEN_MS_0,
        "ms=800 loops to frame 0"
    );
}
