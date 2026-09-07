# Feature 001 Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the software-verifiable portion of Feature 001 in PR #1 and leave only honest manual acceptance tasks outstanding.

**Architecture:** Keep the production architecture unchanged unless the new Windows runtime smoke reproduces a failure. Add one executable Windows startup acceptance boundary, reconcile stale spec/task evidence against actual source and dated validation records, then run every existing CI workflow on the final head.

**Tech Stack:** Rust, Tauri v2, React/Vite/pnpm, GitHub Actions, PowerShell, ESP32-C3 firmware, Wokwi.

**Spec:** `docs/superpowers/specs/2026-09-07-feature-001-closure-design.md`

## Global Constraints

- Target platform remains Windows + USB serial only for Feature 001.
- Device Studio remains development-only and absent from production end-user builds.
- Physical profile facts remain those already validated in `docs/validation-checklist.md`; do not alter them without new hardware evidence.
- Manual tasks never receive a checkmark from CI-only evidence.
- PR #1 is not merged by this plan.

---

### Task 1: Add real Windows Device Studio startup acceptance

**Files:**
- Create: `scripts/test-windows-device-studio-startup.ps1`
- Modify: `.github/workflows/host.yml`

**Interfaces:**
- Consumes: built frontend at `apps/desktop/dist` and default-feature `kivori-desktop.exe`.
- Produces: a CI assertion that the real Tauri process survives startup without stack overflow/fatal runtime error.

- [ ] **Step 1: Add the startup smoke script**

Create a PowerShell script that:
1. resolves `target\debug\kivori-desktop.exe` from the repository root;
2. launches it with stdout/stderr redirected into `$env:RUNNER_TEMP`;
3. observes the process for 10 seconds;
4. fails and prints both logs if the process exits during that window;
5. fails if stderr contains `stack overflow` or `fatal runtime error`;
6. force-terminates a still-running process after the observation window and reports success.

The script must treat intentional termination after the observation window as success, not as an application crash.

- [ ] **Step 2: Wire a Windows-only CI job**

Add `windows-device-studio-startup` to `.github/workflows/host.yml` on `windows-latest`. Install Node 20 and stable Rust, run `corepack enable`, `pnpm install --frozen-lockfile`, `pnpm --filter kivori-desktop-ui build`, `cargo build -p kivori-desktop`, then execute `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-device-studio-startup.ps1`.

- [ ] **Step 3: Run GitHub Actions and inspect the Windows startup result**

Expected result is evidence, not assumption:
- PASS: the old runtime stack-overflow report is not reproducible on the corrected PR baseline; no production runtime patch is justified.
- FAIL: print captured logs, identify the exact failing component, then append a root-cause-specific regression/fix task to this plan before changing production code.

- [ ] **Step 4: Commit the acceptance gate**

Commit message: `test(windows): exercise Device Studio startup`

---

### Task 2: Reconcile physical runtime implementation evidence

**Files:**
- Modify: `specs/001-device-connection-foundation/tasks.md`
- Modify: `docs/validation-checklist.md`

**Interfaces:**
- Consumes: `firmware/esp32-c3/src/profile.rs`, `firmware/esp32-c3/src/physical_st7789.rs`, existing dated checklist evidence.
- Produces: task state that matches implemented code and recorded physical observations.

- [ ] **Step 1: Correct T074**

Replace the stale PARTIAL annotation with a completed entry documenting that `physical_st7789::run_mode` owns `UsbJtagTransport`, SPI2, the verified ST7789 profile, `MipidsiSink`, compiled assets, and invokes the shared production `runtime::run` loop.

- [ ] **Step 2: Correct Phase 15 checkboxes conservatively**

Mark T115 complete because checklist item 5 records enumeration, auto-discovery, and handshake identity. Mark T118 complete because checklist items 9, 23, and 25 record successful ST7789 initialization and `(0,0)` geometry. Keep T116/T117/T119/T120/T121/T122 incomplete because the checklist still lacks their required complete measurements/observations.

- [ ] **Step 3: Remove stale hardware-unknown prose**

Where T074/T131 historical text says the physical controller, real pin map, or offsets remain unknown, rewrite it to distinguish historical Wokwi scope from the later physical facts recorded on 2026-08-11. Do not rewrite simulator claims into physical evidence.

- [ ] **Step 4: Clarify manual checklist status**

Add a short status note explaining that implementation and automated acceptance can be complete while T093 and the remaining Phase 15 measurements stay manual. Preserve every existing measurement blank that was never actually measured.

- [ ] **Step 5: Commit tracking reconciliation**

Commit message: `docs(feature-001): reconcile implementation evidence`

---

### Task 3: Mark software closure status

**Files:**
- Modify: `specs/001-device-connection-foundation/spec.md`
- Modify: `docs/superpowers/specs/2026-09-07-feature-001-closure-design.md`
- Modify: `docs/superpowers/plans/2026-09-07-feature-001-closure.md`

**Interfaces:**
- Consumes: successful Task 1 Windows runtime result and reconciled Task 2 evidence.
- Produces: an explicit Feature 001 status separating software completion from manual acceptance.

- [ ] **Step 1: Update spec status only after automated acceptance is green**

Change the feature status line from `Draft` to `Software complete; manual acceptance outstanding`.

- [ ] **Step 2: Record the Windows startup evidence**

Add the final workflow run/head SHA and result to the closure design. If the startup smoke passed without a production runtime change, state `not reproducible on corrected baseline`; do not claim a root cause. If a runtime fix was required, record the exact root cause and regression test instead.

- [ ] **Step 3: Check off completed plan steps**

Update this plan's checkboxes to match actions that were actually executed.

- [ ] **Step 4: Commit status update**

Commit message: `docs(feature-001): record software closure status`

---

### Task 4: Final verification and PR reconciliation

**Files:**
- Modify: PR #1 title/body only; no source files unless a verified failing gate requires a fix.

**Interfaces:**
- Consumes: final PR head.
- Produces: reviewable Feature 001 closure PR with fresh CI evidence.

- [ ] **Step 1: Require fresh workflow success on final head**

Verify `host`, `firmware`, `determinism`, and `wokwi` all complete with `success`. Inside host, verify frontend, Ubuntu Rust, Windows Rust, and `windows-device-studio-startup` all complete successfully.

- [ ] **Step 2: Review final PR diff**

Confirm changes remain Feature 001 closure/stabilization only. No Feature 002 files or unrelated refactors.

- [ ] **Step 3: Update PR metadata**

Title: `fix: close Feature 001 software baseline`

Body must summarize CI stabilization, Windows runtime startup acceptance, tracking reconciliation, fresh workflow evidence, and the remaining manual T093/Phase 15 checks. It must explicitly state that manual hardware measurements are not claimed complete.

- [ ] **Step 4: Do not merge**

Leave PR #1 open and mergeable for explicit user review/merge action.
