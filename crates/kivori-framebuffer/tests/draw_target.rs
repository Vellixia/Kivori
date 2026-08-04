//! The tile framebuffer is a working embedded-graphics `DrawTarget`, and a band captures only its
//! slice of a full-screen draw (tiling).

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use kivori_framebuffer::TileBand;
use kivori_model::{Rect, Rgb565 as KColor};

#[test]
fn draws_a_filled_rect_via_embedded_graphics() {
    let mut buf = vec![KColor::from_raw(0); 240 * 240];
    let mut band = TileBand::new(Rect::new(0, 0, 240, 240), &mut buf).unwrap();

    Rectangle::new(Point::new(10, 10), Size::new(20, 20))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::new(31, 0, 0))) // full red
        .draw(&mut band)
        .unwrap();

    let red = KColor::from_rgb888(255, 0, 0);
    assert_eq!(band.get(10, 10), Some(red));
    assert_eq!(band.get(29, 29), Some(red));
    assert_eq!(band.get(30, 30), Some(KColor::from_raw(0))); // just outside the rect
}

#[test]
fn band_captures_only_its_slice_of_a_full_screen_draw() {
    // A 240x40 band covering rows [40,80).
    let mut buf = vec![KColor::from_raw(0); 240 * 40];
    let mut band = TileBand::new(Rect::new(0, 40, 240, 40), &mut buf).unwrap();

    // Fill the entire 240x240 screen; e-g clips to the band's bounding box + out-of-band pixels drop.
    Rectangle::new(Point::new(0, 0), Size::new(240, 240))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0, 0, 31))) // full blue
        .draw(&mut band)
        .unwrap();

    let blue = KColor::from_rgb888(0, 0, 255);
    assert_eq!(band.get(100, 40), Some(blue)); // first row of the band
    assert_eq!(band.get(100, 79), Some(blue)); // last row of the band
    assert_eq!(band.get(100, 39), None); // above the band
    assert_eq!(band.get(100, 80), None); // below the band
}
