//! Render-parity test (T069; SC-005): the firmware's change-driven, tile-by-tile render path
//! produces exactly the same pixels as a single full-frame render of the shared renderer. This is
//! what lets the host golden frames stand in for the on-device image.
#![cfg(feature = "host-sim")]

use kivori_asset_compiler::compile_default_blob;
use kivori_assets::AssetBlob;
use kivori_firmware::render::{TileRenderer, TILE_COUNT};
use kivori_firmware::sim::{CaptureDisplay, FRAME_PIXELS};
use kivori_framebuffer::{hash_rgb565, TileBand};
use kivori_model::{CompanionState, Rect, Rgb565};
use kivori_renderer::render_scene;

#[test]
fn buffered_frames_clear_old_pixels_and_retry_a_failed_transfer() {
    use kivori_firmware::ports::DisplaySink;
    use kivori_model::MascotAnimator;

    struct FailOnce {
        display: Box<CaptureDisplay>,
        remaining: Option<usize>,
    }
    impl DisplaySink for FailOnce {
        type Error = ();
        fn blit_tile(&mut self, rect: Rect, pixels: &[Rgb565]) -> Result<(), ()> {
            if let Some(remaining) = &mut self.remaining {
                if *remaining == 0 {
                    self.remaining = None;
                    return Err(());
                }
                *remaining -= 1;
            }
            self.display.blit_tile(rect, pixels).unwrap();
            Ok(())
        }
    }

    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).unwrap();
    let mut storage: Box<[Rgb565; FRAME_PIXELS]> = vec![Rgb565::from_raw(0); FRAME_PIXELS]
        .into_boxed_slice()
        .try_into()
        .unwrap();
    let mut renderer = TileRenderer::with_frame_buffer(&mut storage);
    let mut sink = FailOnce {
        display: Box::new(CaptureDisplay::new()),
        remaining: Some(7),
    };
    let mut animator = MascotAnimator::new(CompanionState::Happy, 0);
    let pose = animator.pose_at(200);
    assert!(renderer
        .render_animation(&blob, animator.target(), &pose, &mut sink)
        .is_err());
    renderer
        .render_animation(&blob, animator.target(), &pose, &mut sink)
        .unwrap();
    assert_eq!(
        sink.display.blits as usize, TILE_COUNT,
        "only successful transfers enter the cache"
    );
    for ms in [200, 400, 700, 1200, 1600, 3600] {
        if ms == 700 {
            animator.set_state(CompanionState::Idle, ms);
        }
        let pose = animator.pose_at(ms);
        let mut expected = vec![Rgb565::from_raw(0); FRAME_PIXELS];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut expected).unwrap();
        kivori_renderer::render_pose(
            &blob,
            blob.scene(animator.target()).unwrap(),
            &pose,
            &mut band,
        )
        .unwrap();
        renderer
            .render_animation(&blob, animator.target(), &pose, &mut sink)
            .unwrap();
        assert_eq!(sink.display.frame(), expected, "prepared pose at {ms} ms");
    }
    let before = sink.display.blits;
    renderer
        .render_animation(&blob, animator.target(), &animator.pose_at(3600), &mut sink)
        .unwrap();
    assert_eq!(
        sink.display.blits, before,
        "identical buffered frame sends nothing"
    );
    renderer.invalidate();
    renderer
        .render_animation(&blob, animator.target(), &animator.pose_at(3600), &mut sink)
        .unwrap();
    assert_eq!((sink.display.blits - before) as usize, TILE_COUNT);
}

#[test]
fn a_late_composition_error_cannot_partially_update_a_buffered_frame() {
    let mut bytes = compile_default_blob();
    let manifest_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    // Keep the valid manifest but remove its pixel pool: the first background tiles render,
    // then sampling the lower mascot discovers the missing bitmap data.
    bytes.truncate(16 + manifest_len);
    bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
    let blob = AssetBlob::parse(&bytes).unwrap();
    let mut unbuffered = TileRenderer::new();
    let mut display = Box::new(CaptureDisplay::new());
    assert!(unbuffered
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .is_err());
    assert!(
        display.blits > 0,
        "fixture must fail after an earlier tile was composed"
    );

    let mut storage: Box<[Rgb565; FRAME_PIXELS]> = vec![Rgb565::from_raw(0); FRAME_PIXELS]
        .into_boxed_slice()
        .try_into()
        .unwrap();
    let mut buffered = TileRenderer::with_frame_buffer(&mut storage);
    let before = display.blits;
    assert!(buffered
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .is_err());
    assert_eq!(
        display.blits, before,
        "no display writes until the entire pose is composed"
    );
}

#[test]
fn interrupted_animation_matches_full_frame_at_every_tile_boundary() {
    use kivori_model::MascotAnimator;
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).unwrap();
    let mut animator = MascotAnimator::new(CompanionState::Idle, 0);
    animator.set_state(CompanionState::Happy, 100);
    animator.set_state(CompanionState::Sleeping, 250);
    let mut renderer = TileRenderer::new();
    let mut display = Box::new(CaptureDisplay::new());
    for ms in [250, 251, 400, 600, 849, 850, 3600, 4001] {
        let pose = animator.pose_at(ms);
        let mut full = vec![Rgb565::from_raw(0); FRAME_PIXELS];
        let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut full).unwrap();
        kivori_renderer::render_pose(
            &blob,
            blob.scene(animator.target()).unwrap(),
            &pose,
            &mut band,
        )
        .unwrap();
        renderer
            .render_animation(&blob, animator.target(), &pose, display.as_mut())
            .unwrap();
        assert_eq!(display.frame(), &full[..], "transition at {ms}ms");
    }
}

/// Renders `state` at `elapsed_ms` into a single full-frame band and returns its content hash.
fn full_frame_hash(blob: &AssetBlob, state: CompanionState, elapsed_ms: u32) -> u64 {
    let mut buf = vec![Rgb565::from_raw(0); FRAME_PIXELS];
    let scene = blob.scene(state).expect("scene present");
    let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut buf).expect("full-frame band");
    render_scene(blob, scene, elapsed_ms, &mut band).expect("render");
    hash_rgb565(&buf)
}

#[test]
fn stitched_tiles_equal_the_full_frame() {
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");

    for state in CompanionState::ALL {
        let expected = full_frame_hash(&blob, state, 0);

        let mut renderer = TileRenderer::new();
        let mut display = Box::new(CaptureDisplay::new());
        renderer
            .render(&blob, state, 0, display.as_mut())
            .expect("tile render");

        assert_eq!(
            hash_rgb565(display.frame()),
            expected,
            "tiling parity for {state:?}"
        );
        assert_eq!(
            display.blits as usize, TILE_COUNT,
            "all tiles flushed on first render for {state:?}"
        );
    }
}

#[test]
fn unchanged_frame_flushes_nothing_on_rerender() {
    let bytes = compile_default_blob();
    let blob = AssetBlob::parse(&bytes).expect("valid blob");

    let mut renderer = TileRenderer::new();
    let mut display = Box::new(CaptureDisplay::new());
    renderer
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .expect("first render");
    let after_first = display.blits;
    renderer
        .render(&blob, CompanionState::Idle, 0, display.as_mut())
        .expect("second render");

    assert_eq!(after_first as usize, TILE_COUNT);
    assert_eq!(
        display.blits, after_first,
        "an identical frame flushes no tiles (FR-013)"
    );
}
