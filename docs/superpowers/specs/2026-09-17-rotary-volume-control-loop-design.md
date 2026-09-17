# Slice 002 Design: Rotary Volume Control Loop

**Date:** 2026-09-17
**Status:** Approved direction; awaiting spec review
**Product contract:** [`../../product/prd.md`](../../product/prd.md), [`../../product/user-story-contract.md`](../../product/user-story-contract.md)
**Engineering baseline:** [`../../engineering-principles.md`](../../engineering-principles.md)
**Foundation:** [`../../features/001-device-connection-foundation/`](../../features/001-device-connection-foundation/)

## Goal

Deliver the smallest **real physical end-to-end proof** of the Kivori product loop:

```
HW-040 A/B  →  validated logical detents  →  protocol  →  Windows master volume
            →  observed Core Audio state  →  StateConfirmed  →  Kivori volume presentation
```

Turning the knob changes Windows default-output master volume, and the device displays the volume Windows actually reports. This is the first slice that satisfies the product thesis — *control the desktop physically, understand the desktop visually* — rather than only establishing a link.

Success is a physically working device, not maximum architecture.

## Current evidence

Established by repository inspection on 2026-09-17 at PR #2 head `8f4b9b9`:

- Feature 001 provides USB serial discovery, a versioned COBS/CRC/postcard protocol, reconnect, diagnostics, a deterministic shared renderer, and a verified physical ESP32-C3 + ST7789 profile.
- Firmware is a bare synchronous tick loop written against hardware-neutral ports (`Clock`, `Transport`, `DisplaySink`) with host-sim adapters — `firmware/esp32-c3/src/ports.rs:9`, `firmware/esp32-c3/src/runtime.rs:254-275`. This is the pattern input follows.
- **No input exists anywhere.** No encoder, button, detent, GPIO input, or PCNT reference in firmware, shared crates, protocol, or desktop.
- **No action, binding, capability, confirmation, presentation, or persistence model exists** in `apps/desktop/src-tauri`. Product state today is the connection FSM, one in-memory `desired: SendableState`, and a cached `reported`.
- `kivori-protocol::Message` has 11 append-only variants (tags 0–10). `Capabilities` is a `u32` bitset with **zero flags allocated** — `crates/kivori-model/src/capabilities.rs`.
- The desktop device runtime is a plain OS thread with a fixed 50 ms poll tick — `apps/desktop/src-tauri/src/runtime/device_task.rs:31,35,129`. There is no tokio anywhere.
- `SequenceTracker` resets on `Bye`; duplicates are dropped before side effects — `crates/kivori-protocol/src/codec.rs:64-102`, `firmware/esp32-c3/src/proto.rs:197-204,233`.
- Physical hardware evidence (2026-08-11) was recorded on Windows. CI runs `ubuntu-latest` and `windows-latest`; there is no macOS runner.
- Claimed GPIO on the verified physical profile: GPIO2 (D/C), GPIO3 (RST), GPIO6 (SCK), GPIO7 (MOSI), GPIO8 (backlight), plus native USB. Other pins appear unclaimed; **this design assigns none of them.**

## Why rotary-volume is the right first slice

Detent-qualified quadrature decoding is a **pure state machine over A/B levels**, so it is fully host-testable with adversarial bounce vectors. The genuine hardware unknowns — bounce duration, required sample rate, whether external conditioning is needed — live in the adapter *below* the port, not in the product semantics.

Rotary-volume is also the only candidate that exercises **continuous preview and reconciliation against confirmed desktop state** (US3 mid-gesture rule, US4 "desktop truth wins"). That is the most load-bearing behavior in the contract, and designing the action/presentation seams without their hardest consumer would risk building the wrong boundary.

A 0–100 value with clamping, external-change reconciliation, and genuine readback is a materially stronger `StateConfirmed` proof than a boolean toggle.

## Scope

### In scope

- HW-040 A/B decoding to **validated logical detents** in firmware.
- Rotary gesture formation and the 250 ms gesture-end boundary in firmware.
- Two capability-gated protocol variants: device→desktop `InputEvent`, desktop→device `Presentation`.
- The first two allocated capability bits.
- Desktop input ingress with reconnect/gesture staleness rejection.
- A single Global binding: `Rotate → Master Volume`, **fixed 1× step**.
- Minimum capability / availability / confirmation types, extensible but not generic.
- A narrow Windows Core Audio backend behind a trait, with a deterministic fake.
- Preview ownership during a gesture and reconciliation to confirmed state at gesture end.
- A deterministic shared volume overlay, golden-frame covered.
- **The physical HW-040 adapter**, with CLK/DT/SW pin mapping isolated in the profile layer.

### Out of scope

Acceleration and the 5× ceiling · master mute · the push-switch gesture FSM, Hold semantics, and recovery ownership/arbitration · heartbeat rewiring · profiles/ContextEngine · DeviceRegistry and multi-device · ExecutionTracker · ConfigStore · UpdateCoordinator · SessionOwnershipGate · macros · any second OS backend · display-idle/ambient policy · `SessionEpoch` (see §4.1).

### Deferred-scope consequences recorded honestly

- With a fixed 1× step there is no acceleration multiplier state, so contract invariants 15, 16, and 34 (reversal resets acceleration, acceleration accumulates only across same-direction detents, 5× ceiling) are **vacuously satisfied** in this slice. They become live behavior in the acceleration slice. `InputEvent` carries per-detent `device_ms` so that slice needs no protocol change.
- Contract invariants 33, 40, 44, and 52 (recovery arbitration, release-qualified short press, release-qualified Hold, device-wide single-gesture ownership) are **not proven** by this slice because it has only one control. They belong to the push-switch/recovery slice.
- The contract's boundary rule (US1 *Bounded values*) is implemented for clamping and repeat-suppression; the optional subtle boundary reaction is represented in the wire type and rendered minimally.

## Architecture

### 1. Firmware input, behind a port

A fourth hardware-neutral port joins `Clock`, `Transport`, and `DisplaySink`:

```rust
/// Instantaneous encoder levels. Never blocks. Pin mapping is NOT this trait's concern.
pub trait InputSource {
    fn sample(&mut self) -> InputLevels;   // { a: Level, b: Level, sw: Level }
}
```

`sw` is sampled and carried but **unused** in this slice; it exists so the push-switch slice needs no port change.

Adapters: the physical HW-040 adapter, and a host-sim adapter that replays scripted level sequences. All semantics live above the port, so every behavioral rule is host-testable without hardware.

### 2. Quadrature decoding — a legal transition is not a detent

Two pure layers, in `firmware/esp32-c3/src/input/`:

**`quadrature.rs`** consumes `(InputLevels, now_ms)` and maintains Gray-code phase state. It rejects electrically invalid transitions (counting them as a quality signal) and emits `ValidatedDetent(Cw | Ccw)` **only when a complete, mechanically meaningful detent has been traversed**. Individual legal quarter-step transitions and partial back-and-forth phase motion are input conditioning detail and never reach the gesture layer. This is contract invariant 46 and research R-82.

Consequences that follow directly, and which the tests assert:

- Bounce that traverses adjacent Gray states and returns without completing a detent produces **no** detent.
- Detents the hardware did not observe are never reconstructed (invariant 42). Missing input is not failed input; it never produces Error.
- Invalid-transition counts are recorded as diagnostics, never converted into motion.

**`gesture.rs`** consumes validated detents plus the clock and owns gesture identity: a gesture opens on the first detent, and closes after **250 ms without a new detent** (contract §5 timing reference). `gesture_id` is a free-running `u16` from boot, **never reset per session** — see §4.1.

### 3. Tuning constants are hardware-validation parameters

Gathered into one `InputTuning` struct with provisional defaults, each annotated with the research item that must measure it. They are not treated as settled constants.

| Parameter | Provisional | Measured by |
|---|---|---|
| A/B sample interval | per runtime tick, bounded by clock | HW-validation #1, #5 |
| detent qualification | full-step | HW-validation #2 |
| invalid-transition threshold | diagnostic only | HW-validation #6 |
| gesture-end inactivity | 250 ms | contract §5 initial target |
| base step | 2 volume points per detent | UX tuning |

The switch debounce interval is deliberately absent; it arrives with the push-switch slice.

### 4. Protocol — two appended variants

Tags 0–10 are untouched. `Capabilities` receives its first allocations.

| Bit | Capability | Meaning |
|---:|---|---|
| 0 | `PHYSICAL_INPUT_V1` | device may emit `InputEvent` |
| 1 | `PRESENTATION_V1` | device renders `Presentation` |

| Tag | Message | Dir | Payload |
|---:|---|---|---|
| 11 | `InputEvent` | V→D | `{ gesture_id: u16, control: ControlId, kind: InputKind, device_ms: u32 }` |
| 12 | `Presentation` | D→V | `{ revision: u32, primary: PrimaryState, value: Option<ValueDisplay>, transient_ms: u16 }` |

```rust
enum ControlId  { Rotary }                                  // extensible
enum InputKind  { GestureStarted, Detent(Direction), GestureEnded }
enum Direction  { Cw, Ccw }

enum PrimaryState { Idle, Active, Error, Unknown }
enum ValueKind    { Volume }                                // extensible
struct ValueDisplay { kind: ValueKind, current_percent: u8, at_boundary: bool }
```

Three deliberate decisions:

- **New variants, never new fields on existing variants.** ADR-0002's rule is an append-only *enum*; postcard structs carry no field tags, so appending a field to an existing variant such as `Ready` would be a silent wire break. Variant append is the only safe evolution.
- **`Presentation` rather than reusing `SetState`.** `SendableState` has four values and no continuous value, so it cannot represent an observed volume or distinguish confirmation classes. `SetState` remains for backward compatibility; `Presentation` is additive and capability-gated, so an older firmware continues to work with behavior simply off.
- **`revision` is monotonic.** The device ignores any `Presentation` whose revision is not newer. This implements contract invariant 32 — reconnection restores current truth and never replays expired presentation — by construction rather than by timing.

**`value` and `primary` are two different layers carried in one message, and the distinction is load-bearing.** `value` is the *transient overlay* — the volume bar — and `primary` is the *underlying state* that remains true beneath it. `transient_ms` (0 = persistent) applies only to `value`.

Firmware expires `value` locally after `transient_ms` and falls back to rendering `primary` from the same message, which is why the message must carry both. Firmware-local expiry means the overlay cannot stick if the host disappears mid-transient, and it implements invariant 50 — a transient restores the *current underlying truth* rather than blindly returning to Idle — without a host timer and without a second round trip. In this slice `primary` is `Idle` while nothing else is happening; when later slices introduce running work, the same mechanism restores `Busy` instead, with no protocol change.

`device_ms` is carried now although nothing consumes it, because it is exactly what the deferred acceleration slice requires; including it avoids a protocol change later.

#### 4.1 `SessionEpoch` is excluded until a failing test demands it

A prior revision of this design proposed a `SessionEpoch` message so the desktop could reject pre-disconnect input after a reconnect. It is **excluded** as speculative. The simpler mechanism is four rules and no new message:

1. Firmware clears input state on `Hello` and on `Bye`/link loss.
2. `gesture_id` free-runs from boot and is never reset per session, so an old id cannot alias a new one.
3. The desktop clears its open-gesture set on every exit from `Connected`.
4. The desktop rejects any `InputEvent` for a gesture whose `GestureStarted` it did not observe **in this connection**.

Analysis of each failure mode:

| Scenario | Covered? |
|---|---|
| User still turning the knob across a reconnect | Yes. Post-reconnect detents are newly observed, not replayed; the contract permits them once the takeover condition clears. |
| Desktop restarts while firmware holds an open gesture | Yes. Rule 4 rejects every later event for a gesture whose start was never seen. |
| Stale bytes buffered across port close/reopen | Yes, twice. They surface during `Connecting`, where input is already invalid-for-phase (protocol contract §8), and COBS discards any straddling partial frame. |
| `gesture_id` reuse across sessions | Yes, by rule 2. |

**This is proven, not asserted.** A task writes the adversarial test first: inject pre-disconnect input bytes immediately after a reconnect handshake and assert no volume change occurs. If that test cannot be made to pass with rules 1–4, `SessionEpoch` is added at that point as recorded evidence. That is the only condition under which it returns.

### 5. Desktop modules

Four focused modules inside `apps/desktop/src-tauri/src`. No new crates (research R-3, engineering principle 10). Everything except `platform/windows` compiles and tests on any host.

```
input/          decode InputEvent → open-gesture validation → LogicalInput
action/         the single Global binding · rotary value model (step, clamp, boundary) · Outcome
platform/       trait VolumeBackend + deterministic fake
  windows/      #[cfg(windows)] Core Audio
  unimplemented.rs   other targets → NotImplementedYet { target }
presentation/   PresentationResolver: ProductSnapshot → Presentation   (pure)
```

Binding is deliberately **not** its own module. This slice has exactly one binding — `Global: Rotate → MasterVolume` — so it is a table inside `action/`. A binding module is introduced when a slice has more than one scope to resolve between.

### 6. Capability, availability, and confirmation are three separate things

Conflating "the OS cannot do this" with "we have not written it yet" would put a falsehood into the configuration UI, which the cross-platform rule forbids.

**The semantic rule — normative for all future slices:**

- **Platform capability** is a fact about the *machine*: what the OS, session, and granted permissions genuinely permit. It does not change when Kivori is rebuilt.
- **Backend availability** is a fact about this *Kivori build and its runtime*: whether we compiled an implementation, and whether its runtime dependencies are present right now. It does not change when the OS changes.
- **Confirmation class** is a fact about an *executed action*: how well its outcome can be observed. It is not a capability at all.

These never collapse into one another. Concretely: **on macOS today, master volume is `NotImplementedYet` — it is not "unsupported".** macOS Core Audio can do it; Kivori has not written it. Something the OS genuinely cannot expose would be `UnavailableOnThisPlatform`. Reporting the first as the second would tell a user their Mac cannot change its own volume.

**What this slice actually implements** — the minimum that carries the distinction, and no generic framework:

```rust
/// A fact about this Kivori build and its runtime dependencies.
enum BackendAvailability {
    Available,
    NotImplemented { target: &'static str },   // Kivori has no backend compiled for this OS
    RuntimeUnavailable { reason },             // implemented, but a dependency is missing now
}

/// How well an executed action's outcome can be observed.
enum ConfirmationClass { StateConfirmed, ExecutionConfirmed, TriggeredUnverified }

/// The derived value, and the only one the UI consumes.
enum ActionAvailability {
    Available { confirmation: ConfirmationClass },
    NotImplementedYet { target: &'static str },
    RuntimeUnavailable { reason },
    Unknown,
}
```

A `PlatformCapability` type is deliberately **not** introduced yet. Every action in this slice is one the OS can do, so such a type would have no producer and no consumer — dead code asserting an architecture rather than implementing one. It materializes, together with the `UnavailableOnThisPlatform` and `NeedsPermission` arms of `ActionAvailability`, in the first slice that owns an action the OS genuinely restricts or gates behind a permission. `ActionAvailability` is shaped to accept those arms without restructuring its consumers.

`ConfirmationClass` and the outcome type are nevertheless defined in full even though this slice produces only three arms (`StateConfirmed`, `TriggeredUnverified`, `Failed`), because the hard constraint is that execution must never collapse into `bool success`:

```rust
enum Outcome {
    Running,
    StateConfirmed { volume_percent: u8 },
    ExecutionConfirmed,
    TriggeredUnverified,
    Failed { reason: FailureReason },
}
```

### 7. Windows backend behind a narrow interface

```rust
trait VolumeBackend {
    fn availability(&self) -> ActionAvailability;
    fn read(&self) -> Result<u8, BackendError>;          // 0..=100
    fn set(&self, percent: u8) -> Result<u8, BackendError>;  // returns the READ-BACK value
}
```

`set` returning the read-back value rather than `()` is what makes `StateConfirmed` structurally honest: the confirmed value comes from the OS, never from the request.

The Windows implementation uses `IAudioEndpointVolume` on the default output endpoint via `GetMasterVolumeLevelScalar`/`SetMasterVolumeLevelScalar`, matching the scalar taper the Windows volume slider presents.

**Threading.** There is no tokio in this repository, so the backend matches the existing shape: a dedicated `kivori-audio` OS thread performs `CoInitializeEx`, owns the endpoint, and registers an `IAudioEndpointVolumeCallback`. **Callbacks are ingress only** — they post to an `mpsc` channel and never perform work on the COM thread (research R-50, R-76). External volume changes therefore reach Kivori without a Kivori-originated command, which is US4 satisfied directly.

When no default endpoint exists, availability is `RuntimeUnavailable` — not a failure, and not silence.

### 8. Preview ownership and reconciliation

The behavior that only the rotary path exercises:

- While a gesture is open, the desktop's locally computed target value **owns** the displayed value. Rapid detents do not require a confirmation round trip each (US3 rapid rotary input).
- An external Core Audio volume change during an active gesture does **not** overwrite the in-progress preview (US3 mid-gesture desktop state changes).
- On `GestureEnded`, the desktop reads back the confirmed value and the display reconciles to it. **Confirmed state wins** (US3, US4, invariant 1).
- Values clamp at 0 and 100 immediately. Repeated detents further into a boundary set `at_boundary` once and do not re-trigger feedback; the first reverse detent takes effect immediately without requiring the gesture to end.

With a fixed 1× step there is no multiplier to reset, so reversal is correct by construction.

### 9. Deterministic volume display

Principle 2 requires one canonical visual model: Device Studio preview and the physical panel must render identically. The volume overlay is therefore a **pure, integer-only compositor step added to the shared `kivori-renderer`**, consumed by both the host preview and firmware:

```rust
pub fn render_volume_overlay(tile: &mut TileBand, percent: u8, at_boundary: bool);
```

A solid-rect bar, no numerals and no glyphs. This deliberately avoids font work and **requires no change to the compiled asset blob or the asset compiler**. Golden frames cover sampled percents including both boundaries.

### 10. Latency is measured, not assumed

The device thread currently sleeps a fixed 50 ms, so detents could be handled up to 50 ms late before any round trip. Rather than pre-emptively restructuring the loop, detent→display latency becomes a **measured acceptance criterion** on physical Windows hardware. If measurement shows the target is missed, the fix is a short-timeout blocking read in place of the fixed sleep — a small change requiring no new architecture. No speculative change is made first.

### 11. Heartbeat is excluded

`send_ping`, `heartbeat_timed_out`, and `ManagerEvent::HeartbeatTimeout` have **zero production call sites**; a stalled-but-open link reports Connected indefinitely. This design examined whether the rotary-volume loop is incorrect without it and **could not demonstrate that it is**. A silent stall yields a stale *presentation* while claiming Connected — a system-health and `Degraded` problem belonging to US10 — but it cannot produce a wrong volume, a replayed detent, or a false confirmation.

Heartbeat rewiring is therefore recorded as **separate technical debt** (research R-69, implementation spike #18) and is not pulled into this slice.

## Error handling

| Condition | Behavior |
|---|---|
| Electrically invalid A/B transition | rejected below detent formation; counted as a quality diagnostic; never becomes motion |
| Unobserved/dropped detents at high speed | not reconstructed; not an error (invariant 42) |
| Unknown `ControlId`/`InputKind` from newer firmware | ignored plus a safe diagnostic; never a panic |
| No backend for this OS | `NotImplementedYet { target }`; action is not offered; no execution attempt |
| No default audio endpoint on Windows | `RuntimeUnavailable`; honest presentation, not Error |
| Core Audio call fails | `Failed { reason }` → `PrimaryState::Error`; connection remains Connected (a capability fault is not a transport fault) |
| Read-back unavailable after a successful set | `TriggeredUnverified` — never presented as success |
| Input event for an unseen gesture | dropped silently for execution; recorded as a diagnostic |

Diagnostics continue to obey the ADR-0005 allowlist; no new field carries raw payloads or raw identity.

## Verification plan

Test-driven: each task writes a failing test, confirms the expected failure, implements the minimum, then runs focused and surrounding tests.

| Layer | Runs on | Proves |
|---|---|---|
| Pure unit | every host including macOS | quadrature decode against bounce and invalid-transition vectors · a legal transition is not a detent · gesture boundary at 250 ms · clamping and boundary suppression · outcome classification · availability derivation · presentation resolution including transient→underlying-truth and revision monotonicity · capability negotiation, and that an unnegotiated bit leaves behavior off |
| Backend contract suite | every host, via the fake | one suite that both the fake and the real Windows backend must satisfy |
| Firmware host-sim | CI | scripted A/B level sequences produce the expected `InputEvent` stream |
| Desktop integration | CI | `InputEvent` bytes in → `Presentation` bytes out, against the fake backend; plus the §4.1 adversarial staleness test |
| Golden frames | CI, all OSes | volume overlay determinism at sampled percents |
| Windows CI | `windows-latest` | **compiles** the Core Audio backend and runs the fake-backend contract suite. Anything requiring a real endpoint is **explicitly reported as skipped, never silently passed** |
| **Physical Windows** | maintainer hardware | real Core Audio readback and change callbacks, external-change reconciliation, end-to-end latency |
| **Physical device** | maintainer hardware | HW-040 wiring, detent fidelity, no false reversals, panel overlay |

**GitHub-hosted Windows runners are not assumed to expose a usable audio endpoint.** Real Core Audio state and callback behavior is physical Windows evidence recorded in the Slice 002 validation checklist, and simulation evidence is never promoted to physical evidence.

No existing assertion is weakened or deleted.

## Candidate ADRs

Recorded only if the implemented design settles a durable choice against real alternatives. Neither is pre-authorized by this spec.

- **Capability / availability / confirmation as three orthogonal concepts.** Alternative: one flat enum. Durable and cross-cutting, so likely — decided once the shape survives implementation.
- **Input-semantics ownership split** (firmware forms validated detents; desktop owns action meaning). Alternative: desktop-side classification. May already be settled by contract invariant 24 and therefore not need an ADR.

Epoch-free staleness handling, the `Presentation` shape, and the tuning constants belong in the feature record and the protocol contract, not in an ADR.

## Documentation outputs

- `docs/features/002-rotary-volume-control/` — requirements, architecture notes, contract deltas, and a validation checklist carrying the unpopulated physical rows.
- Updates to `docs/features/001-device-connection-foundation/contracts/protocol.md` and `data-model.md` for the appended variants and allocated capability bits.

## Open questions

1. **HW-040 CLK/DT/SW pin mapping is not recorded anywhere in the repository.** The verified physical profile claims GPIO2, 3, 6, 7, 8 and native USB; other pins appear unclaimed but no encoder wiring is documented. The encoder is assumed already wired, so implementation is not blocked on a research spike: the mapping is isolated as a `RotaryProfile { clk, dt, sw }` in `firmware/esp32-c3/src/profile.rs` so it can be corrected in one place. **The maintainer is asked for the actual pins when the physical-adapter task is reached, not before.**

## Non-goals

- Proving every future input invariant. Recovery arbitration, release-qualified press semantics, and device-wide single-gesture ownership belong to the push-switch slice.
- Building a generic action, binding, or capability framework ahead of a second consumer.
- Feature parity across operating systems. macOS and Linux honestly report `NotImplementedYet`.
- Merging PR #2.
