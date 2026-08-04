//! Change-driven tile rendering (FR-013).
//!
//! Renders the current companion scene one tile at a time through the shared [`render_scene`], hashes
//! each tile, and flushes only tiles whose content changed since the last frame — the bandwidth
//! discipline the SPI panel needs. The tile scratch buffer lives in the renderer (not on the stack)
//! so the device stack stays small.

use crate::ports::DisplaySink;
use kivori_assets::AssetBlob;
use kivori_framebuffer::{hash_rgb565, TileBand};
use kivori_model::{CompanionState, ElapsedMs, Rect, Rgb565};
use kivori_renderer::render_scene;

/// Tile width (full panel width).
pub const TILE_W: u16 = 240;
/// Tile height (one SPI band).
pub const TILE_H: u16 = 40;
/// Panel height in pixels.
pub const PANEL_H: u16 = 240;
/// Pixels per tile.
pub const TILE_PIXELS: usize = TILE_W as usize * TILE_H as usize;
/// Number of tile bands covering the panel.
pub const TILE_COUNT: usize = (PANEL_H / TILE_H) as usize;

/// Why a render pass failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError<E> {
    /// The blob has no scene for the requested state.
    MissingScene,
    /// The tile buffer size did not match the tile rectangle (a build-time invariant).
    Band,
    /// The shared compositor rejected the scene.
    Compositor,
    /// The display sink failed.
    Sink(E),
}

/// A change-driven tile renderer holding the per-tile scratch buffer and last-flushed signatures.
pub struct TileRenderer {
    buf: [Rgb565; TILE_PIXELS],
    signatures: [Option<u64>; TILE_COUNT],
}

impl TileRenderer {
    /// Creates a renderer with an empty (all-black) scratch buffer and no cached signatures.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buf: [Rgb565::from_raw(0); TILE_PIXELS],
            signatures: [None; TILE_COUNT],
        }
    }

    /// Forces every tile to be re-flushed on the next [`Self::render`] (e.g. after a display re-init).
    pub fn invalidate(&mut self) {
        self.signatures = [None; TILE_COUNT];
    }

    /// Renders `state` at `elapsed_ms` from `blob`, flushing only changed tiles to `sink`.
    ///
    /// # Errors
    /// [`RenderError`] if the scene is missing, a tile can't be built, the compositor fails, or the
    /// sink errors.
    pub fn render<S: DisplaySink>(
        &mut self,
        blob: &AssetBlob,
        state: CompanionState,
        elapsed_ms: ElapsedMs,
        sink: &mut S,
    ) -> Result<(), RenderError<S::Error>> {
        let scene = blob.scene(state).ok_or(RenderError::MissingScene)?;
        for tile in 0..TILE_COUNT {
            let rect = Rect::new(0, tile as u16 * TILE_H, TILE_W, TILE_H);
            let mut band = TileBand::new(rect, &mut self.buf).ok_or(RenderError::Band)?;
            render_scene(blob, scene, elapsed_ms, &mut band)
                .map_err(|_| RenderError::Compositor)?;
            let signature = hash_rgb565(band.pixels());
            if self.signatures[tile] != Some(signature) {
                sink.blit_tile(rect, band.pixels())
                    .map_err(RenderError::Sink)?;
                self.signatures[tile] = Some(signature);
            }
        }
        Ok(())
    }
}

impl Default for TileRenderer {
    fn default() -> Self {
        Self::new()
    }
}
