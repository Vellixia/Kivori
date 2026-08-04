//! Render-parity test (T069; SC-005): the firmware's change-driven, tile-by-tile render path
//! produces exactly the same pixels as a single full-frame render of the shared renderer. This is
//! what lets the host golden frames stand in for the on-device image.
#![cfg(feature = "host-sim")]

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_firmware::render::TileRenderer;
use kivori_firmware::sim::{CaptureDisplay, FRAME_PIXELS};
use kivori_framebuffer::{hash_rgb565, TileBand};
use kivori_model::{CompanionState, Rect, Rgb565};
use kivori_renderer::render_scene;

/// Renders `state` at `elapsed_ms` into a single full-frame band and returns its content hash.
fn full_frame_hash(blob: &AssetBlob, state: CompanionState, elapsed_ms: u32) -> u64 {
    let mut buf = vec![Rgb565::from_raw(0); FRAME_PIXELS];
    let scene = blob.scene(state).expect("scene present");
    let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut buf).expect("full-frame band");
    render_scene(blob, scene, elapsed_ms, &mut band).expect("render");
    hash_rgb565(&buf)
}

#[test]
fn stitched_tiles_equal_the_full_frame() {
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");

    for state in CompanionState::ALL {
        let expected = full_frame_hash(&blob, state, 0);

        let mut renderer = TileRenderer::new();
        let mut display = Box::new(CaptureDisplay::new());
        renderer
            .render(&blob, state, 0, display.as_mut())
            .expect("tile render");

        assert_eq!(
            hash_rgb565(display.frame()),
            expected,
            "tiling parity for {state:?}"
        );
        assert_eq!(
            display.blits, 6,
            "all six tiles flushed on first render for {state:?}"
        );
    }
}

#[test]
fn unchanged_frame_flushes_nothing_on_rerender() {
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");

    let mut renderer = TileRenderer::new();
    let mut display = Box::new(CaptureDisplay::new());
    renderer
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .expect("first render");
    let after_first = display.blits;
    renderer
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .expect("second render");

    assert_eq!(after_first, 6);
    assert_eq!(
        display.blits, after_first,
        "an identical frame flushes no tiles (FR-013)"
    );
}
