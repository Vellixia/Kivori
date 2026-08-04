//! Per-scene frame selection: maps a canonical elapsed time to the scene's active frame using the
//! shared integer primitive (`kivori_model::scene_frame`, ADR-0003).

use crate::render::Scene;
use kivori_model::{scene_frame, ElapsedMs};

/// Selects the active frame of `scene` at `elapsed_ms` (deterministic, integer-only). Static scenes
/// and invalid rates yield frame `0`.
#[must_use]
pub fn select_frame<S: Scene + ?Sized>(scene: &S, elapsed_ms: ElapsedMs) -> u16 {
    scene_frame(elapsed_ms, scene.frame_rate(), scene.frame_count())
}
