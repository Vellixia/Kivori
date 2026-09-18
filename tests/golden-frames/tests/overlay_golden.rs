use kivori_framebuffer::TileBand;
use kivori_model::presentation::ValueConfidence;
use kivori_model::{Rect, Rgb565};
use kivori_renderer::hash::frame_hash;
use kivori_renderer::overlay::render_volume_overlay;

const W: u16 = 240;
const H: u16 = 240;

fn draw(percent: u8, confidence: ValueConfidence, at_boundary: bool) -> Vec<Rgb565> {
    let mut pixels = vec![Rgb565::BLACK; (W as usize) * (H as usize)];
    // Rect::new takes (x, y, w, h) in absolute display coordinates.
    let rect = Rect::new(0, 0, W, H);
    let mut band = TileBand::new(rect, &mut pixels).expect("full-frame band");
    render_volume_overlay(&mut band, percent, confidence, at_boundary);
    pixels
}

#[test]
fn overlay_is_deterministic_across_repeated_renders() {
    let a = draw(50, ValueConfidence::Confirmed, false);
    let b = draw(50, ValueConfidence::Confirmed, false);
    assert_eq!(frame_hash(&a), frame_hash(&b));
}

/// The whole point of ValueConfidence: a preview must not LOOK like confirmed truth.
#[test]
fn the_three_confidences_are_visually_distinguishable() {
    let preview = frame_hash(&draw(50, ValueConfidence::Preview, false));
    let confirmed = frame_hash(&draw(50, ValueConfidence::Confirmed, false));
    let unverified = frame_hash(&draw(50, ValueConfidence::Unverified, false));

    assert_ne!(preview, confirmed, "Preview must not render as Confirmed");
    assert_ne!(preview, unverified, "Preview must not render as Unverified");
    assert_ne!(
        confirmed, unverified,
        "Confirmed must not render as Unverified"
    );
}

#[test]
fn different_percentages_render_differently() {
    assert_ne!(
        frame_hash(&draw(0, ValueConfidence::Confirmed, false)),
        frame_hash(&draw(100, ValueConfidence::Confirmed, false))
    );
    assert_ne!(
        frame_hash(&draw(49, ValueConfidence::Confirmed, false)),
        frame_hash(&draw(51, ValueConfidence::Confirmed, false))
    );
}

#[test]
fn the_boundary_flag_changes_the_rendering() {
    assert_ne!(
        frame_hash(&draw(100, ValueConfidence::Confirmed, false)),
        frame_hash(&draw(100, ValueConfidence::Confirmed, true))
    );
}

#[test]
fn bounds_render_without_panicking_and_are_distinct() {
    let zero = frame_hash(&draw(0, ValueConfidence::Confirmed, false));
    let full = frame_hash(&draw(100, ValueConfidence::Confirmed, false));
    assert_ne!(zero, full);
}

/// Out-of-range input is clamped, never allowed to index outside the band.
#[test]
fn percent_above_one_hundred_is_clamped_to_full() {
    assert_eq!(
        frame_hash(&draw(200, ValueConfidence::Confirmed, false)),
        frame_hash(&draw(100, ValueConfidence::Confirmed, false))
    );
}
