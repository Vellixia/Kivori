#!/usr/bin/env bash
# Direct-dependency firewall (ADR-0001 / plan.md "Direct-dependency firewall"; T051 runtime-SVG guard).
#
# Fails if any first-party no_std shared crate (crates/kivori-*) declares a prohibited crate as a
# DIRECT dependency. Covers desktop/std crates AND runtime SVG/raster crates (Principle XI: source
# SVG/PNG is never parsed at runtime). Inspects DIRECT dependencies only (via `cargo metadata
# --no-deps`); it does NOT reject legitimate transitive crates. The RISC-V `no_std` compile is the
# hard backstop — resvg/usvg/image are std-only and cannot link into the firmware build.
set -euo pipefail

BANNED=(tauri tokio serialport tokio-serial axum tower sqlx resvg usvg tiny-skia image png)
SHARED=(kivori-model kivori-protocol kivori-framebuffer kivori-renderer kivori-assets)

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required for the crate-boundary check" >&2
  exit 2
fi

meta="$(cargo metadata --no-deps --format-version 1)"
fail=0

for crate in "${SHARED[@]}"; do
  deps="$(printf '%s' "$meta" | jq -r --arg n "$crate" \
    '.packages[] | select(.name==$n) | .dependencies[].name' 2>/dev/null || true)"
  for banned in "${BANNED[@]}"; do
    if printf '%s\n' "$deps" | grep -qx "$banned"; then
      echo "FIREWALL VIOLATION: '$crate' directly depends on prohibited crate '$banned'"
      fail=1
    fi
  done
done

if [ "$fail" -ne 0 ]; then
  echo "Direct-dependency firewall check FAILED (see violations above)." >&2
  exit 1
fi

echo "Direct-dependency firewall OK: no crates/kivori-* directly depends on a prohibited crate."
echo "  (checked crates: ${SHARED[*]})"
echo "  (banned direct deps: ${BANNED[*]}, plus OS-specific desktop crates by review)"
