# ADR-0002: Device USB-serial wire protocol

**Status**: Accepted · **Date**: 2026-07-17 · **Feature**: 001-device-connection-foundation

## Context

The desktop and the ESP32-C3 exchange small, semantic messages over a USB Serial/JTAG byte stream
(64-byte FIFO). We need robust framing over an unreliable-boundaries byte stream, integrity checking,
version compatibility, and forward-compatible evolution — all `no_std` on the device.

## Decision

The single shared crate `kivori-protocol` defines the wire contract (see
[contracts/protocol.md](../../specs/001-device-connection-foundation/contracts/protocol.md)).

- **Framing**: COBS with a `0x00` delimiter (self-synchronizing; resynchronizes after corruption).
  One wire packet = `COBS(frame) || 0x00`.
- **Frame** (little-endian): `magic:u16 (0x4B56)` · `ver_major:u16` · `ver_minor:u16` · `seq:u16` ·
  `payload_len:u16` · `payload:[u8; payload_len]` · `crc32:u32` (CRC-32/IEEE over header+payload).
  `payload_len <= MAX_PAYLOAD (512)`.
- **Payloads**: `serde` + `postcard` (stable wire format, `no_std`, no alloc). The top-level `Message`
  is an **append-only enum** (the postcard variant index is the wire tag); new variants/fields are
  appended within a major version and older peers ignore unnegotiated features.
- **Version compatibility**: compatible iff **major matches** a supported major (data-model §3);
  minor differences are backward-compatible and capability-negotiated (`min` minor + capability
  intersection).
- **Sequence policy**: `seq` is a `u16` correlation/gap signal (USB CDC is reliable+in-order).
  Duplicates are ignored (no double-apply), gaps are counted (informational), wraparound is normal.
- **Malformed handling**: every decode error (bad COBS, bad magic, unsupported version, oversized/
  mismatched length, bad CRC, bad postcard) is a typed `ProtoError` — never a panic (SC-008).

CRC-32 and COBS are implemented in-crate (no extra dependencies) so both are fully unit-testable.

## Alternatives considered

- **Length-prefix framing** (no COBS): cannot resynchronize after a dropped byte. Rejected — COBS is
  self-synchronizing.
- **`bincode`/hand-rolled payloads**: `postcard` is embedded-focused with a documented stable format.
  Rejected in favor of the locked `serde`+`postcard` decision.
- **CRC-16**: cheaper but weaker; CRC-32 is inexpensive for our small frames. Chose CRC-32.

## Consequences

- Two peers agree on the schema by depending on the same `Message` enum crate (Principle II for the
  protocol).
- Evolution is append-only; reordering/removing variants or fields is a breaking (major) change.
- A fuzz/malformed corpus (T034) and golden byte vectors (T035) guard framing stability.
