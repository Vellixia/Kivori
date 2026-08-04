# ADR-0001: Two-workspace Cargo split (host root + isolated firmware embedded workspace)

**Status**: Accepted · **Date**: 2026-07-17 · **Feature**: 001-device-connection-foundation

## Context

Kivori is a monorepo containing a `std` host side (Tauri desktop core, asset tooling, tests) and a
`no_std` embedded side (ESP32-C3 firmware, RISC-V target `riscv32imc-unknown-none-elf`). Both sides
consume the same shared crates (`kivori-model`, `kivori-protocol`, `kivori-framebuffer`,
`kivori-renderer`, `kivori-assets`).

A single Cargo workspace **unifies feature flags across all members** and shares one profile set and
target directory. If the host desktop crate enabled a `std`/`alloc` feature on a shared crate, Cargo's
feature unification would turn that feature on for the firmware build too, silently breaking the
firmware's `no_std` guarantee. The embedded target also needs its own `panic` strategy, target triple,
and `.cargo/config.toml` runner (`espflash`), which do not belong in the host workspace.

## Decision

Use **two Cargo workspaces**:

- **Root workspace** (`/Cargo.toml`) — host targets only: `crates/*`, `apps/desktop/src-tauri`,
  `tools/asset-compiler`, `tests/golden-frames`.
- **Firmware workspace** (`/firmware/esp32-c3/Cargo.toml`) — its own `[workspace]`, isolated from the
  root, consuming the shared crates through **path dependencies**.

This forms a hard **feature-unification firewall**: the firmware workspace resolves features
independently of the host workspace. Path dependencies keep a single source of truth for the shared
crates with no publishing.

A lightweight architecture check (`scripts/check-crate-boundaries.sh`) additionally forbids the shared
`crates/kivori-*` from declaring `tauri`, `tokio`, `serialport`, `tokio-serial`, `axum`, `tower`,
`sqlx`, or OS-specific desktop crates as **direct** dependencies. CI also compiles the shared crates for
the RISC-V target to prove `no_std` cleanliness.

## Alternatives considered

- **Single root workspace containing firmware** — simpler (one lockfile), but feature unification across
  a `std` host and a `no_std` target is the exact hazard we must avoid. Rejected.
- **Independent per-crate packages without a workspace** — loses the shared lockfile and unified tooling
  for the host side. Unnecessary.

## Consequences

- Two lockfiles (`/Cargo.lock`, `/firmware/esp32-c3/Cargo.lock`) and two target dirs.
- Shared-crate changes are picked up by both workspaces via path deps with no release step.
- CI runs a host job (root workspace) and a firmware job (embedded workspace + shared-crate riscv
  compile). The firmware binary itself is built once its runtime lands (Phase 8); until then the
  isolation proof is the shared-crate riscv compile.
