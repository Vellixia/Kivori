use kivori_asset_compiler::{compile_default_blob, BlobBuilder};
use kivori_assets::AssetBlob;
use kivori_model::{CompanionState, DeviceProfile, LayerKind, LayerRole, Size};

#[test]
fn blob_is_byte_reproducible_and_within_budget() {
    let first = compile_default_blob();
    assert_eq!(first, compile_default_blob());
    assert!(
        first.len() <= 131_072,
        "compiled mascot is {} bytes",
        first.len()
    );
}

#[test]
fn blob_has_six_four_layer_scenes_and_reused_body() {
    let blob = compile_default_blob();
    let asset = AssetBlob::parse(&blob).expect("blob parses");
    assert_eq!(
        asset.manifest().bitmaps.len(),
        13,
        "one body + six eyes + six mouths"
    );

    for state in CompanionState::ALL {
        let scene = asset.scene(state).expect("state present");
        assert_eq!(scene.layers.len(), 4);
        assert_eq!(scene.frame_count, 120);
        assert_eq!(scene.fps.num, 30);
        assert_eq!(scene.fps.den, 1);
        assert_eq!(scene.layers[0].role, LayerRole::Body);
        assert_eq!(scene.layers[1].role, LayerRole::Eyes);
        assert_eq!(scene.layers[2].role, LayerRole::Eyes);
        assert_eq!(scene.layers[3].role, LayerRole::Mouth);
        assert_eq!(scene.layers[1].origin.x, 66);
        assert_eq!(scene.layers[2].origin.x, 142);
        assert_eq!(scene.layers[1].origin.y, 110);
        assert_eq!(scene.layers[2].origin.y, 110);
        assert!(
            matches!(scene.layers[0].kind, LayerKind::Sprite { frame_size, .. } if frame_size.w == 184 && frame_size.h == 160)
        );
        assert!(
            matches!(scene.layers[1].kind, LayerKind::Sprite { frame_size, .. } if frame_size.w == 32 && frame_size.h == 40)
        );
        assert!(
            matches!(scene.layers[3].kind, LayerKind::Sprite { frame_size, .. } if frame_size.w == 48 && frame_size.h == 24)
        );
    }

    let first_body = match asset.scene(CompanionState::Booting).unwrap().layers[0].kind {
        LayerKind::Sprite { asset, .. } => asset,
        _ => panic!("body is sprite"),
    };
    for state in CompanionState::ALL {
        let body = match asset.scene(state).unwrap().layers[0].kind {
            LayerKind::Sprite { asset, .. } => asset,
            _ => panic!("body is sprite"),
        };
        assert_eq!(body, first_body);
    }
}

#[test]
fn alpha4_packs_even_pixels_low_odd_pixels_high_including_odd_count() {
    let mut builder = BlobBuilder::new(DeviceProfile::KIVORI_240);
    let id = builder.add_masked_bitmap(Size::new(3, 1), 1, &[0; 6], &[0, 17, 85]);
    let blob = builder.finish();
    let asset = AssetBlob::parse(&blob).unwrap();
    assert_eq!(
        asset.bitmap_alpha(asset.bitmap(id).unwrap()).unwrap(),
        &[0x10, 0x05]
    );
}

#[test]
fn masked_bitmap_dedup_includes_alpha_and_dimensions() {
    let mut builder = BlobBuilder::new(DeviceProfile::KIVORI_240);
    let pixels = [0x12, 0x34, 0x56, 0x78];
    let alpha = [255, 0];
    let same = builder.add_masked_bitmap(Size::new(2, 1), 1, &pixels, &alpha);
    let duplicate = builder.add_masked_bitmap(Size::new(2, 1), 1, &pixels, &alpha);
    let different_alpha = builder.add_masked_bitmap(Size::new(2, 1), 1, &pixels, &[255, 17]);
    let different_dimensions = builder.add_masked_bitmap(Size::new(1, 2), 1, &pixels, &alpha);
    assert_eq!(same, duplicate);
    assert_ne!(same, different_alpha);
    assert_ne!(same, different_dimensions);
}

#[test]
fn mascot_layers_preserve_transparency_and_intermediate_alpha() {
    let blob = compile_default_blob();
    let asset = AssetBlob::parse(&blob).unwrap();
    let body_id = match asset.scene(CompanionState::Idle).unwrap().layers[0].kind {
        LayerKind::Sprite { asset, .. } => asset,
        _ => panic!("body is sprite"),
    };
    let alpha = asset.bitmap_alpha(asset.bitmap(body_id).unwrap()).unwrap();
    assert!(alpha
        .iter()
        .any(|byte| byte & 0x0f != 0 && byte & 0x0f != 0x0f));
    assert!(alpha.iter().any(|byte| byte >> 4 != 0 && byte >> 4 != 0x0f));
}
