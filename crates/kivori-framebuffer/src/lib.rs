#![no_std]
#![warn(missing_docs)]
//! `kivori-framebuffer` — RGB565 tile/scanline buffers and dirty-region primitives.
//!
//! `no_std`, no alloc: a [`TileBand`] borrows caller-provided pixel storage (a full-frame buffer on
//! the host, a small tile buffer on the device). [`TileSignature`] hashes a tile's content so the
//! firmware can flush only the tiles that changed (change-driven rendering, FR-013).

pub mod dirty;
mod draw_target;
pub mod tile;

pub use dirty::{hash_rgb565, TileSignature};
pub use tile::TileBand;
