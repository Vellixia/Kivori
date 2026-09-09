# Flash firmware from Overview

In the native app, connect the ESP32-C3 and wait for **Connected**. Under **Overview → Device firmware**,
click **Flash firmware** to replace the device firmware with the bundled Kivori ESP32-C3/ST7789 build.
This includes the mascot artwork and animation engine. Keep USB connected until the update completes.
The action reports preparation, flashing, and reconnection, and is disabled while an update is running.

**Mirror to device** sends an expression command; it does not transfer artwork or install firmware.
Use it after the firmware update to select idle, happy, busy, or sleeping.

## Build a desktop with firmware included

On Windows, with the Rust RISC-V target and `espflash` installed:

```powershell
./scripts/build-desktop-firmware.ps1
# Or a production desktop without Device Studio:
./scripts/build-desktop-firmware.ps1 -Release
```

The script first builds the isolated firmware workspace in release mode with `physical-st7789`, then
builds the desktop with that exact ELF embedded. It does not flash anything. The verified physical
profile is recorded in `firmware/esp32-c3/src/profile.rs`; this package targets that ESP32-C3/ST7789
board and pin map. The updater uses the installed `espflash` utility; it is not included in the app.

For other build pipelines, set `KIVORI_FIRMWARE_PATH` to the absolute path of the freshly built physical
firmware ELF while building the desktop. The build rejects a non-RISC-V/ELF32 image. A regular host
build without this environment variable has no bundled firmware and disables the update action.
This prevents a stale local firmware artifact from silently becoming part of a desktop build.

## Behavior and verification boundary

The updater releases the current serial connection before launching `espflash` on that same port.
The webview cannot choose files, ports, or shell arguments. Flash verification remains enabled.
Connection handling resumes on failure; a successful flash waits for a compatible handshake from
the same device before reporting completion. Firmware status survives Overview navigation.

Automated checks exercise UI gating, duplicate requests, errors, navigation, and native workflow
state transitions without writing to hardware. A physical flash and panel inspection are separate
checks; building this feature does not perform them.

### Checks performed on 2026-09-08

- Desktop suite: 71 tests passed with the firmware package included; Clippy passed with warnings denied.
- Frontend: 39 tests passed, production build and focused ESLint passed.
- The physical release firmware built successfully. `espflash save-image` accepted it as an ESP32-C3
  application: 180,976 bytes (the embedded ELF is 344,512 bytes).
- No flash was performed. Native window automation could not connect to its helper, so the actual
  click-through, post-flash handshake, and physical display still need hardware validation.

### Connecting recovery (2026-09-08)

The ordinary connection loop previously had no HelloAck deadline and never scheduled heartbeat
pings. A lost startup handshake could therefore leave Overview at Connecting indefinitely, disabling
the flash action. It now times out after five seconds, closes the stale link, retries with backoff,
and maintains one-second heartbeats after connecting. Serial command write errors also enter recovery.

Verification: 73 desktop tests and Clippy passed. On the attached COM3 ESP32-C3, early probes received
health packets without a handshake; resetting the board's USB session and allowing startup to finish
restored communication. The production serial/session diagnostic then connected and exchanged
heartbeats for eight seconds with reported state Idle. The rebuilt native app subsequently logged
Disconnected → Connecting → Connected in about 2.1 seconds and remained responsive. No firmware was
flashed. This establishes live connection behavior, not physical flashing or display parity.
