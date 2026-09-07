# Feature 001 Closure Design

**Date:** 2026-09-07

## Goal

Turn PR #1 into the closure PR for `001-device-connection-foundation`: preserve the restored green CI baseline, prove the real Windows Device Studio runtime can start, reconcile stale implementation/task evidence, and leave only checks that genuinely require a human-controlled Windows desktop or physical ESP32-C3 as manual acceptance.

## Current evidence

- Host, firmware, determinism, and Wokwi are green on PR head `fbf9127`.
- The production physical firmware path exists in `firmware/esp32-c3/src/physical_st7789.rs` and constructs USB Serial/JTAG transport, SPI2, the verified ST7789 display profile, compiled assets, and the shared production `runtime::run` loop.
- `docs/validation-checklist.md` already records physical discovery/handshake, ST7789 initialization, controller identity, pin map, and offsets from 2026-08-11.
- `tasks.md` still contains stale text claiming T074 is partial and that the physical controller/pins/offsets are unknown.
- The remaining Phase 15 tasks include measurements and behavioral checks that cannot be manufactured by CI.

## Architecture

### Windows runtime acceptance

Add a Windows-only GitHub Actions job that builds the real frontend, builds the default `device-studio` desktop binary, launches `kivori-desktop.exe`, captures stdout/stderr, and requires the process to remain alive through a short startup observation window. Early exit, `stack overflow`, or `fatal runtime error` is a failed acceptance test. The job then terminates the still-running process deliberately.

This tests the actual Tauri/WebView startup path instead of inferring runtime health from `cargo test`.

### Runtime debugging rule

Do not change production runtime code unless the Windows startup smoke fails. If it fails, use the captured process output as the root-cause boundary, add the narrowest regression test possible for the failing component, then make one minimal fix. If the smoke passes on the current branch, record the old stack-overflow report as no longer reproducible on the corrected baseline rather than inventing a cause.

### Tracking reconciliation

Update Feature 001 tracking from source and recorded evidence:

- T074 becomes complete because `physical_st7789::run_mode` constructs all three production ports and calls `runtime::run`.
- T115 and T118 become complete because the checklist contains dated physical evidence for enumeration/identity and panel initialization/offsets.
- T116, T117, T119, T120, T121, T122 and T093 remain incomplete unless new evidence exists.
- Remove stale statements that the physical profile is unknown.
- Keep partial checklist rows partial. A successful observation without a measured latency does not satisfy a latency criterion.

### Spec status

Change the spec status from `Draft` to `Software complete; manual acceptance outstanding` once the Windows startup smoke and all automated gates are green. This distinguishes implementation completion from physical/manual sign-off.

## Non-goals

- No Feature 002 work.
- No new transport, protocol, rendering, or UI feature.
- No fabricated hardware measurements.
- No changing verified physical profile values without new physical evidence.
- No merging PR #1 as part of this work.

## Acceptance

PR #1 is ready to represent Feature 001 software closure when:

1. host CI passes on Ubuntu and Windows;
2. Windows Device Studio startup smoke passes using the real binary;
3. firmware CI passes;
4. determinism CI passes;
5. Wokwi CI passes;
6. Feature 001 tasks and validation docs agree with source/evidence;
7. all remaining unchecked tasks are explicitly manual and evidence-dependent.
