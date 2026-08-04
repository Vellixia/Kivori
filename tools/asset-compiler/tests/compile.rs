//! T048 — the compiled blob is reproducible and parses back with all six scenes + valid sprites.

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_model::CompanionState;

#[test]
fn blob_is_byte_reproducible() {
    assert_eq!(compile_default_blob(), compile_default_blob());
}

#[test]
fn blob_parses_with_all_six_scenes_and_full_frame_sprites() {
    let blob = compile_default_blob();
    let asset = AssetBlob::parse(&blob).expect("blob parses");

    for state in CompanionState::ALL {
        let scene = asset
            .scene(state)
            .unwrap_or_else(|| panic!("scene {state:?} present"));
        assert_eq!(scene.layers.len(), 1, "{state:?} has one sprite layer");
    }

    // Each state maps to a distinct full-frame (240x240) RGB565 sprite.
    for id in 0..6u16 {
        let bmp = asset.bitmap(id).unwrap();
        assert_eq!(bmp.size.w, 240);
        assert_eq!(bmp.size.h, 240);
        assert_eq!(asset.bitmap_pixels(bmp).unwrap().len(), 240 * 240 * 2);
    }
}
