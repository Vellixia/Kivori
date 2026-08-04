# Quickstart: Device Connection Foundation

**Feature**: `001-device-connection-foundation` | **Date**: 2026-07-17

How to set up the monorepo, run the pieces, and **validate** that the feature works end-to-end. This is
a run/validation guide — implementation lives in `tasks.md` and the code. See [plan.md](./plan.md),
[data-model.md](./data-model.md), and [contracts/](./contracts/) for design detail.

## Prerequisites

- **Rust** (host toolchain per `rust-toolchain.toml`) + the embedded target
  `rustup target add riscv32imc-unknown-none-elf`.
- **Node** + **pnpm** (frontend workspace).
- **Tauri v2** system prerequisites for Windows (WebView2, MSVC build tools).
- **espflash** (`cargo install espflash`) for firmware flashing.
- **just** (`cargo install just`) for command recipes.
- Everything runs **offline** after dependencies are fetched (FR-029).

## Repository layout

See [plan.md → Project Structure](./plan.md#project-structure). Key: a **host root Cargo workspace**, a
**separate firmware Cargo workspace** (`firmware/esp32-c3/`), and a **pnpm workspace** for the frontend.

## Setup

```bash
pnpm install                                   # frontend + shared UI
cargo fetch                                    # host workspace deps
just assets                                    # compile source SVGs → runtime blob + verify hash
(cd firmware/esp32-c3 && cargo fetch)          # firmware workspace deps
```

## Development commands (justfile)

| Command | Does |
|---------|------|
| `just dev` | `tauri dev` with Device Studio enabled (dev build) |
| `just test` | all host Rust tests + frontend Vitest/axe |
| `just lint` | `clippy -D warnings` + `rustfmt --check` + ESLint + Prettier + `tsc --noEmit` |
| `just assets` | compile assets + assert blob hash matches committed manifest |
| `just fw-build` | build firmware (embedded workspace, compile-only) |
| `just fw-flash` | flash firmware to a connected ESP32-C3 via espflash |
| `just golden` | run golden-frame / frame-hash determinism suite |
| `just golden-bless` | regenerate golden frames (deliberate visual change only) |

## Automated validation (no hardware required)

Run `just test` + `just golden`. These prove the host-verifiable slice:

| Scenario | Command | Expected | Guards |
|----------|---------|----------|--------|
| Renderer determinism | `just golden` | Every state at sampled `elapsed_ms` matches the committed RGB565 hash; identical on Windows & Linux | FR-019/033, SC-005/009 |
| Timing drift-free | `cargo test -p kivori-renderer timing` | `elapsed_ms=(n*1000+15)/30` matches expected over long `n`; periodic every 30 | FR-019, SC-011 |
| Protocol round-trip | `cargo test -p kivori-protocol codec` | Every `Message` encodes/decodes identically | FR-034 |
| Malformed frames / fuzz | `cargo test -p kivori-protocol malformed` | Arbitrary/truncated/bad-CRC/bad-COBS input never panics; decoder resyncs | FR-034, SC-008 |
| Version matrix | `cargo test -p kivori-protocol version` | major mismatch → incompatible; minor → `min`; caps → intersection | FR-003 |
| Connection FSM | `cargo test -p kivori-desktop fsm` | Injected events drive correct transitions, backoff, and desired-state resync | FR-005/007/008/009/010 |
| Host-side firmware logic | `cargo test -p kivori-firmware --features host-sim` | Handshake/state/tile-render over an in-memory pipe; stitched tiles == golden full-frame | FR-035, SC-005 |
| IPC + redaction | `cargo test -p kivori-desktop ipc` | Command shapes correct; no log/DTO carries sensitive data; dev-only commands absent from release | FR-031/032, SC-010, FR-028 |
| Frontend + a11y | `pnpm --filter desktop test` | Controls behave; keyboard-operable, focus visible, labels present (axe); canvas blits given bytes verbatim | a11y reqs, constraint 2 |
| `no_std` firmware build | `just fw-build` | Shared crates compile for `riscv32imc-unknown-none-elf` with no alloc | Principle IV |
| Asset determinism | `just assets` | Recompiled blob hash == committed manifest | Principle XI/III |

## Device Studio validation (no hardware)

1. `just dev` → open the Device Studio route (dev build only).
2. Select each state (`booting, idle, happy, busy, sleeping, offline`) → preview shows that scene (FR-023).
3. Set the elapsed-time field to a fixed value → the exact frame renders; re-entering the value renders
   the identical frame (SC-011).
4. Play, then pause → animation halts; step → timeline advances exactly one 33.333 ms step (FR-025/026).
5. Confirm the preview updates come only from `render_preview_frame`/the preview channel (the canvas
   never draws content itself — constraint 2).

## Physical ESP32-C3 validation procedure (manual)

Hardware automation is unavailable, so this checklist covers true integration (Principle X, FR-035).
Record measured latencies and the [hardware-validation numbers](./research.md#hardware-validation-required-do-not-trust-without-a-physical-esp32-c3).

**Step 0 — pre-flash gate (do this first)**

Run `just sim-test`. All eight Wokwi scenarios must pass, including `production-runtime`, which executes the
*same* run loop the physical firmware calls. This catches protocol, lifecycle, renderer, and tile-output
regressions before a board is involved. It proves **nothing** about the panel — see
[sim/wokwi/README.md](../../sim/wokwi/README.md) "What this gate does not prove".

**Step 0b — capture the three unknown panel facts (BLOCKS steps 7-8 and 12-13)**

The display half of this procedure cannot run until these are measured and recorded in
[validation-checklist.md](../../docs/validation-checklist.md) items 23-25:

| Fact | Why it blocks | Where it goes |
|---|---|---|
| Panel controller (GC9A01 vs ST7789) | `mipidsi` needs the model; init sequences differ | `DeviceProfile::KIVORI_240.controller` |
| SPI/CS/D-C/RST + backlight pin map | `bsp::display_bus` takes pin handles, never defaults | a new profile in `firmware/esp32-c3/src/profile.rs` |
| Visible-area offsets | `PanelGeometry` has no `Default` on purpose | the same profile |

Nothing in the tree guesses these. `firmware/esp32-c3/src/profile.rs` holds **only** a simulation-only Wokwi
profile; add a sibling module behind its own feature once the facts are known.

**Setup**
1. `just fw-build`, then `just fw-flash` to flash the device; leave it powered via USB.

   **Until step 0b is done, the `embedded` build brings up the clock and USB Serial/JTAG and then
   deliberately stops before the render loop — it will NOT draw scenes.** That is intentional: entering the
   loop requires a controller, pin map, and offsets nobody has measured. Steps 2, 7, 8, and the display half
   of 12-13 therefore only become runnable once a real profile exists.
2. Once a profile exists: on power-on, confirm the device shows the **device-originated `booting`** scene,
   then **`offline`** before any desktop drives it (FR-014).
3. `just dev` (or run a release build) to launch the desktop.

**US1 — discovery & connection (P1)**
4. Confirm the app auto-discovers the device with **no port selection** and shows `connecting` →
   `connected`; connected info shows protocol/firmware version. Measure time from plug-in → connected
   (**target < 5 s**, SC-001).
5. Attach an unrelated USB-serial device → confirm it is **not** reported as connected (FR-004).
6. Flash/attach a deliberately **wrong-major** firmware build → confirm `incompatible` with a clear
   reason, surfaced **< 5 s** (SC-003), and no state commands are sent.

**US2 — companion states on the device (P2)**
7. Set each of `idle/happy/busy/sleeping` from the app → the device shows the matching scene with its
   canonical animation; each change appears **< 1 s** (SC-004).
8. For a fixed state + elapsed time, compare the device against the Device Studio preview → **identical
   image** (SC-005 spot-check).
9. Confirm only semantic state is sent (no pixel/coordinate traffic) — verify via safe diagnostics
   message types, not payload bytes (FR-015).

**US3 — recovery & restoration (P3)**
10. With the device showing `busy`, **unplug** it → app shows `disconnected`.
11. **Replug** → app auto-reconnects and the device returns to `busy` with no user action; measure
    reconnect+restore (**target < 10 s**, SC-002).
12. Rapidly unplug/replug several times → the app settles into a stable connected state without thrash
    (FR-010).
13. Close the desktop window to background → the connection is maintained (SC-007); the device keeps its
    last desired state until the session actually ends, then falls back to `offline`.
14. Fully **restart** the desktop app → desired state resets to `idle` (no cross-restart persistence,
    clarified).

**Safety/robustness**
15. While connected, confirm no sensitive data appears in the diagnostics view or log file (SC-010).

## Troubleshooting

- **Device resets on connect**: DTR/RTS control-line handling — see
  [plan → Windows serial behavior](./plan.md#windows-serial-device-behavior-implemented) (R3).
- **`COM10+` not found**: handled by the serial crate's `\\.\COM10` form; if manual, use that prefix.
- **Flaky async serial on Windows**: switch the transport to the blocking `serialport` +
  `spawn_blocking` fallback (R2 / [research §R-5](./research.md#r-5-desktop-serial-tokio-serial-with-a-spawn_blocking-fallback)).
- **Golden-frame mismatch after an asset change**: if intentional, `just golden-bless` and review the
  diff; if not, an asset-toolchain or renderer nondeterminism regressed (R5/R6).
```
