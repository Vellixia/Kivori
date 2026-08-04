use std::path::Path;

fn main() {
    // Compile the canonical asset blob into OUT_DIR at build time so the binary bundles it. The runtime
    // reads it via `include_bytes!` — no SVG/resvg at runtime (Constitution XI). Build-time only.
    let blob = kivori_asset_compiler::compile_default_blob();
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    std::fs::write(Path::new(&out_dir).join("kivori.assets"), &blob)
        .expect("write compiled asset blob");

    tauri_build::build();
}
