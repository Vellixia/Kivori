---
description: "Validate tasks.md integrity: IDs, ordering, evidence references, and simulation-vs-physical separation"
---

# Kivori Task-Integrity Audit

Runs after task generation or any task change. **Read-only by default**: it reports proposed corrections and
does not edit `tasks.md` unless the user explicitly asks for a repair in the same turn.

## Hard prohibitions

Do not commit, push, reset, or silently flip a checkbox. Do not read `.env`. Do not print credentials.

## Checks

Operate on the active feature's `tasks.md`.

### 1. Identifier integrity

- Every task line matches `- \[[ xX]\] T\d+`.
- IDs are **unique** — duplicates are a FAIL naming each duplicate.
- IDs are contiguous from the lowest to the highest present; report any gap.
- IDs are non-decreasing in file order; a task appearing out of order is reported as accidental reordering.

### 2. Dependency ordering

- A task whose text names a prerequisite (`after T0xx`, `once T0xx exists`, `BLOCKED ON T0xx`) must appear
  after that prerequisite, and the prerequisite must not be unchecked while the dependant is checked.
- A checked task depending on an unchecked task is a **FAIL**.

### 3. Evidence references on checked tasks

A checked task must carry evidence proportional to its literal wording:

| Task wording implies | Required evidence in the note |
|---|---|
| a file/module deliverable | the path, and that path must exist on disk |
| tests | the test file or count |
| simulation execution | scenario name, marker, and artifact identity |
| physical execution | board/display revision, timestamp, and a `docs/validation-checklist.md` row |

A checked task whose only support is prose ("verified", "works", "done") with no path, test, artifact, or
checklist row is a **FAIL** — no task may be marked complete solely from a narrative report.

### 4. Manual and physical separation

- Tasks needing a human or a board are labelled (`[MANUAL]` or explicit wording).
- **A simulation task must never satisfy a physical task.** If a physical task's note cites only Wokwi,
  VCD, or host evidence, that is a **FAIL**.
- Report the manual/physical tasks that remain unchecked as expected residue, not as errors.

### 5. Stale annotations

- No `PARTIAL`, `DEFERRED`, or `TODO` annotation may survive on a task that is now checked and fully
  implemented.
- No annotation may reference a phase or task as "pending" when that task is checked.

### 6. Appended convergence tasks

- Tasks appended by convergence use the next unused IDs, never a reused or interleaved ID.
- Appended tasks carry the same evidence standard as any other task.

### 7. No retroactive requirements

Compare each checked task's note against its own literal requirement line. If the note imposes a stricter
requirement than the task text (for example a repeated-stability run the task never asked for) and uses it to
justify leaving the task open, report it: **a requirement invented after implementation is not a criterion.**
Conversely, a note that quietly narrows the task's stated requirement is also a FAIL.

## Output

Print a table of findings — severity, task ID, and the exact rule — then counts of checked/unchecked, then:

```
TASK AUDIT: PASS | FAIL
```

If corrections are warranted, list them as explicit proposed diffs (task ID, current state, proposed state,
reason) and stop. Do not apply them in this command.
