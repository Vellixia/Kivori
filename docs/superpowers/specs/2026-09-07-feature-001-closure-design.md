# Feature 001 Closure Design

**Date:** 2026-09-07

## Goal

Turn PR #1 into the closure PR for `001-device-connection-foundation`: preserve the restored green CI baseline, prove the real Windows Device Studio runtime can start, reconcile stale implementation/task evidence, and leave only checks that genuinely require a human-controlled Windows desktop or physical ESP32-C3 as manual acceptance.

## Current evidence

- Host, firmware, determinism, and Wokwi were green before closure work began.
- The production physical firmware path exists in `firmware/esp32-c3/src/physical_st7789.rs` and constructs USB Serial/JTAG transport, SPI2, the verified ST7789 display profile, compiled assets, and the shared production `runtime::run` loop.
- `docs/validation-checklist.md` records physical discovery/handshake, ST7789 initialization, controller identity, pin map, and offsets from 2026-08-11.
- Historical `tasks.md` annotations predate that physical validation and therefore contain stale statements about T074 and unknown hardware facts.
- The remaining Phase 15 tasks include measurements and behavioral checks that cannot be manufactured by CI.

## Architecture

### Windows runtime acceptance

A Windows-only GitHub Actions job builds the frontend, builds the default-feature desktop binary, launches `kivori-desktop.exe`, captures stdout/stderr, and requires the process to remain alive through a 10-second startup observation window. Early exit, `stack overflow`, or `fatal runtime error` fails the acceptance test. The job then terminates the still-running process deliberately.

This tests the actual Tauri/WebView startup path instead of inferring runtime health from `cargo test`.

### Runtime debugging rule

Production runtime code is changed only if the Windows startup smoke reproduces a failure. If it passes, the old stack-overflow report is recorded as no longer reproducible on the corrected baseline rather than assigning an unsupported root cause.

### Tracking reconciliation

Current closure status is recorded in `specs/001-device-connection-foundation/closure-status.md`. That ledger supersedes stale historical annotations while keeping `tasks.md` intact as audit history:

- T074 is complete because `physical_st7789::run_mode` constructs the production transport/display/clock integration and calls `runtime::run`.
- T115 is complete because the dated checklist records enumeration, automatic discovery, and handshake identity.
- T118 is complete because the dated checklist records physical ST7789 initialization and `(0,0)` geometry.
- T093, T116, T117, T119, T120, T121, and T122 remain incomplete unless new manual evidence exists.
- Partial checklist rows remain partial. A successful observation without a measured latency does not satisfy a latency criterion.

The large historical task ledger is not reserialized solely to rewrite old annotations. This avoids accidental unrelated churn while making the current authoritative state explicit beside the feature spec.

### Feature status

Feature 001 is described as **software complete; manual acceptance outstanding** once the final PR head is green across host, firmware, determinism, Wokwi, and the Windows startup smoke. This distinguishes implementation completion from physical/manual sign-off.

## Windows startup evidence

PR #1 host run #16 on head `1f9f6f89e279fe1ba2f483d48ca73462cf0bcc46` executed the new `windows-device-studio-startup` job on Windows Server 2025. The job:

1. built the Device Studio frontend;
2. built default-feature `kivori-desktop.exe` with Rust 1.98.1;
3. launched the real Tauri binary;
4. observed it for 10 seconds;
5. recorded `KIVORI-WINDOWS-STARTUP PASS: process remained alive for 10 seconds with no fatal runtime signature.`

The historical Windows startup stack-overflow is therefore **not reproducible on the corrected PR baseline**. No production runtime change was made because there is no failing runtime symptom to fix.

Run #16 also completed successfully for frontend, Ubuntu Rust, Windows Rust, firmware, determinism, and Wokwi. A final fresh run is still required after the closure-document commit before the PR is called ready.

## Non-goals

- No Feature 002 work.
- No new transport, protocol, rendering, or UI feature.
- No fabricated hardware measurements.
- No changing verified physical profile values without new physical evidence.
- No merging PR #1 as part of this work.

## Acceptance

PR #1 represents Feature 001 software closure when:

1. host CI passes on Ubuntu and Windows;
2. Windows Device Studio startup smoke passes using the real binary;
3. firmware CI passes;
4. determinism CI passes;
5. Wokwi CI passes;
6. the closure ledger and validation docs agree with current source/evidence;
7. all remaining outstanding acceptance items are explicitly manual and evidence-dependent.
