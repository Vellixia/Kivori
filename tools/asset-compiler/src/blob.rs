//! Deterministic assembly of the compiled asset blob (ADR-0004): rasterize each scene's SVG to a
//! full-frame RGB565 sprite, build the `Manifest` + data pool, and emit `header || manifest || pool`.
//!
//! Placeholder scenes use one full-frame sprite each; richer per-layer composition (and smaller
//! sprites) arrive with real artwork. The scene set is embedded via `include_bytes!` so the blob is
//! reproducible from the compiler binary alone.

use crate::rasterize::svg_to_rgb565;
use heapless::Vec as HVec;
use kivori_assets::manifest::{BitmapEntry, LayerDef, Manifest, PoolRef, SceneDef};
use kivori_assets::{FORMAT_VERSION, MAGIC, MAX_BITMAPS, MAX_SCENES, MAX_STRINGS};
use kivori_model::{
    CompanionState, DeviceProfile, FrameRate, Keyframe, LayerKind, Point, Rgb565, Size,
};

const DIM: u16 = 240;

/// A placeholder scene spec: `(state, embedded SVG bytes, background RGB)`.
type SceneSpec = (CompanionState, &'static [u8], (u8, u8, u8));

/// Assembles a compiled asset blob from rasterized sprites and scene definitions.
pub struct BlobBuilder {
    profile: DeviceProfile,
    pool: Vec<u8>,
    bitmaps: HVec<BitmapEntry, MAX_BITMAPS>,
    strings: HVec<PoolRef, MAX_STRINGS>,
    scenes: HVec<SceneDef, MAX_SCENES>,
}

impl BlobBuilder {
    /// A new builder for the given device profile.
    #[must_use]
    pub fn new(profile: DeviceProfile) -> Self {
        Self {
            profile,
            pool: Vec::new(),
            bitmaps: HVec::new(),
            strings: HVec::new(),
            scenes: HVec::new(),
        }
    }

    /// Adds a bitmap's RGB565 bytes to the pool and returns its `AssetId`.
    pub fn add_bitmap(&mut self, size: Size, frames: u16, pixels: &[u8]) -> u16 {
        let offset = self.pool.len() as u32;
        self.pool.extend_from_slice(pixels);
        let id = self.bitmaps.len() as u16;
        self.bitmaps
            .push(BitmapEntry {
                size,
                frames,
                data: PoolRef {
                    offset,
                    len: pixels.len() as u32,
                },
            })
            .expect("bitmap table capacity");
        id
    }

    /// Adds a static scene consisting of a single full-frame sprite layer.
    pub fn add_full_sprite_scene(&mut self, id: CompanionState, background: Rgb565, asset: u16) {
        let mut keyframes = HVec::new();
        keyframes
            .push(Keyframe {
                at_ms: 0,
                offset: Point::ORIGIN,
                sprite_frame: 0,
                visible: true,
            })
            .expect("keyframe capacity");
        let mut layers = HVec::new();
        layers
            .push(LayerDef {
                kind: LayerKind::Sprite {
                    asset,
                    frame_size: Size::new(DIM, DIM),
                },
                origin: Point::ORIGIN,
                keyframes,
            })
            .expect("layer capacity");
        self.scenes
            .push(SceneDef {
                id,
                background,
                fps: FrameRate::fps(1),
                frame_count: 1,
                layers,
            })
            .expect("scene capacity");
    }

    /// Finalizes the blob bytes: `header (16) || postcard(manifest) || pool`.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        let manifest = Manifest {
            profile: self.profile,
            bitmaps: self.bitmaps,
            strings: self.strings,
            scenes: self.scenes,
        };
        let mbytes = postcard::to_stdvec(&manifest).expect("manifest serializes");
        let mut blob = Vec::with_capacity(16 + mbytes.len() + self.pool.len());
        blob.extend_from_slice(&MAGIC.to_le_bytes());
        blob.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes()); // reserved
        blob.extend_from_slice(&(mbytes.len() as u32).to_le_bytes());
        blob.extend_from_slice(&(self.pool.len() as u32).to_le_bytes());
        blob.extend_from_slice(&mbytes);
        blob.extend_from_slice(&self.pool);
        blob
    }
}

/// The placeholder scene set.
const SCENES: [SceneSpec; 6] = [
    (
        CompanionState::Booting,
        include_bytes!("../../../assets/scenes/booting.svg"),
        (10, 10, 18),
    ),
    (
        CompanionState::Idle,
        include_bytes!("../../../assets/scenes/idle.svg"),
        (12, 16, 28),
    ),
    (
        CompanionState::Happy,
        include_bytes!("../../../assets/scenes/happy.svg"),
        (28, 24, 8),
    ),
    (
        CompanionState::Busy,
        include_bytes!("../../../assets/scenes/busy.svg"),
        (28, 16, 8),
    ),
    (
        CompanionState::Sleeping,
        include_bytes!("../../../assets/scenes/sleeping.svg"),
        (16, 12, 28),
    ),
    (
        CompanionState::Offline,
        include_bytes!("../../../assets/scenes/offline.svg"),
        (10, 10, 10),
    ),
];

/// Compiles the built-in placeholder scenes into a deterministic asset blob.
///
/// # Panics
/// If a placeholder SVG fails to rasterize (a build-time bug, not runtime input).
#[must_use]
pub fn compile_default_blob() -> Vec<u8> {
    let mut builder = BlobBuilder::new(DeviceProfile::KIVORI_240);
    for (state, svg, (r, g, b)) in SCENES {
        let pixels = svg_to_rgb565(svg, u32::from(DIM), u32::from(DIM)).expect("placeholder SVG");
        let asset = builder.add_bitmap(Size::new(DIM, DIM), 1, &pixels);
        builder.add_full_sprite_scene(state, Rgb565::from_rgb888(r, g, b), asset);
    }
    builder.finish()
}
