//! Frame hashing for golden-frame / frame-hash tests (FR-033). Uses the same content hash as tile
//! signatures so goldens and dirty-detection stay consistent.

use kivori_model::Rgb565;

/// Computes a deterministic, platform-independent hash of a rendered RGB565 frame.
#[must_use]
pub fn frame_hash(pixels: &[Rgb565]) -> u64 {
    kivori_framebuffer::hash_rgb565(pixels)
}
