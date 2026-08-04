---
description: "Verify each checked task's evidence is sufficient and that simulation is never cited as physical proof"
---

# Kivori Evidence Audit

Runs before convergence. It judges whether the **evidence** behind each checked task actually supports that
task's literal wording. It reports proposed checkbox corrections and **never edits `tasks.md` silently**.

## Hard prohibitions

You **MUST NOT** edit tasks, commit, push, or reset. Do not read or `source` `.env`, print any
credential, or run `env`/`printenv`/`set`/`export -p`.

## Evidence classes

Classify every checked task's evidence into exactly one class, and never let a weaker class satisfy a
stronger requirement:

| Class | Means | Can satisfy |
|---|---|---|
| **HOST** | a test on the developer/CI machine | host-testable requirements |
| **COMPILE** | it builds for the target | compile-only requirements |
| **SIM** | an executed Wokwi scenario with a recorded log | simulation requirements only |
| **DESKTOP-MANUAL** | a human ran the real desktop app | desktop lifecycle requirements |
| **PHYSICAL** | a real board and/or display | physical requirements only |

**SIM never satisfies PHYSICAL.** COMPILE never satisfies HOST. HOST never satisfies DESKTOP-MANUAL.

## Checks

### 1. Sufficiency per literal wording

For each checked task, restate its requirement, name the evidence, name its class, and judge sufficiency.
Insufficient evidence is a proposed correction to unchecked (or to a precise PARTIAL note).

### 2. Simulation evidence integrity

Where a task cites a Wokwi run, verify against the artifacts on disk — not a narrative:

- the scenario log exists and contains the required success marker;
- the marker appears **after** the postconditions it claims to summarise (order matters: `wait-serial`
  scans forward only, so a marker emitted before its postconditions is not proof of them);
- failure-signature scan is empty (`panicked`, `KIVORI-*-FAIL`, `ALL FAIL`);
- exactly one boot banner (`ESP-ROM:`) — two means a reset loop;
- `summary.json` agrees with the logs on exit code, marker presence, and derived pass status;
- the ELF identity (sha256) recorded for the run matches the artifact the claim relies on. If the artifact
  was rebuilt after the run, the identity is **not pinned** and the claim is weaker than stated.

### 3. Non-vacuity

A success marker that appears in *every* log proves nothing about the specific scenario. Confirm each
scenario's completion marker is absent from the other scenarios' logs.

### 4. Destroyed or missing evidence

If a task cites a log, VCD, or summary that no longer exists on disk, that citation is stale. Report it as
such and state whether the underlying verification is re-runnable. **Never let a task keep citing evidence
that is gone** without saying so.

### 5. Physical claims

For every physical task: require a `docs/validation-checklist.md` row with a result, a date, board/display
revision, and the firmware/desktop commit. Absent that, the task must be unchecked. A physical task
supported by SIM evidence is the most serious finding this command can report.

### 6. No agent-summary-only tasks

A task whose sole support is a previous agent report is unsupported. Say so explicitly.

## Output

A table: task ID · literal requirement · evidence · class · sufficient? · finding. Then:

- proposed checkbox corrections, as explicit before/after with a reason;
- the count of tasks whose evidence is stale, missing, or misclassified;
- a final line:

```
EVIDENCE AUDIT: PASS | FAIL
```

`FAIL` when any checked task lacks sufficient evidence for its own wording, or when any simulation result is
presented as physical proof.
