#!/usr/bin/env bash
# Durable evidence helpers for the Wokwi gate.
#
# Terminal output disappears; CI forensics need machine-readable facts. These helpers capture, per
# scenario: the real exit code, wall-clock duration, the artifact identity actually simulated, and the
# result of scanning the captured serial log. They are pure functions over files + arguments so they can
# be unit-tested without a simulator (see scripts/test-wokwi-evidence.sh).
#
# No credential is ever read, accepted, or emitted here.

# Milliseconds since the epoch (macOS `date` has no %N, so use python3 which is already required).
evidence_now_ms() {
  python3 -c 'import time; print(int(time.time() * 1000))'
}

# evidence_artifact_id <path> → "sha256:<first 16 hex>" identifying the exact bytes simulated, or
# "missing" when the artifact is absent. Pins artifact identity at run time so a later rebuild cannot
# retroactively blur which binary produced a log.
evidence_artifact_id() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "missing"
    return 0
  fi
  local sum
  if command -v shasum >/dev/null 2>&1; then
    sum=$(shasum -a 256 "$path" | awk '{print $1}')
  else
    sum=$(sha256sum "$path" | awk '{print $1}')
  fi
  echo "sha256:${sum:0:16}"
}

# evidence_marker_present <log> <marker> → 0 if the marker appears in the log.
evidence_marker_present() {
  grep -aqF -- "$2" "$1" 2>/dev/null
}

# evidence_failure_signature <log> → prints the first failure signature found, or empty.
evidence_failure_signature() {
  local log="$1" sig
  for sig in "panicked" "KIVORI-EXT FAIL" "KIVORI-SIM FAIL" "KIVORI-SIM ALL FAIL" \
             "KIVORI-SPI FAIL" "KIVORI-SPI ALL FAIL"; do
    if grep -aqF -- "$sig" "$log" 2>/dev/null; then
      printf '%s' "$sig"
      return 0
    fi
  done
  printf ''
}

# evidence_boot_banners <log> → count of ESP boot banners; >1 means the device reset mid-run.
evidence_boot_banners() {
  local n
  n=$(grep -acF 'ESP-ROM:' "$1" 2>/dev/null)
  echo "${n:-0}"
}

# Minimal JSON string escaping for the few fields that are not numbers/booleans.
evidence_json_escape() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

# evidence_json_row <name> <elf_path> <elf_id> <exit> <duration_ms> <log> <marker> [vcd_path] [vcd_result]
#
# Derives marker/failure/boot facts from the log itself, so a row can never claim a pass the log does not
# support. Field order is fixed for deterministic output.
#
# `vcd_result` is one of `pass`, `fail`, or `none` (the default, for scenarios that capture no VCD). A
# `fail` makes the row fail, so a broken logic-analyzer capture cannot be hidden behind a green serial log.
evidence_json_row() {
  local name="$1" elf_path="$2" elf_id="$3" code="$4" ms="$5" log="$6" marker="$7"
  local vcd_path="${8:-}" vcd_result="${9:-none}"
  local present="false" failure boots
  evidence_marker_present "$log" "$marker" && present="true"
  failure=$(evidence_failure_signature "$log")
  boots=$(evidence_boot_banners "$log")
  local failure_present="false"
  [ -n "$failure" ] && failure_present="true"
  printf '    {\n'
  printf '      "name": "%s",\n' "$(evidence_json_escape "$name")"
  printf '      "elf_path": "%s",\n' "$(evidence_json_escape "$elf_path")"
  printf '      "elf_id": "%s",\n' "$(evidence_json_escape "$elf_id")"
  printf '      "exit_code": %d,\n' "$code"
  printf '      "duration_ms": %d,\n' "$ms"
  printf '      "log_path": "%s",\n' "$(evidence_json_escape "$log")"
  printf '      "success_marker": "%s",\n' "$(evidence_json_escape "$marker")"
  printf '      "success_marker_present": %s,\n' "$present"
  printf '      "failure_signature": "%s",\n' "$(evidence_json_escape "$failure")"
  printf '      "failure_signature_present": %s,\n' "$failure_present"
  printf '      "boot_banner_count": %d,\n' "$boots"
  printf '      "vcd_path": "%s",\n' "$(evidence_json_escape "$vcd_path")"
  printf '      "vcd_result": "%s",\n' "$(evidence_json_escape "$vcd_result")"
  printf '      "passed": %s\n' \
    "$([ "$code" -eq 0 ] && [ "$present" = "true" ] && [ "$failure_present" = "false" ] && [ "$boots" -le 1 ] && [ "$vcd_result" != "fail" ] && echo true || echo false)"
  printf '    }'
}

# evidence_write_summary <outfile> <cli_version> <api_version> <rows_file>
#
# `rows_file` holds comma-separated row objects produced by evidence_json_row.
evidence_write_summary() {
  local out="$1" cli="$2" api="$3" rows_file="$4"
  mkdir -p "$(dirname "$out")"
  {
    printf '{\n'
    printf '  "tool": "wokwi-cli",\n'
    printf '  "cli_version": "%s",\n' "$(evidence_json_escape "$cli")"
    printf '  "api_version": "%s",\n' "$(evidence_json_escape "$api")"
    printf '  "scenarios": [\n'
    cat "$rows_file"
    printf '\n  ]\n'
    printf '}\n'
  } > "$out"
}
