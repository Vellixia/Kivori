//! Kivori asset compiler (host) — compiles the placeholder scene SVGs into the deterministic RGB565
//! asset blob and writes it to `assets/compiled/kivori.assets` (ADR-0004, Principle XI).

use std::fs;
use std::path::Path;

fn main() {
    let blob = kivori_asset_compiler::compile_default_blob();
    let out = Path::new("assets/compiled/kivori.assets");
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir).expect("create assets/compiled");
    }
    fs::write(out, &blob).expect("write asset blob");
    eprintln!(
        "kivori-asset-compiler: wrote {} bytes to {}",
        blob.len(),
        out.display()
    );
}
