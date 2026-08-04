//! Layer compositor: draws a scene (from a compiled [`AssetBlob`]) into a [`TileBand`] via
//! embedded-graphics (ADR-0004). Background fill, then z-ordered layers — sprite blits
//! (`ImageRawLE`), solid rects, and bitmap text — positioned by the step-held keyframe resolver.

use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::{ascii, MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565 as EgRgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use kivori_assets::{AssetBlob, SceneDef};
use kivori_framebuffer::TileBand;
use kivori_model::{resolve_transform, ElapsedMs, FontId, LayerKind, Rgb565};

/// Error compositing a scene: a layer referenced an asset/string/frame that was missing or malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// A sprite layer referenced a bitmap not present in the blob.
    MissingBitmap,
    /// A sprite frame's pixel bytes were shorter than `frame_size` requires.
    ShortBitmapData,
    /// A text layer referenced a string not present in the blob.
    MissingString,
}

/// Converts the canonical [`Rgb565`] to embedded-graphics' `Rgb565` (identical bit layout).
fn eg_color(c: Rgb565) -> EgRgb565 {
    EgRgb565::from(RawU16::new(c.raw()))
}

/// Maps a [`FontId`] to a built-in embedded-graphics mono bitmap font (ADR-0004: text uses e-g's
/// compiled bitmap fonts rather than a custom font in this slice).
fn mono_font(font: FontId) -> &'static MonoFont<'static> {
    match font {
        1 => &ascii::FONT_9X15,
        _ => &ascii::FONT_6X10,
    }
}

/// Reduces `elapsed_ms` into the scene's animation loop `[0, loop_ms)`; static scenes pass through.
fn loop_time(scene: &SceneDef, elapsed_ms: ElapsedMs) -> ElapsedMs {
    if scene.frame_count > 1 && scene.fps.num > 0 {
        let loop_ms = (u64::from(scene.frame_count) * 1000 * u64::from(scene.fps.den)
            / u64::from(scene.fps.num)) as u32;
        if loop_ms > 0 {
            return elapsed_ms % loop_ms;
        }
    }
    elapsed_ms
}

/// Renders `scene` at `elapsed_ms` into `band`: background fill, then each visible layer resolved and
/// drawn. Deterministic and integer-only. Out-of-band pixels are dropped by the tile (tiling).
///
/// # Errors
/// [`RenderError`] if a layer references a missing/short asset or a missing string.
pub fn render_scene(
    blob: &AssetBlob,
    scene: &SceneDef,
    elapsed_ms: ElapsedMs,
    band: &mut TileBand,
) -> Result<(), RenderError> {
    band.fill(scene.background);
    let t = loop_time(scene, elapsed_ms);

    for layer in &scene.layers {
        let tf = resolve_transform(&layer.keyframes, t);
        if !tf.visible {
            continue;
        }
        let x = i32::from(layer.origin.x.saturating_add(tf.offset.x));
        let y = i32::from(layer.origin.y.saturating_add(tf.offset.y));
        let pos = Point::new(x, y);

        match layer.kind {
            LayerKind::SolidRect { size, color } => {
                Rectangle::new(pos, Size::new(u32::from(size.w), u32::from(size.h)))
                    .into_styled(PrimitiveStyle::with_fill(eg_color(color)))
                    .draw(band)
                    .unwrap(); // DrawTarget error is Infallible
            }
            LayerKind::Sprite { asset, frame_size } => {
                let entry = blob.bitmap(asset).ok_or(RenderError::MissingBitmap)?;
                let pixels = blob
                    .bitmap_pixels(entry)
                    .ok_or(RenderError::MissingBitmap)?;
                let fw = u32::from(frame_size.w);
                let frame_bytes = (fw as usize) * usize::from(frame_size.h) * 2;
                if frame_bytes == 0 {
                    continue;
                }
                let frames = entry.frames.max(1);
                let frame = usize::from(tf.sprite_frame % frames);
                let start = frame * frame_bytes;
                let end = start + frame_bytes;
                let data = pixels.get(start..end).ok_or(RenderError::ShortBitmapData)?;
                let raw = ImageRawLE::<EgRgb565>::new(data, fw);
                Image::new(&raw, pos).draw(band).unwrap(); // Infallible
            }
            LayerKind::Text {
                font,
                string,
                color,
            } => {
                let text = blob.string(string).ok_or(RenderError::MissingString)?;
                let style = MonoTextStyle::new(mono_font(font), eg_color(color));
                Text::new(text, pos, style).draw(band).unwrap(); // Infallible
            }
        }
    }
    Ok(())
}
