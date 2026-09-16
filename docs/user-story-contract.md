# Kivori User Story Contract

**Status:** Product behavior baseline  
**Date:** 2026-09-16  
**Product requirements:** [`../PRD.md`](../PRD.md)

## 1. Purpose

This document defines the normative behavioral contract for Kivori's product user stories.

It describes **what users must be able to rely on**, not the internal implementation architecture. Implementation plans, feature specs, protocol revisions, and platform-specific designs should reference this contract rather than re-deciding these behaviors independently.

Implementation status is separate from this contract. A behavior being specified here does not imply it is already implemented.

## 2. Normative Language

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

- **MUST / MUST NOT**: required product behavior.
- **SHOULD / SHOULD NOT**: expected default behavior unless a documented platform limitation requires otherwise.
- **MAY**: optional behavior that must not violate any MUST-level rule.

Timing values marked **initial target** are product UX targets and may be tuned after hardware/platform validation without changing the underlying behavioral rule.

## 3. Global Invariants

The following rules apply across all user stories.

1. **Desktop truth wins.** Confirmed observable state overrides Kivori's local prediction, previous command, or requested value.
2. **Observable truth only.** Kivori MUST NOT invent state it cannot observe reliably.
3. **Acknowledgement is not confirmation.** Immediate input feedback MUST NOT be presented as proof of desktop success.
4. **Confirm only what is knowable.** Known success, known failure, and unknown outcome MUST remain distinct.
5. **No stale replay.** Expired input MUST NOT execute after a connection or service recovers.
6. **Explicit action scope.** An action's meaning MUST NOT silently change because a different app profile is active.
7. **Stable context.** Profile changes MUST follow stable OS focus, not mouse hover or momentary focus theft.
8. **Interaction commits intent.** If a pending foreground application is valid but still inside the focus-stabilization window, deliberate Kivori interaction MUST commit that foreground application before dispatching the action.
9. **Security context beats profile fallback.** Protected/secure foreground context MUST be classified before profile commitment and MUST NOT fall back to General merely because normal app actions are unavailable.
10. **Dispatch target is revalidated.** A gesture MUST NOT be silently retargeted if its committed application disappears between input registration and action dispatch.
11. **Known target loss cancels the gesture.** If a committed application target is known to disappear during a continuous gesture, the remaining gesture input MUST be discarded until that gesture ends.
12. **Gesture ownership.** A physical gesture remains bound to the context in which it began.
13. **One gesture owns input in MVP.** Simultaneous inputs MUST NOT implicitly become combo triggers.
14. **Wake gestures are atomic.** A gesture that begins while the display is fully asleep MUST be consumed as wake-only in its entirety.
15. **Rotary reversal resets acceleration.** A direction reversal MUST restart rotary acceleration at baseline before acceleration may build again in the new direction.
16. **Acceleration represents sustained direction.** Acceleration MUST accumulate only across consecutive same-direction detents; rapid oscillation repeatedly resets to baseline.
17. **User isolation.** One OS user's mappings and session-specific presentation MUST NOT remain visible or actionable in another user's session.
18. **Session-private render state becomes non-renderable on ownership change.** Previous-user app/profile names, custom icons, transient feedback, and user-specific indicators MUST be invalidated before another local OS user can see them.
19. **No hidden fallback.** If an action cannot run in the current context, Kivori MUST communicate unavailability rather than silently switch mechanisms.
20. **Localized uncertainty.** Unknown/restricted state SHOULD propagate only as far as necessary.
21. **Known host/session state wins.** Explicit sleep, lock, switching-user, update, or restart state outranks generic communication symptoms.
22. **Passive is not Waiting.** Passive means a usable desktop exists but this device is not assigned as the controller.
23. **Passive is not Monitor Mode.** Passive devices MUST NOT mirror ordinary real-time desktop state in MVP.
24. **Hardware recovery is out-of-band.** Basic MCU recovery MUST remain available independently of desktop software, profile assignment, display-sleep state, and normal action restrictions whenever firmware can still observe the recovery gesture.
25. **Latest deliberate interaction wins transient feedback.** A newer deliberate user action MAY replace an older interaction transient immediately; user-action transients MUST NOT form a mandatory visual queue.
26. **Interaction feedback has priority over ordinary background transients.** Ordinary unsolicited reactions SHOULD NOT interrupt active user-action feedback.
27. **Urgent state may wake the display.** A small defined class of immediate-attention desktop states MAY wake Display Sleep; ordinary background changes MUST NOT.
28. **System/privacy indicators outrank custom indicators.** User customization MUST NOT allow app-specific indicators to displace higher-priority safety/privacy state.
29. **Indicator escalation is immediate; promotion is stabilized.** A newly active higher-priority indicator SHOULD appear immediately, while newly exposed lower-priority indicators SHOULD wait for a short stabilization window before causing layout reflow.
30. **Continuous gestures reconcile at gesture boundaries.** External continuous-value updates SHOULD NOT visually fight an active Kivori rotary gesture; final confirmed state MUST reconcile when the gesture ends.
31. **Execution health is separate from transport health.** A confirmed host-side long-running operation MUST NOT be considered failed solely because the Kivori device link becomes Degraded or Disconnected.
32. **Reconnection restores current truth, not expired presentation.** Reconnecting MUST NOT replay stale Success/Error/Unverified transients for historical events that completed while the device was disconnected.
33. **Recovery hold has absolute input arbitration.** While the primary recovery-capable button remains continuously held and its recovery timer is active, incidental rotary detents MUST NOT cancel, reset, or replace the recovery hold and MUST NOT execute ordinary rotary actions.
34. **Rotary acceleration is bounded.** MVP acceleration MUST NOT exceed the default 5x base-step ceiling; actions MAY use a lower ceiling or disable acceleration.
35. **Explicit desktop test intent may wake Display Sleep.** A user-triggered Test Action or Preview Buddy State command from Kivori Desktop is explicit intent and MAY wake the display without requiring a prior physical wake gesture.
36. **Takeover states preempt active gestures.** When a Layer 1 takeover state makes normal interaction unavailable, Kivori MUST cancel the active normal gesture and surface the takeover without waiting for the gesture-end timer.
37. **Recognized transient overlays do not own profiles by default.** A transient overlay shown over a stable foreground application SHOULD preserve the underlying application's profile unless the overlay becomes a normal independently focused context or is classified as Protected.

---

# US1 — Physical Desktop Control

## User Story

**As a user, I want to control frequent desktop functions through Kivori, so I can perform common actions without interrupting my current work.**

## Contract

Kivori MUST support an action model where each assigned action has explicit semantics and scope.

Examples include:

- System Volume;
- Play/Pause;
- Microphone Mute;
- keyboard shortcut;
- launch application;
- run script;
- application-specific action;
- configured macro.

`System Volume` and `Discord Volume`, for example, are distinct actions. Activating a Discord profile MUST NOT silently redefine a generic `System Volume` action as Discord process volume.

### Rotary action contract

Each rotary-capable action MAY define:

- minimum and maximum range;
- step size;
- sensitivity;
- acceleration behavior;
- acceleration reset interval.

Initial default acceleration-reset target: **250 ms without a new detent -> reset to 1x**.

Acceleration MUST be action-specific and MAY be disabled for precision-oriented actions.

Acceleration MUST build only from consecutive detents in the same direction. Every direction reversal MUST reset the multiplier to baseline before the first detent in the new direction is applied.

Example:

`CW 1x -> CW 2x -> CW 3x -> reverse -> CCW 1x -> CCW 2x -> reverse -> CW 1x`

Rapid one-detent oscillation therefore remains effectively at baseline sensitivity. This is intentional: sustained direction expresses speed intent, while repeated reversal expresses precision intent.

For actions such as frame-accurate or fine audio scrubbing, acceleration MAY be disabled entirely.

### Acceleration ceiling

Initial MVP maximum acceleration multiplier: **5x the action's base step**.

Acceleration MAY ramp through implementation-defined intermediate levels, but the effective multiplier MUST NOT exceed 5x in MVP.

An action MAY configure:

- a lower multiplier ceiling;
- no acceleration at all.

Example:

`1x -> 2x -> 3x -> 4x -> 5x -> 5x -> 5x`

The 5x ceiling limits runaway jumps during extreme high-speed continuous rotation while preserving useful coarse adjustment.

### Bounded values

When a bounded action reaches its minimum or maximum:

- the value MUST clamp immediately;
- the first continued tick beyond the limit MAY produce one subtle boundary reaction;
- repeated ticks farther into the same boundary SHOULD NOT spam visual or buzzer feedback;
- the first reverse tick MUST take effect immediately;
- reversing direction MUST NOT require the user to pause or end the gesture;
- the first reverse tick MUST use baseline acceleration, not the accumulated multiplier from the previous direction.

Example:

`98 -> 99 -> 100 -> extra clockwise ticks ignored -> first counter-clockwise tick -> 99`

## Acceptance Criteria

- [ ] Assigned actions expose explicit action identity/scope.
- [ ] App profiles select actions without mutating action meaning.
- [ ] Rotary actions can define action-specific sensitivity and acceleration.
- [ ] Acceleration accumulates only across same-direction detents.
- [ ] Every direction flip resets acceleration to 1x.
- [ ] Rapid oscillation remains effectively baseline unless multiple same-direction detents accumulate.
- [ ] Precision-oriented actions can disable acceleration.
- [ ] MVP acceleration never exceeds 5x the action's base step.
- [ ] Actions can choose a lower acceleration ceiling.
- [ ] Bounded controls clamp correctly.
- [ ] Boundary feedback is not repeated for every excess tick.
- [ ] Reverse movement exits boundary suppression immediately.
- [ ] Direction reversal resets accumulated acceleration to 1x before the first reverse step.

---

# US2 — Responsive Input and Honest Confirmation

## User Story

**As a user, I want Kivori to respond immediately to physical input while clearly distinguishing acknowledgement from actual action success.**

## Contract

The conceptual interaction pipeline is:

**Input -> Acknowledged -> Executed -> Confirmed when possible**

Kivori MUST distinguish at least three confirmation classes.

### State Confirmed

The resulting state is directly observable.

Example: microphone mute is requested and the desktop subsequently reports the microphone as muted.

### Execution Confirmed

The operation is known to have started or completed, but no persistent resulting state exists.

Example: a configured process launch is known to have started successfully.

### Triggered / Unverified

The action was dispatched but its final effect cannot be verified.

Example: a keyboard macro is emitted but there is no reliable downstream completion signal.

Kivori MUST NOT present Triggered / Unverified as confirmed success.

### Initial interactive timing targets

- **< 50 ms:** local input acknowledgement;
- **~500 ms:** unresolved interactive action MAY show Processing / Delayed;
- **~1,500 ms:** unresolved ordinary interactive action reaches timeout handling.

Timeout alone MUST NOT automatically become Error.

- positive evidence of failure -> **Error**;
- dispatched action with unknowable outcome -> **Unverified**;
- unhealthy communication -> **Degraded system health**.

Long-running operations MUST NOT be forced into the ordinary 1,500 ms interactive timeout model; they require an explicit running/pending behavior.

### Long-running host execution

Once a long-running action has been positively confirmed as started by the desktop host, its execution lifetime is distinct from the Kivori device-link lifetime.

Examples include:

- a multi-minute build;
- a long-running script;
- an export/render process.

If the Kivori USB/device link becomes Degraded or Disconnected while the desktop service still observes the job:

- the host-side job MUST NOT be marked failed solely because device transport is unhealthy;
- the job MAY continue running normally on the host;
- device communication health and host execution state MUST remain separate concepts;
- on reconnection, Kivori MUST reconcile to the current observed job state rather than replay the start command.

If the desktop service itself loses the ability to determine whether the job completed, the execution result MUST become **Unverified / Execution Unknown** unless positive evidence of failure exists.

A lost or uncertain execution MUST NOT be automatically restarted merely because the Kivori link later recovers.

### Reconnection and historical completion

Reconnection is a current-state reconciliation event, not a replay of missed presentation events.

If a previously known long-running action completed while Kivori was disconnected:

- Kivori MUST NOT replay an expired Success, Error, or Unverified transient merely because transport reconnects;
- if the job is still running, current Running state MAY be restored;
- if the job completed in the past and no current running state remains, Kivori SHOULD settle directly into the current underlying buddy state;
- historical completion MAY remain available in desktop-side history/logging without occupying the live transient surface.

Example:

`Build starts -> Kivori disconnects -> build succeeds -> five minutes later reconnect -> current state Idle`

The reconnect itself MUST NOT fabricate an 800 ms Success transient for that five-minute-old completion.

### Transient reaction targets

Initial display targets:

- Success: ~800 ms;
- Triggered / Unverified: ~1.2 s;
- Error / Failed: ~2 s.

A transient reaction MUST return to the actual underlying state rather than blindly returning to Idle.

### Deliberate interaction collision

A new valid deliberate interaction MAY immediately replace a transient reaction from an earlier deliberate interaction.

Example:

`Action 1 Success overlay begins -> 200 ms later Action 2 occurs -> Action 1 overlay ends -> Action 2 feedback appears immediately`

User-action transients MUST NOT be required to queue sequentially. The newest deliberate interaction owns the interaction-feedback surface.

Takeover states MUST NOT be dismissed merely because another physical input occurred.

### Background transient collision priority

When an ordinary unsolicited background transient occurs while user-action feedback is active:

- the active interaction feedback SHOULD finish first;
- the ordinary background transient MAY be queued and shown afterward if still relevant;
- ordinary background feedback MUST NOT silently overwrite active user-action feedback.

A defined urgent state MAY preempt user-action feedback when immediate attention is more important than preserving the transient.

Default transient priority is:

**Urgent state -> newest active interaction feedback -> ordinary unsolicited transient**

## Acceptance Criteria

- [ ] Local acknowledgement is distinguishable from action success.
- [ ] State Confirmed, Execution Confirmed, and Unverified outcomes remain distinct.
- [ ] Timeout logic distinguishes known failure from unknown outcome.
- [ ] A target app crash during a pending action becomes Error when the crash is known.
- [ ] Missing callback without known failure becomes Unverified.
- [ ] Confirmed long-running host execution is not failed solely because the device link degrades.
- [ ] Reconnection reconciles long-running job state without replaying the start command.
- [ ] Historical job completion does not generate a stale transient on reconnection.
- [ ] A still-running job may restore current Running state after reconnection.
- [ ] Unknown host-job outcome becomes Unverified rather than automatic failure or retry.
- [ ] Transient reactions restore the underlying current state.
- [ ] A newer deliberate interaction may truncate and replace an older interaction transient immediately.
- [ ] Deliberate user-action transients do not form a mandatory visual queue.
- [ ] Ordinary background errors do not interrupt active user-action feedback.
- [ ] Urgent states can preempt transients when defined as immediate-attention conditions.

---

# US3 — Smooth, Atomic Physical Interaction

## User Story

**As a user, I want physical interaction with Kivori to remain smooth and deterministic during rapid input, simultaneous input, context changes, or temporary communication problems.**

## Contract

### Rapid rotary input

Rapid rotary movement MUST behave as a continuous interaction rather than requiring a full request/confirmation round trip for every detent.

Kivori MAY:

- render an immediate local target preview;
- combine/coalesce rapid updates;
- reconcile the final preview against confirmed desktop state.

Example:

`local preview 90% -> desktop confirms 88% -> display settles at 88%`

Confirmed state MUST win.

### Rotary gesture boundary

Initial MVP rotary gesture-end target: **250 ms without a new rotary detent**.

Until that inactivity threshold is reached, consecutive detents MAY be treated as one continuous rotary gesture.

This gesture boundary is distinct from action semantics even if it initially shares the same 250 ms target as acceleration reset.

### Gesture context ownership

A gesture MUST remain attached to the profile/context that was committed when the gesture began.

If application focus changes during a rotary burst, the current rotary burst MUST continue using its starting context. The newly focused profile becomes eligible for the next gesture.

### Continuous gesture target loss

If the committed target application/window is known to close, crash, or otherwise become invalid while a continuous gesture is active:

- any detents already dispatched remain historical and MUST NOT be replayed or undone automatically;
- the current gesture MUST be marked cancelled for application-scoped execution;
- all later detents belonging to that same gesture MUST be ignored for desktop execution;
- later detents MUST NOT independently retarget to General, the previous app, or the app that gains focus next;
- Kivori SHOULD surface **Error / Context Lost** once for the cancelled gesture rather than producing an error per ignored detent;
- a new application context may become eligible only after the cancelled gesture ends and a new gesture begins.

Example:

`VS Code gesture -> detents 1-3 execute -> VS Code exits -> Context Lost -> detents 4-10 ignored -> gesture ends -> next new gesture may bind to new context`

This preserves the invariant that one physical gesture has one context.

### Mid-gesture desktop state changes

For a continuous value that is actively controlled by a Kivori rotary gesture, an external desktop update to that same continuous value SHOULD NOT immediately overwrite the local in-progress preview.

The active gesture temporarily owns the continuous preview until the gesture ends, after which Kivori MUST reconcile to the latest confirmed desktop value.

Important discrete state changes MAY still appear immediately without destroying the continuous preview. For example, an independently observed mute/unmute transition may appear as a status indicator while a volume gesture remains in progress.

This exception MUST NOT allow Kivori to preserve a stale preview after the gesture completes.

### One gesture at a time

For MVP, the first active gesture owns physical interaction until it ends.

Examples:

- rotation starts -> button input does not become a second action;
- button hold starts -> rotary movement does not become another implicit action.

Future combinations such as `Hold + Rotate` MAY be introduced only as explicit input types with explicit configuration.

The hardware recovery hold is an explicit exception to ordinary gesture arbitration. While the primary recovery-capable button remains continuously held and its recovery timer is active:

- rotary detents MUST NOT cancel or reset the recovery timer;
- rotary detents MUST NOT execute ordinary mapped rotary actions;
- rotary detents MUST NOT replace recovery ownership with another gesture;
- releasing the primary button before the recovery threshold ends the recovery attempt normally.

This rule makes recovery robust on push-encoders where incidental shaft rotation can occur while the user is holding the encoder down.

### Interrupted communication

Kivori MUST NOT queue stale user intent for later replay.

During **Reconnecting**:

- local acknowledgement MAY still occur;
- desktop action MUST NOT be deferred for later execution;
- old intent MUST be discarded.

If Reconnecting or Disconnected becomes known while an ordinary continuous gesture is active:

- the active normal gesture MUST be cancelled immediately;
- remaining detents in that physical gesture MUST be discarded for desktop execution;
- the Layer 1 takeover presentation MUST appear without waiting for the 250 ms gesture-end threshold;
- a new actionable gesture MUST NOT begin until the takeover condition has cleared and normal interaction is available again.

During **Degraded** communication:

- an action MAY be attempted while communication is still usable;
- unresolved/expired actions MUST NOT execute later after recovery;
- Degraded alone does not require gesture cancellation if the current interaction remains valid and communication is still usable.

For a currently active continuous gesture, the system MAY retain the latest target while the gesture and connection are still valid, but MUST NOT replay the historical tick sequence later.

## Acceptance Criteria

- [ ] Rapid rotary use does not require one full confirmation round trip per tick.
- [ ] Final display reconciles against confirmed state.
- [ ] 250 ms without a detent ends a rotary gesture for the initial MVP target.
- [ ] Focus change mid-gesture does not split one gesture across profiles.
- [ ] Known target loss mid-gesture cancels the remaining stream.
- [ ] Remaining detents after target loss are ignored until that gesture ends.
- [ ] Target loss does not cause per-detent retargeting to another application.
- [ ] Context Lost feedback is not spammed once per ignored detent.
- [ ] External updates do not visually fight the same continuous value during an active gesture.
- [ ] Important discrete state changes may still surface during a continuous gesture.
- [ ] Implicit chord actions are not produced in MVP.
- [ ] Rotary detents during a recovery hold do not cancel/reset recovery and do not execute ordinary actions.
- [ ] Reconnecting/Disconnected cancels an active normal gesture immediately.
- [ ] Layer 1 connection takeover does not wait for the rotary gesture-end timer.
- [ ] Inputs during Reconnecting are not replayed after recovery.
- [ ] Expired inputs during Degraded communication do not execute later.

---

# US4 — Real and Observable Desktop State

## User Story

**As a user, I want Kivori to represent desktop state it can actually observe, so I can trust what the buddy shows.**

## Contract

Kivori SHOULD update from observable state changes regardless of origin, including:

- Kivori itself;
- keyboard;
- mouse;
- OS controls;
- application UI;
- supported external hardware.

Kivori MUST NOT assume its last requested state remains true when the desktop reports otherwise.

If a physical device changes behavior without exposing a trustworthy state signal, Kivori MUST represent the observable state rather than infer hidden physical reality.

Example: if a headset's hardware mute switch electrically blocks a microphone but the OS continues reporting the microphone as active, Kivori MUST NOT invent a muted state unless a trustworthy hardware/device signal becomes available.

The gesture-local preview rule in US3 is temporary presentation ownership only. It MUST NOT weaken the requirement that the final represented state converges to observable desktop truth.

## Acceptance Criteria

- [ ] External desktop changes can update Kivori state without requiring a Kivori-originated command.
- [ ] Confirmed desktop state overrides local previews and requested values after gesture reconciliation.
- [ ] Unobservable hardware-only states are not fabricated.
- [ ] Stale state is not presented as current after the source becomes invalid.

---

# US5 — Application and User-Aware Controls

## User Story

**As a user, I want Kivori's controls to adapt to the active application and OS user while ignoring momentary or ambiguous context changes.**

## Contract

### Focus definition

The active application profile MUST follow OS keyboard/window focus.

Mouse hover alone MUST NOT change profiles.

Initial passive focus-stabilization target: **300-500 ms** before committing to a new profile.

Momentary overlays and brief focus theft SHOULD NOT produce visible profile thrashing.

Transient system overlays MAY be classified as non-profile-owning contexts.

### Transient in-app and game overlays

Recognized transient overlays displayed over a stable foreground application SHOULD be non-profile-owning by default.

Examples include temporary in-game overlays such as communication, platform, capture, or GPU-control overlays that appear above a borderless/fullscreen game while the game remains the underlying stable activity.

When an overlay is classified as transient/non-profile-owning:

- the underlying stable application's profile SHOULD remain active;
- opening or closing the overlay SHOULD NOT restart the 300-500 ms profile-stabilization cycle merely because the overlay surface became visible;
- Kivori MUST NOT require process injection, anti-cheat hooks, or hidden game instrumentation solely to preserve profile ownership;
- classification SHOULD rely on ordinary OS-observable context and/or explicit user configuration;
- if the overlay becomes a normal independently focused persistent application/window, normal focus-stabilization rules MAY apply;
- if the overlay or game context is Protected/restricted, Protected behavior MUST override the underlying game profile.

This rule preserves game-control continuity without requiring invasive hooks that could conflict with anti-cheat systems.

### Interaction commits pending focus

Focus stabilization protects against passive focus flicker; it MUST NOT cause deliberate Kivori input to execute against a stale foreground application.

If Application B currently owns valid normal OS foreground focus but remains inside the passive stabilization window, a deliberate Kivori interaction MUST:

1. classify whether the pending foreground context is normal or protected/secure;
2. for a normal context, commit Application B as the interaction context immediately;
3. select Application B's profile if one exists, otherwise the appropriate fallback profile;
4. bind the complete gesture to that committed context.

The action MUST NOT execute under previously committed Application A solely because the stabilization timer has not expired.

General MUST NOT be used merely as an intermediate race-condition profile when a valid pending foreground app is known.

### Protected pending foreground context

If the newly focused context is UAC, a secure desktop, a protected administrator surface, or another context classified as Protected, deliberate Kivori input during the stabilization window MUST commit the **Protected** behavior immediately.

Protected context MUST NOT fall back to General merely because no ordinary application profile may execute there.

Security/restriction classification takes precedence over ordinary app-profile selection.

### Target revalidation before dispatch

After an interaction context is committed but before an application-scoped action is dispatched, the target context SHOULD be revalidated when practical.

If the committed target application/window terminates before dispatch and that loss is known:

- the gesture MUST NOT be silently retargeted to General;
- the gesture MUST NOT be silently retargeted to the previous app;
- the gesture MUST NOT be silently transferred to whatever app gains focus next;
- the action SHOULD end as **Error / Context Lost** or an equivalent known-failure result.

If target validity cannot be determined reliably, the action follows the normal Unverified rules rather than guessing.

If a target becomes invalid after a continuous gesture has already begun, US3's gesture-level cancellation rule applies: the remaining gesture stream is discarded until gesture end rather than revalidating and retargeting every later detent independently.

### Foreground/background rule

The committed foreground application owns app-specific physical mappings.

Background applications MAY contribute observable status but MUST NOT steal physical mapping ownership merely because they remain active in the background.

### Neutral desktop context

When no normal application profile applies, neutral desktop/taskbar contexts SHOULD use the General profile.

Protected or secure contexts MUST NOT be treated as ordinary General context.

### Application identity

Kivori SHOULD prefer stable application identity over volatile window text.

Generic hosts such as `chrome.exe`, `python.exe`, or `cmd.exe` use their host profile by default.

Users MAY create more specific targeting with available identifiers such as:

- executable identity;
- binary path;
- package/application identity;
- explicit window matching.

Kivori MUST NOT automatically guess that ambiguous windows inside a generic host are separate logical applications.

### OS user scope

Configuration MUST belong to the active OS user.

When user ownership changes:

- the previous user's mappings MUST stop applying;
- the next user's configuration MAY activate only after that user's Kivori session is available;
- one user's mappings MUST NOT carry into another user's desktop session;
- previous-user session-specific render state MUST become non-renderable before the next user can observe the device.

MVP operation is scoped to the user's normal interactive session rather than requiring the entire Kivori desktop application to run permanently as SYSTEM/root.

## Acceptance Criteria

- [ ] Mouse hover does not switch profiles.
- [ ] Passive focus changes require stable OS focus before visible profile commitment.
- [ ] Very brief focus theft does not produce a committed profile swap.
- [ ] Recognized transient in-game overlays preserve the underlying game profile by default.
- [ ] Overlay handling does not require process injection or anti-cheat hooks solely for profile ownership.
- [ ] A persistent independently focused overlay/application may enter normal focus stabilization.
- [ ] Protected overlay/context overrides the game profile.
- [ ] Deliberate interaction during the stabilization window commits the valid pending normal foreground app before execution.
- [ ] A pending Protected context enters Protected behavior immediately rather than falling back to General.
- [ ] The previous app profile is not used for a new gesture merely because the stabilization timer is still running.
- [ ] An app that disappears before dispatch does not cause the gesture to be silently retargeted.
- [ ] An app that disappears mid-gesture cancels the remaining gesture stream.
- [ ] Known target loss is surfaced as known failure/Context Lost.
- [ ] Background apps do not take physical control ownership.
- [ ] Generic-host sub-app behavior is explicit rather than guessed.
- [ ] User A mappings cannot remain active after User B becomes the active user.
- [ ] User A session-specific visual data is not renderable to User B.

---

# US6 — Restricted, Protected, and Permission-Limited Contexts

## User Story

**As a user, I want Kivori to clearly communicate when the operating system prevents certain actions, so permission problems are not mistaken for connection failures.**

## Contract

Restricted/protected contexts include:

- UAC or equivalent secure prompts;
- secure system screens;
- lock screen;
- protected administrator context;
- missing or revoked OS permissions.

During protected contexts for MVP:

- normal app-specific custom actions MUST be suspended;
- custom macros MUST NOT execute;
- local physical acknowledgement MAY still occur;
- unavailable actions MUST be communicated clearly;
- Kivori MUST NOT silently change execution mechanism;
- the hardware recovery gesture defined in US10 MUST remain available whenever firmware can still observe it.

Protected/secure context classification MUST occur before any General-profile fallback.

### Permission restrictions

If Kivori Desktop remains connected while a capability permission is missing:

- connection state MUST remain Connected unless communication itself is lost;
- only affected capabilities SHOULD become unavailable;
- unaffected capabilities SHOULD continue working;
- blocked actions SHOULD NOT be retried continuously without a permission/state change.

If restrictions prevent most useful operation, Kivori MAY escalate to a **Permission Required / Setup Required** takeover state.

Permission failure MUST remain conceptually distinct from connection failure.

## Acceptance Criteria

- [ ] Connected-but-restricted is not reported as Disconnected.
- [ ] Capability restrictions can remain localized.
- [ ] Protected contexts suspend unsafe/custom mappings.
- [ ] A newly focused Protected context is not treated as General during focus stabilization.
- [ ] Blocked actions do not silently fall back to another mechanism.
- [ ] Repeated blocked actions are not spam-retried without state change.
- [ ] Protected context does not disable the hardware MCU recovery gesture.

---

# US7 — Clear Visual State Hierarchy

## User Story

**As a user, I want Kivori to communicate multiple desktop and device conditions without making the display cluttered or ambiguous.**

## Contract

Kivori uses four visual layers.

## Layer 1 — Takeover State

Takeover states replace normal buddy presentation when normal desktop interaction is unavailable or intentionally suspended.

Defined takeover concepts include:

- Sleeping / Locked;
- Switching User;
- Protected;
- Permission Required;
- Host Starting / Resuming;
- Firmware Updating;
- Reconnecting;
- Disconnected;
- Waiting;
- Passive / Unassigned.

When a Layer 1 takeover condition becomes known and normal interaction is no longer valid, the takeover presentation MUST preempt lower visual layers without waiting for an active rotary gesture to reach its 250 ms end boundary.

If a normal gesture is active, US3's cancellation rules apply: remaining ordinary gesture input is discarded and the takeover becomes visible immediately.

### Switching User

When the OS is known to be transitioning between users, Kivori SHOULD preserve a Switching User/Locked transition state rather than briefly falling through Waiting.

Transition ends when:

- the next user's Kivori session is confirmed; or
- the normal startup grace period expires without a usable session, after which Waiting is appropriate.

During Switching User, the display MUST use neutral takeover presentation and MUST NOT render previous-user session-specific app/profile names, custom icons, transient feedback, or user-specific secondary indicators.

### Passive / Unassigned

Passive means a usable desktop session exists but this Kivori is not the active controller.

In MVP, Passive devices:

- MUST NOT execute normal mappings;
- MUST NOT mirror ordinary real-time desktop indicators;
- SHOULD show a calm connected-but-unassigned presentation;
- MUST still permit the hardware recovery gesture when firmware can observe it.

Passive MUST NOT be overloaded into visual Monitor Mode.

### Host Starting / Resuming

Initial startup/resume grace target: **15-30 seconds**.

During this grace state, absence of the user's Kivori session MUST NOT immediately be presented as an unexpected disconnect.

### Firmware Updating

Firmware update MUST be represented as an intentional state rather than an unexplained crash.

If full rendering is unavailable during low-level update, Kivori MAY use:

- a minimal hardware-supported indication; or
- a temporarily blank display while desktop software presents update progress where possible.

## Layer 2 — System Health

System health describes trustworthiness of current communication/state freshness.

- Healthy does not need permanent visual real estate.
- Degraded SHOULD receive dedicated visual priority.
- Degraded MUST NOT consume an ordinary secondary-indicator slot.

## Layer 3 — Primary Buddy State

Primary buddy states include concepts such as:

- Idle;
- Active;
- Busy;
- Success;
- Error;
- Unknown.

Only one primary buddy state should dominate at a time.

## Layer 4 — Secondary State Indicators

Secondary indicators may include:

- microphone muted;
- call active;
- master audio muted;
- media active;
- supported app-specific persistent state.

The default priority baseline is:

1. **Microphone state**;
2. **Call Active**;
3. **Master Audio Mute**;
4. **Media Playing/Active**;
5. **Custom app-specific indicators**.

This baseline is intentionally not fully user-configurable. System/privacy indicators MUST NOT be displaced by lower-priority custom app indicators.

Users MAY configure ordering/visibility within the custom app-specific tier, and future product settings MAY expose limited adjustments that preserve the protected system/privacy priority guarantees.

When the visible indicator budget is exceeded, lower-priority indicators SHOULD be condensed or hidden rather than shrinking all content indefinitely.

### Indicator layout stabilization

Indicator priority and layout stabilization use asymmetric behavior.

When a newly active indicator has higher priority than something currently visible:

- it SHOULD claim the required indicator slot immediately;
- Kivori SHOULD NOT delay urgent/high-priority information merely to preserve layout stability.

When a higher-priority indicator disappears and exposes a previously hidden lower-priority indicator:

- the lower-priority promotion SHOULD wait for an initial **~250 ms stable period** before reflowing the layout;
- multiple lower-priority changes during that window SHOULD be batched into one resulting layout update;
- if another higher-priority state appears during the stabilization window, the pending lower-priority promotion SHOULD be recalculated or cancelled.

Example with two visible slots:

`[Mic] [Call] -> Call ends -> hold current layout transition for ~250 ms -> [Mic] [Audio Mute]`

If another higher-priority indicator becomes active during that ~250 ms period, it may preempt immediately instead of waiting.

The product principle is:

**Priority escalation is immediate; lower-priority promotion is stabilized.**

Active profile information SHOULD normally appear transiently after a profile change rather than occupy a permanent indicator slot.

## Acceptance Criteria

- [ ] Takeover states cannot be confused with normal buddy state.
- [ ] Layer 1 takeover becomes visible immediately when normal interaction becomes unavailable.
- [ ] An active rotary gesture does not delay Reconnecting/Disconnected takeover presentation.
- [ ] Degraded health has distinct visual allocation from secondary indicators.
- [ ] Switching User presentation contains no previous-user session-specific visual data.
- [ ] Passive displays do not mirror ordinary desktop status in MVP.
- [ ] Passive assignment does not disable MCU recovery.
- [ ] Fast User Switching does not unnecessarily flash through Waiting.
- [ ] Firmware update does not look like an unexplained disconnect.
- [ ] Mic and other protected system/privacy indicators cannot be displaced by custom app indicators.
- [ ] A newly active higher-priority indicator can preempt immediately.
- [ ] Lower-priority indicator promotion waits for the stabilization window before causing layout reflow.
- [ ] Multiple changes during the promotion window are batched when practical.
- [ ] Secondary-state overload is handled through fixed baseline priority plus condensation/hiding.

---

# US8 — Localized Uncertainty

## User Story

**As a user, I want uncertainty or capability loss to affect only the information that cannot be verified or controlled, so one problem does not make the whole device appear broken.**

## Contract

If one secondary state cannot be verified while other states remain trustworthy, Kivori SHOULD preserve the trustworthy states and localize uncertainty to the affected state.

Example:

- desktop communication: confirmed healthy;
- master audio: confirmed muted;
- Discord microphone: unknown.

Expected result:

- the normal primary buddy remains;
- confirmed audio status remains visible;
- only the Discord microphone state is marked unknown/unavailable.

The entire primary buddy SHOULD enter Unknown only when the primary represented state itself cannot be determined reliably.

Permission-limited capability SHOULD likewise remain localized unless the restriction is broad enough to justify a takeover state.

For long-running execution, device-link uncertainty MUST NOT automatically propagate into job-failure state when the desktop host still has reliable job status.

## Acceptance Criteria

- [ ] One unknown secondary state does not force whole-device Unknown.
- [ ] Confirmed states remain visible when unrelated state is uncertain.
- [ ] Whole-buddy Unknown is reserved for primary-state uncertainty.
- [ ] Permission/capability loss follows the same localization principle.
- [ ] Device-link degradation does not falsely convert known-running host work into Error.

---

# US9 — System, Audio, Feedback, and Display-Idle Awareness

## User Story

**As a user, I want Kivori to adapt its controls, feedback, and presentation to important operating-system conditions.**

## Contract

### Audio routing

Global audio controls MUST follow the OS logical default endpoint rather than silently bypassing configured routing.

This includes virtual endpoints when the OS treats them as the default.

Application-specific audio controls MUST be explicitly configured and MUST remain distinct from global audio actions.

### Do Not Disturb / Focus Mode

Kivori distinguishes two feedback classes:

**User-initiated feedback**

Examples:

- button acknowledgement;
- rotary acknowledgement.

This follows Kivori's own feedback settings and MAY remain enabled during OS Do Not Disturb.

**Background feedback**

Examples:

- unsolicited desktop event;
- background notification.

Background buzzer feedback SHOULD respect OS Do Not Disturb / Focus Mode by default.

### Monitor idle presentation

When the host remains awake while its monitor blanks, Kivori MUST remain Connected if the desktop session is still healthy.

Presentation MAY progress:

**Normal -> Dim -> Low Motion -> Display Sleep**

Burn-in/glare protection MAY include:

- reduced brightness;
- reduced animation;
- pixel repositioning;
- eventual complete display blanking.

### Display wake semantics

When Kivori is fully in Display Sleep:

- deliberate button press MAY wake it;
- deliberate valid rotary detent MAY wake it;
- passive vibration/motion/USB activity MUST NOT wake it by default;
- future motion/proximity sensors MAY become explicit configurable wake sources.

A physical gesture that **begins** while fully in Display Sleep MUST be consumed as wake-only for the entire gesture.

For rotary input, the initial MVP wake-gesture boundary is **250 ms without a new detent**.

Therefore, if a user begins rotating while the display is asleep and continues rotating continuously for two seconds, all detents in that continuous burst are wake-only. The first actionable rotary input may occur only after the wake gesture ends and a new gesture begins.

Examples:

- a button press that wakes the screen MUST NOT also execute its assigned desktop action;
- a five-detent rotary burst that begins while asleep MUST wake the screen but MUST NOT execute ticks 2-5 as an action;
- a continuously spinning rotary wake gesture remains swallowed until at least 250 ms without another detent marks the gesture end;
- after the wake gesture ends, the next new gesture MAY execute normally.

When Kivori is merely Dim or Low Motion, the interaction SHOULD execute normally while restoring full presentation.

### Explicit desktop configuration test commands

`Test Action` and `Preview Buddy State` initiated deliberately from Kivori Desktop are explicit user test intent, not passive background activity and not physical wake gestures.

When the targeted device is in Display Sleep:

- `Preview Buddy State` SHOULD wake the display and render the requested preview without executing an ordinary desktop mapping;
- `Test Action` SHOULD wake the display and MAY execute the requested test immediately when the current assignment, permission, connection, and protection rules permit it;
- these software-originated tests do not require the user to perform a separate physical wake gesture first;
- the physical wake-only rule MUST NOT consume or defer a software-originated test solely because the panel was asleep;
- a test command MUST NOT bypass Protected, permission, connection, device-assignment, or higher-priority takeover restrictions;
- a higher-priority takeover state MAY refuse or interrupt a preview when normal preview presentation is inappropriate.

This keeps configuration testing useful while preserving the distinction between physical wake intent and an explicit software test request.

### Display Sleep and hardware recovery hold

The hardware recovery detector is out-of-band from ordinary desktop button-action handling.

If the primary hardware button is pressed while Kivori is fully in Display Sleep:

- key-down SHOULD wake the display immediately so the device visibly acknowledges the interaction;
- the ordinary configured button action MUST remain suppressed because the gesture began as a wake gesture;
- the hardware recovery hold timer MUST continue running independently after the display wakes;
- incidental rotary movement while the button remains held MUST NOT reset/cancel recovery and MUST NOT execute ordinary rotary actions;
- if the button remains held for the recovery threshold, the MCU MUST reboot according to US10;
- if the user releases before the recovery threshold, the display remains awake and no ordinary desktop action from that press is executed.

Example:

`Display Sleep -> button down -> display wakes -> normal action suppressed -> incidental rotation ignored -> hold reaches ~10 s -> MCU reboot`

Early release example:

`Display Sleep -> button down -> display wakes -> release at 4 s -> no desktop action`

### Urgent desktop wake events

A small explicit class of immediate-attention desktop states MAY wake Display Sleep without physical interaction.

Examples include:

- an incoming call that requires timely user awareness;
- a microphone state transition to active when that transition is considered privacy/safety relevant.

Ordinary background changes MUST NOT wake the fully sleeping display merely because state changed. Examples include routine app-focus changes, media progress, ordinary volume changes, and non-urgent status refreshes.

Urgent wake classification SHOULD be narrow and deterministic. It MUST NOT become a generic notification-wakes-display mechanism.

## Acceptance Criteria

- [ ] Global audio follows the current OS logical endpoint.
- [ ] App-specific audio remains explicitly scoped.
- [ ] Background buzzer feedback respects DND by default.
- [ ] Monitor blanking does not become Sleeping or Disconnected by itself.
- [ ] Display Sleep can fully blank the panel.
- [ ] A wake gesture that starts while asleep is swallowed in its entirety.
- [ ] A multi-tick rotary wake gesture does not partially execute desktop actions.
- [ ] Continuous rotary wake input remains wake-only until 250 ms of rotary inactivity ends the gesture.
- [ ] Explicit Preview Buddy State can wake a sleeping display without a physical wake gesture.
- [ ] Explicit Test Action can wake a sleeping display and execute immediately only when current restrictions permit.
- [ ] Desktop test commands do not bypass Protected/permission/assignment/takeover restrictions.
- [ ] A recovery-button key-down wakes the display immediately from Display Sleep.
- [ ] Waking the display does not cancel the recovery hold timer.
- [ ] Incidental rotary detents during the recovery hold do not cancel recovery or execute normal rotary actions.
- [ ] Releasing a recovery-capable wake press before ~10 s does not execute the ordinary mapped button action.
- [ ] Passive incidental events do not wake Display Sleep by default.
- [ ] Narrowly defined urgent desktop states may wake the display.
- [ ] Ordinary background state changes do not wake Display Sleep.

---

# US10 — Connection, Power, Recovery, Session Transitions, and Execution Continuity

## User Story

**As a user, I want Kivori to distinguish between normal absence, startup, intentional interruption, sleep, user switching, passive assignment, communication failure, hardware recovery, and host execution state.**

## Contract

### Connected

A usable desktop session exists and this Kivori is assigned for normal use.

### Waiting

Kivori is powered but no usable Kivori Desktop session is available.

Examples:

- wall power;
- secondary PC without Kivori Desktop;
- desktop software intentionally exited;
- current OS user has no Kivori session.

Waiting MUST NOT silently become a driverless macro mode.

### Passive / Unassigned

A usable desktop session exists but this Kivori is not currently assigned as the active controller.

Passive MUST remain distinct from Waiting.

Normal mappings are disabled in Passive, but the hardware recovery gesture MUST remain available whenever firmware can observe it.

### Host Starting / Resuming

The host is booting/waking and Kivori is waiting for the user's desktop component.

Initial grace target: **15-30 seconds**.

If a usable session appears, transition to Connected. If it does not appear after the grace period, Waiting is appropriate.

A failure/disconnect semantic SHOULD apply only after a healthy session had actually been established and was then lost unexpectedly.

### Switching User and privacy boundary

When the OS is known to be transitioning between user sessions, Kivori SHOULD preserve the transition until the next user session is confirmed or the startup grace expires.

At the moment the previous user's ownership ends, Kivori MUST invalidate session-specific presentation data so it cannot be rendered to the next user.

Session-specific presentation includes, at minimum:

- previous-user app/profile names;
- user-specific custom icons/assets currently cached for display;
- transient action feedback tied to the previous session;
- user-specific buddy/secondary state that is not safe as a neutral machine-level state.

The product contract requires that previous-user session data becomes **non-renderable immediately**. Implementations SHOULD clear or invalidate relevant MCU/session buffers, but this product contract does not require forensic memory-erasure guarantees beyond preventing cross-user presentation or reuse.

If User B logs in and no Kivori Desktop session becomes available after the normal grace period, Kivori may transition from Switching User to Waiting only after the previous-user visual/session state has already been invalidated.

### Reconnecting

The desktop service is intentionally restarting, for example during a software update.

Actions MUST NOT be buffered for execution after Reconnecting completes.

Reconnection MUST restore current observable state rather than replaying expired UI feedback from events that occurred while disconnected.

If Reconnecting becomes known during an active ordinary gesture, Reconnecting takeover MUST surface immediately and the active gesture MUST be cancelled according to US3 rather than waiting for gesture-end inactivity.

For host-side jobs:

- a currently running job MAY restore a Running representation after reconnect;
- a job that completed in the past MUST NOT cause a stale Success/Error transient merely because the device reconnected;
- historical results MAY remain available in desktop-side logs/history;
- the original job command MUST NOT be reissued on reconnect.

### Degraded

Communication remains partially alive but is late/unhealthy.

Degraded SHOULD appear as System Health rather than automatically replacing the buddy with Disconnected.

Degraded device transport MUST NOT automatically mean a previously confirmed host-side long-running action has failed.

### Disconnected

A previously healthy device connection is lost unexpectedly.

Examples:

- service crash;
- USB data failure;
- unexpected heartbeat loss.

Disconnected describes device/session communication, not necessarily the lifetime of a host-side process that was already confirmed started.

If Disconnected becomes known during an active ordinary gesture, the gesture MUST be cancelled and Disconnected takeover MUST render immediately; Kivori MUST NOT wait for the 250 ms gesture-end inactivity threshold.

If Kivori later reconnects while the desktop service can still observe that process/job, current execution state MUST be reconciled without reissuing the original command or replaying expired historical transients.

### Sleeping / Locked

The host is known to be sleeping, hibernating, or locked.

Known sleep/lock/session state MUST outrank later generic heartbeat-loss symptoms.

### Hardware recovery

MVP recovery gesture:

**Hold the primary hardware button for approximately 10 seconds -> force MCU reboot.**

Recovery MUST:

- work without Kivori Desktop;
- not depend on healthy USB communication;
- not erase normal configuration;
- remain available when the host PC is frozen or unavailable, provided firmware can still observe the recovery gesture;
- remain available while the device is Passive / Unassigned;
- remain available during Protected/restricted desktop contexts;
- remain available while the display is in Display Sleep;
- bypass normal profile/action suppression because it is an out-of-band device recovery path.

While the primary recovery-capable button remains continuously depressed and the recovery timer is active:

- incidental rotary detents MUST NOT cancel, reset, pause, or restart the recovery timer;
- incidental rotary detents MUST NOT execute ordinary mapped rotary actions;
- incidental rotary detents MUST NOT transfer gesture ownership away from recovery;
- only release of the primary button before threshold ends the recovery attempt under normal operation.

This recovery arbitration takes precedence over the ordinary single-gesture rule because recovery is an escape path rather than a normal configurable input gesture.

When the recovery-capable button is pressed from Display Sleep:

- the display SHOULD wake immediately at key-down;
- ordinary mapped button execution MUST remain suppressed for that wake gesture;
- waking the display MUST NOT reset or cancel the recovery hold timer;
- reaching the recovery threshold MUST reboot the MCU;
- releasing before the threshold MUST NOT retroactively execute the normal mapped button action.

Factory reset MUST use a separate, harder-to-trigger mechanism.

If firmware is so compromised that it cannot observe the recovery gesture, this contract does not claim a software-detectable hold can recover that condition; hardware-level boot/recovery mechanisms may be specified separately.

## Acceptance Criteria

- [ ] Waiting, Passive, Reconnecting, Degraded, and Disconnected remain semantically distinct.
- [ ] Intentional restart does not immediately appear as a crash.
- [ ] Startup/wake receives a grace state.
- [ ] Explicit sleep/lock state outranks heartbeat-loss symptoms.
- [ ] Switching User invalidates previous-user session-specific presentation before another user can see the device.
- [ ] Transition to Waiting after a user switch does not restore previous-user visual state.
- [ ] Degraded/Disconnected transport does not automatically mark confirmed host execution as failed.
- [ ] Reconnection reconciles current host execution without replaying its start command.
- [ ] Reconnection does not replay stale completion transients for historical host jobs.
- [ ] Reconnecting/Disconnected takeover preempts and cancels active normal gestures immediately.
- [ ] Recovery gesture works without desktop communication.
- [ ] Recovery remains available in Passive state.
- [ ] Recovery remains available during Protected/restricted desktop contexts.
- [ ] Recovery remains available in Display Sleep and wakes the display immediately on key-down.
- [ ] Display wake does not interrupt the recovery hold timer.
- [ ] Rotary detents during a push-encoder recovery hold do not cancel/reset recovery or execute normal rotary mappings.
- [ ] Early release of a recovery-capable wake press does not execute the mapped button action.
- [ ] Recovery does not erase configuration.
- [ ] Factory reset is not accidentally triggered by the ordinary recovery gesture.

---

# US11 — Desktop App Configuration, Machine Scope, and Device Assignment

## User Story

**As a user, I want Kivori Desktop to act as the control center while keeping machine, user, and device ownership explicit.**

## Contract

Kivori Desktop SHOULD provide configuration for:

- connected-device visibility;
- active/passive assignment;
- device and system-health status;
- General controls;
- application profiles;
- advanced app targeting;
- explicit action assignment;
- rotary sensitivity;
- acceleration behavior/reset timing;
- global bindings;
- binding conflict detection;
- visual feedback;
- buzzer feedback;
- Do Not Disturb behavior;
- ambient/display-sleep timing;
- configurable display-wake sources where supported;
- custom indicator visibility/order within the allowed custom tier;
- buddy-state preview;
- action testing;
- firmware updates;
- configuration reset.

The desktop configuration window MUST NOT need to remain visible for ordinary background operation.

### Explicit test and preview behavior

User-triggered configuration commands such as `Preview Buddy State` and `Test Action` are explicit test intent.

When the selected Kivori is in Display Sleep:

- Preview Buddy State SHOULD wake the selected device and show the preview without requiring a physical wake gesture;
- Test Action SHOULD wake the selected device and MAY execute immediately when the target action is currently permitted;
- these commands MUST NOT be treated as passive background events;
- these commands MUST NOT bypass Protected state, missing permissions, unavailable connection, assignment restrictions, or other higher-priority takeover rules;
- Preview Buddy State MUST NOT execute an ordinary mapped desktop action merely because it woke the display.

The configuration UI SHOULD make a blocked test distinguishable from a test that was actually dispatched.

### Machine scope

MVP profiles are local per machine and per OS user.

Cloud synchronization is not required.

Future portability MAY distinguish:

**Portable configuration**

- logical application identity;
- action choices;
- preferences.

**Machine-local configuration**

- executable/file paths;
- scripts;
- hardware/device identifiers;
- machine-specific overrides.

### Multiple devices

MVP supports **one Active Kivori per desktop session**.

Additional connected devices remain **Passive / Unassigned**.

When Device B becomes Active:

- Device B becomes Active;
- the previous Device A becomes Passive / Unassigned;
- Device A MUST NOT become Waiting solely because assignment changed;
- Device A MUST NOT silently mirror Device B;
- Device A MUST NOT execute ordinary mappings;
- Device A MUST NOT behave as Monitor Mode;
- Device A MUST retain the hardware recovery gesture.

Future multi-device roles MAY be introduced only through explicit assignment semantics.

## Acceptance Criteria

- [ ] Configuration UI can identify which device is Active.
- [ ] Activating a second device demotes the previous Active device to Passive.
- [ ] Passive devices do not execute ordinary mappings.
- [ ] Passive devices do not mirror ordinary desktop state.
- [ ] Passive devices retain hardware recovery.
- [ ] Preview Buddy State can explicitly wake a selected sleeping device without executing a mapped action.
- [ ] Test Action can explicitly wake a selected sleeping device when testing is otherwise permitted.
- [ ] Test/preview commands do not bypass Protected/permission/connection/assignment restrictions.
- [ ] System/privacy indicator priority cannot be overridden by custom app indicator ordering.
- [ ] Profiles are scoped per machine and OS user in MVP.
- [ ] Desktop background behavior continues without the configuration window being visible.

---

# 4. State Reference

| State | Meaning | Normal controls? |
| --- | --- | --- |
| Connected | Healthy usable desktop session; device assigned | Yes |
| Waiting | Powered, but no usable Kivori Desktop session | No |
| Passive / Unassigned | Desktop session exists; this device is not the active controller | No ordinary mappings; recovery remains available |
| Host Starting / Resuming | Host session is expected to become available | No |
| Switching User | Known OS user-session transition; previous-user session presentation invalidated | No |
| Reconnecting | Intentional temporary service interruption | No deferred execution; immediate takeover; restore current truth only |
| Degraded | Device communication unhealthy but partially alive | Limited/current device actions only; host execution may remain valid |
| Disconnected | Established device/session connection unexpectedly lost | Immediate takeover; no new device actions; confirmed host work may continue independently |
| Sleeping / Locked | Host explicitly inactive/restricted | No normal custom actions |
| Protected | Secure/protected system context | No normal custom actions; recovery remains available |
| Permission Required | Broad capability restriction requiring user intervention | Only unaffected capabilities |
| Firmware Updating | Intentional device firmware update | No normal controls |

# 5. Initial Timing Reference

| Behavior | Initial target |
| --- | ---: |
| Local input acknowledgement | < 50 ms |
| Delayed/processing indication | ~500 ms |
| Ordinary interactive unresolved timeout | ~1,500 ms |
| Success transient | ~800 ms |
| Triggered/Unverified transient | ~1,200 ms |
| Error/Failed transient | ~2,000 ms |
| Passive stable-focus commit | 300-500 ms |
| Rotary acceleration inactivity reset | 250 ms |
| Rotary gesture-end inactivity | 250 ms |
| Lower-priority indicator promotion stabilization | ~250 ms |
| Host startup/resume grace | 15-30 s |
| Hardware recovery hold | ~10 s |

These values are validation targets, not protocol constants. Tuning them MUST preserve the semantics defined by the relevant user story.

A deliberate Kivori interaction is not required to wait out the passive 300-500 ms focus-stabilization target when a valid pending foreground application is known; that interaction commits the pending context immediately after security/protection classification.

A direction reversal resets acceleration immediately even when the prior direction has not been idle for 250 ms.

Acceleration builds only while detents continue in the same direction; repeated direction flips therefore repeatedly restart at baseline.

Initial MVP rotary acceleration multiplier ceiling: **5x the action's base step**. Actions may choose a lower ceiling or disable acceleration, but MUST NOT exceed 5x in MVP.

A rotary wake gesture remains one wake-only gesture until the 250 ms rotary gesture-end inactivity target is reached.

Lower-priority secondary indicators use an initial ~250 ms promotion-stabilization target; higher-priority escalation is not delayed by that timer.

# 6. Default Priority References

## Transient Reaction Priority

1. Urgent immediate-attention state;
2. newest active user-interaction feedback;
3. ordinary unsolicited/background transient.

A new deliberate interaction replaces older interaction feedback rather than waiting in a visual queue.

Reconnection does not replay expired historical action transients.

## Secondary Indicator Priority

1. Microphone state;
2. Call Active;
3. Master Audio Mute;
4. Media Playing/Active;
5. custom app-specific indicators.

Custom indicator settings MAY reorder or hide indicators within the custom tier but MUST NOT displace protected system/privacy indicators.

Priority escalation SHOULD appear immediately. When a higher-priority indicator disappears and exposes a lower-priority indicator, lower-priority promotion SHOULD use the initial ~250 ms stabilization window before reflowing the layout.

# 7. MVP Boundaries

The following are explicitly outside this contract's MVP guarantees:

- cloud profile synchronization;
- independent virtual-pet progression;
- generic news/weather/dashboard behavior;
- workflow orchestration as the flagship use case;
- automatic inference of ambiguous sub-app identity;
- implicit simultaneous-input chords;
- acceleration above 5x the configured action base step;
- replaying physical input after communication recovery;
- delaying Reconnecting/Disconnected takeover presentation until an active rotary gesture naturally ends;
- replaying historical Success/Error presentation transients after reconnection;
- automatic restart of an uncertain long-running host action after device reconnection;
- silent retargeting of a gesture after its committed application disappears;
- continuing application-scoped execution after known target loss within the same gesture;
- process injection, anti-cheat hooks, or hidden game instrumentation solely to preserve profile ownership across transient overlays;
- multiple simultaneously Active Kivori controllers;
- Passive devices acting as Monitor Mode;
- driverless macro/media fallback without Kivori Desktop;
- host wake-from-Kivori;
- visibility into physical device states that are not exposed through a trustworthy signal;
- fully user-defined priority that can demote system/privacy indicators below custom app indicators;
- forensic guarantees about erasing every historical RAM byte during local OS-user switching, beyond the requirement that previous-user session state becomes non-renderable and non-reusable.

# 8. Change Control

Any implementation or future feature that intentionally violates a MUST-level rule in this contract should update this contract and the root PRD in the same product decision/PR, including the reason for the behavior change.

Implementation-specific timing, APIs, protocols, storage formats, or OS adapters may evolve without changing this contract as long as the user-observable behavior remains compliant.