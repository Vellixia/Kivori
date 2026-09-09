use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../../assets/mascot.svg");
    // Compile the canonical asset blob into OUT_DIR at build time so the binary bundles it. The runtime
    // reads it via `include_bytes!` — no SVG/resvg at runtime (Constitution XI). Build-time only.
    let blob = kivori_asset_compiler::compile_default_blob();
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    std::fs::write(Path::new(&out_dir).join("kivori.assets"), &blob)
        .expect("write compiled asset blob");

    // Firmware is built in its isolated workspace first. Opt in explicitly so ordinary host/CI
    // builds do not silently package an old cross-compiled artifact from a previous local build.
    println!("cargo:rerun-if-env-changed=KIVORI_FIRMWARE_PATH");
    let firmware = match std::env::var_os("KIVORI_FIRMWARE_PATH") {
        Some(path) => {
            let path = Path::new(&path);
            println!("cargo:rerun-if-changed={}", path.display());
            let bytes = std::fs::read(path).expect("read requested firmware ELF");
            assert!(
                bytes.len() >= 52 && &bytes[..6] == b"\x7fELF\x01\x01" && bytes[18..20] == [243, 0],
                "firmware must be a little-endian ELF32 RISC-V image"
            );
            bytes
        }
        None => Vec::new(),
    };
    std::fs::write(Path::new(&out_dir).join("kivori-firmware.elf"), firmware)
        .expect("write bundled firmware");

    tauri_build::build();
}
