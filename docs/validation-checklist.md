# Manual validation checklist

Human-run checks that host CI **cannot** cover: the window-hidden background behaviour (T093) and the
on-device integration on a physical ESP32-C3 (Phase 15, T115–T122). These never gate a merge. The full
procedure lives in [quickstart.md](../specs/001-device-connection-foundation/quickstart.md); this file
is where results are recorded. Targets come from the spec Success Criteria and the
[hardware-validation numbers](../specs/001-device-connection-foundation/research.md).

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
| 5 | Device enumerates as `0x303A:0x1001`; auto-discovered with **no** port selection; identity confirmed via handshake | SC-001 | | |
| 6 | Time from plug-in → `connected` | < 5 s (SC-001) | | |
| 7 | An unrelated USB-serial device is **not** reported as connected | FR-004 | | |
| 8 | A wrong-major firmware → `incompatible` with a clear reason, no state commands sent | < 5 s (SC-003) | | |

## Hardware — US2 companion states (T118, T119, T120)

| # | Check | Target | Result | Notes |
|---|-------|--------|--------|-------|
| 9 | Display initialization + panel offsets correct (GC9A01 / ST7789) | R-3 | | |
| 10 | Each of `idle/happy/busy/sleeping` shows the matching animated scene | — | | |
| 11 | State change appears on the device | < 1 s (SC-004) | | |
| 12 | On-device image matches the Device Studio preview for a fixed state + elapsed time | SC-005 | | |
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
| 23 | **Panel controller identified** on the physical module (GC9A01 vs ST7789) and `DeviceProfile::KIVORI_240.controller` updated to match | R-3 | | |
| 24 | **Real SPI pin map recorded** (SCK / MOSI / CS / D/C / RST / backlight) — the `wokwi-spi` profile is simulation-only and must not be reused | R-3 | | |
| 25 | Panel offsets measured on the physical module and passed to `PanelGeometry` (there is no default) | R-3 | | |

Items 23–25 are the facts the Wokwi generic SPI probe deliberately does **not** supply. The probe validates
bus behaviour and the RGB565 tile stream; the controller, its init sequence, its offsets, and the board pin
map can only come from hardware.
