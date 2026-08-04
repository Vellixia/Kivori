# Kivori Quality Gates — Spec Kit extension

Fail-closed Spec Kit gates for Kivori. Five namespaced commands, registered as mandatory hooks around the
task, implement, and converge workflows.

Verified against **Spec Kit 0.13.0** (`specify --version`), integration **claude** in skills mode
(`.specify/integration.json` → `"integration": "claude"`, `"invoke_separator": "-"`).

## Why each gate exists

| Command | Hook event | Purpose |
|---|---|---|
| `speckit.kivori-gates.task-audit` | `after_tasks` | Task IDs, ordering, evidence references, simulation-vs-physical separation |
| `speckit.kivori-gates.preflight` | `before_implement` | Artifacts, checklists, credential/evidence hygiene, hardware-fact honesty |
| `speckit.kivori-gates.verify` | `after_implement` | The full validation battery; never hides a failure |
| `speckit.kivori-gates.evidence-audit` | `before_converge` | Does each checked task's evidence match its literal wording? |
| `speckit.kivori-gates.release-gate` | `after_converge` | Tree, staged content, secrets, tags, remote — refuses history rewriting |

All five are **mandatory** (`optional: false`) and carry **no `condition`**.

## Installed-version compatibility findings

These were read from the installed CLI source, not from documentation:

1. **Hook events are read by the command templates**, not by a runtime engine. `core_pack/commands/*.md`
   each read their own `before_*`/`after_*` key from `.specify/extensions.yml`.
2. **`converge` does read hooks.** `core_pack/commands/converge.md` reads `before_converge` and
   `after_converge`, so no project-local override or wrapper is needed.
3. **Mandatory hooks are surfaced with a specific marker.** For `optional: false` the template emits
   `**Automatic Pre-Hook**: {extension}` plus `EXECUTE_COMMAND: {command}`, and the agent must then actually
   invoke the command. Emitting the block alone does not run the hook.
4. **A non-empty `condition` is skipped.** The templates explicitly refuse to evaluate conditions and defer
   to `HookExecutor`, so a conditional hook silently does not run. This extension therefore never sets one.
   The installer writes `condition: null`, which the templates treat as executable.
5. **Command names must be `speckit.<extension-id>.<command>`.** `specify_cli/extensions/__init__.py`
   raises *"must use extension namespace"* for anything else. The requested `speckit.kivori.*` names would be
   rejected with id `kivori-gates`, so commands are namespaced `speckit.kivori-gates.*`.
6. **Priority does not control presentation order.** Templates surface entries in file order, so
   `extension.yml` lists events in the intended execution order and the installer preserves it.
7. **The installer rewrites script paths.** Any literal `scripts` + `/` token in a command body becomes
   `.specify/extensions/<id>/scripts/…`, which does not exist here. Repo scripts are therefore reached via
   `SCRIPTS="$REPO/scripts"`, assembled once per command so the literal never appears again. The validator
   asserts this.
8. `settings.auto_execute_hooks: true` is written by the installer but is **not** relied upon — the proof of
   a mandatory hook is the template's own marker plus an actual invocation.

## Developer commands

Install or reinstall from the version-controlled source (idempotent):

```bash
specify extension add tools/spec-kit-extensions/kivori-gates --dev --force
```

Inspect:

```bash
specify extension list
specify extension info kivori-gates
```

Enable / disable without removing:

```bash
specify extension disable kivori-gates
specify extension enable kivori-gates
```

Run a gate manually (skills-mode claude invokes the registered command directly):

```text
/speckit-kivori-gates-preflight
/speckit-kivori-gates-task-audit
/speckit-kivori-gates-verify
/speckit-kivori-gates-evidence-audit
/speckit-kivori-gates-release-gate
```

Validate the extension itself — manifest, command registration, hook registry, and credential hygiene:

```bash
~/.local/share/uv/tools/specify-cli/bin/python tools/spec-kit-extensions/kivori-gates/tests/test_extension.py
```

Uninstall:

```bash
specify extension remove kivori-gates
```

## Diagnosing a hook that did not run

1. `test -f .specify/extensions.yml` — no file means no hooks, and the templates skip silently.
2. Confirm the event key exists and the entry is `enabled: true` with `optional: false`.
3. Confirm `condition` is absent or null. A non-empty condition is skipped by design.
4. Confirm the command is registered for the active agent: `ls .claude/skills | grep kivori-gates`.
5. Re-register after changing the manifest — editing the source alone does not update the agent:
   `specify extension add tools/spec-kit-extensions/kivori-gates --dev --force`.
6. Run the validator above; it checks every one of these mechanically.

## Configuration

`config-template.yml` is copied to `.specify/kivori-gates.yml`. Every group defaults to on. Authenticated
Wokwi execution is **off** by default and, when enabled, reads `WOKWI_CLI_TOKEN` from the inherited process
environment only — never from `.env`, and never printed. Disabling a group must be reported by the verify
gate as an explicit skip with its reason.

## Non-goals

No upstream Spec Kit core file is patched. No generated agent command is hand-edited — registration is the
installer's job. No gate stores or echoes a credential.
