# Kivori architecture

A built-system overview of the Device Connection Foundation: the desktop app, the ESP32-C3 firmware,
and the shared crates they both build on. Design rationale lives in the
[ADRs](./adr/); requirements in [specs/001-device-connection-foundation](../specs/001-device-connection-foundation).

## Two workspaces, one shared core (ADR-0001)

```
kivori/
├── crates/                     # shared, no_std, no-alloc — the single source of truth
│   ├── kivori-model            # state axes, device profile, timeline, version/caps, geometry/color, scene schema
│   ├── kivori-protocol         # COBS + header + CRC-32 + postcard wire codec, handshake, negotiation, seq policy
│   ├── kivori-framebuffer      # RGB565 tile bands + content hashing (change detection)
│   ├── kivori-renderer         # deterministic scene compositor (embedded-graphics DrawTarget)
│   └── kivori-assets           # zero-copy runtime reader for the compiled asset blob
├── apps/desktop/
│   ├── src-tauri/              # kivori-desktop: native core (root Cargo workspace member)
│   └── src/                    # kivori-desktop-ui: React frontend (pnpm workspace)
├── tools/asset-compiler/       # kivori-asset-compiler: SVG → RGB565 blob (host build tool; resvg)
├── tests/golden-frames/        # cross-OS frame-hash determinism harness
└── firmware/esp32-c3/          # kivori-firmware: SEPARATE Cargo workspace (feature-isolation firewall)
```

Two Cargo workspaces are deliberate: the firmware is `no_std`/no-alloc for `riscv32imc`, and keeping it
out of the host workspace prevents Cargo feature unification from pulling std-only features into the
device build. The shared crates are consumed by **both** the desktop and the firmware via path
dependencies, so the renderer that draws the Device Studio preview is byte-for-byte the renderer that
drives the panel (constitution Principle II). A direct-dependency firewall
([`scripts/check-crate-boundaries.sh`](../scripts/check-crate-boundaries.sh)) fails CI if any
`crates/kivori-*` gains a std/host/OS dependency; the RISC-V compile is the hard backstop.

## Three state axes (kivori-model)

- **Companion** (6): `booting, idle, happy, busy, sleeping, offline` — what the device *shows*.
- **Sendable** (4): `idle, happy, busy, sleeping` — the subset the desktop may *command*. `booting`
  and `offline` are device-originated and never transmitted (enforced at the type level).
- **Connection** (5): `disconnected, connecting, connected, incompatible, error` — the desktop↔device
  link lifecycle, a pure `(state, event) → state` machine.

## Data flows

**Wire (desktop ↔ device, USB serial — ADR-0002).** Frames are
`COBS(magic ‖ ver ‖ seq ‖ len ‖ postcard-payload ‖ CRC-32) ‖ 0x00`. `MAX_PAYLOAD = 512`. Compatibility
is major-version gated; minor + capabilities are negotiated (min-minor, capability intersection). A
sequence policy classifies duplicate/gap/wraparound; malformed frames are dropped without panicking and
the decoder resyncs on the next delimiter. Messages: `Hello/HelloAck/Ready/Bye`, `SetState/StateReport`,
`Ping/Pong`, `Health/Diagnostic/Error`.

**IPC (native core ↔ webview — [contracts/ipc.md](../specs/001-device-connection-foundation/contracts/ipc.md)).**
The webview may call only a fixed set of typed commands and subscribe to `connection://status` /
`diagnostics://event`; frame bytes stream over a `Channel<ArrayBuffer>`. No raw serial, filesystem,
shell, or credential surface is exposed. The canvas is a pure blit target — the RGB565→RGBA expansion
happens in Rust so the frontend cannot diverge from device pixels.

**Asset pipeline (ADR-0004, build-time).** `kivori-asset-compiler` rasterizes layered SVGs with
resvg/tiny-skia to deterministic RGB565 and emits a blob = 16-byte header + postcard manifest +
zero-copy data pool. The blob hash is reproducible and asserted in CI; the desktop and firmware read it
with `kivori-assets` (no runtime SVG/PNG parser anywhere).

The mascot source is `assets/mascot.svg`, reconstructed from `assets/mascot.png`. Format v2 stores
cropped RGB565 sprites with packed alpha4. Body/shine/cheeks are shared across all six states;
each eye texture is reused for both eyes. The full pack is limited to 128 KiB at build time.
`MascotAnimator` resolves blinking, state-specific motion, and 350 ms eased transitions (600 ms entering
sleep) in fixed-point arithmetic. Expression changes are hidden inside a blink so eye and mouth sprites
do not overlap. The firmware retains this controller between state changes and renders its pose through
the same compositor as native Device Studio.

Physical ST7789 mode stages a complete RGB565 pose in 112.5 KiB of static SRAM before sending changed
40x40 tiles through SPI DMA. Composition and hashing never interrupt the transfer phase. This is one
prepared frame, not panel double buffering; the unwired TE signal means refresh synchronization is
not available. Simulation can retain the small-buffer path through the same runtime and compositor.

Device Studio keeps up to 256 semantic selection events for replayable pause, step, and backward
seek. While playing, one externally-clocked stream coalesces timestamp updates and discards frames
superseded during rendering; two in-flight RGBA frames remain the maximum. The browser-only mock
is still a placeholder. Use the native Tauri app to review actual mascot/device pixels.

**Render path.** `render_scene(blob, scene, elapsed_ms, tile_band)` composites a scene into an RGB565
tile using integer-only, drift-free timing (`elapsed_ms = (n·1000 + 15) / 30`). The firmware renders
tile-by-tile and flushes only tiles whose content hash changed; the stitched tiles are byte-identical to
a full-frame render (proven host-side), which is what lets the golden frames stand in for the panel.

## Runtime shape

- **Firmware** ([firmware/esp32-c3/src](../firmware/esp32-c3/src)): a `no_std` library (device
  lifecycle FSM, protocol dispatcher, heartbeat, change-driven renderer) written against
  hardware-neutral `Transport`/`DisplaySink`/`Clock` ports, plus a bare-metal binary gated behind the
  `embedded` feature. Host simulation adapters (`host-sim`) run the entire device core on the host.
- **Desktop core** ([apps/desktop/src-tauri/src](../apps/desktop/src-tauri/src)): discovery (VID/PID
  filter), handshake verification, the connection manager (FSM + retry/backoff + hashed-identity
  summary), heartbeat, the desired-state orchestrator, the window-lifecycle policy, safe diagnostics,
  and the host-side Device Studio preview renderer. These are pure/host-testable; the Tauri runtime, the
  concrete serial adapter, and the async run loop bind them together in the integration layer.
- **Frontend** ([apps/desktop/src](../apps/desktop/src)): React + Zustand. Typed IPC wrappers with a
  browser-mock fallback; a blit-only device canvas; the dev-only Device Studio (state selection,
  timeline scrub, play/pause/step, mirror-to-device), gated to dev builds.

## Cross-cutting guarantees

- **Deterministic rendering** (Principle III): integer-only, golden-frame hashed, byte-identical across
  Windows and Linux.
- **Offline-first** ([offline-boundary.md](./offline-boundary.md)): no network client in any first-party
  crate; no remote frontend assets; USB is the only link.
- **Least privilege + privacy** (ADR-0005, [diagnostics-and-logging.md](./diagnostics-and-logging.md)):
  raw device identity is hashed at the transport boundary; diagnostics carry only an allowlisted, safe
  field set; raw-payload logging is a dev-only, compiled-out path.
