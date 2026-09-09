//! Host-side Device Studio preview rendering (T052 core).
//!
//! Renders a companion state from the compiled asset blob into a 240x240 frame using the shared
//! `kivori-renderer`, then expands the canonical RGB565 to RGBA8888 for the browser Canvas — done in
//! Rust so the Canvas is a pure blit surface (constraint 2). The blob is supplied by the caller (the
//! app bundles a pre-compiled blob); this keeps the SVG compiler out of the desktop's runtime deps.

use kivori_assets::AssetBlob;
use kivori_framebuffer::TileBand;
use kivori_model::{CompanionState, Rect, Rgb565};
use kivori_renderer::render_scene;

pub mod animation;

/// Renders an explicit animation history, including interrupted transitions, into opaque RGBA.
pub fn render_animation_rgba(
    blob: &AssetBlob,
    timeline: &animation::AnimationTimeline,
    elapsed_ms: u32,
) -> Result<Vec<u8>, String> {
    let (state, pose) = timeline.resolve(elapsed_ms)?;
    let scene = blob.scene(state).ok_or("missing mascot scene")?;
    let mut rgb = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    let mut band = TileBand::new(Rect::new(0, 0, DIM, DIM), &mut rgb).expect("full-frame band");
    kivori_renderer::render_pose(blob, scene, &pose, &mut band)
        .map_err(|e| format!("mascot render: {e:?}"))?;
    Ok(rgb
        .into_iter()
        .flat_map(|px| {
            let (r, g, b) = px.to_rgb888();
            [r, g, b, 255]
        })
        .collect())
}

/// Preview dimension (square).
pub const DIM: u16 = 240;

/// Renders `state` at `elapsed_ms` from `blob` into a 240x240 RGBA8888 buffer (row-major, opaque).
///
/// Falls back to the `Idle` scene if `state` is absent, and to a black frame if neither exists. The
/// canonical RGB565 output is the source of truth (golden-tested); this RGBA copy is presentation-only.
#[must_use]
pub fn render_preview_rgba(blob: &AssetBlob, state: CompanionState, elapsed_ms: u32) -> Vec<u8> {
    let mut rgb = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    if let Some(scene) = blob
        .scene(state)
        .or_else(|| blob.scene(CompanionState::Idle))
    {
        let mut band = TileBand::new(Rect::new(0, 0, DIM, DIM), &mut rgb).expect("full-frame band");
        // Rendering only fails on a malformed blob (missing referenced asset); treat as a black frame.
        let _ = render_scene(blob, scene, elapsed_ms, &mut band);
    }

    let mut rgba = Vec::with_capacity(DIM as usize * DIM as usize * 4);
    for px in &rgb {
        let (r, g, b) = px.to_rgb888();
        rgba.extend_from_slice(&[r, g, b, 0xFF]);
    }
    rgba
}

/// The canonical asset blob bundled into the binary at build time (`build.rs` → `OUT_DIR`), parsed
/// once. Dev-only: only the Device Studio preview renders host-side, so it is gated behind the feature.
#[cfg(feature = "device-studio")]
#[must_use]
pub fn bundled_blob() -> &'static AssetBlob<'static> {
    use std::sync::OnceLock;
    static BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kivori.assets"));
    static PARSED: OnceLock<Box<AssetBlob<'static>>> = OnceLock::new();
    PARSED
        .get_or_init(|| {
            // Deserializing the bounded, inline manifest creates large debug-build temporaries.
            // Windows UI threads have only a 1 MiB stack. Decode once on a dedicated stack and
            // return a box so moving the result back does not put the manifest on the UI stack.
            // The worker's stack is released after initialization; sprite pixels still borrow BYTES.
            std::thread::Builder::new()
                .name("kivori-asset-loader".into())
                .stack_size(2 * 1024 * 1024)
                .spawn(|| Box::new(AssetBlob::parse(BYTES).expect("bundled asset blob is valid")))
                .expect("asset loader thread starts")
                .join()
                .expect("asset loader thread completes")
        })
        .as_ref()
}

/// Renders `state` at `elapsed_ms` from the bundled blob into a 240x240 RGBA8888 buffer. Dev-only.
#[cfg(feature = "device-studio")]
#[must_use]
pub fn render_preview_bundled(state: CompanionState, elapsed_ms: u32) -> Vec<u8> {
    render_preview_rgba(bundled_blob(), state, elapsed_ms)
}
