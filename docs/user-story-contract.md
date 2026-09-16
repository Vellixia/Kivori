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
9. **Gesture ownership.** A physical gesture remains bound to the context in which it began.
10. **One gesture owns input in MVP.** Simultaneous inputs MUST NOT implicitly become combo triggers.
11. **Wake gestures are atomic.** A gesture that begins while the display is fully asleep MUST be consumed as wake-only in its entirety.
12. **User isolation.** One OS user's mappings MUST NOT remain active in another user's session.
13. **No hidden fallback.** If an action cannot run in the current context, Kivori MUST communicate unavailability rather than silently switch mechanisms.
14. **Localized uncertainty.** Unknown/restricted state SHOULD propagate only as far as necessary.
15. **Known host/session state wins.** Explicit sleep, lock, switching-user, update, or restart state outranks generic communication symptoms.
16. **Passive is not Waiting.** Passive means a usable desktop exists but this device is not assigned as the controller.
17. **Passive is not Monitor Mode.** Passive devices MUST NOT mirror ordinary real-time desktop state in MVP.
18. **Hardware recovery is out-of-band.** Basic MCU recovery MUST remain available independently of desktop software, profile assignment, and normal action restrictions whenever firmware can still observe the recovery gesture.
19. **Interaction feedback has priority over ordinary background transients.** Ordinary unsolicited reactions SHOULD NOT interrupt active user-action feedback.
20. **Urgent state may wake the display.** A small defined class of immediate-attention desktop states MAY wake Display Sleep; ordinary background changes MUST NOT.
21. **System/privacy indicators outrank custom indicators.** User customization MUST NOT allow app-specific indicators to displace higher-priority safety/privacy state.
22. **Continuous gestures reconcile at gesture boundaries.** External continuous-value updates SHOULD NOT visually fight an active Kivori rotary gesture; final confirmed state MUST reconcile when the gesture ends.

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

### Bounded values

When a bounded action reaches its minimum or maximum:

- the value MUST clamp immediately;
- the first continued tick beyond the limit MAY produce one subtle boundary reaction;
- repeated ticks farther into the same boundary SHOULD NOT spam visual or buzzer feedback;
- the first reverse tick MUST take effect immediately;
- reversing direction MUST NOT require the user to pause or end the gesture.

Example:

`98 -> 99 -> 100 -> extra clockwise ticks ignored -> first counter-clockwise tick -> 99`

## Acceptance Criteria

- [ ] Assigned actions expose explicit action identity/scope.
- [ ] App profiles select actions without mutating action meaning.
- [ ] Rotary actions can define action-specific sensitivity and acceleration.
- [ ] Bounded controls clamp correctly.
- [ ] Boundary feedback is not repeated for every excess tick.
- [ ] Reverse movement exits boundary suppression immediately.

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

### Transient reaction targets

Initial display targets:

- Success: ~800 ms;
- Triggered / Unverified: ~1.2 s;
- Error / Failed: ~2 s.

A transient reaction MUST return to the actual underlying state rather than blindly returning to Idle.

A new valid interaction MAY immediately replace a transient reaction with feedback for the new interaction.

Takeover states MUST NOT be dismissed merely because another physical input occurred.

### Transient collision priority

When an ordinary unsolicited background transient occurs while user-action feedback is active:

- the active interaction feedback SHOULD finish first;
- the ordinary background transient MAY be queued and shown afterward if still relevant;
- ordinary background feedback MUST NOT silently overwrite active user-action feedback.

A defined urgent state MAY preempt user-action feedback when immediate attention is more important than preserving the transient.

Default transient priority is:

**Urgent state -> active interaction feedback -> ordinary unsolicited transient**

## Acceptance Criteria

- [ ] Local acknowledgement is distinguishable from action success.
- [ ] State Confirmed, Execution Confirmed, and Unverified outcomes remain distinct.
- [ ] Timeout logic distinguishes known failure from unknown outcome.
- [ ] A target app crash during a pending action becomes Error when the crash is known.
- [ ] Missing callback without known failure becomes Unverified.
- [ ] Transient reactions restore the underlying current state.
- [ ] Ordinary background errors do not interrupt active user-action success feedback.
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

### Gesture context ownership

A gesture MUST remain attached to the profile/context that was active when the gesture began.

If application focus changes during a rotary burst, the current rotary burst MUST continue using its starting context. The newly focused profile becomes eligible for the next gesture.

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

### Interrupted communication

Kivori MUST NOT queue stale user intent for later replay.

During **Reconnecting**:

- local acknowledgement MAY still occur;
- desktop action MUST NOT be deferred for later execution;
- old intent MUST be discarded.

During **Degraded** communication:

- an action MAY be attempted while communication is still usable;
- unresolved/expired actions MUST NOT execute later after recovery.

For a currently active continuous gesture, the system MAY retain the latest target while the gesture and connection are still valid, but MUST NOT replay the historical tick sequence later.

## Acceptance Criteria

- [ ] Rapid rotary use does not require one full confirmation round trip per tick.
- [ ] Final display reconciles against confirmed state.
- [ ] Focus change mid-gesture does not split one gesture across profiles.
- [ ] External updates do not visually fight the same continuous value during an active gesture.
- [ ] Important discrete state changes may still surface during a continuous gesture.
- [ ] Implicit chord actions are not produced in MVP.
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

### Interaction commits pending focus

Focus stabilization protects against passive focus flicker; it MUST NOT cause deliberate Kivori input to execute against a stale foreground application.

If Application B currently owns valid OS foreground focus but remains inside the passive stabilization window, a deliberate Kivori interaction MUST:

1. commit Application B as the interaction context immediately;
2. select Application B's profile if one exists, otherwise the appropriate fallback profile;
3. bind the complete gesture to that committed context.

The action MUST NOT execute under previously committed Application A solely because the stabilization timer has not expired.

General MUST NOT be used merely as an intermediate race-condition profile when a valid pending foreground app is known.

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
- one user's mappings MUST NOT carry into another user's desktop session.

MVP operation is scoped to the user's normal interactive session rather than requiring the entire Kivori desktop application to run permanently as SYSTEM/root.

## Acceptance Criteria

- [ ] Mouse hover does not switch profiles.
- [ ] Passive focus changes require stable OS focus before visible profile commitment.
- [ ] Very brief focus theft does not produce a committed profile swap.
- [ ] Deliberate interaction during the stabilization window commits the valid pending foreground app before execution.
- [ ] The previous app profile is not used for a new gesture merely because the stabilization timer is still running.
- [ ] Background apps do not take physical control ownership.
- [ ] Generic-host sub-app behavior is explicit rather than guessed.
- [ ] User A mappings cannot remain active after User B becomes the active user.

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

### Switching User

When the OS is known to be transitioning between users, Kivori SHOULD preserve a Switching User/Locked transition state rather than briefly falling through Waiting.

Transition ends when:

- the next user's Kivori session is confirmed; or
- the normal startup grace period expires without a usable session, after which Waiting is appropriate.

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

Active profile information SHOULD normally appear transiently after a profile change rather than occupy a permanent indicator slot.

## Acceptance Criteria

- [ ] Takeover states cannot be confused with normal buddy state.
- [ ] Degraded health has distinct visual allocation from secondary indicators.
- [ ] Passive displays do not mirror ordinary desktop status in MVP.
- [ ] Passive assignment does not disable MCU recovery.
- [ ] Fast User Switching does not unnecessarily flash through Waiting.
- [ ] Firmware update does not look like an unexplained disconnect.
- [ ] Mic and other protected system/privacy indicators cannot be displaced by custom app indicators.
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

## Acceptance Criteria

- [ ] One unknown secondary state does not force whole-device Unknown.
- [ ] Confirmed states remain visible when unrelated state is uncertain.
- [ ] Whole-buddy Unknown is reserved for primary-state uncertainty.
- [ ] Permission/capability loss follows the same localization principle.

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

Examples:

- a button press that wakes the screen MUST NOT also execute its assigned desktop action;
- a five-detent rotary burst that begins while asleep MUST wake the screen but MUST NOT execute ticks 2-5 as an action;
- after the wake gesture ends, the next new gesture MAY execute normally.

When Kivori is merely Dim or Low Motion, the interaction SHOULD execute normally while restoring full presentation.

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
- [ ] Passive incidental events do not wake Display Sleep by default.
- [ ] Narrowly defined urgent desktop states may wake the display.
- [ ] Ordinary background state changes do not wake Display Sleep.

---

# US10 — Connection, Power, Recovery, and Session Transitions

## User Story

**As a user, I want Kivori to distinguish between normal absence, startup, intentional interruption, sleep, user switching, passive assignment, communication failure, and hardware recovery.**

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

### Switching User

When the OS is known to be transitioning between user sessions, Kivori SHOULD preserve the transition until the next user session is confirmed or the startup grace expires.

### Reconnecting

The desktop service is intentionally restarting, for example during a software update.

Actions MUST NOT be buffered for execution after Reconnecting completes.

### Degraded

Communication remains partially alive but is late/unhealthy.

Degraded SHOULD appear as System Health rather than automatically replacing the buddy with Disconnected.

### Disconnected

A previously healthy connection is lost unexpectedly.

Examples:

- service crash;
- USB data failure;
- unexpected heartbeat loss.

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
- bypass normal profile/action suppression because it is an out-of-band device recovery path.

Factory reset MUST use a separate, harder-to-trigger mechanism.

If firmware is so compromised that it cannot observe the recovery gesture, this contract does not claim a software-detectable hold can recover that condition; hardware-level boot/recovery mechanisms may be specified separately.

## Acceptance Criteria

- [ ] Waiting, Passive, Reconnecting, Degraded, and Disconnected remain semantically distinct.
- [ ] Intentional restart does not immediately appear as a crash.
- [ ] Startup/wake receives a grace state.
- [ ] Explicit sleep/lock state outranks heartbeat-loss symptoms.
- [ ] Recovery gesture works without desktop communication.
- [ ] Recovery remains available in Passive state.
- [ ] Recovery remains available during Protected/restricted desktop contexts.
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
| Switching User | Known OS user-session transition | No |
| Reconnecting | Intentional temporary service interruption | No deferred execution |
| Degraded | Communication unhealthy but partially alive | Limited/current actions only; no stale replay |
| Disconnected | Established connection unexpectedly lost | No |
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
| Rotary acceleration reset | 250 ms |
| Host startup/resume grace | 15-30 s |
| Hardware recovery hold | ~10 s |

These values are validation targets, not protocol constants. Tuning them MUST preserve the semantics defined by the relevant user story.

A deliberate Kivori interaction is not required to wait out the passive 300-500 ms focus-stabilization target when a valid pending foreground application is known; that interaction commits the pending context immediately.

# 6. Default Priority References

## Transient Reaction Priority

1. Urgent immediate-attention state;
2. active user-interaction feedback;
3. ordinary unsolicited/background transient.

## Secondary Indicator Priority

1. Microphone state;
2. Call Active;
3. Master Audio Mute;
4. Media Playing/Active;
5. custom app-specific indicators.

Custom indicator settings MAY reorder or hide indicators within the custom tier but MUST NOT displace protected system/privacy indicators.

# 7. MVP Boundaries

The following are explicitly outside this contract's MVP guarantees:

- cloud profile synchronization;
- independent virtual-pet progression;
- generic news/weather/dashboard behavior;
- workflow orchestration as the flagship use case;
- automatic inference of ambiguous sub-app identity;
- implicit simultaneous-input chords;
- replaying physical input after communication recovery;
- multiple simultaneously Active Kivori controllers;
- Passive devices acting as Monitor Mode;
- driverless macro/media fallback without Kivori Desktop;
- host wake-from-Kivori;
- visibility into physical device states that are not exposed through a trustworthy signal;
- fully user-defined priority that can demote system/privacy indicators below custom app indicators.

# 8. Change Control

Any implementation or future feature that intentionally violates a MUST-level rule in this contract should update this contract and the root PRD in the same product decision/PR, including the reason for the behavior change.

Implementation-specific timing, APIs, protocols, storage formats, or OS adapters may evolve without changing this contract as long as the user-observable behavior remains compliant.
