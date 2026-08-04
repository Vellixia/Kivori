//! `embedded-graphics` `DrawTarget` for [`TileBand`] (ADR-0004: embedded-graphics is the drawing
//! stack).
//!
//! The band draws in **absolute** display coordinates and reports its rectangle as the bounding box,
//! so `embedded-graphics` clips fills to the band and any pixel outside the band is dropped. That lets
//! a full scene be drawn into each tile, with the band keeping only its slice. e-g's `Rgb565` is
//! converted to the canonical [`kivori_model::Rgb565`] at the boundary (lossless — identical bits).

use crate::TileBand;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::geometry::{Dimensions, Point, Size};
use embedded_graphics_core::pixelcolor::{IntoStorage, Rgb565 as EgRgb565};
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::Pixel;
use kivori_model::Rgb565;

impl Dimensions for TileBand<'_> {
    fn bounding_box(&self) -> Rectangle {
        let r = self.rect();
        Rectangle::new(
            Point::new(i32::from(r.x), i32::from(r.y)),
            Size::new(u32::from(r.w), u32::from(r.h)),
        )
    }
}

impl DrawTarget for TileBand<'_> {
    type Color = EgRgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            if let (Ok(x), Ok(y)) = (u16::try_from(coord.x), u16::try_from(coord.y)) {
                self.set(x, y, Rgb565::from_raw(color.into_storage()));
            }
        }
        Ok(())
    }
}
