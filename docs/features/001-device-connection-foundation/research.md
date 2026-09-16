# Research: Device Connection Foundation

**Feature**: `001-device-connection-foundation` | **Date**: 2026-07-17

**Purpose**: Resolve uncertain dependencies and ESP32-C3 target limitations behind the
[plan](./plan.md). Findings verified against current library releases (July 2026). Each item is stated
as **Decision · Rationale · Alternatives · Caveats**. A hardware-validation list at the end enumerates
facts that cannot be trusted without a physical device.

---

## R-1. Device transport: ESP32-C3 native USB Serial/JTAG

**Decision**: Use the ESP32-C3's built-in **USB Serial/JTAG** controller as the host↔device transport —
no external USB-UART bridge. In firmware, `esp_hal::usb_serial_jtag::UsbSerialJtag` (async via
`.into_async()`, implements `embedded-io`/`embedded-io-async`). Filter host-side by VID:PID
**`0x303A:0x1001`**.

**Rationale**: One USB cable carries power, flashing, logging, JTAG, and app data. It enumerates as a
standard CDC-ACM device (Windows: `usbser.sys`, "USB Serial Device (COMx)"), so the desktop serial stack
treats it as an ordinary COM port. Matches the feature's "USB serial initially" constraint with the
fewest moving parts.

**Alternatives**: External CP210x/CH34x UART bridge (more BOM, different VID/PID, but "fully stable"
esp-hal UART). Kept as a documented fallback; the VID/PID allowlist is configurable to support it.

**Caveats**: Full-Speed USB (12 Mbps theoretical); **ignores baud-rate settings** (runs at USB speed);
**64-byte** endpoint FIFO per direction; **TX can stall if the host is not draining** the port. The
esp-hal driver is behind the **`unstable`** feature (see R-2). → Feeds risks **R2**; small semantic
frames (not pixels) keep us far from the bandwidth ceiling.

## R-2. Firmware HAL: esp-hal 1.0 (no_std)

**Decision**: Target **esp-hal 1.0** (`no_std`, bare-metal). Use blocking or async peripheral modes;
adopt **`esp-hal-embassy`** only if the firmware goes fully async. **Pin the exact esp-hal version.**

**Rationale**: esp-hal reached **1.0.0 stable (30 Oct 2025)** and is Espressif's vendor-backed Rust SDK.
GPIO/UART/SPI/I2C, `esp_hal::init`, and `time` are stabilized with blocking + async modes — enough for a
display + serial companion.

**Alternatives**: `esp-idf-hal` (`std`, wraps the C ESP-IDF; threads, mature Wi-Fi/BT). Rejected: pulls
`std` and a heavier runtime; unnecessary without radios and contrary to the `no_std`/hardware-conscious
constitution.

**Caveats**: The 1.0 stability promise is **narrow** — `usb_serial_jtag` (and RMT/I2S/etc.) sit behind
the **`unstable`** feature and may churn across minor bumps. → Risk **R11**: pin the version, isolate USB
access in `transport.rs`, upgrade deliberately.

## R-3. Display driver: mipidsi (240×240 SPI, windowed writes)

**Decision**: Use **`mipidsi` (pin ~0.10.x)** for the 240×240 SPI panel (GC9A01 round or ST7789). Drive
dirty-region updates with `Display::set_pixels(sx, sy, ex, ey, colors)` and `embedded-graphics`
`DrawTarget` (`fill_contiguous`).

**Rationale**: `mipidsi` is the unified MIPI-DCS driver supporting both candidate controllers and
**exposes arbitrary address-window / sub-rectangle writes** — exactly what tile/scanline/dirty-region
rendering needs. **No full framebuffer required**; partial regions stream straight over SPI.

**Alternatives**: Single-model crates (`gc9a01-rs`, `st7789`). Rejected: `mipidsi` is the recommended
unified path and keeps the panel choice configurable via `DeviceProfile`.

**Caveats**: Notable API churn (0.9→0.10 `interface`/`SpiInterface` refactor) — **pin the version**. The
exact 240×240 controller (round GC9A01 vs square ST7789) is a `DeviceProfile` parameter; the renderer is
resolution-driven and controller-agnostic.

## R-4. ESP32-C3 memory & framebuffer strategy

**Decision**: **Tile/scanline rendering with dirty regions; no full framebuffer** (also mandated by the
spec/constitution). Budget a few KB per tile band.

**Rationale**: The C3 has **400 KB SRAM (~384 KB usable after cache)** with no fixed DRAM/IRAM split. A
full 240×240×2 = **112.5 KB** RGB565 buffer is feasible (~29% of SRAM) and — since this feature runs **no
Wi-Fi/BLE** — would actually fit comfortably. But double-buffering (~225 KB) is unrealistic, and tiling
coexists with future radio use, pairs naturally with `mipidsi::set_pixels`, and honors Principle IV.

**Alternatives**: Single full framebuffer (simpler render loop, more RAM, more SPI per frame if not
diffed). Rejected by constraint "no requirement for a full 240×240 framebuffer" and Principle IV;
revisit only for a radio-light, animation-heavy future.

**Caveats**: Whether 112.5 KB fits *a given* build depends on code/stack/heap — irrelevant here since we
tile, but relevant if the decision is ever revisited.

## R-5. Desktop serial: tokio-serial with a spawn_blocking fallback

**Decision**: Primary = **`tokio-serial` (5.4.x)** async on Tauri's Tokio runtime. Enumerate with
`available_ports()` → `SerialPortType::UsbPort(UsbPortInfo { vid, pid, serial_number, .. })` and filter
by `0x303A:0x1001`. Wrap serial behind a small internal transport trait so a **blocking `serialport`
(4.x) in `tokio::task::spawn_blocking`** fallback is a local swap.

**Rationale**: `tokio-serial` is the async wrapper over `serialport-rs` (via `mio-serial`) and fits an
async read loop. VID/PID filtering is identical across both crates. Filtering by VID/PID (not COM
number) satisfies "no fixed COM port" (FR-001).

**Alternatives**: Pure blocking `serialport` from the start (simpler, very robust) — kept as the
fallback rather than the default to preserve an async-first core.

**Caveats**: Windows quirks — `COM10+` needs `\\.\COM10` (crate-handled); **no native device-removal
event** (detect via I/O errors, heartbeat, or `available_ports()` polling); Windows async serial has
historically rough overlapped-I/O edges → the `spawn_blocking` fallback exists for exactly this. →
Risks **R2**, informs the disconnect-detection strategy.

## R-6. Protocol payloads: serde + postcard (no_std)

**Decision**: **`postcard` (1.1.x)** + `serde` for payloads in a **single shared `kivori-protocol`
crate** of message types used by both firmware and desktop. `no_std`, serialize into fixed slices /
`heapless::Vec` (no `alloc` on device).

**Rationale**: `postcard` is purpose-built for `no_std` embedded, has a **stable wire format since 1.0**,
uses compact varint (LEB128) encoding, and needs no allocator. One shared message crate guarantees both
ends agree on the schema.

**Alternatives**: `bincode` (not embedded-focused), hand-rolled binary (error-prone), `postcard-rpc`
(more structure than needed now). Rejected for footprint/complexity; `postcard-rpc`/`postcard-schema`
noted for possible future use.

**Caveats**: postcard is **not self-describing and has no field tags** — schema evolution is manual and
**append-only**: appending new variants at the end of a top-level message `enum` is the tolerable
evolution path; reordering/removing fields is a breaking change. → Directly shapes **ADR-0002**: explicit
version header bytes + capability negotiation + append-only evolution within a major version.

## R-7. Tauri v2 binary frame transport (Rust → webview)

**Decision**: Tauri **v2 (GA)**. Push preview frames as **raw bytes**: single-frame fetches return
**`tauri::ipc::Response`** wrapping `Vec<u8>` (arrives in JS as an `ArrayBuffer`, `application/octet-
stream`); continuous **play** streaming uses the **`Channel`** API. Convert RGB565→RGBA8888 for the
canvas.

**Rationale**: v2's custom-protocol IPC carries raw bytes with no JSON/base64 overhead —
`ipc::Response` for request/response, `Channel` for ordered push (the same API Tauri uses for download/
stdout streaming). Keeps the canvas a pure blit surface.

**Alternatives**: Event system (payloads are JSON strings — "not suitable for bigger messages") and
base64 (+33% size + CPU). Both rejected for the ~112 KB (RGB565) / ~225 KB (RGBA) frame path.

**Caveats**: The buffer is still **copied across the IPC boundary**, and the webview must **convert
RGB565→RGBA8888** before blitting — both real per-frame costs. Do the conversion in Rust (constraint 2)
and **measure achievable FPS**; cap preview FPS if needed. → Risk **R7**.

## R-8. Drift-free millisecond timing (design decision, no external dep)

**Decision**: Canonical timebase = integer `elapsed_ms: u32`. Device Studio keeps an integer step index
`n`; derive `elapsed_ms = (n * 1000) / 30` (round with `+15`). Never accumulate `+33.333`.

**Rationale**: Deriving time from an integer index is a pure function of `n`, eliminating cumulative
floating-point drift while still giving a 30 Hz inspection cadence and exact 1000 ms periodicity every 30
steps. Satisfies the explicit "without cumulative floating-point drift" requirement.

**Alternatives**: Accumulate a `f64` millisecond counter (drifts); track microseconds with a 33333 µs
step (drifts ~10 µs/s). Both rejected. → **ADR-0003**, verified by a drift unit test.

## R-9. Deterministic asset compilation (host)

**Decision**: `tools/asset-compiler` rasterizes layered SVGs with **`resvg`/`usvg` + `tiny-skia`** to
RGB565, packs bitmap fonts, and emits a byte-reproducible compiled blob + manifest. CI recompiles and
diffs the hash.

**Rationale**: `resvg`/`tiny-skia` are pure-Rust and deterministic for identical inputs; pinning
versions + fixed rounding + stable ordering + no timestamps yields byte-reproducible output — the
foundation for cross-platform golden frames (Principle III/XI).

**Alternatives**: Runtime SVG parsing (violates Principle XI), or a C rasterizer (harder to pin
deterministically). Rejected. → **ADR-0004**.

**Caveats**: Any rasterizer upgrade can change output bytes; treat asset-toolchain bumps as deliberate,
golden-refreshing changes.

## R-10. Firmware workspace isolation (design decision)

**Decision**: Firmware in a **separate Cargo workspace** with path deps into `crates/*` (see plan
[Workspace boundaries](./plan.md#workspace-boundaries)).

**Rationale**: Prevents Cargo feature unification from leaking `std`/`alloc` features into shared crates
and breaking the firmware `no_std` build; lets the embedded target own its `.cargo/config.toml`, target
triple, panic strategy, and runner. → **ADR-0001**, enforced by a CI job that builds shared crates for
`riscv32imc-unknown-none-elf`.

**Alternatives**: Single root workspace (simpler, one lockfile) — rejected for the feature-unification
hazard. Per-crate independent packages without a workspace (loses shared lockfile/tooling) — unnecessary.

---

## Hardware-validation-required (do NOT trust without a physical ESP32-C3)

These cannot be settled by research and are folded into the [hardware procedure](./quickstart.md) and
risks R1–R3/R11:

1. **USB Serial/JTAG sustained throughput** — 12 Mbps is theoretical; real figure with the 64-byte FIFO
   and esp-hal flush behavior is unknown. (Ample for our small frames, but measure.)
2. **USB TX write-blocking / robustness** — the "TX stalls when no host reads" behavior and
   disconnect/reconnect recovery must be exercised on-device.
3. **Achievable display FPS over SPI** — full-frame vs tile updates at the chosen SPI clock; datasheet
   numbers don't predict real refresh rate. (Drives whether SC-004 latency holds.)
4. **`tokio-serial` async reliability on Windows** with the C3's native CDC port specifically, and
   device-removal detection latency.
5. **End-to-end preview latency/FPS** through Tauri IPC + webview RGB565→RGBA blit (measure the whole
   pipe, not just the Rust side).
6. **112.5 KB framebuffer fit** — only relevant if the tile decision is ever revisited; confirm against a
   real linked build.
7. **esp-hal `usb_serial_jtag` API stability** — behind `unstable`; expect possible breakage on upgrade.

## Sources

esp-hal 1.0 announcement (Espressif); `esp_hal::usb_serial_jtag` docs; ESP-IDF USB Serial/JTAG console
guide; ESP32-C3 memory-types guide; `mipidsi` (docs.rs / GitHub almindor/mipidsi); `serialport-rs` +
`tokio-serial` `UsbPortInfo` docs; `postcard` (GitHub jamesmunns/postcard); Tauri 2.0 stable blog + Tauri
IPC documentation.
