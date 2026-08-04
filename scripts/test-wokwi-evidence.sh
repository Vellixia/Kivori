#!/usr/bin/env bash
# Unit tests for the Wokwi evidence helpers (scripts/lib/wokwi-evidence.sh).
#
# Runs offline with synthetic logs — no simulator, no token, no network. Covers the pass case and every
# failure case the gate must catch, plus the JSON field contract.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=lib/wokwi-evidence.sh
. "$ROOT/scripts/lib/wokwi-evidence.sh"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
pass=0
fail=0

ok() { echo "  ✓ $1"; pass=$((pass + 1)); }
bad() { echo "  ✗ $1"; fail=$((fail + 1)); }
check() { if [ "$2" = "$3" ]; then ok "$1"; else bad "$1 (expected '$3', got '$2')"; fi }

MARKER="KIVORI-EXT ALL PASS state-cycle"

printf 'ESP-ROM:esp32c3\nKIVORI-EXT READY\n%s\n' "$MARKER" > "$TMP/good.log"
printf 'ESP-ROM:esp32c3\nKIVORI-EXT READY\nKIVORI-EXT STATE current=happy\n' > "$TMP/no_marker.log"
printf 'ESP-ROM:esp32c3\n%s\npanicked at src/main.rs\n' "$MARKER" > "$TMP/panic.log"
printf 'ESP-ROM:esp32c3\n%s\nKIVORI-EXT FAIL sendable-guard\n' "$MARKER" > "$TMP/extfail.log"
printf 'ESP-ROM:esp32c3\n%s\nESP-ROM:esp32c3\n' "$MARKER" > "$TMP/reset.log"
printf 'binary\x00\x01bytes then %s\n' "$MARKER" > "$TMP/binary.log"
printf 'ESP-ROM:esp32c3\nKIVORI-SPI PASS bus-init\nKIVORI-SPI FAIL tile-bytes\n' > "$TMP/spifail.log"
printf 'ESP-ROM:esp32c3\nKIVORI-SPI PASS bus-init\nKIVORI-SPI ALL PASS\n' > "$TMP/spigood.log"

echo "── log scanning ──"
evidence_marker_present "$TMP/good.log" "$MARKER" && ok "marker found in a good log" || bad "marker found in a good log"
evidence_marker_present "$TMP/no_marker.log" "$MARKER" && bad "absent marker must not match" || ok "absent marker does not match"
evidence_marker_present "$TMP/binary.log" "$MARKER" && ok "marker found despite binary bytes" || bad "marker found despite binary bytes"
check "no failure signature in a good log" "$(evidence_failure_signature "$TMP/good.log")" ""
check "panic detected" "$(evidence_failure_signature "$TMP/panic.log")" "panicked"
check "KIVORI-EXT FAIL detected" "$(evidence_failure_signature "$TMP/extfail.log")" "KIVORI-EXT FAIL"
check "KIVORI-SPI FAIL detected" "$(evidence_failure_signature "$TMP/spifail.log")" "KIVORI-SPI FAIL"
check "clean SPI probe log has no signature" "$(evidence_failure_signature "$TMP/spigood.log")" ""
check "single boot banner counted" "$(evidence_boot_banners "$TMP/good.log")" "1"
check "reset loop counted" "$(evidence_boot_banners "$TMP/reset.log")" "2"

echo "── artifact identity ──"
printf 'fake elf bytes' > "$TMP/a.elf"
id_a="$(evidence_artifact_id "$TMP/a.elf")"
case "$id_a" in sha256:*) ok "artifact id is a sha256 prefix" ;; *) bad "artifact id is a sha256 prefix (got $id_a)" ;; esac
check "identical bytes give a stable id" "$(evidence_artifact_id "$TMP/a.elf")" "$id_a"
printf 'different bytes' > "$TMP/b.elf"
if [ "$(evidence_artifact_id "$TMP/b.elf")" != "$id_a" ]; then ok "different bytes give a different id"; else bad "different bytes give a different id"; fi
check "missing artifact reported" "$(evidence_artifact_id "$TMP/nope.elf")" "missing"

echo "── JSON rows: passed flag is derived, never asserted ──"
row_pass="$(evidence_json_row scen "$TMP/a.elf" "$id_a" 0 1234 "$TMP/good.log" "$MARKER")"
echo "$row_pass" | grep -q '"passed": true' && ok "clean run + exit 0 → passed true" || bad "clean run + exit 0 → passed true"
echo "$row_pass" | grep -q '"duration_ms": 1234' && ok "duration recorded" || bad "duration recorded"
echo "$row_pass" | grep -q '"exit_code": 0' && ok "exit code recorded" || bad "exit code recorded"

row_exit="$(evidence_json_row scen "$TMP/a.elf" "$id_a" 42 10 "$TMP/good.log" "$MARKER")"
echo "$row_exit" | grep -q '"passed": false' && ok "non-zero exit → passed false even with marker" || bad "non-zero exit → passed false even with marker"

row_missing="$(evidence_json_row scen "$TMP/a.elf" "$id_a" 0 10 "$TMP/no_marker.log" "$MARKER")"
echo "$row_missing" | grep -q '"success_marker_present": false' && ok "missing marker recorded" || bad "missing marker recorded"
echo "$row_missing" | grep -q '"passed": false' && ok "exit 0 without marker → passed false" || bad "exit 0 without marker → passed false"

row_panic="$(evidence_json_row scen "$TMP/a.elf" "$id_a" 0 10 "$TMP/panic.log" "$MARKER")"
echo "$row_panic" | grep -q '"failure_signature": "panicked"' && ok "panic signature recorded" || bad "panic signature recorded"
echo "$row_panic" | grep -q '"passed": false' && ok "panic → passed false" || bad "panic → passed false"

row_reset="$(evidence_json_row scen "$TMP/a.elf" "$id_a" 0 10 "$TMP/reset.log" "$MARKER")"
echo "$row_reset" | grep -q '"boot_banner_count": 2' && ok "reset loop count recorded" || bad "reset loop count recorded"
echo "$row_reset" | grep -q '"passed": false' && ok "reset loop → passed false" || bad "reset loop → passed false"

echo "── VCD result gates the row ──"
SPI_MARKER="KIVORI-SPI ALL PASS"
row_vcd_none="$(evidence_json_row spi "$TMP/a.elf" "$id_a" 0 10 "$TMP/spigood.log" "$SPI_MARKER")"
echo "$row_vcd_none" | grep -q '"vcd_result": "none"' && ok "vcd_result defaults to none" || bad "vcd_result defaults to none"
echo "$row_vcd_none" | grep -q '"passed": true' && ok "a scenario with no VCD still passes on its log" || bad "a scenario with no VCD still passes on its log"

row_vcd_pass="$(evidence_json_row spi "$TMP/a.elf" "$id_a" 0 10 "$TMP/spigood.log" "$SPI_MARKER" "$TMP/x.vcd" pass)"
echo "$row_vcd_pass" | grep -q '"vcd_path": "'"$TMP"'/x.vcd"' && ok "vcd path recorded" || bad "vcd path recorded"
echo "$row_vcd_pass" | grep -q '"passed": true' && ok "green log + green VCD → passed true" || bad "green log + green VCD → passed true"

row_vcd_fail="$(evidence_json_row spi "$TMP/a.elf" "$id_a" 0 10 "$TMP/spigood.log" "$SPI_MARKER" "$TMP/x.vcd" fail)"
echo "$row_vcd_fail" | grep -q '"passed": false' && ok "a failed VCD check → passed false despite a green log" || bad "a failed VCD check → passed false despite a green log"

row_spi_fail="$(evidence_json_row spi "$TMP/a.elf" "$id_a" 0 10 "$TMP/spifail.log" "$SPI_MARKER")"
echo "$row_spi_fail" | grep -q '"passed": false' && ok "KIVORI-SPI FAIL → passed false" || bad "KIVORI-SPI FAIL → passed false"

echo "── summary document ──"
printf '%s,\n%s' "$row_pass" "$row_panic" > "$TMP/rows"
evidence_write_summary "$TMP/summary.json" "0.26.1 (abc)" "1.0.0-x" "$TMP/rows"
python3 - "$TMP/summary.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
assert doc["tool"] == "wokwi-cli", doc
assert doc["cli_version"].startswith("0.26.1")
assert len(doc["scenarios"]) == 2, doc
first = doc["scenarios"][0]
for field in ["name", "elf_path", "elf_id", "exit_code", "duration_ms", "log_path",
              "success_marker", "success_marker_present", "failure_signature",
              "failure_signature_present", "boot_banner_count", "vcd_path", "vcd_result",
              "passed"]:
    assert field in first, f"missing field {field}"
assert first["passed"] is True and doc["scenarios"][1]["passed"] is False
print("  ✓ summary.json is valid JSON with the full deterministic field set")
PY
[ $? -eq 0 ] && pass=$((pass + 1)) || fail=$((fail + 1))

echo "── no credential material may appear ──"
if grep -aqiE "wok_[A-Za-z0-9]|WOKWI_CLI_TOKEN" "$TMP/summary.json"; then
  bad "summary contains no token material"
else
  ok "summary contains no token material"
fi

echo
echo "evidence tests: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
