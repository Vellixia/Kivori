---
name: speckit-kivori-gates-preflight
description: 'Read-only implementation preflight: artifacts, task integrity, credential/evidence
  hygiene, and hardware-fact honesty'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: github-spec-kit
  source: kivori-gates:commands/speckit.kivori-gates.preflight.md
---

# Kivori Implementation Preflight

Fail-closed gate that runs **before** implementation. It only inspects; it never changes the repository.

> **Repository script paths.** The extension installer rewrites any literal `scripts` + `/` token in a
> command body to an extension-local path that does not exist. Repo scripts are therefore reached through
> `$SCRIPTS`, assembled once below so the literal never appears again in this file.

```bash
REPO="$(git rev-parse --show-toplevel)"
SCRIPTS="$REPO/scripts"
```

## Hard prohibitions

You **MUST NOT**, in this command: commit, tag, push, reset, revert, stash, delete files, edit `tasks.md`
or any checkbox, `source` or read the contents of `.env`, print any credential, run `env`/`printenv`/`set`/
`export -p`, or invoke an authenticated external service.

## 1. Identify the feature and tree state

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse --verify HEAD 2>&1 | head -1
git status --short | head -20
git tag --points-at HEAD
```

Report branch, short HEAD (or that the repository has no commit yet), whether the tree is clean, and any
tag at HEAD. A dirty tree is reported, not corrected.

## 2. Confirm the required Spec Kit artifacts exist

For the active feature directory under `specs/`:

| Artifact | Required |
|---|---|
| `.specify/memory/constitution.md` | yes |
| `spec.md` | yes |
| `plan.md` | yes |
| `tasks.md` | yes |
| `contracts/` | yes when the plan references contracts |
| `checklists/` | reported when present |

Any missing required artifact is a **FAIL**.

## 3. Report checklist status

Count `- [ ]` / `- [X]` / `- [x]` per file in `checklists/` and print a table with totals and PASS/FAIL.
Incomplete checklists are reported, not silently tolerated.

## 4. Task-integrity spot checks

- Task IDs match `T\d+`, are unique, and are contiguous with no gaps.
- Every checked task that names a file path has that path present on disk. A checked task naming a missing
  file is a **FAIL** — that is the classic false-complete.
- Manual/physical tasks are labelled (`[MANUAL]`, or wording naming a board, display, or OS interaction).

## 5. Credential and evidence hygiene

```bash
git check-ignore -q .env && echo ignored || echo NOT-IGNORED
git ls-files | grep -icE '(^|/)\.env|wokwi-logs|\.vcd$|summary\.json|\.elf$|user\.tok'
git diff --cached --name-only | grep -icE '(^|/)\.env|wokwi-logs|\.vcd$|summary\.json|\.elf$|user\.tok'
git grep -IE 'wok_[A-Za-z0-9]{8,}' HEAD -- >/dev/null 2>&1 && echo FOUND || echo none
```

`.env` not ignored, any tracked/staged credential-or-evidence artifact, or any token-shaped string in the
committed tree is a **FAIL**. Report counts only — never a matched value.

## 6. Architecture boundaries

```bash
bash "$SCRIPTS"/check-crate-boundaries.sh
bash "$SCRIPTS"/check-offline-deps.sh
```

Both must pass.

## 7. Production vs simulation boundary

Confirm the production surface excludes development-only surface, and that simulation-only configuration is
separated from physical configuration:

- `firmware/esp32-c3/src/profile.rs` (or equivalent) must contain **no** physical panel profile unless the
  controller, pin map, and offsets are recorded in `docs/validation-checklist.md`.
- Grep the firmware for a hard-coded controller choice outside a profile module. A controller named in
  `runtime.rs`, `main.rs`, or `display.rs` is a **FAIL** — the adapter must stay generic.

## 8. Hardware-fact honesty

Report, from files rather than memory:

- which physical facts are still unconfirmed (controller, pin map, offsets, orientation, colour order,
  backlight polarity);
- whether any task claims physical validation while `docs/validation-checklist.md` rows are unchecked;
- whether Wokwi evidence is described anywhere as physical proof. Any such claim is a **FAIL**.

## Output

Print each check with its command, exit code, and a one-line result, then a final line:

```
PREFLIGHT: PASS | FAIL | BLOCKED
```

`BLOCKED` only when a check could not run (missing tool), naming the tool. Never downgrade a failure to a
warning, and never emit PASS with an unexplained skipped check.