//! T046 — the compiled-asset reader round-trips a blob and rejects malformed blobs.

use heapless::Vec;
use kivori_assets::manifest::{BitmapEntry, LayerDef, Manifest, PoolRef, SceneDef};
use kivori_assets::{AssetBlob, AssetError, FORMAT_VERSION, HEADER_LEN, MAGIC};
use kivori_model::{
    CompanionState, DeviceProfile, FrameRate, Keyframe, LayerKind, Point, Rgb565, Size,
};

const PIXELS: [u8; 8] = [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00, 0xFF, 0xFF]; // 4 RGB565 pixels, LE

fn build_blob(manifest: &Manifest, pool: &[u8]) -> std::vec::Vec<u8> {
    let mut mbuf = [0u8; 4096];
    let mbytes = postcard::to_slice(manifest, &mut mbuf).unwrap();
    let mut blob = std::vec::Vec::new();
    blob.extend_from_slice(&MAGIC.to_le_bytes());
    blob.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    blob.extend_from_slice(&0u16.to_le_bytes()); // reserved
    blob.extend_from_slice(&(mbytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(&(pool.len() as u32).to_le_bytes());
    blob.extend_from_slice(mbytes);
    blob.extend_from_slice(pool);
    blob
}

fn sample_blob() -> std::vec::Vec<u8> {
    let mut pool = std::vec::Vec::new();
    let bmp = PoolRef {
        offset: pool.len() as u32,
        len: PIXELS.len() as u32,
    };
    pool.extend_from_slice(&PIXELS);
    let string = PoolRef {
        offset: pool.len() as u32,
        len: 4,
    };
    pool.extend_from_slice(b"idle");

    let mut bitmaps = Vec::new();
    bitmaps
        .push(BitmapEntry {
            size: Size::new(2, 2),
            frames: 1,
            data: bmp,
        })
        .unwrap();
    let mut strings = Vec::new();
    strings.push(string).unwrap();

    let mut keyframes = Vec::new();
    keyframes
        .push(Keyframe {
            at_ms: 0,
            offset: Point::ORIGIN,
            sprite_frame: 0,
            visible: true,
        })
        .unwrap();
    let mut layers = Vec::new();
    layers
        .push(LayerDef {
            kind: LayerKind::Sprite {
                asset: 0,
                frame_size: Size::new(2, 2),
            },
            origin: Point::new(10, 10),
            keyframes,
        })
        .unwrap();
    let mut scenes = Vec::new();
    scenes
        .push(SceneDef {
            id: CompanionState::Idle,
            background: Rgb565::from_rgb888(0, 0, 0),
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
fn round_trips_a_blob() {
    let blob = sample_blob();
    let asset = AssetBlob::parse(&blob).unwrap();

    assert_eq!(asset.profile(), DeviceProfile::KIVORI_240);

    let scene = asset.scene(CompanionState::Idle).unwrap();
    assert_eq!(scene.background, Rgb565::from_rgb888(0, 0, 0));
    assert_eq!(scene.layers.len(), 1);

    let bmp = asset.bitmap(0).unwrap();
    assert_eq!(bmp.size, Size::new(2, 2));
    assert_eq!(asset.bitmap_pixels(bmp), Some(&PIXELS[..]));

    assert_eq!(asset.string(0), Some("idle"));

    assert!(asset.scene(CompanionState::Happy).is_none());
    assert!(asset.bitmap(99).is_none());
}

#[test]
fn rejects_malformed_blobs() {
    assert_eq!(AssetBlob::parse(&[]).unwrap_err(), AssetError::TooShort);

    let mut bad_magic = sample_blob();
    bad_magic[0] ^= 0xFF;
    assert_eq!(
        AssetBlob::parse(&bad_magic).unwrap_err(),
        AssetError::BadMagic
    );

    let truncated = &sample_blob()[..HEADER_LEN + 2];
    assert_eq!(
        AssetBlob::parse(truncated).unwrap_err(),
        AssetError::LengthMismatch
    );
}
