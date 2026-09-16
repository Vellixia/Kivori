# Manual validation checklist

Human-run checks that host CI **cannot** cover: the window-hidden background behaviour (T093) and the
on-device integration on a physical ESP32-C3 (Phase 15, T115–T122). These never gate a merge. The full
procedure lives in [quickstart.md](../specs/001-device-connection-foundation/quickstart.md); this file
is where results are recorded. Targets come from the spec Success Criteria and the
[hardware-validation numbers](../specs/001-device-connection-foundation/research.md).

**Software-closure note (2026-09-07):** GitHub Actions host run #16 on PR #1 added and passed a real
Windows startup smoke for the default-feature Tauri binary: `kivori-desktop.exe` remained alive for the
10-second observation window with no `stack overflow` or `fatal runtime error` signature. That removes
the previous *startup* blocker from automated acceptance, but it does not manufacture any of the manual
checks below. Rows without a result remain outstanding until they are executed on a real Windows desktop
and/or physical device.

Fill `Result` with ✅ / ❌ and date; put measurements and observations in `Notes`.

## Background operation (T093 — no hardware; Windows/macOS window manager)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 1 | Close the window → it hides; the process keeps running | FR-030 | | |
| 2 | While hidden, the device heartbeat + synchronization stay active | SC-007 | | |
| 3 | Re-activate (dock/taskbar click or second launch) → the single window reappears | FR-030 | | |
| 4 | Explicit Quit → the process terminates and the device task stops | FR-030 | | |

## Hardware — US1 discovery & connection (T115, T117)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 5 | Device enumerates as `0x303A:0x1001`; auto-discovered with **no** port selection; identity confirmed via handshake | SC-001 | ✅ 2026-08-11 | Windows detected the ESP32-C3 as `0x303A:0x1001` on COM3. Kivori Desktop auto-discovered the device with no manual port selection and completed the handshake. Desktop reported Connected, firmware 1.0.0, protocol 1.0. |
| 6 | Time from plug-in → `connected` | < 5 s (SC-001) | | Connection was observed successfully, but plug-in → connected latency was not measured from a controlled plug-in event. |
| 7 | An unrelated USB-serial device is **not** reported as connected | FR-004 | | |
| 8 | A wrong-major firmware → `incompatible` with a clear reason, no state commands sent | < 5 s (SC-003) | | |

## Hardware — US2 companion states (T118, T119, T120)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 9 | Display initialization + panel offsets correct (GC9A01 / ST7789) | R-3 | ✅ 2026-08-11 | Physical ST7789 240x240 initialized successfully on ESP32-C3. Kivori rendered correctly using physical geometry 240x240 with offset `(0,0)`. |
| 10 | Each of `idle/happy/busy/sleeping` shows the matching animated scene | — | | `idle` confirmed on the physical ST7789. `happy`, `busy`, and `sleeping` still need physical verification. |
| 11 | State change appears on the device | < 1 s (SC-004) | | Desktop → device `idle` state propagation was observed, but latency was not measured. |
| 12 | On-device image matches the Device Studio preview for a fixed state + elapsed time | SC-005 | | The historical Windows startup blocker is no longer reproduced by the PR #1 Windows startup smoke (2026-09-07), but preview ↔ physical-panel parity itself has not been manually executed, so this row remains open. |
| 13 | Sustainable SPI frame rate (full-frame vs tile updates) recorded | SC-004 / R-4 | | |
| 14 | Only semantic state is sent (verified via safe diagnostics, not payload bytes) | FR-015 | | |

## Hardware — US3 recovery & restoration (T116, T117, T121)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 15 | Unplug while `busy` → app shows `disconnected` | — | | |
| 16 | Replug → auto-reconnect, device returns to `busy` with no user action | < 10 s reconnect+restore (SC-002) | | |
| 17 | Rapid unplug/replug settles into a stable connected state (no thrash) | FR-010 | | |
| 18 | USB throughput + transmit-stall recovery under sustained traffic | R-1/R-2 | | |
| 19 | Full desktop restart → desired state resets to `idle` (no cross-restart persistence) | clarified | | |

## Hardware — platform & safety (T115, T122)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 20 | End-to-end latencies within targets: state < 1 s, connect < 5 s, reconnect+restore < 10 s | SC-001/002/004 | | |
| 21 | esp-hal `usb_serial_jtag` `unstable` API behaves as documented on the pinned version | R-2/R-11 | | |
| 22 | No sensitive data appears in the diagnostics view or the log file while connected | SC-010 | | |
| 23 | **Panel controller identified** on the physical module (GC9A01 vs ST7789) and `DeviceProfile::KIVORI_240.controller` updated to match | R-3 | ✅ 2026-08-11 | Physical panel confirmed as ST7789. Shared `DeviceProfile::KIVORI_240.controller` reconciled to `PanelController::St7789`. |
| 24 | **Real SPI pin map recorded** (SCK / MOSI / CS / D/C / RST / backlight) — the `wokwi-spi` profile is simulation-only and must not be reused | R-3 | ✅ 2026-08-11 | Verified physical profile: SCK GPIO6, MOSI GPIO7, CS unused, D/C GPIO2, RST GPIO3, backlight GPIO8 active-high. SPI2 runs at 20 MHz, Mode 3. Physical panel uses RGB color order, 90° rotation, and inversion enabled. |
| 25 | Panel offsets measured on the physical module and passed to `PanelGeometry` (there is no default) | R-3 | ✅ 2026-08-11 | Physical ST7789 uses a 240x240 visible area at controller offset `(0,0)`. Kivori rendered correctly on the physical panel using this geometry. |

Items 23–25 are physical facts established from the real ESP32-C3 + ST7789 hardware. The Wokwi generic
SPI probe remains simulation-only and does not establish controller identity, physical pin routing,
panel offsets, orientation, color order, inversion, backlight polarity, or electrical timing margins.

## Verified physical Kivori profile

The physical hardware validated on 2026-08-11 is:

| Property | Verified value |
|----------|----------------|
| MCU | ESP32-C3 |
| USB | Native USB Serial/JTAG |
| USB VID:PID | `0x303A:0x1001` |
| Display controller | ST7789 |
| Resolution | 240x240 |
| Pixel format | RGB565 |
| SPI peripheral | SPI2 |
| SPI clock | 20 MHz |
| SPI mode | Mode 3 |
| SCK | GPIO6 |
| MOSI | GPIO7 |
| MISO | Unused |
| CS | Unused |
| D/C | GPIO2 |
| Reset | GPIO3 |
| Backlight | GPIO8, active-high |
| Panel offset | `(0,0)` |
| Rotation | 90° |
| Color order | RGB |
| Color inversion | Enabled |

This profile is the physical Kivori hardware profile. It must remain separate from the Wokwi simulation
profile.