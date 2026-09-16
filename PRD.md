# Kivori Product Requirements Document

**Status:** Product baseline  
**Date:** 2026-09-16  
**Behavioral contract:** [`docs/user-story-contract.md`](docs/user-story-contract.md)

## 1. Product Summary

Kivori is a physical desktop companion that lets people control their computer through tactile inputs while visually representing important desktop state.

The product is built around a two-way relationship:

- **User -> Kivori -> Desktop**: the user performs a physical action and Kivori requests a desktop action.
- **Desktop -> Kivori -> User**: the desktop reports observable state and Kivori represents it through the buddy, overlays, indicators, and feedback.

Kivori is not primarily a virtual pet, notification dashboard, or generic macro pad. The buddy exists to make desktop state understandable, glanceable, and personal.

> **Product thesis:** Control the desktop physically. Understand the desktop visually.

## 2. Product Goal

Kivori should make frequent desktop interactions faster and more tangible without sacrificing trust or predictability.

The product should:

1. provide useful physical controls without requiring the user to leave their current task;
2. represent important desktop state without requiring another window to be opened;
3. acknowledge physical interaction immediately;
4. distinguish requested state from confirmed desktop state;
5. adapt controls to context without surprising the user;
6. remain understandable during disconnects, permissions problems, sleep, updates, and other edge conditions;
7. make waiting, processing, recovery, restriction, and failure states visibly intentional rather than appearing frozen or unresponsive.

The buddy represents the desktop. It should not claim a state that Kivori cannot observe or verify.

## 3. User Problem

Desktop users repeatedly perform small actions such as changing volume, muting a microphone, controlling media, launching commands, or triggering application actions. Existing options create tradeoffs:

- keyboard shortcuts are fast but provide little persistent physical feedback;
- software controls require visual attention and window switching;
- macro pads can trigger commands but often do not represent the resulting state;
- status information is fragmented across applications, tray icons, menus, and overlays.

Kivori addresses this by combining **physical control** and **persistent visual feedback** in the same desk object.

## 4. Core Experience

The canonical interaction loop is:

1. the desktop has an observable state;
2. Kivori represents that state;
3. the user interacts physically;
4. Kivori acknowledges the input immediately;
5. the desktop executes the action when permitted;
6. Kivori confirms only what can actually be verified;
7. Kivori reconciles the display with the real desktop state.

A responsive preview may appear before confirmation, but confirmed desktop state remains authoritative.

## 5. Product Principles

### 5.1 Desktop truth wins

Kivori represents observable desktop truth, regardless of whether a change originated from Kivori, keyboard, mouse, OS controls, application UI, or supported hardware.

### 5.2 Acknowledgement is not confirmation

Kivori may react immediately to input, but must not present an action as successful until the appropriate level of success can be observed.

### 5.3 Explicit intent beats inferred intent

Application context follows stable OS focus, not mouse hover or momentary focus theft. A physical gesture belongs to the context in which it started.

### 5.4 Predictability beats maximum flexibility

Kivori should avoid silent overrides, hidden fallbacks, replayed stale actions, implicit multi-device mirroring, or ambiguous context changes.

### 5.5 Uncertainty stays local

If one state cannot be verified, only that state should become unknown when possible. One uncertain capability should not make the entire device appear broken.

### 5.6 Recovery must remain available

Hardware recovery should not depend on the desktop software or host computer being healthy.

### 5.7 Offline-first core

Core control, state representation, configuration, and device operation should not require cloud connectivity. Cloud profile synchronization is not required for MVP.

### 5.8 No ambiguous silence

Whenever Kivori is powered and capable of rendering, a user-relevant waiting, processing, restricted, transitional, failure, or recovery condition should have an intentional visual representation.

A blank, static, or unchanged presentation must not accidentally look like a frozen device. Motion may communicate that Kivori is alive and working, while the buddy, iconography, text, or state treatment communicates what the device is doing.

If progress cannot be measured truthfully, Kivori should show indeterminate activity rather than inventing a percentage or completion estimate.

Deliberate Display Sleep is the primary normal exception: the panel may be blank because blanking is itself the intended presentation. Low-level states in which the renderer is technically unavailable may also temporarily lack the normal visual surface, but Kivori should use any reliable hardware indication available and Kivori Desktop should communicate the state where possible.

## 6. Primary Product Capabilities

### 6.1 Physical desktop control

Kivori supports explicit actions such as:

- system volume;
- media play/pause;
- microphone mute;
- keyboard shortcuts;
- application launch;
- scripts and macros;
- application-specific actions.

Actions have explicit scope. For example, `System Volume` and `Discord Volume` are different actions.

### 6.2 Application-aware profiles

The active application profile follows stable OS keyboard/window focus. The General profile applies when no application-specific profile is active.

Profiles can define different actions for the same physical input, but they do not silently change the meaning of an action.

### 6.3 Desktop state representation

The buddy and indicators may represent states such as:

- idle or active desktop;
- busy, success, error, or unknown primary state;
- microphone mute;
- master audio mute;
- media or call activity;
- connection/system health;
- restricted or unavailable capabilities.

### 6.4 Responsive physical feedback

Physical input should receive immediate local acknowledgement. The display may show local previews for continuous controls, then reconcile them against confirmed state.

### 6.5 Desktop configuration

Kivori Desktop is the configuration center for:

- General controls;
- application profiles;
- explicit actions;
- rotary sensitivity and acceleration;
- global bindings;
- visual and buzzer feedback;
- ambient/display behavior;
- application targeting;
- device assignment;
- firmware updates.

The configuration window does not need to remain visible during normal use.

## 7. Interaction Model

### 7.1 Input model

MVP physical input is based on a rotary encoder with press behavior.

MVP uses one active gesture at a time. Simultaneous controls are not interpreted as implicit chords. Future combinations such as `Hold + Rotate` must be introduced as explicit input types.

### 7.2 Continuous controls

Rotary-capable actions may define:

- range;
- step size;
- sensitivity;
- optional acceleration;
- acceleration reset interval.

The default acceleration reset target is **250 ms** without a new detent.

Bounded values clamp immediately. Excess movement at a boundary may receive one subtle reaction, and reversing direction takes effect immediately.

### 7.3 Confirmation model

Kivori distinguishes:

- **State Confirmed**: resulting state is directly observable;
- **Execution Confirmed**: operation is known to have started or completed, but no persistent state exists;
- **Triggered / Unverified**: action was dispatched but final effect cannot be verified.

For ordinary interactive actions, initial product targets are:

- local acknowledgement: **< 50 ms**;
- delayed/processing indication: around **500 ms**;
- unresolved timeout: around **1,500 ms**.

Known failure becomes Error. Unknown outcome becomes Unverified.

## 8. Context Model

### 8.1 Active application

Application context is based on stable OS keyboard/window focus.

Mouse hover does not switch profiles. Momentary focus changes should not immediately switch profiles. Initial focus stabilization target: **300-500 ms**.

### 8.2 Gesture ownership

A gesture belongs to the context in which it began. If focus changes mid-rotation, the entire current gesture remains bound to its starting profile. The new profile applies to the next interaction.

### 8.3 Active OS user

Configuration belongs to the active OS user. One user's mappings must never carry into another user's desktop session.

Kivori Desktop is expected to operate in the user's normal session for MVP rather than requiring the entire product to run as a permanently privileged machine-wide service.

### 8.4 Machine scope

MVP profiles are local per machine and per OS user. Machine-local paths, scripts, device identities, and application installations are not cloud-synchronized.

## 9. Visual State Model

Kivori uses four visual layers.

Every user-relevant state should look intentional. When Kivori can render, transitions such as Waiting, Reconnecting, Processing, Protected, Permission Required, Firmware Updating, recovery, and known failure should not be represented by unexplained visual silence or a frozen-looking frame.

When exact progress is unavailable, Kivori should prefer truthful indeterminate activity over invented progress percentages.

### 9.1 Takeover state

Takeover states replace normal desktop representation when normal interaction is unavailable or intentionally suspended.

Examples:

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

### 9.2 System health

System health indicates whether current desktop information can be trusted.

- Healthy does not require persistent screen space.
- Degraded receives dedicated visual priority and does not consume an ordinary secondary-indicator slot.

### 9.3 Primary buddy state

The buddy represents the main current desktop condition, such as:

- Idle;
- Active;
- Busy;
- Success;
- Error;
- Unknown.

### 9.4 Secondary indicators

Secondary indicators represent simultaneous supporting state, such as microphone mute, master audio mute, media activity, call activity, or application-specific state.

The UI should prioritize a small number of meaningful indicators instead of shrinking indefinitely to display every possible status.

## 10. Connection and Session State Model

### Connected

A usable desktop session exists and this Kivori is assigned for normal use.

### Waiting

Kivori has power but no usable Kivori Desktop session is available. Examples include wall power, a secondary PC without Kivori Desktop, or an intentionally closed desktop app.

### Passive / Unassigned

A desktop session exists, but this Kivori is not the active assigned controller. Passive is not Waiting and is not a visual Monitor Mode.

### Host Starting / Resuming

The host is starting or waking and Kivori is allowing the user-session software time to become available. Initial grace target: **15-30 seconds**.

### Switching User

The OS is transitioning between user sessions. Kivori should preserve the transition state until the next user session is confirmed or determined unavailable.

### Reconnecting

The desktop service is intentionally restarting, such as during an update. Inputs are not buffered for later execution.

### Degraded

Communication is unhealthy but partially available.

### Disconnected

A previously healthy connection disappears unexpectedly.

### Sleeping / Locked

The host is known to be sleeping, hibernating, or locked. Known host/session state outranks generic communication-loss symptoms.

## 11. Restricted and Permission-Limited Operation

Kivori must distinguish connection failure from capability restriction.

When Kivori Desktop remains connected but an OS permission is missing or revoked:

- unaffected capabilities continue working;
- affected capabilities become unavailable;
- blocked actions are not repeatedly retried;
- a broad restriction may escalate to a Permission Required takeover state.

Normal custom actions are suspended in protected or locked contexts for MVP.

## 12. Display Idle and Burn-In Protection

If the host remains awake while its monitor blanks, Kivori remains Connected and changes presentation rather than connection state.

Presentation may progress through:

**Normal -> Dim -> Low Motion -> Display Sleep**

Protection may include lower brightness, reduced animation, subtle pixel repositioning, and eventual full display blanking.

When fully in Display Sleep, only deliberate configured controls wake the display by default. The first deliberate interaction wakes the display only and does not execute the assigned desktop action. In Dim or Low Motion states, interaction executes normally while restoring full presentation.

Display Sleep is intentionally blank; it must not be confused with an unexplained blank caused by a normal waiting, processing, or error condition.

## 13. Feedback and Do Not Disturb

Kivori distinguishes user-initiated feedback from unsolicited background feedback.

- user-initiated acknowledgement follows Kivori's own feedback settings;
- background buzzer feedback respects OS Do Not Disturb / Focus Mode by default.

## 14. Device Recovery and Firmware Update

MVP should provide a software-independent recovery gesture:

**Hold the primary hardware button for approximately 10 seconds -> force MCU reboot.**

Recovery must not erase configuration. Factory reset must use a separate, harder-to-trigger procedure.

Recovery should visibly acknowledge the hold and, where practical, communicate that the device is progressing toward reboot rather than appearing stuck.

Firmware update is an intentional takeover state and must not look like an unexplained connection failure. While the normal renderer is available, the device should show an update/activity presentation. If exact firmware-update progress is not trustworthy, Kivori should show indeterminate activity rather than fake percentage progress. If the renderer becomes unavailable during a low-level update, Kivori should use the simplest reliable hardware indication available, while Kivori Desktop shows update state/progress where possible.

## 15. Multi-Device Behavior

MVP supports **one Active Kivori per desktop session**.

Additional connected units remain **Passive / Unassigned**. They must not silently mirror desktop state, execute mappings, or become visual-only monitors.

Future multi-device support may introduce explicit roles, including a separate Monitor Mode, but this is outside MVP.

## 16. MVP Scope

MVP should prove the product loop rather than maximize integrations.

Required product capabilities:

- one Kivori connected to one active desktop session;
- rotary + button physical input;
- General profile and application-aware profiles;
- explicit global and application actions;
- core controls such as system volume, media, microphone mute, and configured desktop actions;
- immediate input acknowledgement and honest confirmation semantics;
- real desktop state synchronization where observable;
- buddy + takeover + system-health + secondary-indicator presentation model;
- intentional visual feedback for user-relevant waiting, processing, restriction, transition, failure, and recovery states;
- predictable focus and gesture ownership;
- connection/session recovery states;
- desktop configuration UI;
- local per-user, per-machine configuration;
- safe display-idle behavior;
- software-independent MCU reboot gesture.

## 17. MVP Non-Goals

The following are intentionally outside the MVP product contract:

- cloud profile synchronization;
- independent virtual-pet progression;
- generic news/weather/crypto dashboard features;
- workflow orchestration as a flagship concept;
- automatic inference of arbitrary web apps or scripts from window titles;
- implicit physical-input chords;
- replaying actions after a connection recovers;
- multiple simultaneously active Kivori controllers;
- Passive devices acting as Monitor Mode;
- driverless macro/media fallback when Kivori Desktop is unavailable;
- waking the host PC through Kivori;
- guaranteeing visibility into hardware states the OS/device does not expose.

## 18. Product Acceptance Gates

A product increment is aligned with this PRD only if it preserves these invariants:

1. **No false confirmation:** Kivori never presents unverified action outcome as confirmed success.
2. **No stale replay:** physical intent that expires during interruption is not replayed later.
3. **No silent context mutation:** app/profile changes are based on stable, explicit context.
4. **No cross-user leakage:** one OS user's configuration never remains active for another user.
5. **No hidden execution fallback:** unavailable actions do not silently switch to another mechanism.
6. **No state invention:** Kivori represents observable state only.
7. **No passive-device ambiguity:** Passive devices do not behave like Active or Monitor devices.
8. **Recoverability:** basic hardware recovery remains possible without healthy desktop software.
9. **No ambiguous silence:** when Kivori can render, user-relevant waiting, processing, restricted, transitional, failure, and recovery states have intentional visual feedback rather than appearing frozen or accidentally blank.

Detailed normative behavior and acceptance criteria live in [`docs/user-story-contract.md`](docs/user-story-contract.md).
