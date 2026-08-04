---
description: "Task list for Device Connection Foundation"
---

# Tasks: Device Connection Foundation

**Input**: Design documents from `specs/001-device-connection-foundation/`
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [data-model.md](./data-model.md),
[research.md](./research.md), [contracts/protocol.md](./contracts/protocol.md),
[contracts/ipc.md](./contracts/ipc.md)

**Tests**: Included. This feature explicitly requires deterministic rendering tests, protocol codec /
malformed-frame tests, and a host-side firmware simulation path (FR-033, FR-034, FR-035, spec Success
Criteria, constitution III & X). Contract-defining tests are placed before or alongside the
implementation they pin.

## Format: `[ID] [P?] [Story?] Description with exact file path`

- **[P]**: safely parallelizable (different files, no dependency on an incomplete task).
- **[Story]**: `[US1]`–`[US4]` on vertical-slice phases; Setup / Foundational / Cross-cutting phases
  carry no story label (they map to stories/requirements via the description).
- **[MANUAL]**: human-run OS or physical-device check — excluded from ordinary host CI; never gates a
  merge.

## Phase ↔ user-story map

Build order follows the required dependency progression (host-testable foundations first), not raw story
priority. Traceability to spec user stories / requirements:

| Phase | Title | Role | Stories / Requirements |
|------:|-------|------|------------------------|
| 1 | Repository & Workspace Setup | Setup | — |
| 2 | Shared Domain Foundation | Foundational | all |
| 3 | Protocol Contract | Foundational | US1, US2, US3 |
| 4 | Deterministic Rendering Foundation | Foundational | US2, US4 |
| 5 | Deterministic Asset Pipeline | Foundational | US2, US4 |
| 6 | Device Studio Vertical Slice | Slice | **US4** (P4) |
| 7 | Firmware Host-Testable Core | Slice | **US2** (P2) |
| 8 | ESP32 Hardware Adapter | Slice | **US2** (P2) |
| 9 | Desktop Connection Manager | Slice | **US1** (P1), **US3** (P3) |
| 10 | Desktop Lifecycle & Background Operation | Cross-cutting | FR-030, SC-007 (US1/US3 continuity) |
| 11 | End-to-End Desktop/Device Slice | Slice | US1, US2, US3 |
| 12 | Diagnostics & Security Controls | Cross-cutting | all |
| 13 | Offline Runtime Boundary | Cross-cutting | FR-029, SC-006 |
| 14 | CI & Documented Validation | Cross-cutting | all |
| 15 | Hardware Validation | Manual | US1, US2, US3 |

**ADR gating**: ADR-0001 → T001 (before Phase 1 scaffolding); ADR-0003 → T012 (before the timeline type);
ADR-0002 → T021 (before protocol impl); ADR-0004 → T043 (before the asset compiler); ADR-0005 → T100
(before diagnostics impl). The major-version compatibility *policy* is already fixed by the spec
clarification, so domain negotiation helpers (T014) may precede ADR-0002's wire formalization.

---

## Phase 1: Repository & Workspace Setup

**Goal**: Monorepo scaffolding with the two-workspace Cargo split, locked package names, and `no_std`
feature isolation + dependency firewall enforced in CI.
**Independent test**: `cargo metadata` shows the locked package names; `just` recipes run; the firmware
CI job compiles shared crates for `riscv32imc-unknown-none-elf`; the boundary check passes.

- [x] T001 [P] Write ADR-0001 (two-workspace Cargo split, path deps, feature-unification firewall) in `docs/adr/0001-two-workspace-cargo-split.md`
- [x] T002 Create root Cargo workspace `Cargo.toml` (members: `crates/*`, `apps/desktop/src-tauri`, `tools/asset-compiler`, `tests/golden-frames`) and host `rust-toolchain.toml`
- [x] T003 [P] Create `pnpm-workspace.yaml` (members `apps/desktop`, `packages/ui`) and root `package.json`
- [x] T004 [P] Create the isolated firmware workspace `firmware/esp32-c3/Cargo.toml` with `[workspace]` + `[package] name = "kivori-firmware"`, `firmware/esp32-c3/.cargo/config.toml` (target `riscv32imc-unknown-none-elf`, `espflash` runner), and `firmware/esp32-c3/rust-toolchain.toml`
- [x] T005 [P] Scaffold `no_std` shared-crate skeletons with locked package names `kivori-model`, `kivori-protocol`, `kivori-framebuffer`, `kivori-renderer`, `kivori-assets` in `crates/*/Cargo.toml` + `src/lib.rs` (`#![no_std]`)
- [x] T006 Create the desktop core crate manifest `apps/desktop/src-tauri/Cargo.toml` with `[package] name = "kivori-desktop"`, a minimal `apps/desktop/src-tauri/tauri.conf.json`, and frontend scaffold `apps/desktop/{package.json, vite.config.ts, index.html}`
- [x] T007 Create the `justfile` with recipes: `dev`, `test`, `lint`, `assets`, `fw-build`, `fw-flash`, `golden`, `golden-bless`, `check-boundaries`
- [x] T008 [P] Add CI workflow `.github/workflows/host.yml` skeleton (fmt, `clippy -D warnings`, `cargo test`; frontend lint/`tsc`/vitest/build)
- [x] T009 [P] Add CI workflow `.github/workflows/firmware.yml` that builds the firmware workspace + shared crates for `riscv32imc-unknown-none-elf` (proves `no_std`/no-alloc feature isolation; Principle IV)
- [x] T010 [P] Add CI workflow `.github/workflows/determinism.yml` skeleton (golden-hash + asset-hash jobs, filled in later phases)
- [x] T011 [P] Add the direct-dependency firewall check `scripts/check-crate-boundaries.sh` (via `cargo metadata`, direct deps only) asserting `crates/kivori-*` do not directly depend on `tauri`/`tokio`/`serialport`/`tokio-serial`/`axum`/`tower`/`sqlx`/OS-desktop crates, wired into `.github/workflows/host.yml` (plan Direct-dependency firewall; Principle IV, constraint 4)

---

## Phase 2: Shared Domain Foundation (`kivori-model`)

**Goal**: The three separate state axes, device profile, elapsed-ms timeline, and protocol/capability
version types, with validation and state-transition tests.
**Independent test**: `cargo test -p kivori-model` passes; crate builds `no_std` with no alloc.

- [x] T012 Write ADR-0003 (timing model: integer-ms timebase; drift-free 33.333 ms step via derived index) in `docs/adr/0003-timing-model.md`
- [x] T013 [P] Define companion/sendable/lifecycle state axes + `ConnectionState` in `crates/kivori-model/src/state.rs` and `crates/kivori-model/src/connection.rs` (data-model §1–2; FR-005, FR-011, FR-012, FR-014)
- [x] T014 [P] Define `ProtocolVersion`, `Capabilities` bitflags, and compatibility/negotiation helpers in `crates/kivori-model/src/version.rs` and `crates/kivori-model/src/capabilities.rs` (data-model §3; FR-003)
- [x] T015 [P] Define `DeviceProfile`, `ColorFormat`, `PanelController`, `TileConfig` in `crates/kivori-model/src/profile.rs` (data-model §7; FR-020)
- [x] T016 [P] Define `ElapsedMs`, `StudioTimeline`, and drift-free `elapsed_ms(step_index)=(n*1000+15)/30` in `crates/kivori-model/src/timeline.rs` (data-model §5; ADR-0003; FR-019)
- [x] T017 [P] Define geometry (`Point`/`Size`/`Rect`, integer) and `Rgb565` in `crates/kivori-model/src/geometry.rs` and `crates/kivori-model/src/color.rs` (data-model §6)
- [x] T018 [P] Tests: state-transition legality — only `SendableState` is sendable; `booting`/`offline` device-originated — in `crates/kivori-model/tests/state.rs` (FR-012, FR-014, FR-015)
- [x] T019 [P] Tests: version compatibility matrix (major match/mismatch, minor `min`, capability intersection) in `crates/kivori-model/tests/version.rs` (FR-003)
- [x] T020 [P] Tests: timeline drift-free over long `n`, periodicity every 30 steps, `±1` stepping in `crates/kivori-model/tests/timeline.rs` (FR-019, SC-011)

---

## Phase 3: Protocol Contract (`kivori-protocol`)

**Goal**: COBS framing, fixed header, CRC-32, sequence numbers, postcard payloads, handshake,
major-version compatibility, capability negotiation, malformed-frame handling, and golden protocol
vectors — each required behavior represented by an explicit test.
**Independent test**: `cargo test -p kivori-protocol` passes (all validation/negotiation/malformed files
+ vectors); builds `no_std`.

- [x] T021 Write ADR-0002 (wire protocol: COBS + header + CRC-32 + seq + postcard, append-only evolution, negotiation, sequence policy) in `docs/adr/0002-wire-protocol.md`
- [x] T022 [P] Define the append-only `Message` enum + payload structs in `crates/kivori-protocol/src/message.rs` (contracts/protocol.md §3; data-model §10)
- [x] T023 Implement the frame codec: COBS framing, fixed little-endian header (magic + version + seq + len), `payload_len` bound, CRC-32 in `crates/kivori-protocol/src/frame.rs` (protocol §2)
- [x] T024 Implement postcard payload encode/decode + message dispatch + sequence policy (duplicate/gap/wraparound) in `crates/kivori-protocol/src/codec.rs` (protocol §3, §6)
- [x] T025 [P] Implement handshake types/logic (`Hello`/`HelloAck`/`Ready`/`Bye`, nonce echo) in `crates/kivori-protocol/src/handshake.rs` (protocol §4; FR-002)
- [x] T026 [P] Implement version + capability negotiation (major gate, `min` minor, capability intersection) in `crates/kivori-protocol/src/negotiate.rs` (protocol §5; FR-003)
- [x] T027 [P] Implement the error taxonomy + `ByeReason` in `crates/kivori-protocol/src/error.rs`
- [x] T028 [P] Test: round-trip encode/decode of every `Message` variant in `crates/kivori-protocol/tests/roundtrip.rs` (FR-034)
- [x] T029 [P] Test: header validation — invalid magic, invalid/unsupported header version, truncated header — in `crates/kivori-protocol/tests/header_validation.rs` (FR-034; protocol §8)
- [x] T030 [P] Test: payload validation — declared `payload_len` > received, payload > `MAX_PAYLOAD`, CRC mismatch, malformed postcard, unknown message discriminant/variant — in `crates/kivori-protocol/tests/payload_validation.rs` (FR-034, SC-008)
- [x] T031 [P] Test: sequence handling — duplicate, gap, wraparound as valid decoded frames (protocol policy, not malformed bytes) — in `crates/kivori-protocol/tests/sequence_handling.rs` (FR-034; protocol §6)
- [x] T032 [P] Test: version negotiation — incompatible major version, compatible minor-version difference — in `crates/kivori-protocol/tests/version_negotiation.rs` (FR-003)
- [x] T033 [P] Test: capability negotiation — unsupported capability ignored/rejected per negotiation rules — in `crates/kivori-protocol/tests/capability_negotiation.rs` (FR-003)
- [x] T034 [P] Test: malformed frames — arbitrary malformed COBS never panics; recovery after a malformed frame followed by a valid frame — in `crates/kivori-protocol/tests/malformed_frames.rs` (FR-034, SC-008)
- [x] T035 Create committed golden protocol vectors (canonical byte encodings) in `crates/kivori-protocol/tests/vectors/` + stability test in `crates/kivori-protocol/tests/vectors.rs` (wire-format lock)

---

## Phase 4: Deterministic Rendering Foundation (`kivori-framebuffer`, `kivori-renderer`)

**Goal**: RGB565 types, tile/scanline output, integer frame-step derivation, per-scene frame selection,
change detection, frame hashing, and golden-frame tests (bootstrapped with a code-defined test scene;
asset-backed goldens added in Phase 5).
**Independent test**: `cargo test -p kivori-renderer` and `just golden` pass identically on Windows and
Linux; crates build `no_std`.

- [x] T036 [P] Implement tile/scanline framebuffer (`TileBand`, `Rect`, address-window helpers) in `crates/kivori-framebuffer/src/tile.rs` (data-model §8)
- [x] T037 [P] Implement per-tile change detection (`TileSignature`) in `crates/kivori-framebuffer/src/dirty.rs` (FR-013)
- [x] T038 Implement the deterministic renderer core `render(scene, elapsed_ms, assets) -> tile` (integer math only, no float) in `crates/kivori-renderer/src/render.rs` (FR-016, FR-017, FR-019)
- [x] T039 [P] Implement per-scene frame selection (integer fps mapping / keyframe lookup) in `crates/kivori-renderer/src/frame_select.rs` (per-scene rate)
- [x] T040 [P] Implement RGB565 frame hashing in `crates/kivori-renderer/src/hash.rs` (FR-033)
- [x] T041 Create the golden-frame harness (package `kivori-golden-frames`) + `manifest.toml` rendering a code-defined test scene at sampled `elapsed_ms` and comparing RGB565 hashes in `tests/golden-frames/{Cargo.toml,tests/golden.rs,manifest.toml}` (FR-033, SC-005, SC-009)
- [x] T042 [P] Determinism test: repeated-run + cross-run frame identity in `tests/golden-frames/tests/determinism.rs` (SC-009)

---

## Phase 5: Deterministic Asset Pipeline (`kivori-asset-compiler`, `kivori-assets`, `assets/`)

**Goal**: Layered SVG authoring inputs → compiled RGB565 bitmaps + bitmap fonts + manifest; reproducible
output checks; manifest validation; no runtime SVG/PNG parsing.
**Independent test**: `just assets` reproduces an identical blob hash; `kivori-assets` reads the blob;
asset-backed golden frames pass.

- [x] T043 Write ADR-0004 (compiled asset format + deterministic compilation contract) in `docs/adr/0004-asset-format.md`
- [x] T044 [P] Author placeholder layered SVGs + animation metadata for all six scenes in `assets/scenes/{booting,idle,happy,busy,sleeping,offline}/{layers.svg,animation.ron}`
- [x] T045 Define compiled-asset types (`AssetManifest`, `BitmapEntry`, `FontEntry`, `StrEntry`, `Scene`, `Layer`, `Keyframe`) in `crates/kivori-model/src/scene.rs` and `crates/kivori-assets/src/manifest.rs` (data-model §6, §9)
- [x] T046 [P] Implement the zero-copy `no_std` runtime reader + manifest validation (hash, bounds, id resolution) in `crates/kivori-assets/src/reader.rs` (FR-021)
- [x] T047 Implement the asset compiler (package `kivori-asset-compiler`): deterministic `resvg`/`tiny-skia` rasterization → RGB565, bitmap-font packing, manifest + hash emit in `tools/asset-compiler/src/{main.rs,rasterize.rs,pack.rs,manifest.rs}` (ADR-0004)
- [x] T048 [P] Reproducibility test: compile twice → byte-identical blob + hash in `tools/asset-compiler/tests/compile.rs` (Principle XI/III) — path corrected 2026-08-04: the test lives in `compile.rs` (`blob_is_byte_reproducible`), never in a `reproducible.rs`; `determinism.yml` also re-compiles twice and `cmp`s the blob
- [x] T049 Extend golden frames with asset-backed frames for all six scenes in `tests/golden-frames/tests/golden.rs` + committed frames under `tests/golden-frames/frames/` (SC-005)
- [x] T050 [P] Fill `.github/workflows/determinism.yml` with the asset-recompile hash check (Principle XI)
- [x] T051 [P] Guard test: no runtime SVG/PNG parser is linked into `kivori-assets`/firmware runtime deps in `crates/kivori-assets/tests/no_runtime_svg.rs` (FR-021) — **the named test was MISSING and is now implemented** (3 tests): it asserts none of `resvg`/`usvg`/`tiny-skia`/`image`/`png`/`jpeg-decoder`/`svgtypes`/`roxmltree` is declared by the runtime reader, any shared `no_std` crate, or the firmware; a second test fails if a guarded manifest moves (so the guard cannot go vacuous); and a negative control proves the detector catches inline, `[dependencies.x]`, and dev-dependency declarations. The host-tool asset compiler is deliberately out of scope. `scripts/check-crate-boundaries.sh` enforces the same rule in CI via `cargo metadata`

---

## Phase 6: Device Studio Vertical Slice — US4 (Priority: P4)

**Goal**: Dev-only Device Studio: Rust-generated frames, Canvas as pixel display only, all six states
simulatable, play/pause/seek/frame-step, mirror of a sendable state, keyboard accessibility + visible
focus, no independent React/Canvas renderer.
**Independent test**: With no hardware, open Device Studio, select each of the six states, use
scrub/play/pause/step deterministically; axe passes; the canvas blits provided bytes verbatim.

- [x] T052 [US4] Implement the host preview render command `render_preview_frame` (RGB565→RGBA via `kivori-renderer`, returns `tauri::ipc::Response`) in `apps/desktop/src-tauri/src/render/mod.rs` and `apps/desktop/src-tauri/src/ipc/commands.rs` (FR-022, FR-023; research R-7)
- [x] T053 [US4] Implement the preview stream `Channel` (`open_preview_stream`/`close_preview_stream`/`ack_preview_frame`, drift-free integer origin, fps cap, bounded in-flight back-pressure, cancellation + registry cleanup) in `apps/desktop/src-tauri/src/ipc/channels.rs`; consumed by Device Studio playback while scrub/step keeps the exact per-frame `render_preview_frame` path (ipc §3)
- [x] T054 [P] [US4] Gate Device Studio behind the `device-studio` cargo feature + frontend build flag; ensure absent from release in `apps/desktop/src-tauri/Cargo.toml` (feature) and `apps/desktop/src/App.tsx` (`import.meta.env.DEV`) (FR-028)
- [x] T055 [P] [US4] Implement typed IPC wrappers (invoke/events/channel) in `apps/desktop/src/lib/ipc/index.ts` (contracts/ipc.md)
- [x] T056 [P] [US4] Implement the Canvas blit surface (`putImageData` only — no drawing logic) in `apps/desktop/src/lib/canvas/DevicePreview.tsx` (constraint 2, FR-018)
- [x] T057 [US4] Implement Device Studio controls (select all six states, timeline seek, play/pause, frame-step) with a Zustand ephemeral store in `apps/desktop/src/features/device-studio/{Studio.tsx,store.ts,controls.tsx}` (FR-022, FR-023, FR-024, FR-025, FR-026)
- [x] T058 [US4] Implement the dev-only `mirror_state` command (sendable states only) + Studio mirror control in `apps/desktop/src-tauri/src/ipc/commands.rs` and `apps/desktop/src/features/device-studio/Controls.tsx` (FR-027)
- [x] T059 [P] [US4] Add keyboard operability, visible focus states, and accessible labels/roles to all Studio controls in `apps/desktop/src/features/device-studio/controls.tsx` (a11y requirements)
- [x] T060 [P] [US4] Externalize UI strings to an i18n-ready catalog in `apps/desktop/src/lib/i18n/strings.ts` (a11y/localization requirement)
- [x] T061 [P] [US4] Frontend tests: control behavior, deterministic 33.333 ms step, axe a11y, and canvas-blits-verbatim in `apps/desktop/src/features/device-studio/__tests__/{controls.test.tsx,store.test.ts}` + `apps/desktop/src/lib/canvas/__tests__/blit.test.ts` (SC-011, FR-018) — path corrected 2026-08-04: there is no `studio.test.tsx`; all four requirements are covered — control behaviour and axe in `controls.test.tsx`, the one-frame step in `store.test.ts` (`step advances one frame and pauses playback`), verbatim blitting in `blit.test.ts`

**Checkpoint**: Device Studio renders and controls all six states on the host with zero hardware; the
canonical renderer is validated through the UI.

---

## Phase 7: Firmware Host-Testable Core — US2 (Priority: P2)

**Goal**: Firmware logic — lifecycle-state ownership, protocol dispatcher, desired-state application,
heartbeat, offline transition, renderer integration through hardware-neutral adapters — all testable on
the host.
**Independent test**: from `firmware/esp32-c3/`, `cargo test --features host-sim --target <host>` passes
(host-target override needed — `.cargo/config.toml` forces the RISC-V build target); stitched tiles equal the golden full-frame per state.

- [x] T062 [US2] Define hardware-neutral adapter traits (`Transport`, `DisplaySink`, `Clock`) in `firmware/esp32-c3/src/ports.rs` (enables host testing; constraint 4)
- [x] T063 [US2] Implement the device lifecycle FSM (`booting`→`offline`/driven; applies `SetState`; emits `StateReport`) in `firmware/esp32-c3/src/state.rs` (FR-014; data-model §1)
- [x] T064 [US2] Implement the protocol dispatcher (handshake responder, seq/CRC validation, capability advertise) over `Transport` in `firmware/esp32-c3/src/proto.rs` (contracts/protocol.md §4; FR-002)
- [x] T065 [US2] Implement heartbeat `Pong` + safe `Health` reporting in `firmware/esp32-c3/src/health.rs` (protocol §7)
- [x] T066 [US2] Implement renderer integration: change-driven tile render → `DisplaySink` via `kivori-renderer`/`kivori-framebuffer` in `firmware/esp32-c3/src/render.rs` (FR-013)
- [x] T067 [P] [US2] Implement host-sim adapters (in-memory byte-pipe `Transport`, capture `DisplaySink`, virtual `Clock`) behind the `host-sim` feature in `firmware/esp32-c3/src/sim/mod.rs` (FR-035)
- [x] T068 [P] [US2] Host-sim tests: handshake, desired-state application, offline transition, heartbeat, dispatcher malformed-safety in `firmware/esp32-c3/tests/host_sim.rs` (FR-035, SC-008)
- [x] T069 [P] [US2] Host-sim test: stitched tiles equal the golden full-frame per state in `firmware/esp32-c3/tests/render_parity.rs` (SC-005)

**Checkpoint**: All device logic is proven on the host against the shared golden frames — no hardware
required.

---

## Phase 8: ESP32 Hardware Adapter — US2 (Priority: P2)

**Goal**: Real esp-hal board setup, USB Serial/JTAG transport, SPI display adapter, tile transmission,
and device diagnostics — with hardware-dependent code isolated from host CI (compile-only).
**Independent test**: `just fw-build` compiles for the riscv target; `just fw-flash` boots the device.
Real on-device behavior is verified in Phase 15.

- [x] T070 [US2] Implement `bsp` (clocks, SPI pins, USB Serial/JTAG bring-up on esp-hal 1.0) in `firmware/esp32-c3/src/bsp.rs` (research R-1/R-2) — **DONE.** `bsp::clock()` (system-timer `EspClock`), `bsp::serial()` (USB Serial/JTAG), and `bsp::display_bus()` (SPI2 mode 0 at `DISPLAY_SPI_HZ`, `ExclusiveDevice` owning CS) on esp-hal 1.1.1, with `BringUpError` distinguishing an unconfigurable frequency from an undriveable CS. **Every board fact is a parameter, never a default**: the caller passes the concrete SCK/MOSI/CS pin handles, the panel model, and the `PanelGeometry` offset. **Residual physical fact (not a code gap):** the real GPIO assignment and SPI ceiling are unmeasured, so no physical profile exists — `profile.rs` holds only the simulation-only Wokwi profile (validation-checklist items 23-25, T119)
- [x] T071 [US2] Implement the USB Serial/JTAG `Transport` adapter (64-byte FIFO aware, bounded non-blocking writes) in `firmware/esp32-c3/src/transport.rs` (research R-1; R2) — **DONE, after an audit found the previous version did NOT satisfy the literal wording**: it called `write` + `flush_tx`, which *block* until the host drains the FIFO, and then returned `buf.len()` claiming every byte was accepted. Now genuinely FIFO-aware and bounded: the peripheral is `split()` so receive uses `drain_rx_fifo` (one bulk read, empty means empty), and transmit offers at most `FIFO_BYTES = 64` per call via `write_byte_nb`, stops on the hardware's `WouldBlock`, returns the count actually accepted, and flushes with `flush_tx_nb` — so a host that stops reading can never stall the render loop. `TxBuffered` (queue = `MAX_WIRE * 2`) sits between the dispatcher and the hardware so a full FIFO cannot truncate a frame mid-flight, and the run loop drains it every tick. Integrated into the production runtime (T074) and covered by the 10 host tests in `tests/production_runtime.rs`
- [x] T072 [US2] Implement the SPI `DisplaySink` adapter via `mipidsi` (windowed `set_pixels`, panel offsets) in `firmware/esp32-c3/src/display.rs` (research R-3) — **DONE.** `MipidsiSink` implements the `DisplaySink` port over `mipidsi` 0.10 (pinned per R-3's version caveat): each changed tile becomes ONE windowed `Display::set_pixels(sx, sy, ex, ey, …)` write, and `PanelGeometry` carries the visible size plus the controller-frame offset applied through `Builder::display_offset` — with **no `Default`**, so an unmeasured panel offset cannot silently become `(0, 0)`. The adapter is generic over `mipidsi`'s `Model`, so the panel controller stays a `DeviceProfile` parameter exactly as R-3 requires and **no controller is hard-coded** (the physical one is still unconfirmed). It depends only on `embedded-hal` (`SpiDevice`/`OutputPin`/`DelayNs`) and not on `esp-hal`, so it compiles for host and `riscv32imc` alike and board bring-up stays outside it. `set_pixels` performs no bounds checking and wraps on a wrong pixel count, so the adapter rejects empty/out-of-range rects (`TileOutOfBounds`) and `pixels.len() != w*h` (`PixelCountMismatch`) BEFORE touching the bus; every SPI/DC failure propagates as `DisplayError::Interface` and is never swallowed. Proven by 9 host tests (`firmware/esp32-c3/tests/display_spi.rs`) driving the real `mipidsi` driver against a recording `SpiDevice`: reset asserted low then released before the first command; a tile blit is exactly CASET/RASET/RAMWR with inclusive coordinates; panel offset shifts the address window; RGB565 goes out high-byte-first; wrong pixel count and out-of-bounds rects clock nothing; a dead bus fails init; a full frame is 6 tiles x 9,600 px = 115,200 pixel bytes with Y offsets 0/40/80/120/160/200; an identical frame touches the bus not at all and a changed state retransmits whole tiles. Kept out of the dev-only surface: `check-release-surface.sh` now asserts `MipidsiSink`/`PanelGeometry` ARE present in the production firmware library (positive control)
- [x] T073 [US2] Implement the monotonic `Clock` (hardware timer → `elapsed_ms`) in `firmware/esp32-c3/src/clock.rs` — **DONE.** `EspClock` implements the `Clock` port over `esp_hal::time::Instant::now().duration_since_epoch()`, converted to canonical integer milliseconds and saturating rather than wrapping, because wrapping time would break the monotonicity the deterministic renderer depends on (ADR-0003). Brought up by `bsp::clock()` and owned by the production runtime. Execution-verified in Wokwi: `scenarios/boot.yaml` asserts `clock-monotonic` and `clock-advances`, which passed in the authenticated seven-scenario run. The earlier `[ ]` was a stale annotation — T125 had already recorded the clock as satisfied
- [ ] T074 [US2] Wire the main run loop / embassy tasks binding hardware adapters to the host-tested core in `firmware/esp32-c3/src/main.rs` (constraint 4) — **PARTIAL. Corrected from `[x]` on 2026-08-04 by the evidence audit: the previous checkbox over-claimed.** Traced from source: `runtime::run` has exactly ONE call site, `wokwi_runtime::run_mode` (`main.rs:40`), gated behind the `wokwi-runtime` feature. The plain-`embedded` (physical) branch of `main.rs` brings up `bsp::clock()` and `bsp::serial()`, prints one boot line, and then idles in a `Delay::delay_millis(1000)` loop — it never calls `runtime::run`, never constructs a `DisplaySink`, and never processes protocol. **DONE**: the loop itself (`runtime.rs` — lifecycle, decode, sequence policy, dispatch, change-driven render, safe diagnostics, health cadence, malformed recovery, never-returning) plus its integration for the simulation profile, proven by 11 host tests (`tests/production_runtime.rs`) and the executed `production-runtime` Wokwi scenario. **EXACT MISSING CRITERION**: the physical binary cannot construct the third port. `runtime::run` requires a `DisplaySink`; building one requires a `mipidsi` model (the panel controller), the SCK/MOSI/CS/DC/RST pin handles, and a `PanelGeometry` offset — none of which is measured (validation-checklist items 23-25). A null or stand-in sink is deliberately NOT provided: it would compile, run, render nothing, and read as a working loop. Unblocked by adding one physical profile to `firmware/esp32-c3/src/profile.rs` and calling `runtime::run` from the `embedded` branch; no other code change is required
- [x] T075 [P] [US2] Implement compact device diagnostics over the `Diagnostic` message (safe categories only) in `firmware/esp32-c3/src/health.rs` (FR-031, FR-032) — **DONE.** `DeviceDiagnostic` is the safe-category allowlist **made a type** — the device mirror of the desktop's `SafeDiagnostic` — with variants `FrameRejected(RejectReason)`, `SequenceGap`, `DisplayFault`, `LinkLost`, each mapping to one fixed `(ErrorCategory, code)` pair via `build_diagnostic`. No variant can carry payload bytes, a raw identity, a path, or free-form text, so an unsafe diagnostic is unrepresentable rather than merely discouraged (ADR-0005 §2). `RejectReason::of` reduces a `ProtoError` to Framing/Checksum/Version/Payload and discards everything else about it. The dispatcher records at most one pending diagnostic (a burst of garbage yields one report, not a flood) and the runtime transmits it as `Message::Diagnostic`. Host-tested: a malformed frame produces a `Diagnostic` whose category is one of the safe set and whose code is non-zero, and `Bye` produces an `Io` diagnostic
- [x] T076 [P] [US2] Configure `.github/workflows/firmware.yml` to build the hardware adapter compile-only and run **no** hardware test in host CI (isolation requirement) — **DONE.** The `no-std-isolation` job now builds the hardware adapters compile-only — `cargo build --release --features embedded` and `--features embedded,debug-payloads` — and lints every shipping/gated configuration (`--lib`, `embedded`, `wokwi-runtime`, `wokwi-spi`). No hardware test runs, and none can: the job has no board and no simulator. The stale `TODO(Phase 8 / T070+)` is gone. Device checks stay manual (Phase 15); the host-sim job continues to validate the hardware-neutral core

---

## Phase 9: Desktop Connection Manager — US1 (P1) + US3 (P3)

**Goal**: Candidate-port enumeration, Kivori handshake verification, connection state machine, heartbeat,
read/write-failure detection, incompatible-device handling, bounded reconnect backoff, and
within-process desired-state restoration.
**Independent test**: `cargo test -p kivori-desktop` drives injected FSM events → correct transitions,
backoff, and desired-state resync; discovery filters by VID/PID; identity is authoritative only via the
handshake.

- [x] T077 [US1] Implement the transport abstraction behind a `SerialLink` trait with a production blocking-`serialport` adapter driven on a dedicated OS thread (research R-5 fallback path; async `tokio-serial` not used) in `apps/desktop/src-tauri/src/device/{transport.rs,serial.rs}` — real-device behaviour is manual (no hardware in CI)
- [x] T078 [US1] Implement candidate-port enumeration + VID/PID pre-filter (`0x303A:0x1001`, configurable allowlist, no fixed COM port; Windows + USB serial only) in `apps/desktop/src-tauri/src/device/{discovery.rs,serial.rs}` (FR-001, FR-036)
- [x] T079 [US1] Implement handshake verification (`Hello`→`HelloAck`, nonce, identity via handshake only) in `apps/desktop/src-tauri/src/device/connection.rs` (FR-002, FR-004)
- [x] T080 [US1] Implement the connection state machine (`connecting`/`connected`/`incompatible`/`disconnected`/`error`) in `apps/desktop/src-tauri/src/device/fsm.rs` (FR-005; plan state machine)
- [x] T081 [US1] Implement incompatible-device handling (major mismatch → `Incompatible`, `Bye`, stop retry, reason surfaced) in `apps/desktop/src-tauri/src/device/fsm.rs` (FR-003, SC-003)
- [x] T082 [US1] Implement heartbeat (`Ping`/`Pong`, N-miss timeout) + read/write I/O-failure detection (pump error → `ManagerEvent::IoError`) in `apps/desktop/src-tauri/src/device/heartbeat.rs` + the run loop (FR-007; protocol §7)
- [x] T083 [US3] Implement bounded reconnect backoff (250 ms→5 s, jitter, reset on success) in `apps/desktop/src-tauri/src/device/reconnect.rs` (FR-008, FR-010)
- [x] T084 [US3] Implement within-process desired-state restoration on reconnect (re-send `DesiredState`) in `apps/desktop/src-tauri/src/device/reconnect.rs` (FR-009; US3)
- [x] T085 [US1] Implement orchestrator `DesiredState` ownership (default `idle`, cold-start reset, no persistence) in `apps/desktop/src-tauri/src/orchestrator/mod.rs` (data-model §4; clarified)
- [x] T086 [P] [US1] FSM tests: injected events (port up, handshake ok/incompatible/timeout, I/O error, heartbeat timeout, port removed) → transitions in `apps/desktop/src-tauri/tests/fsm.rs` (FR-005, FR-007)
- [x] T087 [P] [US3] Reconnect tests: backoff schedule, desired-state resync, rapid unplug/replug stability in `apps/desktop/src-tauri/tests/reconnect.rs` (FR-008, FR-009, FR-010; US3)

**Checkpoint (MVP for US1)**: With host-sim firmware, the desktop discovers, verifies, and reports
`connected`; incompatible devices are surfaced; disconnects trigger bounded reconnect.

---

## Phase 10: Desktop Lifecycle & Background Operation (cross-cutting) — FR-030 / SC-007

**Goal**: Closing the settings window hides it and keeps the native core (connection manager, heartbeat,
reconnect, synchronization) running; a distinct Quit terminates; reopening activates the single instance
and shows the window; the device task is not owned by the React window lifecycle and survives webview
loss.
**Independent test**: `cargo test -p kivori-desktop window_lifecycle` validates the hide-vs-quit /
show-on-reactivate policy and device-task survival without a real OS window.

- [x] T088 Implement the pure, host-testable window lifecycle policy (decide hide-vs-quit, show-on-reactivate, single-instance activation) in `apps/desktop/src-tauri/src/window_lifecycle.rs` (FR-030)
- [x] T089 Wire close-request→hide, show-window, explicit-quit, and single-instance activation, and spawn the connection-manager/device task on the core runtime independent of any window, in `apps/desktop/src-tauri/src/{lib.rs,main.rs,runtime/}` (FR-030, SC-007)
- [x] T090 Configure the Tauri application lifecycle (window close → hide via the close handler, single-instance plugin, app stays alive while hidden) in `apps/desktop/src-tauri/src/lib.rs` + `tauri.conf.json` (FR-030)
- [x] T091 [P] Implement tray Open/Quit actions in `apps/desktop/src-tauri/src/runtime/lifecycle.rs` (FR-030)
- [x] T092 [P] Host-side tests for lifecycle decisions (hide vs quit, reactivate → show) and native device task survival across a simulated webview loss, independent of a real OS window, in `apps/desktop/src-tauri/tests/window_lifecycle.rs` (FR-030, SC-007)
- [ ] T093 [MANUAL] Validate that hiding/closing the window leaves the device heartbeat + synchronization active, and reopening restores the UI; record the result in `docs/validation-checklist.md` (FR-030, SC-007)

**Checkpoint**: The connection survives window hide/close; only explicit Quit ends the process.

---

## Phase 11: End-to-End Desktop/Device Slice — US1 + US2 + US3

**Goal**: Selecting a state updates Device Studio and transmits the same semantic state to firmware;
reconnect resynchronizes the current desired state; cold desktop launch starts from `idle`; `booting`
and `offline` remain device-originated.
**Independent test**: With host-sim firmware, set a state → correct `SetState` on the wire + Device
Studio preview matches; disconnect/reconnect → resync; restart → `idle`.

- [x] T094 [US2] Wire `set_desired_state`/`mirror_state` → orchestrator → `SetState` transmission (commands → device thread → `Session`) in `apps/desktop/src-tauri/src/ipc/commands.rs`, `src/runtime/device_task.rs`, and `src/orchestrator/mod.rs` (FR-012; ipc §1)
- [x] T095 [US1] Implement connection-status events (`connection://status`) + `ConnectionStatusDto` (three axes) in `apps/desktop/src-tauri/src/ipc/events.rs` and `apps/desktop/src-tauri/src/ipc/dto.rs` (ipc §2/§4; FR-006)
- [x] T096 [P] [US1] Implement the connection-status UI (six UI states incl. reconnecting, device version, desired-vs-reported, a11y live region) in `apps/desktop/src/features/connection/ConnectionStatus.tsx` (FR-005, FR-006)
- [x] T097 [US2] Integration test (host-sim device): set state → correct `SetState` frame + reported convergence, via the cross-workspace harness bridging the desktop `Session` to the real firmware `Dispatcher` over an in-memory pipe, in `tests/e2e-host-sim/tests/e2e.rs` (US2; SC-004 logic)
- [x] T098 [US3] Integration test (host-sim): disconnect→reconnect resyncs current desired state; cold-start starts `idle` in `apps/desktop/src-tauri/tests/session.rs` (US3; FR-009; clarified)
- [x] T099 [US2] Integration test: `booting`/`offline` are never transmitted by the desktop (type-level: only `SendableState` is transmittable via `SetState`/`Session`; enforced by construction) (FR-014, FR-015)

**Checkpoint**: Full US1+US2+US3 loop verified host-only via the firmware simulation path.

---

## Phase 12: Diagnostics & Security Controls (cross-cutting)

**Goal**: Structured redacted logs; release-build prohibition on raw payload logging; least-privilege
Tauri commands; no arbitrary serial/filesystem/credential/shell exposure; automated policy checks.
**Independent test**: redaction test passes; a release build omits `debug-payloads` and dev-only
commands; the Tauri capability allowlist is audited.

- [x] T100 Write ADR-0005 (sensitive-data logging policy: safe allowlist; `debug-payloads` dev-only) in `docs/adr/0005-logging-policy.md`
- [x] T101 Implement `tracing` setup + redaction layer (safe-diagnostics allowlist; hash device identity at the boundary) in `apps/desktop/src-tauri/src/lib.rs` (subscriber) and `src/diagnostics/{mod.rs,redact.rs}`; the ring is fed from the live connection lifecycle by the device thread (FR-031, FR-032)
- [x] T102 Gate raw payload logging behind the `debug-payloads` dev feature, compiled out of release, in `apps/desktop/src-tauri/src/diagnostics/mod.rs` and `firmware/esp32-c3/src/transport.rs` (FR-032; ADR-0005) — **DONE, both halves.** Desktop: the raw path sits behind the `debug-payloads` feature and `check-release-surface.sh` proves `debug_payload_hex` is absent from a `--no-default-features` binary, with a positive control showing the dev build does contain the dev commands. Firmware: the new `debug-payloads` feature gates `debug_payloads::dump`, the only code in the firmware that can turn wire bytes into text; `transport.rs` calls it from `read`/`write` under `#[cfg(feature = "debug-payloads")]`, so the guard is structural rather than a runtime `if`. Both directions are proven by the guard's new step [7/7]: the `KIVORI-DEBUG-PAYLOAD` literal is **absent** from the production `embedded` library and **present** once the feature is on. Audit note: the grep target is the string literal, not the module path — rustc records cfg-disabled module names in rlib metadata, so `debug_payloads`/`spi_probe` appear even in production builds and prove nothing
- [x] T103 [P] Configure least-privilege Tauri capabilities (no fs/shell/http; only the app's own commands) in `apps/desktop/src-tauri/capabilities/default.json` (`core:default` only) (Principle VIII; ipc §5)
- [x] T104 [P] Automated policy test: emitted logs + DTOs carry no sensitive fields (`apps/desktop/src-tauri/tests/redaction.rs`), and `scripts/check-release-surface.sh` proves the compiled production artifacts contain no dev-only commands, no raw-payload path, and no firmware host-sim adapters — with positive controls so the guard cannot pass vacuously; wired into `.github/workflows/host.yml` (SC-010; FR-028, FR-032)
- [x] T105 [P] Implement `get_diagnostics` + a safe-only diagnostics view: the device thread records a redacted `SafeDiagnostic` on every lifecycle transition (connect/incompatible/disconnect/recoverable error/reconnect) into the bounded ring and emits `diagnostics://event`; the view renders empty/populated/bounded states from the DTO only, with a11y + redaction tests (FR-031)

---

## Phase 13: Offline Runtime Boundary (cross-cutting) — FR-029 / SC-006

**Goal**: All core behavior works with no internet; no external service/CDN/telemetry/cloud dependency;
frontend assets bundled locally; no startup network wait.
**Independent test**: dependency + asset checks pass; the offline host smoke test boots the core +
host-sim device with no external services.

- [x] T106 Document the offline runtime boundary (what must work offline; what is prohibited) in `docs/offline-boundary.md` (FR-029, SC-006)
- [x] T107 [P] Add the direct-dependency offline check `scripts/check-offline-deps.sh` (via `cargo metadata`, direct deps only) asserting no first-party crate in this feature directly depends on a network-client crate (e.g., `reqwest`/`hyper`-client/`ureq`); wire into `.github/workflows/host.yml` (FR-029)
- [x] T108 [P] Add the frontend-asset offline check `scripts/check-frontend-offline.mjs` asserting no remote URLs / CDN references in `apps/desktop/{index.html,src/**}` and that fonts/icons/scripts/styles are bundled locally; wire into `.github/workflows/host.yml` (FR-029, SC-006)
- [x] T109 Add an offline host smoke test that constructs the native core (managed state + diagnostics), renders every scene from the bundled blob, and drives handshake → desired-state update → reconnect/resync over an in-memory host-sim link with no external services, asserting no startup network wait and local-only frontend assets, in `apps/desktop/src-tauri/tests/offline_smoke.rs` (FR-029, SC-006)

---

## Phase 14: CI & Documented Validation (cross-cutting / polish)

**Goal**: Full CI gates plus documented manual validation.
**Independent test**: CI is green on host + firmware + determinism jobs; validation checklist present.

- [x] T110 Finalize `.github/workflows/host.yml`: `fmt --check`, `clippy -D warnings`, all host tests, boundary + offline dep checks (T011/T107/T108); frontend strict `tsc`, ESLint, Prettier check, Vitest + axe, `vite build` (constitution quality gates) — all gates present + green locally; the offline-smoke step is added when T109 lands with the run loop
- [x] T111 [P] Finalize `.github/workflows/determinism.yml`: golden frames byte-equal on Windows **and** Linux + asset-hash check (SC-009)
- [x] T112 [P] Confirm `.github/workflows/firmware.yml` compile-only gate proving `no_std` shared crates on the riscv target (Principle IV)
- [x] T113 [P] Author the manual Windows + ESP32-C3 validation checklist in `docs/validation-checklist.md` (mirrors [quickstart.md](./quickstart.md); links Phases 10 & 15)
- [x] T114 [P] Author `docs/architecture.md` (built-system overview) and `docs/diagnostics-and-logging.md` (policy) (constitution runtime guidance)

---

## Phase 15: Hardware Validation (MANUAL — non-blocking for host CI)

**Goal**: On-device verification of everything that cannot be trusted without a physical ESP32-C3
([research hardware-validation list](./research.md#hardware-validation-required-do-not-trust-without-a-physical-esp32-c3)).
**Rule**: every task is `[MANUAL]`, excluded from ordinary host CI, and never gates a merge.
**Independent test**: each result recorded in `docs/validation-checklist.md`.

- [ ] T115 [MANUAL] USB enumeration & identity: device enumerates as `0x303A:0x1001`, is discovered without port selection, and identity is confirmed via handshake; record in `docs/validation-checklist.md` (US1; SC-001; research R-1)
- [ ] T116 [MANUAL] USB throughput & transmit-stall behavior under sustained traffic; recovery when the host is not draining; record in `docs/validation-checklist.md` (research R-1/R2)
- [ ] T117 [MANUAL] Windows reconnect reliability: unplug/replug + rapid cycling; compare `tokio-serial` vs `spawn_blocking` fallback; record in `docs/validation-checklist.md` (US3; SC-002; research R-5)
- [ ] T118 [MANUAL] Display initialization & offsets correct for the chosen panel (GC9A01/ST7789); record in `docs/validation-checklist.md` (research R-3)
- [ ] T119 [MANUAL] Sustainable display frame rate over SPI (full-frame vs tile updates); record in `docs/validation-checklist.md` (SC-004; research R-4)
- [ ] T120 [MANUAL] Tile-transfer correctness: on-device image matches the golden/preview for each state; record in `docs/validation-checklist.md` (SC-005)
- [ ] T121 [MANUAL] End-to-end latencies: state change < 1 s (SC-004), connect < 5 s (SC-001), reconnect+restore < 10 s (SC-002); record in `docs/validation-checklist.md`
- [ ] T122 [MANUAL] esp-hal `unstable` API verification: `usb_serial_jtag` behaves as documented on the pinned esp-hal version; record in `docs/validation-checklist.md` (research R-2/R11)

---

## Dependencies & execution order

### Phase dependencies

- **Phase 1 (Setup)**: no dependencies — start immediately.
- **Phase 2 (Domain)**: after Phase 1. Blocks 3, 4, 5, 6, 7, 9.
- **Phase 3 (Protocol)**: after Phase 2. Blocks 7, 9, 11.
- **Phase 4 (Rendering)**: after Phase 2. Blocks 6, 7. (Parallel with Phase 3.)
- **Phase 5 (Assets)**: after Phase 4 (golden integration); SVG authoring (T044) may start after Phase 1.
  Blocks the asset-backed parts of 6 and 7.
- **Phase 6 (Device Studio, US4)**: after 2, 4, 5 (+ IPC scaffolding). The mirror action (T058) is
  exercised end-to-end in Phase 11 once a connection exists. Independent of firmware.
- **Phase 7 (Firmware core, US2)**: after 2, 3, 4, 5.
- **Phase 8 (Hardware adapter, US2)**: after 7.
- **Phase 9 (Connection manager, US1/US3)**: after 2, 3. Parallel with 4/5/6/7.
- **Phase 10 (Lifecycle/background)**: after 9 (device task exists to keep alive). Precedes 11.
- **Phase 11 (E2E)**: after 7 (host-sim), 9, 10, and 6.
- **Phase 12 (Diagnostics/Security)**: ADR-0005 anytime; implementation after 9/11 (something to log).
- **Phase 13 (Offline boundary)**: dep/asset checks after crates + frontend exist (2/6); smoke test after
  7 + 9.
- **Phase 14 (CI/Validation)**: after all host phases (2–13).
- **Phase 15 (Hardware, MANUAL)**: after 8 (flashable firmware) + 9/10 (desktop). Never blocks CI.

### ADR ordering

ADR-0001 (T001) → Phase 1 scaffolding. ADR-0003 (T012) → T016 timeline. ADR-0002 (T021) → protocol impl
(T022+). ADR-0004 (T043) → asset compiler (T047). ADR-0005 (T100) → diagnostics impl (T101+).

## Parallel opportunities

- **Within Phase 2**: T013–T017 (distinct model files) run in parallel; then T018–T020 tests in parallel.
- **Within Phase 3**: T022, T025, T026, T027 in parallel; validation/negotiation/malformed tests
  T028–T034 all in parallel after the codec (T023/T024) lands.
- **Across phases after Phase 2**: Phase 3 (protocol) and Phase 4 (rendering) proceed in parallel; the
  connection manager (Phase 9) can begin as soon as Phase 3 lands, in parallel with Phases 4–7.
- **Team split**: Dev A → Phases 3+9+10 (protocol + connection + lifecycle, US1/US3); Dev B →
  Phases 4+5+6 (rendering + assets + Device Studio, US4); Dev C → Phases 7+8 (firmware, US2). They
  converge at Phase 11.

```bash
# Example: Phase 3 protocol validation tests in parallel (after the codec lands)
Task: "T029 header validation in crates/kivori-protocol/tests/header_validation.rs"
Task: "T030 payload validation in crates/kivori-protocol/tests/payload_validation.rs"
Task: "T031 sequence handling in crates/kivori-protocol/tests/sequence_handling.rs"
Task: "T032 version negotiation in crates/kivori-protocol/tests/version_negotiation.rs"
Task: "T033 capability negotiation in crates/kivori-protocol/tests/capability_negotiation.rs"
Task: "T034 malformed frames in crates/kivori-protocol/tests/malformed_frames.rs"
```

## Implementation strategy

### MVP first (US1 — device connects)

1. Phase 1 (Setup) → Phase 2 (Domain) → Phase 3 (Protocol).
2. Minimal Phase 7 (handshake responder + dispatcher via host-sim) and, for real hardware, the transport
   parts of Phase 8.
3. Phase 9 (Connection manager) + connection-status UI (T095/T096).
4. **STOP & VALIDATE**: device is discovered, handshakes, and reports `connected`/`incompatible`
   (US1, SC-001/003). The device may show a placeholder scene until US2 rendering lands.

### Incremental delivery

- **US1** (above) → connection foundation.
- **+ US4** (Phases 4, 5, 6): Device Studio + canonical rendering, fully host-testable.
- **+ US2** (Phases 7, 8, 11): companion states on the physical device.
- **+ US3** (Phase 9 reconnect + Phase 11 resync): automatic recovery & restoration.
- **+ background continuity** (Phase 10), **hardening** (Phases 12, 13), CI (14), and **hardware
  sign-off** (Phase 15).

## Notes

- `[P]` = different files, no dependency on an incomplete task. Never mark two tasks touching the same
  file `[P]`.
- Contract-defining tests (protocol vectors, golden frames, FSM, host-sim, lifecycle policy) are
  committed with or before the code they pin (constitution III & X).
- No task introduces out-of-scope surfaces (Wi-Fi, Bluetooth, OTA, production artwork, AI/media, cloud,
  plugins, marketplace) — Principle XII / spec Non-Goals.
- Firmware hardware behavior and the window-hidden OS behavior are validated manually (Phases 15 & T093);
  host CI stays hardware-free and window-automation-free.
- Cargo `-p` targets use the locked package names from plan.md → Cargo package names (locked):
  `kivori-model`, `kivori-protocol`, `kivori-framebuffer`, `kivori-renderer`, `kivori-assets`,
  `kivori-desktop`, `kivori-firmware`, `kivori-asset-compiler`, `kivori-golden-frames`.

---

## Phase 16: Convergence

Appended by `/speckit-converge` (2026-07-24). Assessment verified actual code + tests: all `[x]` tasks
are genuinely complete and all `PARTIAL`/`DEFERRED` annotations are accurate. Shared renderer + compiled
bundle contracts (Const. II/XI), device-originated `booting`/`offline` (FR-014/015), and deterministic
rendering (Const. III) are intact; no contract drift and no constitution MUST violations were found. The
remaining Tauri-command, event, connection-UI, production-serial, esp-hal, tracing, capabilities,
diagnostics-view, offline-smoke, and manual-validation work is **already tracked** as unchecked tasks
(T052/T053/T054/T058, T077/T078/T082, T089-T091/T093, T094/T095/T096, T101/T103/T105, T109, Phase 8, Phase
15) and is deliberately NOT duplicated here. Only genuinely-missing, untracked work is appended:

- [x] T123 Add the Tauri v2 runtime foundation: `tauri` + `tauri-build` deps, `build.rs` (bundles the compiled asset blob), a library `run()` + thin `main.rs`, the `tauri::Builder` with managed `AppState`, background device thread, single-instance plugin, tray, window-event wiring, and `invoke_handler` registering the full ipc.md §1 surface (incl. `get_app_info`/`list_states`; `render_preview_frame` → `tauri::ipc::Response`) per plan (Tauri v2) + ipc.md §1
- [x] T124 Build-gate the browser IPC mock: the dev mock lives in `apps/desktop/src/lib/ipc/mock.ts`, loaded only behind a static `import.meta.env.DEV` guard (`index.ts`) so Vite drops it from production; production without Tauri throws instead of falling back. Proven by `scripts/check-mock-excluded.mjs` (in host CI) per FR-028 / Constitution VIII

---

## Phase 17: Wokwi Pre-Hardware Simulation Gate

**Goal**: Run the REAL firmware in the Wokwi ESP32-C3 simulator to exercise the embedded runtime,
protocol loop, lifecycle FSM, and renderer integration before anyone flashes a board.
**Rule**: an ADDITIONAL gate only. Host tests stay the primary CI gate, and a green simulation is never
evidence of physical correctness — every Phase 15 hardware task remains required and unchecked.
**Independent test**: `bash scripts/build-wokwi-firmware.sh` produces three RISC-V ELFs;
`bash scripts/test-wokwi.sh` passes all eight scenarios — three internal self-test, three external
serial-injection, one generic SPI/RGB565 display probe, and one PRODUCTION-RUNTIME run, whose VCD captures are validated by
`tools/wokwi-vcd` (needs `wokwi-cli` + a CI `WOKWI_CLI_TOKEN`), and writes
`target/wokwi-logs/summary.json` with each scenario's exit code, duration, artifact identity, and VCD
result.

- [x] T125 Add the esp-hal runtime entry point needed for a loadable artifact (boot via `#[esp_hal::main]`, `esp_hal::init`, monotonic `EspClock` implementing the `Clock` port) behind the `embedded` feature in `firmware/esp32-c3/src/{main.rs,clock.rs}` — board peripherals (USB Serial/JTAG transport, SPI panel) remain Phase 8 (T073 clock satisfied)
- [x] T126 Add the in-firmware self-test harness + simulation probes behind the `wokwi` feature (`LoopbackTransport` for `Transport`, `TileProbe` for `DisplaySink`; both behind the existing ports, no separate renderer) in `firmware/esp32-c3/src/{selftest.rs,sim_probe.rs}` + `build.rs` bundling the canonical asset blob
- [x] T127 [P] Add the Wokwi project + scenarios (`sim/wokwi/{wokwi.toml,diagram.json,README.md,scenarios/{boot,protocol,state-cycle}.yaml}`) asserting boot, clock, lifecycle, handshake, version rejection, capability advertisement, Ping/Pong, SetState/StateReport, malformed recovery, sendable boundary, RGB565 tile geometry, change-driven refresh, and safe diagnostics
- [x] T128 [P] Add `scripts/build-wokwi-firmware.sh` (builds BOTH mode artifacts, verifies ELF magic + 32-bit RISC-V `e_machine`) and `scripts/test-wokwi.sh` (per-scenario ELF/diagram selection, loud preflight when `wokwi-cli`/`WOKWI_CLI_TOKEN` are absent, token read from the environment only; never a false pass; no credentials committed)
- [x] T129 [P] Add `.github/workflows/wokwi.yml` as a non-blocking additional gate that self-skips without the `WOKWI_CLI_TOKEN` secret, and extend `scripts/check-release-surface.sh` so the simulation harness/probes are proven absent from production firmware
- [x] T130 Execute all six Wokwi scenarios and record the serial transcripts — **DONE, execution-verified**. One complete `bash scripts/test-wokwi.sh` invocation (Wokwi CLI v0.26.1 `9d71b975b7eb`, Simulation API `1.0.0-20260731-g7e66f2be`, logs written 16:37:55–16:38:33) ran all six and each captured log in `target/wokwi-logs/` contains its required success marker with zero failure signatures and exactly one boot banner (no reset loop): `boot` → `KIVORI-SIM PASS lifecycle-booting-to-offline`; `protocol` → `KIVORI-SIM PASS malformed-recovery`; `state-cycle` → `KIVORI-SIM ALL PASS`; `serial-smoke` → `KIVORI-EXT TX kind=Pong`; `protocol-serial` → `KIVORI-EXT SEQ seq=0 class=ok`; `state-cycle-serial` → `KIVORI-EXT ALL PASS state-cycle`. Outstanding assurance (not a blocker): the 3× consecutive `state-cycle-serial` stability check has one verified run, not three. **Artifact note (2026-08-03):** the original 16:37-16:38 transcripts were destroyed mid-session when an offline control-flow harness (a stub `wokwi-cli`) wrote into the default `target/wokwi-logs/`; they were untracked/gitignored with no backup. `scripts/test-wokwi.sh` now honours `WOKWI_LOGS_DIR`/`WOKWI_VCD_DIR` so a harness can never overwrite the evidence directory again. **Re-executed and superseded**: a fresh authenticated invocation (CLI v0.26.1 `9d71b975b7eb`, Simulation API `1.0.0-20260731-g7e66f2be`) ran all SEVEN scenarios green in one pass, and `target/wokwi-logs/summary.json` records for each the real exit code, duration, sha256-pinned ELF identity, marker presence, empty failure signature, and exactly one boot banner. **The 3x stability assurance is now MET**: `state-cycle-serial` ran three consecutive times from the same ELF (`sha256:e5df6b9bc934a6e3`), passing each time in 7.0 s / 7.1 s / 7.3 s with one boot banner and the completion marker present.
- [x] T131 Stage 2 display probe — **DONE, EXECUTION-VERIFIED.** The generic SPI/RGB565/tile-transfer probe runs and passes in Wokwi. Firmware mode `wokwi-spi` (`firmware/esp32-c3/src/spi_probe.rs`) drives the real T072 `MipidsiSink` over the real esp-hal SPI2 peripheral; artifact `target/wokwi/kivori-spi.elf` (`sha256:6adbcd8d2ec9f906`); diagram `sim/wokwi/diagram-spi.json` (built-in `wokwi-ili9341` + a `wokwi-logic-analyzer` on D0-D4); scenario `sim/wokwi/scenarios/spi-display-probe.yaml`. **Serial evidence** (`target/wokwi-logs/spi-display-probe.log`, exit 0 in 18.5 s, one boot banner, no failure signature): all ten stages PASS in dependency order — `bus-init`, `reset-sequence`, `rgb-red`, `rgb-green`, `rgb-blue`, `checkerboard`, `tile-count-6`, `tile-bytes`, `unchanged-no-reflush`, `changed-reflush` — ending on `KIVORI-SPI ALL PASS`. On-target counters match the predicted geometry exactly: `frame data-bytes=115248 cmd-bytes=18 pixels=57600` (6 x 9,600 px, 2 bytes/px, plus 8 address-window argument bytes and 3 opcodes per tile), init `rst=2 dc=13 cs=28`, totals `transactions=1598 data-bytes=691491`. **VCD evidence** (`target/wokwi-vcd/spi-display-probe.vcd`, 6.7 MB, 5 signals, 1 ns timescale): `tools/wokwi-vcd` passes all eight digital properties — 235,170 SCK rising edges, 166 CS changes, 24 D/C changes, 29,455 MOSI changes, reset low at 164409500 ns released at 164424625 ns, volume above threshold, first command clock 169445125 ns before first data clock 169459250 ns, and monotonic stage ordering. Two real capture facts were discovered and encoded: `--vcd-file` produces nothing without a logic-analyzer part (`Error 6: No logic analyzer in diagram`), and every line reads `1` at time 0 because the pins float until the firmware configures them ~164 ms into boot — so all activity is measured after reset release (15 checker tests, including a negative control proving the window is what makes the difference). **Scope**: A (generic SPI transactions) and B (generic RGB565 tile stream) are claimed. **Scope C is NOT claimed** — no ST7789/GC9A01/ILI9341 init sequence, panel offset, orientation, colour order, or backlight behaviour is validated, no custom controller chip was created, and nothing analog is proven. Still unknown, still required before any controller-specific claim: the physical panel's controller (research.md leaves GC9A01 vs ST7789 open; `DeviceProfile::KIVORI_240` selects `Gc9a01`), the real SPI/CS/DC/reset pin map (`WokwiSpiPins` is simulation-only), and the measured panel offsets — validation-checklist items 23-25
- [x] T132 [P] Add the canonical wire-vector generator (`tools/wokwi-vectors`) producing Wokwi `write-serial` byte arrays from the REAL `kivori-protocol` codec — Hello, compatible-minor, incompatible-major, unsupported-capability, Ping, SetState idle/happy, sequence duplicate/gap/wrap, CRC-invalid, malformed COBS, truncated frame — with determinism + real-decoder round-trip tests (7 passing) and CI staleness detection
- [x] T133 Implement the EXTERNAL serial test mode (`wokwi-serial` feature): the real USB Serial/JTAG transport (`transport.rs`, also the T071 adapter core) plus `external.rs`, which takes Wokwi-injected bytes through the real decoder, sequence policy, and `Dispatcher` and emits redacted `KIVORI-EXT …` markers — **execution-proven**, no longer merely compile-verified. `state-cycle-serial.log` shows the full external path in order: `RX kind=Hello` → `TX kind=HelloAck` → `RX kind=SetState`/`TX kind=StateReport reported=idle`/`STATE current=idle` → same for `happy` → `DROP reason=decode` with `STATE current=happy` unchanged → `RX kind=Ping`/`TX kind=Pong` recovery → `ALL PASS state-cycle` last. `protocol-serial.log` additionally exercises the real sequence policy (`SEQ … class=first|ok|duplicate|gap`).
- [x] T134 [P] Generate the external-serial scenarios (`sim/wokwi/generated/{serial-smoke,protocol-serial,state-cycle-serial}.yaml`) from the vector tool and add `diagram-usb-serial.json` (`serialInterface: USB_SERIAL_JTAG`); both diagrams pass `wokwi-cli lint` (offline). **Correction (verified 2026-08-03, CLI v0.26.1):** scenario *schema* validation is NOT reachable without a credential — `wokwi-cli --scenario …` exits on `Missing WOKWI_CLI_TOKEN` before parsing the scenario, including for a scenario containing the unsupported `wait-pin`/`screenshot` steps. Scenario structure is therefore now gated on the host instead, by `tools/wokwi-vectors/tests/scenario_files.rs` (supported step kinds, required top-level keys, marker/firmware coupling), each check with a negative control
- [x] T135 Fix the `state-cycle-serial` 30-second timeout: the scenario awaited `KIVORI-EXT SENDABLE-GUARD ok` — a marker the firmware emits once at boot — *after* the injection steps, and `wait-serial` only scans forward, so it could never match again (confirmed from `target/wokwi-logs/state-cycle-serial.log`: the firmware's last line was `STATE current=happy` and it remained alive). Firmware now tracks the state-cycle post-conditions from observed events (`StateCycle` in `external.rs`: sendable guard, HelloAck TX, both StateReports matched against the device's real state, a decoder-rejected frame, state-intact-across-drop, and post-drop recovery) and emits `KIVORI-EXT ALL PASS state-cycle` only once all of them hold. The regenerated scenario asserts each post-condition, injects an invalid frame plus a following valid `Ping`, and ends on the completion marker. Guarded against regression by four host tests plus a non-vacuity control in `tools/wokwi-vectors/tests/vectors.rs` (12 passing), and `scripts/test-wokwi.sh` now verifies the captured log after every run (required marker present, no `panicked`/`KIVORI-SIM FAIL`/`KIVORI-EXT FAIL`, no second boot banner) so simulator exit alone cannot pass a scenario **Execution-verified**: the corrected scenario now passes in Wokwi — `target/wokwi-logs/state-cycle-serial.log` contains all nine required markers, ends on `KIVORI-EXT ALL PASS state-cycle` after the drop/recovery steps, and shows no panic, no reset loop, and no failure marker.
- [x] T136 [P] Persist machine-readable Wokwi run evidence: `scripts/lib/wokwi-evidence.sh` captures each scenario's real exit code, wall-clock duration, and the sha256-pinned identity of the ELF actually simulated, then derives marker/failure/boot-banner facts from the captured log and writes `target/wokwi-logs/summary.json` (written on failure too — fail-closed). The `passed` field is *derived* (exit 0 AND marker present AND no failure signature AND ≤1 boot banner), never asserted by the caller. Covered by `scripts/test-wokwi-evidence.sh` (24 assertions, offline, no token): pass case, missing marker, non-zero exit, panic, `KIVORI-EXT FAIL`, reset loop, binary-interleaved logs, artifact-identity stability/uniqueness/absence, JSON field contract, and a check that no token material can appear. The pre-existing `verify_log` gate is unchanged. Extended for the stage-2 probe: `KIVORI-SPI FAIL`/`KIVORI-SPI ALL FAIL` are failure signatures, and each row carries `vcd_path` + `vcd_result` (`pass`/`fail`/`none`) where a `fail` forces `passed: false` — so a broken logic-analyzer capture cannot hide behind a green serial log (32 assertions now)
- [x] T137 Add a PRODUCTION-RUNTIME Wokwi mode + scenario — **DONE, EXECUTION-VERIFIED.** `wokwi-runtime` (`firmware/esp32-c3/src/wokwi_runtime.rs`) calls **the same `runtime::run`** the physical firmware calls — no second runtime implementation — differing only in the injected ports and the simulation-only profile. Artifact `target/wokwi/kivori-runtime.elf` (`sha256:09629bc0cd66a17b`); diagram `sim/wokwi/diagram-runtime.json` (`serialInterface: USB_SERIAL_JTAG`, generic `wokwi-ili9341`, logic analyser on D0-D4); scenario `sim/wokwi/generated/production-runtime.yaml`, generated from the REAL codec. **Executed green** (CLI v0.26.1 `9d71b975b7eb`, Simulation API `1.0.0-20260803-gf69c6c93`, exit 0 in 29.1 s, one boot banner, no failure signature): all ten post-conditions in order — `lifecycle-booting-to-offline`, `first-frame-six-tiles`, `health-report`, `unchanged-frame-no-reflush`, `hello-ack`, `state-idle`, `state-happy`, `diagnostic label=frame-rejected-framing`, `pong`, `recovery-after-reject` — ending on `ALL PASS production-runtime`, which is emitted only once every one of them genuinely held. VCD capture passes all eight digital properties (210,368 SCK rising edges, 154 CS changes, 24 D/C changes, 79,073 MOSI changes, reset released at 164861375 ns, first command clock before first data clock, monotonic ordering). **Bug found and fixed during this run:** the first scenario draft awaited `health-report` *after* `unchanged-frame-no-reflush`, but the runtime emits health on tick 1 and the first quiet frame on tick 2 — and `wait-serial` scans forward only, so the run hung to timeout. Exactly the T135 defect class. The order now matches real behaviour and is pinned by `emission_order_is_stable` in `tests/production_runtime.rs`, so it cannot drift again. `scripts/test-wokwi.sh` also gained a per-scenario wall-clock budget (this scenario re-hashes six tiles every simulated 33 ms, which is slow under simulation); every marker/failure/VCD assertion is unchanged by it. **Stability**: three consecutive runs from the same ELF (`sha256:09629bc0cd66a17b`) passed in 27.4 s / 28.8 s / 27.1 s, each exit 0 with the completion marker, one boot banner, no failure signature, and a passing VCD check. **Not physical evidence**: nothing here validates the panel controller, its init sequence, offsets, orientation, colour order, or backlight

## Phase 18: Convergence

- [ ] T138 Record the Kivori Spec Kit quality-gate extension as project tooling outside Feature 001's specified scope, or justify its inclusion, per plan/tasks traceability (unrequested) — `tools/spec-kit-extensions/kivori-gates/` (manifest, five commands, config template, README, 161 validators) plus the generated `.specify/extensions.yml` and `.claude/skills/speckit-kivori-gates-*` registrations trace to no FR, SC, plan touch-point, or task in this feature. It was requested directly by the user, not by the spec, so convergence surfaces it for awareness only: nothing is to be deleted. Resolve by adding a short tooling note to `docs/` (or the feature's plan) stating that the gates are repository-wide developer tooling with their own lifecycle, independent of Feature 001 acceptance
