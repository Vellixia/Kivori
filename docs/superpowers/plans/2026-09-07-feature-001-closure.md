# Feature 001 Closure Implementation Plan

**Goal:** Close the software-verifiable portion of Feature 001 in PR #1 and leave only manual acceptance tasks that genuinely require a Windows desktop or physical ESP32-C3.

**Architecture:** Keep the production architecture unchanged unless executable Windows startup evidence reproduces a runtime failure. Add a real-app startup acceptance boundary, reconcile current implementation/hardware evidence in an authoritative closure ledger, then require every workflow to pass on the final PR head.

**Spec:** `docs/superpowers/specs/2026-09-07-feature-001-closure-design.md`

## Global constraints

- Target remains Windows + USB serial only for Feature 001.
- Device Studio remains development-only and excluded from production end-user builds.
- Verified physical profile values are not changed without new hardware evidence.
- Manual tasks never receive completion credit from CI-only evidence.
- PR #1 is not merged by this plan.

## Task 1: Real Windows Device Studio startup acceptance

- [x] Add `scripts/test-windows-device-studio-startup.ps1`.
- [x] Add the `windows-device-studio-startup` job to `.github/workflows/host.yml`.
- [x] Execute the real Windows startup path in PR #1 host run #16.
- [x] Confirm the historical stack-overflow is not reproducible on the corrected baseline.
- [x] Do not modify production runtime code without a reproduced failure.

**Evidence:** job `101661595087` built the frontend and native binary, launched `kivori-desktop.exe`, and recorded `KIVORI-WINDOWS-STARTUP PASS: process remained alive for 10 seconds with no fatal runtime signature.`

## Task 2: Reconcile implementation and physical evidence

- [x] Verify T074 against current source: `physical_st7789::run_mode` binds USB transport, SPI2/ST7789 display, compiled assets, and the shared `runtime::run` loop.
- [x] Verify T115 against dated checklist evidence: VID/PID enumeration, automatic discovery, and handshake identity were observed on 2026-08-11.
- [x] Verify T118 against dated checklist evidence: physical ST7789 initialization and `(0,0)` geometry were observed on 2026-08-11.
- [x] Keep T093, T116, T117, T119, T120, T121, and T122 manual/incomplete.
- [x] Preserve unmeasured checklist rows as unmeasured.
- [x] Record the reconciled state in `specs/001-device-connection-foundation/closure-status.md` while preserving the large historical `tasks.md` as audit history.

**Plan deviation:** the initial design proposed rewriting stale entries directly in `tasks.md`. The file is a large historical ledger with extensive superseded execution notes. Replacing the entire file solely to edit a few historical annotations would create avoidable churn and risk accidental damage. The current state is therefore recorded in a dedicated authoritative closure ledger beside the feature spec, with the historical ledger left intact.

## Task 3: Record software closure status

- [x] Add an explicit status of `software complete; manual acceptance outstanding` in the closure ledger.
- [x] Update `docs/validation-checklist.md` with the Windows startup evidence while keeping physical/manual blanks open.
- [x] Update the closure design with run #16 evidence and the `not reproducible on corrected baseline` conclusion.
- [x] Record that no Windows production runtime patch was justified or made.

## Task 4: Final verification and PR reconciliation

- [ ] Require fresh success for host, firmware, determinism, and Wokwi on the final closure-document head.
- [ ] Within host, verify frontend, Ubuntu Rust, Windows Rust, and `windows-device-studio-startup` all succeed.
- [ ] Review the final PR diff for Feature 001-only scope and confirm no Feature 002 files changed.
- [ ] Update PR title to `fix: close Feature 001 software baseline` and rewrite the body with final evidence.
- [ ] Leave PR #1 open and mergeable. Do not merge without an explicit user instruction.
