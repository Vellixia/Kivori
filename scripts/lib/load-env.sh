#!/usr/bin/env bash
# Loads WOKWI_CLI_TOKEN from a local, gitignored .env when it is not already in the environment.
#
# Why this exists: a GUI-launched process (the Claude desktop app, an IDE) does not inherit a terminal's
# exports, so `export WOKWI_CLI_TOKEN=…` in a shell never reaches it. A local .env is the workaround.
#
# Deliberate properties:
#
#   * The REAL environment always wins. The file is consulted only when the variable is unset or empty, so
#     a stray local .env can never override a CI secret.
#   * Only WOKWI_CLI_TOKEN is read. The file is NOT sourced — sourcing would execute whatever is in it,
#     which is a code-execution path for a file that holds a credential.
#   * The value is never printed, logged, echoed, or passed as an argument. This file has no code path that
#     emits it. Warnings describe the file, never its contents.

# evidence_env_file → path of the .env this repository uses.
wokwi_env_file() {
  # ${BASH_SOURCE[0]} is unset under zsh, so fall back to $0 and finally to the working directory. Without
  # this, sourcing from a non-bash shell resolved the wrong path and silently found no token.
  local self="${BASH_SOURCE[0]:-${(%):-%x}}"
  local root
  if [ -n "$self" ] && [ -f "$self" ]; then
    root="$(cd "$(dirname "$self")/../.." && pwd)"
  else
    root="$(pwd)"
  fi
  echo "$root/.env"
}

# wokwi_load_token [env_file] → exports WOKWI_CLI_TOKEN if the file supplies one. Always returns 0; the
# caller's own preflight decides what a missing token means.
wokwi_load_token() {
  local file="${1:-$(wokwi_env_file)}"

  # Already provided by the environment (CI secret, or a terminal export): leave it alone.
  if [ -n "${WOKWI_CLI_TOKEN:-}" ]; then
    return 0
  fi
  [ -f "$file" ] || return 0

  # Nudge toward tight permissions, without ever looking at the contents.
  if [ "$(uname)" = "Darwin" ]; then
    local mode
    mode=$(stat -f '%OLp' "$file" 2>/dev/null || echo "")
    if [ -n "$mode" ] && [ "$mode" != "600" ]; then
      echo "note: $file holds a credential and is mode $mode — consider: chmod 600 .env" >&2
    fi
  fi

  # Take the last NON-EMPTY assignment: a blanked line left over from a previous session must not win over
  # a real value added above it. Tolerates a leading `export` and one layer of matching quotes.
  local line value
  line=$(grep -aE '^[[:space:]]*(export[[:space:]]+)?WOKWI_CLI_TOKEN[[:space:]]*=[[:space:]]*[^[:space:]]' \
    "$file" 2>/dev/null | tail -1)
  [ -n "$line" ] || return 0
  value=${line#*=}
  # Trim surrounding whitespace, then one matching quote pair.
  value=$(printf '%s' "$value" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
  case "$value" in
    \"*\") value=${value#\"}; value=${value%\"} ;;
    \'*\') value=${value#\'}; value=${value%\'} ;;
  esac

  [ -n "$value" ] || return 0
  export WOKWI_CLI_TOKEN="$value"
  # Presence only — never the value, never a prefix, never a length.
  echo "Using WOKWI_CLI_TOKEN from $file (value not shown)."
}
