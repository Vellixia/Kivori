---
name: speckit-kivori-gates-release-gate
description: 'Post-convergence release safety: tree, staged content, secrets, tags,
  and remote reconciliation'
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: github-spec-kit
  source: kivori-gates:commands/speckit.kivori-gates.release-gate.md
---

# Kivori Release Gate

Runs after convergence. It establishes whether the repository is safe to commit, tag, and push — and refuses
history rewriting outright.

## Hard prohibitions

You **MUST NOT** force-push, amend, reset, rebase, squash, drop a tag, move an existing tag, or delete a
remote branch. You must not read `.env` or print a credential. If the only way forward appears to be a force
push, the answer is **BLOCKED**, not a force push.

## 1. Convergence really passed

Confirm convergence actually ran and reported success in this session, and that the evidence audit passed. A
convergence that was skipped, failed, or only assumed is **BLOCKED**.

## 2. Working tree and history

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse --verify HEAD 2>&1 | head -1
git status --short --branch
git log --oneline -5
git tag --points-at HEAD
```

Report the branch, HEAD, whether the tree is clean, and which tags point at HEAD.

## 3. Staged-content safety

```bash
git diff --cached --name-only | wc -l
git diff --cached --name-only | grep -icE '(^|/)\.env|wokwi-logs|\.vcd$|summary\.json|\.elf$|user\.tok'
git diff --cached | grep -acE 'wok_[A-Za-z0-9]{8,}|eyJ[A-Za-z0-9_-]{20,}'
git diff --check
```

Any credential-or-evidence path staged, or any token-shaped staged string, is a **FAIL**. When a literal
`TOKEN=` assignment appears, report file, line, and the value's *shape and length only* — never the value.

## 4. Committed history is clean

```bash
git grep -IE 'wok_[A-Za-z0-9]{8,}' HEAD -- >/dev/null 2>&1 && echo FOUND || echo none
git grep -IE 'eyJ[A-Za-z0-9_-]{20,}' HEAD -- >/dev/null 2>&1 && echo FOUND || echo none
```

A secret already committed is a **FAIL** and requires a user decision — never rewrite history to hide it.

## 5. Tags point where intended

Each tag under audit must resolve to the commit it is meant to mark. Verify by comparing
`git rev-parse <tag>` with the intended commit. Never move a tag to make this true.

## 6. Remote reconciliation

```bash
git remote -v
git ls-remote --heads origin 2>&1 | head
git ls-remote --tags origin 2>&1 | head
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>&1 | head -1
```

Classify the remote, and act only within the safe branch of the classification:

| Remote state | Safe action |
|---|---|
| no remote configured | report; nothing to push |
| remote empty (no heads) | `git push -u origin <branch>` and push the tag |
| remote head is an ancestor of local | fast-forward push is safe |
| remote has commits not in local | **do not force.** Push a new branch and report the divergence for a user decision |
| authentication fails | **BLOCKED** — report the exact user action |

Check authentication non-destructively (`git ls-remote`, or `gh auth status`) before attempting a push.
Report branch protection when discoverable.

## 7. Claim scoping

Confirm no commit message, tag annotation, or document about to be published claims physical hardware
validation that has not happened. Simulation must be described as simulation.

## 8. Final validation is green

Confirm the verification gate passed on the exact tree being released. A release gate over an unverified tree
is **BLOCKED**.

## Output

Each check with its command, exit code, and result; the remote classification; then:

```
RELEASE GATE: PASS | FAIL | BLOCKED
```

Commit and push may proceed **only** on PASS, and **only** when the invoking user request already granted
that permission. On FAIL or BLOCKED, state the exact user action required and stop.