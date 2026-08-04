#![no_std]
#![warn(missing_docs)]
//! `kivori-assets` — the zero-copy runtime reader for Kivori's compiled asset blob (ADR-0004,
//! FR-021).
//!
//! `no_std`, no heap: the structured manifest (scenes + bitmap/string tables) deserializes into
//! bounded `heapless` collections, while sprite pixels and strings are borrowed zero-copy from the
//! blob's data pool. Source SVG/PNG is never parsed at runtime — the host `kivori-asset-compiler`
//! produces this blob. Both Device Studio and the firmware read the same blob (Principle II).

pub mod manifest;
pub mod reader;

pub use manifest::{
    BitmapEntry, LayerDef, Manifest, PoolRef, SceneDef, MAX_BITMAPS, MAX_KEYFRAMES, MAX_LAYERS,
    MAX_SCENES, MAX_STRINGS,
};
pub use reader::{AssetBlob, AssetError, FORMAT_VERSION, HEADER_LEN, MAGIC};
