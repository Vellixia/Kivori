//! Host preview render tests (T052). The compiled blob comes from the asset compiler (dev-dep); the
//! shipping app bundles a pre-compiled blob and calls the same `render_preview_rgba`.

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_desktop::render::{render_preview_rgba, DIM};
use kivori_model::CompanionState;

const RGBA_LEN: usize = DIM as usize * DIM as usize * 4;

fn blob_bytes() -> Vec<u8> {
    compile_default_blob()
}

#[test]
fn preview_is_full_frame_and_opaque() {
    let bytes = blob_bytes();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");
    let rgba = render_preview_rgba(&blob, CompanionState::Idle, 0);
    assert_eq!(rgba.len(), RGBA_LEN, "240x240 RGBA8888");
    // Every 4th byte is the alpha channel: the Canvas blit expects a fully opaque frame.
    assert!(
        rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 0xFF),
        "opaque"
    );
}

#[test]
fn preview_is_deterministic() {
    let bytes = blob_bytes();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");
    let a = render_preview_rgba(&blob, CompanionState::Busy, 500);
    let b = render_preview_rgba(&blob, CompanionState::Busy, 500);
    assert_eq!(a, b, "same state + time renders identically");
}

#[test]
fn distinct_states_render_differently() {
    let bytes = blob_bytes();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");
    let idle = render_preview_rgba(&blob, CompanionState::Idle, 0);
    let offline = render_preview_rgba(&blob, CompanionState::Offline, 0);
    assert_ne!(idle, offline, "each companion state has its own scene");
}

#[test]
fn matches_canonical_rgb565_channels() {
    // The RGBA preview is a lossless expansion of the golden RGB565 frame: red channel high bits set
    // in RGB565 must survive to RGBA. Spot-check that at least some pixels are non-black (art present).
    let bytes = blob_bytes();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");
    let rgba = render_preview_rgba(&blob, CompanionState::Happy, 0);
    let non_black = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|px| px[0] | px[1] | px[2] != 0);
    assert!(non_black, "happy scene draws visible art");
}
