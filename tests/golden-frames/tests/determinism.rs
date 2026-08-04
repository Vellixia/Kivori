//! T042 — determinism: reproducible frames, tiled-vs-full parity, and tile-signature change
//! detection (SC-005, SC-009, FR-013).

use kivori_framebuffer::{TileBand, TileSignature};
use kivori_golden_frames::{render_full, TestScene, DIM};
use kivori_model::{Rect, Rgb565};
use kivori_renderer::{frame_hash, render_tile};

#[test]
fn rendering_is_reproducible() {
    for ms in [0u32, 33, 100, 799, 800, 5000] {
        assert_eq!(
            frame_hash(&render_full(&TestScene, ms)),
            frame_hash(&render_full(&TestScene, ms)),
            "ms={ms}"
        );
    }
}

#[test]
fn tiled_render_matches_full_frame() {
    let ms = 133; // frame 1
    let full = render_full(&TestScene, ms);

    // Render the same content in 240x40 scanline bands and stitch them together.
    let mut tiled = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    let mut band_y = 0u16;
    while band_y < DIM {
        let mut tbuf = vec![Rgb565::from_raw(0); DIM as usize * 40];
        {
            let mut band = TileBand::new(Rect::new(0, band_y, DIM, 40), &mut tbuf).unwrap();
            render_tile(&TestScene, ms, &mut band);
        }
        for (i, p) in tbuf.iter().enumerate() {
            let lx = i % DIM as usize;
            let ly = i / DIM as usize;
            tiled[(band_y as usize + ly) * DIM as usize + lx] = *p;
        }
        band_y += 40;
    }

    assert_eq!(
        full, tiled,
        "tiled rendering must equal the full-frame render"
    );
    assert_eq!(frame_hash(&full), frame_hash(&tiled));
}

#[test]
fn tile_signature_detects_change() {
    let a = render_full(&TestScene, 0);
    let b = render_full(&TestScene, 0);
    let c = render_full(&TestScene, 100);
    assert_eq!(TileSignature::of(&a), TileSignature::of(&b));
    assert_ne!(TileSignature::of(&a), TileSignature::of(&c));
}
