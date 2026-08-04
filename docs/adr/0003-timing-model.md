# ADR-0003: Rendering timing model (integer-ms timebase, drift-free 30 FPS step)

**Status**: Accepted · **Date**: 2026-07-17 · **Feature**: 001-device-connection-foundation

## Context

Deterministic rendering (constitution Principle III, FR-019, SC-011) requires that the renderer be a
pure function of its inputs, including time. The canonical time input is **elapsed milliseconds**, and
Device Studio exposes a nominal **30 FPS** step control. But 1/30 s = 33.333… ms is not an integer, so
naively accumulating `+33.333` (float) — or even `+33` (integer) — per step accumulates error and
drifts out of sync over a long session.

## Decision

- The canonical timebase is **integer elapsed milliseconds** (`ElapsedMs = u32`). The renderer never
  sees wall-clock time or floating point.
- Device Studio tracks an **integer step index** `n`. Canonical time is **derived**, never
  accumulated:

  ```text
  timestamp_ms(n) = (n * 1000 + 15) / 30      // integer division; round-to-nearest via +15 (= 30/2)
  ```

  Because the value is recomputed from `n` each time, stepping has **zero cumulative drift**: it is
  exactly periodic — every 30 steps advances exactly 1000 ms (`n=30 → 1000`, `n=60 → 2000`, …).
- **Per-scene animation** frame selection is a pure integer function of the canonical elapsed time and
  the scene's own integer frame-rate ratio (`num/den`):

  ```text
  frame(elapsed_ms, num, den, frame_count) = floor(elapsed_ms * num / (1000 * den)) mod frame_count
  ```

  The 30 FPS step is only the Device Studio *inspection cadence*; actual animation speed is per-scene.
- **Boundaries / overflow** (safe by construction): negative indices clamp to `0`; the derivation is
  computed in `i128` and **saturates** at `u32::MAX`; static scenes (`frame_count <= 1`) and invalid
  rates return frame `0`.

The shared, canonical implementation lives in `crates/kivori-model/src/timeline.rs`
(`frame_step_ms`, `scene_frame`, `StudioTimeline`). The frontend Device Studio store mirrors the integer
`step_index` and obtains rendered frames from the shared renderer via IPC, so no second timing
implementation exists (Principle II).

## Alternatives considered

- **Float millisecond accumulator** (`t += 33.333`): drifts and is nondeterministic across platforms.
  Rejected.
- **Microsecond integer step** (`t += 33_333 µs`): drifts ~10 µs/s vs true 1/30 s. Rejected.
- **Truncated `+33` ms integer step**: drifts ~0.333 ms/step. Rejected.

## Consequences

- Any frame is addressable directly by `n` (scrub/step/seek) with no replay needed.
- Golden-frame tests sample fixed `elapsed_ms` values and are reproducible across host and device.
- A drift/periodicity unit test (T020) guards the invariant; `frame_step_ms` is `const` and allocation-
  free (`no_std`).
