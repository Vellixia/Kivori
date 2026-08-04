//! Emits the generated Wokwi automation scenarios + the vector catalogue.
//!
//! Run from the repository root:
//!
//! ```text
//! cargo run -p kivori-wokwi-vectors
//! ```
//!
//! Output goes to `sim/wokwi/generated/`. The files are deterministic, so CI regenerates them and fails
//! on any diff (stale vectors). Never edit the generated files by hand. The scenario builders live in the
//! library so they can be asserted on directly by tests.

use kivori_wokwi_vectors::{
    catalogue, production_runtime, protocol_serial, serial_smoke, state_cycle_serial, vectors,
};
use std::path::PathBuf;

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("sim/wokwi/generated"), PathBuf::from);
    std::fs::create_dir_all(&out_dir).expect("create generated dir");

    let files: [(&str, String); 5] = [
        ("vectors.yaml", catalogue()),
        ("protocol-serial.yaml", protocol_serial()),
        ("state-cycle-serial.yaml", state_cycle_serial()),
        ("serial-smoke.yaml", serial_smoke()),
        ("production-runtime.yaml", production_runtime()),
    ];

    for (name, contents) in files {
        let path = out_dir.join(name);
        std::fs::write(&path, contents.as_bytes()).expect("write generated file");
        println!("wrote {} ({} bytes)", path.display(), contents.len());
    }
    println!("{} vectors generated from the real codec", vectors().len());
}
