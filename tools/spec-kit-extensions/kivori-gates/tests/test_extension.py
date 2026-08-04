#!/usr/bin/env python3
"""Validators for the kivori-gates Spec Kit extension.

Checks the manifest, the registered commands, and `.specify/extensions.yml` against what the *installed*
Spec Kit actually requires — the rules are mirrored from
`specify_cli/extensions/__init__.py` (schema 1.0) and `core_pack/commands/*.md`, so this suite fails if the
extension drifts from the installed contract.

Run with the CLI's own interpreter (it already has PyYAML):

    ~/.local/share/uv/tools/specify-cli/bin/python tools/spec-kit-extensions/kivori-gates/tests/test_extension.py

Exit status 0 = all checks passed. No credential is read, printed, or required.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml

# ── contract constants, mirrored from the installed Spec Kit ──────────────────────────────────────────
SCHEMA_VERSION = "1.0"
REQUIRED_MANIFEST_FIELDS = ("schema_version", "extension", "requires", "provides")
REQUIRED_EXTENSION_FIELDS = ("id", "name", "version", "description")
COMMAND_NAME_PATTERN = re.compile(r"^speckit\.([a-z0-9-]+)\.([a-z0-9-]+)$")
VALID_EFFECTS = {"read-only", "read-write"}
EXT_ID = "kivori-gates"

# Events the installed core command templates genuinely read, and the order we want preserved in the file
# (templates surface entries in file order, so this is part of the contract).
EXPECTED_HOOK_ORDER = [
    "after_tasks",
    "before_implement",
    "after_implement",
    "before_converge",
    "after_converge",
]

# The rewrite trap: the installer replaces a literal repo-relative script token in a command body with an
# extension-local path that does not exist.
BAD_SCRIPT_PATH = f".specify/extensions/{EXT_ID}/scripts/"

# Anything that would make a gate read a secret.
CREDENTIAL_SMELLS = ("printenv", "export -p", "$WOKWI_CLI_TOKEN", "source .env", ". .env", "cat .env")

FAILURES: list[str] = []
CHECKS = 0


def check(condition: bool, message: str) -> None:
    global CHECKS
    CHECKS += 1
    if not condition:
        FAILURES.append(message)


def bash_blocks(markdown: str) -> list[str]:
    """Fenced ```bash blocks — the only part of a command file that is meant to be executed."""
    blocks, current, inside = [], [], False
    for line in markdown.splitlines():
        stripped = line.strip()
        if stripped.startswith("```"):
            if inside:
                blocks.append("\n".join(current))
                current, inside = [], False
            elif stripped in ("```bash", "```sh", "```shell"):
                inside = True
            continue
        if inside:
            current.append(line)
    return blocks


def repo_root() -> Path:
    return Path(__file__).resolve().parents[4]


def ext_source() -> Path:
    return Path(__file__).resolve().parents[1]


def main() -> int:
    root = repo_root()
    src = ext_source()

    # ── manifest ─────────────────────────────────────────────────────────────────────────────────────
    manifest_path = src / "extension.yml"
    check(manifest_path.is_file(), "extension.yml missing")
    manifest = yaml.safe_load(manifest_path.read_text())
    check(isinstance(manifest, dict), "manifest must be a mapping")

    for field in REQUIRED_MANIFEST_FIELDS:
        check(field in manifest, f"manifest missing required field: {field}")
    check(manifest.get("schema_version") == SCHEMA_VERSION, "schema_version must be exactly '1.0'")

    ext = manifest.get("extension", {})
    for field in REQUIRED_EXTENSION_FIELDS:
        check(field in ext, f"manifest missing extension.{field}")
    check(ext.get("id") == EXT_ID, f"extension.id must be '{EXT_ID}'")
    check(re.match(r"^[a-z0-9-]+$", ext.get("id", "")) is not None, "extension.id must be lowercase/hyphen")
    check(re.match(r"^\d+\.\d+\.\d+", str(ext.get("version", ""))) is not None, "extension.version must be semantic")
    if "effect" in ext:
        check(ext["effect"] in VALID_EFFECTS, f"extension.effect must be one of {sorted(VALID_EFFECTS)}")
    check("speckit_version" in manifest.get("requires", {}), "manifest missing requires.speckit_version")

    # ── commands ─────────────────────────────────────────────────────────────────────────────────────
    commands = manifest.get("provides", {}).get("commands", [])
    check(isinstance(commands, list) and bool(commands), "provides.commands must be a non-empty list")
    declared: list[str] = []
    for cmd in commands:
        name, rel = cmd.get("name"), cmd.get("file")
        declared.append(name)
        check(bool(name) and bool(rel), "each command needs 'name' and 'file'")
        match = COMMAND_NAME_PATTERN.match(name or "")
        check(match is not None, f"command '{name}' must match speckit.<ext>.<cmd>")
        if match:
            # The installer rejects any namespace that is not the extension id.
            check(match.group(1) == EXT_ID, f"command '{name}' must use namespace '{EXT_ID}'")
        target = src / (rel or "")
        check(target.is_file(), f"command file missing: {rel}")
        if target.is_file():
            body = target.read_text()
            check(body.startswith("---"), f"{rel} must start with YAML front matter")
            check("description:" in body.split("---")[1], f"{rel} front matter needs a description")
            check(
                BAD_SCRIPT_PATH not in body,
                f"{rel} contains the installer-rewritten script path; reach repo scripts via $SCRIPTS",
            )
            # Only *executable* blocks matter: the commands deliberately name these in prose to
            # forbid them, and a prohibition must not be mistaken for an invocation.
            executable = "\n".join(bash_blocks(body))
            for smell in CREDENTIAL_SMELLS:
                check(
                    smell not in executable,
                    f"{rel} must not load or dump credentials in an executable block (found: {smell})",
                )
            check(
                "Do not read" in body or "MUST NOT" in body or "never read" in body.lower(),
                f"{rel} must state its credential prohibitions explicitly",
            )
            check(
                "/Users/" not in body and "/home/" not in body,
                f"{rel} must not contain a machine-specific absolute path",
            )

    # ── hooks in the manifest ────────────────────────────────────────────────────────────────────────
    hooks = manifest.get("hooks", {})
    check(isinstance(hooks, dict) and bool(hooks), "manifest must declare hooks")
    check(list(hooks.keys()) == EXPECTED_HOOK_ORDER, f"manifest hook order must be {EXPECTED_HOOK_ORDER}")
    for event, entry in hooks.items():
        check(isinstance(entry, dict), f"hook '{event}' must be a mapping")
        check(bool(entry.get("command")), f"hook '{event}' missing 'command'")
        check(entry.get("command") in declared, f"hook '{event}' references an undeclared command")
        check(entry.get("optional") is False, f"hook '{event}' must be mandatory (optional: false)")
        priority = entry.get("priority", 10)
        check(isinstance(priority, int) and not isinstance(priority, bool) and priority >= 1,
              f"hook '{event}' priority must be an int >= 1")
        # implement.md skips any hook with a non-empty condition and defers to HookExecutor.
        check(not entry.get("condition"), f"hook '{event}' must not declare a condition")

    # ── the installed registry ───────────────────────────────────────────────────────────────────────
    registry_path = root / ".specify" / "extensions.yml"
    if not registry_path.is_file():
        FAILURES.append(".specify/extensions.yml missing — extension not installed")
    else:
        registry = yaml.safe_load(registry_path.read_text())
        check(EXT_ID in (registry.get("installed") or []), f"'{EXT_ID}' not in extensions.yml installed list")
        reg_hooks = registry.get("hooks") or {}
        check(
            [e for e in reg_hooks if any(x.get("extension") == EXT_ID for x in reg_hooks[e])]
            == EXPECTED_HOOK_ORDER,
            f"registered hook order must be {EXPECTED_HOOK_ORDER}",
        )
        for event in EXPECTED_HOOK_ORDER:
            entries = [x for x in (reg_hooks.get(event) or []) if x.get("extension") == EXT_ID]
            check(len(entries) == 1, f"event '{event}' must have exactly one {EXT_ID} entry (duplicates?)")
            if entries:
                e = entries[0]
                check(e.get("enabled") is True, f"registered hook '{event}' must be enabled")
                check(e.get("optional") is False, f"registered hook '{event}' must be mandatory")
                check(not e.get("condition"), f"registered hook '{event}' must have no non-empty condition")

    # ── registered agent commands (claude integration, skills mode) ──────────────────────────────────
    skills_dir = root / ".claude" / "skills"
    if skills_dir.is_dir():
        for name in declared:
            slug = name.replace("speckit.", "speckit-").replace(".", "-")
            skill = skills_dir / slug / "SKILL.md"
            check(skill.is_file(), f"command '{name}' is not registered for the active agent ({slug})")
            if skill.is_file():
                text = skill.read_text()
                check(BAD_SCRIPT_PATH not in text, f"registered {slug} points at a nonexistent script path")
                check(f"{EXT_ID}:commands/" in text, f"registered {slug} lost its extension source reference")

    print(f"kivori-gates extension validators: {CHECKS - len(FAILURES)}/{CHECKS} passed")
    for failure in FAILURES:
        print(f"  FAIL: {failure}")
    return 1 if FAILURES else 0


if __name__ == "__main__":
    sys.exit(main())
