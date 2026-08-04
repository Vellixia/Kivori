# Kivori dev command recipes — run `just <recipe>`. (Dev prerequisite: https://github.com/casey/just)
set shell := ["bash", "-uc"]

# Default: list recipes.
default:
    @just --list

# ---- host workspace (root Cargo workspace + pnpm) ----

# Format, clippy, eslint, typecheck, prettier.
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm exec eslint .
    pnpm -r --if-present typecheck
    pnpm exec prettier --check .

# Host workspace + frontend tests.
test:
    cargo test --workspace
    pnpm -r --if-present test

# Build the host workspace + frontend.
build:
    cargo build --workspace
    pnpm -r --if-present build

# ---- firmware (isolated workspace; ADR-0001) ----
# no_std isolation proof: compile ONLY the shared crates for the RISC-V target from the firmware
# workspace. Nothing esp-hal here, so this stays fast and proves the firewall.

# Compile the shared crates for RISC-V (no_std / no-alloc isolation proof).
fw-check:
    cd firmware/esp32-c3 && cargo build -p kivori-model -p kivori-protocol -p kivori-framebuffer -p kivori-renderer -p kivori-assets

# The flashable binary needs `--features embedded`: the bin target declares it in `required-features`, so a
# plain `cargo build --release` silently builds only the library and produces NO binary.

# Build the flashable RISC-V firmware binary.
fw-build:
    cd firmware/esp32-c3 && cargo build --release --features embedded

# Host-side device-core tests (no hardware, no simulator).
fw-test:
    cd firmware/esp32-c3 && cargo test --features host-sim --target $(rustc -vV | sed -n 's/^host: //p')

# Flashes and monitors via the `espflash` runner in .cargo/config.toml.
#
# NOTE: the `embedded` build brings up the clock and USB Serial/JTAG and then deliberately STOPS before the
# render loop, because no confirmed panel profile exists (controller, pin map, offsets are unmeasured — see
# docs/validation-checklist.md items 23-25). It will NOT draw scenes. Add a real profile to
# firmware/esp32-c3/src/profile.rs first; until then use `just sim-test` for end-to-end behaviour.

# Flash + monitor a real board (see the note above: no panel profile yet, so no scenes).
fw-flash:
    cd firmware/esp32-c3 && cargo run --release --features embedded

# ---- Wokwi pre-hardware simulation gate (sim/wokwi/README.md) ----

# Build all four simulation artifacts (self-test, external serial, SPI probe, production runtime).
sim-build:
    bash scripts/build-wokwi-firmware.sh

# Run every Wokwi scenario (needs wokwi-cli + a CI WOKWI_CLI_TOKEN; never false-passes).
sim-test: sim-build
    bash scripts/test-wokwi.sh

# Offline checks for the gate's own tooling (no token, no simulator).
sim-check:
    cargo test -p kivori-wokwi-vectors -p kivori-wokwi-vcd
    bash scripts/test-wokwi-evidence.sh

# ---- assets / golden frames ----

# Recompile the canonical asset blob.
assets:
    cargo run -p kivori-asset-compiler

# Verify rendered frames against the committed golden hashes.
golden:
    cargo test -p kivori-golden-frames

# No automated bless exists: golden frames are committed deliberately, so a change is reviewed by hand.
golden-bless:
    @echo "No bless mode. Golden hashes live in tests/golden-frames/; a mismatch means the renderer moved."
    @echo "Investigate with 'just golden' first. Only update the committed frames if the change is intended."

# ---- architecture firewall (ADR-0001) ----

# Direct-dependency firewall over the shared no_std crates.
check-boundaries:
    bash scripts/check-crate-boundaries.sh

# ---- dev ----

# Run the desktop UI dev server.
dev:
    pnpm --filter kivori-desktop-ui dev
