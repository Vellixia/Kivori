//! Compiled-asset manifest types (ADR-0004, data-model §9). The manifest is the `postcard`-encoded
//! structured section of the blob: scene definitions plus the bitmap and string tables. Large pixel
//! and string bytes live in the blob's data pool and are referenced by [`PoolRef`] (read zero-copy).
//!
//! Collections are bounded `heapless` so the manifest deserializes with no heap on the device.

use heapless::Vec;
use kivori_model::{
    CompanionState, DeviceProfile, FrameRate, Keyframe, LayerKind, Point, Rgb565, Size,
};
use serde::{Deserialize, Serialize};

/// Maximum number of compiled sprite bitmaps.
pub const MAX_BITMAPS: usize = 64;
/// Maximum number of compiled strings.
pub const MAX_STRINGS: usize = 16;
/// Maximum number of scenes (six companion states, with headroom).
pub const MAX_SCENES: usize = 8;
/// Maximum layers per scene.
pub const MAX_LAYERS: usize = 12;
/// Maximum keyframes per layer.
pub const MAX_KEYFRAMES: usize = 24;

/// A `(offset, len)` reference into the blob's data pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolRef {
    /// Byte offset into the data pool.
    pub offset: u32,
    /// Length in bytes.
    pub len: u32,
}

/// A compiled sprite bitmap: RGB565 pixels for one or more equally-sized sprite-sheet frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitmapEntry {
    /// Size of a single frame in pixels.
    pub size: Size,
    /// Number of frames stored contiguously in the sprite-sheet.
    pub frames: u16,
    /// RGB565 pixel bytes in the data pool (`frames * size.w * size.h * 2` bytes).
    pub data: PoolRef,
}

/// One layer of a scene: what it draws, where, and its keyframe timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerDef {
    /// What the layer draws.
    pub kind: LayerKind,
    /// The layer's base origin (before keyframe offsets).
    pub origin: Point,
    /// The layer's step-held keyframe timeline (sorted ascending by `at_ms`).
    pub keyframes: Vec<Keyframe, MAX_KEYFRAMES>,
}

/// A compiled scene: background, animation rate, and z-ordered layers, keyed by companion state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneDef {
    /// The companion state this scene renders.
    pub id: CompanionState,
    /// Background fill color.
    pub background: Rgb565,
    /// The scene's animation frame rate.
    pub fps: FrameRate,
    /// Frames in the animation loop (`0`/`1` = static).
    pub frame_count: u16,
    /// Z-ordered layers (drawn first = bottom).
    pub layers: Vec<LayerDef, MAX_LAYERS>,
}

/// The structured section of the compiled blob (postcard-encoded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The device profile this blob was compiled for.
    pub profile: DeviceProfile,
    /// Sprite bitmap table (indexed by `AssetId`).
    pub bitmaps: Vec<BitmapEntry, MAX_BITMAPS>,
    /// String table (indexed by `StringId`); each entry points to UTF-8 bytes in the pool.
    pub strings: Vec<PoolRef, MAX_STRINGS>,
    /// Scene definitions.
    pub scenes: Vec<SceneDef, MAX_SCENES>,
}
