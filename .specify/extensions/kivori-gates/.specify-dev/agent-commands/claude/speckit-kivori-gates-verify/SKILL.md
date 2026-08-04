---
name: speckit-kivori-gates-verify
description: Mandatory post-implementation validation gate across Rust, firmware,
  frontend, guards, and simulation tooling
compatibility: Requires spec-kit project structure with .specify/ directory
metadata:
  author: github-spec-kit
  source: kivori-gates:commands/speckit.kivori-gates.verify.md
---

# Kivori Verification Gate

The mandatory gate after implementation. It runs the project's real checks and **never hides a failure
behind a warning**.

> **Repository script paths.** The extension installer rewrites any literal `scripts` + `/` token in a
> command body to an extension-local path that does not exist. Repo scripts are therefore reached through
> `$SCRIPTS`, assembled once below so the literal never appears again in this file.

```bash
REPO="$(git rev-parse --show-toplevel)"
SCRIPTS="$REPO/scripts"
```

## Hard prohibitions

Do not read or `source` `.env`. Do not print a credential. Do not run `env`/`printenv`/`set`/`export -p`.
Do not mark any task complete. Do not commit or push. Do not overwrite existing simulation evidence.

## Scope selection

Determine changed paths (`git status --short` plus `git diff --name-only HEAD` when a commit exists) and run
every group whose paths were touched. When in doubt, run the group. Groups may be disabled only by
`.specify/kivori-gates.yml`, and a disabled group must be reported as an explicit skip with its reason.

## Group A — host Rust workspace

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Group B — firmware (isolated workspace, ADR-0001)

```bash
cd firmware/esp32-c3 && cargo fmt --all --check
cd firmware/esp32-c3 && cargo build --release --lib                      # production riscv no_std
cd firmware/esp32-c3 && cargo build --release --features embedded        # hardware adapters, compile-only
cd firmware/esp32-c3 && cargo clippy --lib -- -D warnings
cd firmware/esp32-c3 && cargo clippy --lib --features embedded -- -D warnings
```

Also lint every other feature configuration the crate declares (for example `wokwi`, `wokwi-serial`,
`wokwi-spi`, `wokwi-runtime`, `embedded,debug-payloads`) — enumerate them from `Cargo.toml` rather than a
fixed list, so a new feature cannot escape the gate.

Host-side firmware tests, including the display-adapter and production-runtime suites:

```bash
cd firmware/esp32-c3 && cargo test --features host-sim --target "$(rustc -vV | sed -n 's/^host: //p')"
```

Report each test binary and its count separately, so a missing suite is visible.

## Group C — simulation tooling (no token, no simulator)

```bash
cargo test -p kivori-wokwi-vectors      # vectors + scenario structure
cargo test -p kivori-wokwi-vcd          # VCD parser/checker, with negative controls
bash "$SCRIPTS"/test-wokwi-evidence.sh     # evidence writer
```

Protocol-vector drift — regenerate into a temporary directory and diff, so the check works even in a
repository with uncommitted generated files:

```bash
cargo run -q -p kivori-wokwi-vectors -- "$(mktemp -d)/gen"
# then diff that directory against sim/wokwi/generated
```

Any drift is a **FAIL**.

## Group D — guards

```bash
bash "$SCRIPTS"/check-release-surface.sh   # includes positive controls
bash "$SCRIPTS"/check-crate-boundaries.sh
bash "$SCRIPTS"/check-offline-deps.sh
node "$SCRIPTS"/check-mock-excluded.mjs
node "$SCRIPTS"/check-frontend-offline.mjs
```

The release-surface guard must report its positive controls. A guard that passes with a vacuous check is a
**FAIL**: grep targets must be string literals or concrete type names, never module paths — rustc records
`cfg`-disabled module names in rlib metadata, so a module name proves nothing.

## Group E — frontend

```bash
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm exec prettier --check .
```

## Group F — hygiene

```bash
git diff --check
for f in "$SCRIPTS"/*.sh "$SCRIPTS"/lib/*.sh; do bash -n "$f"; done
git diff --cached --name-only | grep -icE '(^|/)\.env|wokwi-logs|\.vcd$|summary\.json|\.elf$|user\.tok'
git diff --cached | grep -acE 'wok_[A-Za-z0-9]{8,}|eyJ[A-Za-z0-9_-]{20,}'
```

Any staged credential-or-evidence artifact, or any token-shaped staged string, is a **FAIL**. Report counts
only.

## Authenticated Wokwi runs

Off by default. Run them **only** when all of these hold:

1. `allow_authenticated_wokwi` is true in `.specify/kivori-gates.yml`, **and**
2. `WOKWI_CLI_TOKEN` is already present in the inherited process environment, **and**
3. that token was never exposed in any transcript or file.

Then:

- never read it from `.env`;
- report availability as a boolean only, never the value;
- write into the durable evidence directories the gate script owns, and use `WOKWI_LOGS_DIR`/`WOKWI_VCD_DIR`
  overrides for any stub or control-flow test so real evidence is never overwritten;
- fail closed — a missing marker, a failure signature, more than one boot banner, or a failed VCD check
  fails the gate.

When no fresh token is available, report the Wokwi group as **SKIPPED — no fresh token in the process
environment**, and state plainly that execution-dependent tasks must stay unchecked. Never mark such a task
complete without a real run.

## Output

For every command: the exact command line, its exit code, and a concise result. Then list skipped checks with
the exact reason. Then a final line:

```
VERIFY: PASS | FAIL | BLOCKED
```

`FAIL` when any check failed. `BLOCKED` only when a required tool is unavailable — name it. Never emit PASS
while a check failed or was silently skipped.