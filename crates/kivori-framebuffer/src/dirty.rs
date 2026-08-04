//! Change detection: a content hash of a tile's pixels (FNV-1a over RGB565 little-endian bytes).
//! The firmware compares signatures to flush only changed tiles (FR-013). This is also the shared
//! hash used for frame-level golden hashing (`kivori-renderer::frame_hash`).

use kivori_model::Rgb565;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a hash of a slice of RGB565 pixels (over their little-endian bytes). Deterministic and
/// platform-independent.
#[must_use]
pub const fn hash_rgb565(pixels: &[Rgb565]) -> u64 {
    let mut h = FNV_OFFSET;
    let mut i = 0;
    while i < pixels.len() {
        let bytes = pixels[i].raw().to_le_bytes();
        let mut j = 0;
        while j < bytes.len() {
            h ^= bytes[j] as u64;
            h = h.wrapping_mul(FNV_PRIME);
            j += 1;
        }
        i += 1;
    }
    h
}

/// A content signature of a tile, used to detect whether the tile changed since the last flush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileSignature(u64);

impl TileSignature {
    /// Computes the signature of a tile's pixels.
    #[must_use]
    pub const fn of(pixels: &[Rgb565]) -> Self {
        TileSignature(hash_rgb565(pixels))
    }

    /// The raw signature value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}
