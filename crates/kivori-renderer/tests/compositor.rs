//! Compositor draws a scene from a compiled asset blob: background, solid rect, sprite, text.

use heapless::Vec;
use kivori_assets::manifest::{BitmapEntry, LayerDef, Manifest, PoolRef, SceneDef};
use kivori_assets::{AssetBlob, FORMAT_VERSION, MAGIC};
use kivori_framebuffer::TileBand;
use kivori_model::{
    CompanionState, DeviceProfile, FrameRate, Keyframe, LayerKind, Point, Rect, Rgb565, Size,
};
use kivori_renderer::render_scene;

const DIM: u16 = 240;

fn build_blob(manifest: &Manifest, pool: &[u8]) -> std::vec::Vec<u8> {
    let mut mbuf = [0u8; 4096];
    let mbytes = postcard::to_slice(manifest, &mut mbuf).unwrap();
    let mut blob = std::vec::Vec::new();
    blob.extend_from_slice(&MAGIC.to_le_bytes());
    blob.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    blob.extend_from_slice(&0u16.to_le_bytes());
    blob.extend_from_slice(&(mbytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(&(pool.len() as u32).to_le_bytes());
    blob.extend_from_slice(mbytes);
    blob.extend_from_slice(pool);
    blob
}

fn one_kf() -> Vec<Keyframe, { kivori_assets::MAX_KEYFRAMES }> {
    let mut kfs = Vec::new();
    kfs.push(Keyframe {
        at_ms: 0,
        offset: Point::ORIGIN,
        sprite_frame: 0,
        visible: true,
    })
    .unwrap();
    kfs
}

/// Scene: blue background, red solid rect at (10,10) 20x20, white 2x2 sprite at (50,50), text at (5,5).
fn sample_blob() -> std::vec::Vec<u8> {
    // Pool: a 2x2 all-white sprite (8 bytes of 0xFF) + the string "hi".
    let mut pool = std::vec::Vec::new();
    let sprite = PoolRef { offset: 0, len: 8 };
    pool.extend_from_slice(&[0xFF; 8]);
    let string = PoolRef {
        offset: pool.len() as u32,
        len: 2,
    };
    pool.extend_from_slice(b"hi");

    let mut bitmaps = Vec::new();
    bitmaps
        .push(BitmapEntry {
            size: Size::new(2, 2),
            frames: 1,
            data: sprite,
        })
        .unwrap();
    let mut strings = Vec::new();
    strings.push(string).unwrap();

    let mut layers: Vec<LayerDef, { kivori_assets::MAX_LAYERS }> = Vec::new();
    layers
        .push(LayerDef {
            kind: LayerKind::SolidRect {
                size: Size::new(20, 20),
                color: Rgb565::from_rgb888(255, 0, 0),
            },
            origin: Point::new(10, 10),
            keyframes: one_kf(),
        })
        .unwrap();
    layers
        .push(LayerDef {
            kind: LayerKind::Sprite {
                asset: 0,
                frame_size: Size::new(2, 2),
            },
            origin: Point::new(50, 50),
            keyframes: one_kf(),
        })
        .unwrap();
    layers
        .push(LayerDef {
            kind: LayerKind::Text {
                font: 0,
                string: 0,
                color: Rgb565::WHITE,
            },
            origin: Point::new(5, 5),
            keyframes: one_kf(),
        })
        .unwrap();

    let mut scenes = Vec::new();
    scenes
        .push(SceneDef {
            id: CompanionState::Idle,
            background: Rgb565::from_rgb888(0, 0, 255),
            fps: FrameRate::fps(10),
            frame_count: 1,
            layers,
        })
        .unwrap();

    let manifest = Manifest {
        profile: DeviceProfile::KIVORI_240,
        bitmaps,
        strings,
        scenes,
    };
    build_blob(&manifest, &pool)
}

#[test]
fn composites_background_rect_and_sprite() {
    let blob = sample_blob();
    let asset = AssetBlob::parse(&blob).unwrap();
    let scene = asset.scene(CompanionState::Idle).unwrap();

    let mut buf = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    let mut band = TileBand::new(Rect::new(0, 0, DIM, DIM), &mut buf).unwrap();
    render_scene(&asset, scene, 0, &mut band).unwrap();

    let px = |x: u16, y: u16| buf[y as usize * DIM as usize + x as usize];
    // Background.
    assert_eq!(px(200, 200), Rgb565::from_rgb888(0, 0, 255));
    // Solid rect (red) at (10,10)-(29,29).
    assert_eq!(px(10, 10), Rgb565::from_rgb888(255, 0, 0));
    assert_eq!(px(29, 29), Rgb565::from_rgb888(255, 0, 0));
    assert_eq!(px(30, 30), Rgb565::from_rgb888(0, 0, 255)); // outside rect
                                                            // Sprite (white) at (50,50)-(51,51).
    assert_eq!(px(50, 50), Rgb565::WHITE);
    assert_eq!(px(51, 51), Rgb565::WHITE);
}

#[test]
fn hidden_layer_is_not_drawn() {
    // Build a scene whose only layer is a hidden rect; background should be untouched under it.
    let mut pool = std::vec::Vec::new();
    let _ = &mut pool;
    let mut kfs = Vec::new();
    kfs.push(Keyframe {
        at_ms: 0,
        offset: Point::ORIGIN,
        sprite_frame: 0,
        visible: false,
    })
    .unwrap();
    let mut layers: Vec<LayerDef, { kivori_assets::MAX_LAYERS }> = Vec::new();
    layers
        .push(LayerDef {
            kind: LayerKind::SolidRect {
                size: Size::new(20, 20),
                color: Rgb565::from_rgb888(255, 0, 0),
            },
            origin: Point::new(10, 10),
            keyframes: kfs,
        })
        .unwrap();
    let mut scenes = Vec::new();
    scenes
        .push(SceneDef {
            id: CompanionState::Idle,
            background: Rgb565::from_rgb888(0, 0, 255),
            fps: FrameRate::fps(10),
            frame_count: 1,
            layers,
        })
        .unwrap();
    let manifest = Manifest {
        profile: DeviceProfile::KIVORI_240,
        bitmaps: Vec::new(),
        strings: Vec::new(),
        scenes,
    };
    let blob = build_blob(&manifest, &pool);
    let asset = AssetBlob::parse(&blob).unwrap();
    let scene = asset.scene(CompanionState::Idle).unwrap();

    let mut buf = vec![Rgb565::from_raw(0); DIM as usize * DIM as usize];
    let mut band = TileBand::new(Rect::new(0, 0, DIM, DIM), &mut buf).unwrap();
    render_scene(&asset, scene, 0, &mut band).unwrap();
    // The hidden rect did not paint over the background.
    assert_eq!(buf[10 * DIM as usize + 10], Rgb565::from_rgb888(0, 0, 255));
}
