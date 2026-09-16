# Research: Kivori Product Technical Implementation

**Date**: 2026-09-16  
**Status**: Technical research and suggested implementation guidance  
**Product requirements**: [`../PRD.md`](../PRD.md)  
**Behavior contract**: [`./user-story-contract.md`](./user-story-contract.md)  
**Current foundation**: [`./architecture.md`](./architecture.md), [`../specs/001-device-connection-foundation/research.md`](../specs/001-device-connection-foundation/research.md)

**Purpose**: Research technical approaches that can implement the Kivori PRD and User Story Contract on the current repository foundation, with explicit consideration for Windows, macOS, Linux/X11, Linux/Wayland, ESP32-C3 firmware, release/update delivery, hardware recovery, and validation constraints. Each research item is stated as **Decision · Rationale · Alternatives · Caveats**, following the style of the Feature 001 research document.

This document is **non-normative technical guidance**. The PRD and User Story Contract define what users must be able to rely on. A technical recommendation here is a researched starting point, not an immutable implementation mandate. It may be challenged or replaced during implementation when platform documentation, prototype results, hardware measurements, security findings, maintainability evidence, dependency constraints, or automated/manual testing demonstrate a better approach, provided the relevant PRD/User Story behavior remains satisfied. Material architectural deviations should be recorded in an ADR or by updating this research.

Recommendation labels used below:

- **CURRENT** — already present in the repository and supported by current code/evidence.
- **SUGGESTED** — preferred starting implementation based on current evidence.
- **CANDIDATE** — plausible alternative that remains worth evaluating.
- **SPIKE REQUIRED** — evidence is insufficient to freeze a technical choice.
- **PLATFORM-LIMITED** — the operating system/runtime does not expose equivalent capability everywhere.
- **REJECT FOR NOW** — conflicts with current product constraints or adds unjustified complexity today.

---

## R-1. Preserve the Feature 001 foundation; evolve around it

**Decision**: **SUGGESTED** — preserve the current split workspaces, shared `no_std` crates, deterministic renderer, USB protocol, device session code, least-privilege Tauri boundary, and testable firmware ports. Add the product-level architecture around these pieces rather than replacing them.

**Rationale**: The repository already has useful hard boundaries: firmware is isolated from the host Cargo workspace; `kivori-model`, `kivori-protocol`, `kivori-framebuffer`, `kivori-renderer`, and `kivori-assets` are shared by host and device; the firmware core is written against `Clock`, `Transport`, and `DisplaySink`; and the desktop native core owns serial communication instead of the React webview. These choices directly support deterministic visuals, simulation, privacy, and portable host logic.

**Alternatives**: Rewrite around a new async firmware framework, replace Tauri, or collapse host/device models into one application-specific crate. Rejected for now because none resolves a demonstrated blocker and each would discard tested foundations.

**Caveats**: Feature 001 solves device connection and canonical rendering, not the whole product contract. Existing state types and the current `Orchestrator` must not be expanded into catch-all product state merely because they already exist.

## R-2. Keep the Tauri desktop application as the per-user host process

**Decision**: **SUGGESTED** — continue using the Tauri v2 native Rust process as the per-user Kivori host for MVP. Do not introduce a privileged machine-wide service/daemon solely to implement product features.

**Rationale**: The current device thread already survives window hide/reload because its lifetime is tied to the native process rather than the webview. Tauri provides the system/webview separation Kivori needs: privileged OS/device integration remains in Rust while React receives a narrow typed IPC surface. Optional start-at-login can be implemented using Tauri's autostart support rather than a custom service.

**Alternatives**: A machine-wide broker/service that owns all Kivori hardware and serves per-user clients. This could become justified later if Fast User Switching, cross-session update coordination, or hardware ownership cannot be made reliable with per-user processes.

**Caveats**: Multi-user machines create a real ownership problem: several logged-in users can have Kivori processes alive simultaneously. R-7 defines a `SessionOwnershipGate`; real Fast User Switching tests must decide whether a broker is eventually necessary.

## R-3. Build a modular AppCore rather than growing `device_task` into a god loop

**Decision**: **SUGGESTED** — introduce product-level modules inside `apps/desktop/src-tauri` while interfaces stabilize: `ContextEngine`, `BindingResolver`, `ActionEngine`, `ExecutionTracker`, `PresentationResolver`, `DeviceRegistry`, `SessionOwnershipGate`, `ConfigStore`, and `UpdateCoordinator`.

**Rationale**: The current `device_task` correctly owns one serial session and the current `Orchestrator` correctly owns one desired companion state. The User Story Contract now includes focus commitment, platform permissions, profiles, macros, long-running jobs, multi-device assignment, display policy, update delivery, and four-layer presentation. Putting all of those into the connection actor would tightly couple unrelated lifecycles and make transport loss appear to invalidate host-side work.

**Alternatives**: Immediately split each concept into a separate Cargo crate. Rejected for now; module boundaries should prove themselves first. Crates are justified later when dependency isolation, compile boundaries, reuse, or testing benefit is concrete.

**Caveats**: “Modular AppCore” is a logical architecture, not a requirement to use an actor framework or a particular event-bus library. Direct typed calls, bounded channels, or task-owned state are all acceptable if invariants remain testable.

## R-4. Model product truth as orthogonal state axes

**Decision**: **SUGGESTED** — keep independent state domains rather than creating one giant `KivoriState` enum. At minimum separate transport, host session, device assignment, restriction/permission, execution, display power, update, persistent indicators, and transient feedback.

**Rationale**: Multiple facts can be true simultaneously. For example, transport may be Degraded while a host build remains Running, the microphone is muted, the display is Dim, and the device remains Assigned. A single enum would either explode combinatorially or discard truth.

**Alternatives**: Expand the existing `ConnectionState` with product states such as `Protected`, `Passive`, `FirmwareUpdating`, and `Running`. Rejected because those are different axes and because the existing transport FSM is intentionally narrow and pure.

**Caveats**: The exact Rust type layout is challengeable. The important technical property is that unrelated state lifecycles remain independently representable and can be reconciled by presentation logic.

## R-5. Use a pure PresentationResolver for visual priority and “no ambiguous silence”

**Decision**: **SUGGESTED** — implement a deterministic function conceptually equivalent to `PresentationResolver(ProductSnapshot) -> PresentationSnapshot`, with the User Story four-layer hierarchy encoded as explicit rules and tests.

**Rationale**: Kivori's UX requirements are priority/reconciliation rules, not ad-hoc animation calls. A pure resolver can prove cases such as: a disconnect takeover preempts a gesture immediately; a Volume transient expires back into a still-running build; urgent indicators replace lower-priority indicators immediately; lower-priority promotion waits the stabilization window; Display Sleep is intentionally blank while Waiting is not.

**Alternatives**: Let each subsystem directly command buddy state or draw overlays. Rejected because independent subsystems would race and stale transients could overwrite current truth.

**Caveats**: Firmware also owns local truth that exists below Desktop, such as Booting, recovery hold/progress, low-level firmware update phases, Display Sleep, and host-link absence. The final device presentation may therefore be a merge of Desktop semantic presentation and local firmware-owned overrides.

## R-6. Report platform capability explicitly instead of pretending OS feature parity

**Decision**: **SUGGESTED** — define runtime capability states such as `Supported`, `SupportedWithPermission`, `SupportedWhenBackendAvailable`, `TriggeredButUnverified`, and `Unsupported`. The configuration UI should consume these states before allowing or explaining platform-specific actions.

**Rationale**: Windows, macOS, X11, and Wayland expose materially different APIs. A shared abstract action such as “app-specific volume” may be strongly observable on Windows, backend-dependent on Linux, and not safely implementable as a generic public API on macOS. Kivori's “observable truth only” rule requires the platform layer to expose this difference instead of hiding it.

**Alternatives**: A single `PlatformAdapter` that returns success/failure for every operation. Rejected because boolean APIs erase permission, observability, and backend availability differences.

**Caveats**: Capability detection itself can become stale. Backends should refresh when relevant OS services, permissions, audio devices, compositor sessions, or desktop environments change.

## R-7. Add a cross-platform SessionOwnershipGate before DeviceRegistry acquisition

**Decision**: **SUGGESTED** — only the Kivori process belonging to the active interactive OS session should be eligible to acquire physical Kivori serial devices. An inactive user's process should release hardware, invalidate session-private presentation, and stop competing for the port until its session becomes active again.

**Rationale**: Windows Fast User Switching documentation explicitly calls out serial ports/shared resources as resources applications must release across session switches. macOS session-switch notifications exist because switched-out applications continue running, and Apple warns against assuming exclusive hardware ownership. Linux `systemd-logind` exposes which session is active. The User Story privacy requirements are easier to enforce by preventing inactive sessions from owning the device rather than cleaning up after accidental cross-user use.

**Alternatives**: Let every per-user process repeatedly race to open the serial port and rely on “port busy.” Rejected because it creates ambiguous Waiting/Disconnected states and can expose the prior user's presentation longer than allowed.

**Caveats**: The transition has races: the incoming session may start before the outgoing process has released the port. Acquisition must tolerate a short bounded “port still owned” interval without treating the hardware as failed. Cross-session updater behavior also needs separate validation.

## R-8. Replace `first_candidate()` with DeviceRegistry + one DeviceActor per physical device

**Decision**: **SUGGESTED** — evolve single-device discovery into a `DeviceRegistry` that normalizes physical USB devices, owns one communication actor/session per Kivori, and applies product assignment (`Active/Assigned` versus `Passive/Unassigned`) separately from transport health.

**Rationale**: The current runtime opens only the first allowlisted candidate. The product contract explicitly supports more than one connected Kivori while allowing only one Active controller. All devices therefore need discoverable independent transport state even though only one executes ordinary mappings.

**Alternatives**: Keep one open serial device and ignore extras. Rejected because ignored devices cannot show Passive/Unassigned truth and cannot be selected reliably in configuration.

**Caveats**: macOS serial enumeration can expose both callout (`/dev/cu.*`) and tty (`/dev/tty.*`) nodes for one physical device; Linux metadata can vary with `libudev`; Windows COM numbering changes. Registry identity must come from verified device handshake identity, not port path alone.

## R-9. Give every physical Kivori a stable unique identity

**Decision**: **SUGGESTED** — replace the current hard-coded firmware `DeviceId`. For prototypes, deriving a stable identifier from manufacturing-programmed chip identity (for example an ESP eFuse base MAC) is a candidate. For production, provision a Kivori-specific random stable 128-bit ID during manufacturing.

**Rationale**: Multi-device assignment, persistent friendly names, update targeting, and diagnostics require distinguishing physical units after ports are renumbered. The current production firmware embeds the same fixed 16-byte ID for every unit, which cannot satisfy those requirements.

**Alternatives**: Use USB port name, VID/PID, or the current short FNV display hash as the persistent key. Rejected: port paths are unstable, VID/PID identifies a model rather than a unit, and the short hash is intended as a privacy-safe UI/log hint rather than a database primary key.

**Caveats**: Device identity is not automatically an authentication credential. If identity later participates in trust/security, provisioning and cryptographic identity need a separate threat model.

## R-10. Preserve the current protocol and extend it through version/capability negotiation

**Decision**: **SUGGESTED** — evolve `kivori-protocol` rather than replacing it. Add append-only message variants gated by negotiated capabilities and keep protocol-major changes for genuinely incompatible wire semantics.

**Rationale**: The existing framing already supplies COBS resynchronization, CRC, protocol major/minor versioning, sequence classification, and a capability bitset. The message enum is intentionally append-only because postcard uses variant order as the wire tag. This is a good base for new input, presentation, device-status, and firmware-update messages.

**Alternatives**: Switch to JSON, protobuf, USB HID reports, or a new RPC framework for product vNext. None currently solves a demonstrated deficiency large enough to justify replacing shared/tested framing.

**Caveats**: The current capability set is empty. Concrete capability bits need central allocation and tests proving older peers ignore unavailable additive behavior safely.

## R-11. Separate ephemeral/latest-wins traffic from reliable update transactions

**Decision**: **SUGGESTED** — normal interaction and presentation traffic should remain ephemeral/current-truth oriented, while firmware transfer gets an explicit reliable transaction protocol with image ID, offsets, acknowledgements, integrity checks, and abort/resume semantics.

**Rationale**: The User Story Contract prohibits stale action replay. A detent lost during disconnect must not execute five seconds later. Conversely, firmware bytes cannot simply be dropped because a sequence gap occurred. The current sequence tracker detects duplicate/gap/wrap but is not a general retransmission layer.

**Alternatives**: Make every Kivori frame automatically retransmitted until acknowledged. Rejected because generic reliable replay creates exactly the stale-input behavior the product prohibits.

**Caveats**: Presentation snapshots should carry a revision/current-state identity so newer snapshots supersede older ones. Firmware transfer resumption must be scoped to the same authenticated update transaction, not generic session replay.

## R-12. Add explicit physical input and gesture messages

**Decision**: **SUGGESTED** — device-originated input should carry enough identity/timing to implement atomic gestures and no-stale-replay semantics, conceptually including `event_id`, `gesture_id`, `control_id`, kind/delta, and device monotonic timestamp. A `GestureEnd` or equivalent boundary should be explicit or unambiguously derivable.

**Rationale**: Context commitment, target-loss cancellation, wake-only gestures, recovery arbitration, acceleration reset, and single-gesture ownership all depend on knowing which input events belong to one physical interaction.

**Alternatives**: Send only `Rotate(+1)` / `ButtonPressed` with no gesture identity. Rejected because Desktop would have to reconstruct ownership from transport timing and could disagree with firmware during stalls.

**Caveats**: Very fast rotary input may be batched to reduce framing overhead, but batching must retain direction/order and enough timing information for the configured acceleration semantics.

## R-13. Keep semantic presentation on the wire; do not stream pixels in normal operation

**Decision**: **SUGGESTED** — extend the protocol with a semantic `PresentationSnapshot` (or equivalent) rather than sending 240×240 frame buffers for ordinary product operation. Keep the shared canonical renderer on device and Desktop.

**Rationale**: Semantic traffic is compact, works over the existing bounded USB transport, and lets firmware preserve local recovery/boot/update truth even when Desktop is absent. The current deterministic renderer is one of the project's strongest cross-device guarantees.

**Alternatives**: Desktop renders every frame and streams pixels. Rejected for normal operation due bandwidth, host dependency, recovery fragility, and loss of local animation autonomy. Pixel/frame IPC remains appropriate for Device Studio preview inside Desktop.

**Caveats**: The semantic schema must be expressive enough for takeover, health, primary buddy, indicators, transient feedback, indeterminate activity, and progress when truly known. Do not encode product logic as arbitrary host-provided pixels merely to avoid designing the semantic model.

## R-14. Firmware owns raw electrical truth and recovery; Desktop owns action meaning

**Decision**: **SUGGESTED** — firmware should own input conditioning, logical detent formation, button timing, wake-gesture ownership, single-device gesture arbitration, and MCU recovery. Desktop should own profile/context selection, action identity, sensitivity, acceleration policy, ranges, and OS execution.

**Rationale**: Hardware recovery and input validity must still work when Desktop is frozen or absent, while acceleration/sensitivity can differ by action (`Master Volume` versus frame scrub). Separating these responsibilities keeps firmware independent of application semantics.

**Alternatives**: Perform all gesture timing on Desktop. Rejected for recovery/wake reliability. Perform all acceleration/action mapping in firmware. Rejected because it couples device firmware to host profiles and action configuration.

**Caveats**: If hardware measurements show Desktop-computed acceleration introduces unacceptable latency/jitter, the placement can be revisited while retaining a clear semantic configuration boundary.

## R-15. HW-040 decoding: compare ESP32-C3 PCNT against GPIO-interrupt software quadrature

**Decision**: **SPIKE REQUIRED** — treat ESP32-C3 PCNT as the leading candidate, not a mandate. Measure the actual HW-040 electrical behavior and compare PCNT + glitch filtering against GPIO edge interrupts + a software quadrature state machine.

**Rationale**: `esp-hal`'s PCNT module explicitly supports edge counting and includes a quadrature-encoder example. Hardware counting can reduce missed high-speed edges and separate electrical filtering from gesture semantics. However HW-040 modules vary in bounce, detents-per-cycle, pull-up quality, and mechanical behavior.

**Alternatives**: Poll GPIOs in the main render loop. Rejected as the default because it couples input capture to rendering/transport load and is more likely to miss high-speed transitions.

**Caveats**: PCNT is currently behind `esp-hal`'s `unstable` feature. Pin the HAL version and isolate decoder-specific code. A valid logical reverse detent must still reset acceleration immediately; filtering must reject electrical noise, not “protect” acceleration from genuine reversal.

## R-16. Implement button/recovery behavior as a firmware gesture state machine

**Decision**: **SUGGESTED** — model the primary push-encoder button explicitly, for example `ShortCandidate -> HoldArmed -> RecoveryOwned -> Reboot`, with monotonic deadlines and release-qualified action emission.

**Rationale**: This directly supports the agreed short-press cutoff, release-qualified Hold action, recovery takeover at the recovery-arming threshold, out-of-band 10-second MCU reboot, Display Sleep wake behavior, and suppression of accidental rotary/auxiliary actions while recovery owns input.

**Alternatives**: Emit raw button down/up to Desktop and let Desktop determine the 10-second recovery action. Rejected because recovery must exist independently of host software.

**Caveats**: Exact intermediate hold timing is a UX target and may change after hardware/user testing. The state machine should expose recovery progress semantically so rendering does not depend on duplicating timer logic elsewhere.

## R-17. Compute action-specific rotary acceleration from validated device timing

**Decision**: **SUGGESTED** — firmware reports validated detents/gesture timing; Desktop's binding/action layer computes base step, sensitivity, same-direction acceleration, 5× ceiling, and precision-mode disabling.

**Rationale**: The same physical gesture can intentionally have different semantics by action. Keeping acceleration configuration near the action definition avoids pushing profile data into firmware. Device timing still prevents host scheduling jitter from inventing detent cadence.

**Alternatives**: Firmware computes only a universal acceleration multiplier. Candidate if latency measurements demand it, but it would need host-provided gesture policy and careful synchronization whenever profiles change.

**Caveats**: The implementation must define batching behavior so a rapid batch does not erase intermediate direction reversals. A batch of deltas should preserve ordered segments or equivalent information.

## R-18. Use typed action attempts/outcomes, not `execute() -> bool`

**Decision**: **SUGGESTED** — represent action execution with an ID, committed context, confirmation strategy, lifecycle, and outcome such as `Running`, `StateConfirmed`, `ExecutionConfirmed`, `TriggeredUnverified`, and `Failed`.

**Rationale**: Confirmation quality differs by action and platform. Windows master volume can be read back/callback-confirmed; a synthetic keyboard shortcut often cannot be; a process launch can be confirmed at spawn but a later required window may still be pending; a script has a running lifetime and exit status. A boolean API would collapse “known failed” and “unknown outcome.”

**Alternatives**: Have every adapter throw errors and assume no error means success. Rejected because several OS APIs explicitly do not prove the downstream effect.

**Caveats**: Confirmation strategy is not purely an action-type property. It can depend on platform/backend and permissions, so it should be resolved at execution time.

## R-19. Give macros their own runner and composite outcome model

**Decision**: **SUGGESTED** — implement macros as explicit sequential executions composed from ordinary action attempts, with required/optional step semantics, cancellation, delays/waits, and derived overall status. Do not add hidden transactional rollback.

**Rationale**: The User Story Contract says known failure of a required step produces Partial Failure; an Unverified required step caps the composite at Unverified; completed earlier side effects remain truth unless compensation was explicitly authored. Reusing the ordinary action engine keeps confirmation semantics consistent.

**Alternatives**: Compile macros into opaque scripts and report only script process exit. Candidate for an explicit “run script” action, but not adequate for first-class Kivori macros where per-step feedback/permissions are important.

**Caveats**: Future explicit compensation (`on failure -> ...`) is not the same as implicit rollback. Cancellation semantics also need care once a step launches a long-running child process.

## R-20. Separate ExecutionTracker from device transport lifetime

**Decision**: **SUGGESTED** — host-side long-running jobs should live in an `ExecutionTracker` whose lifetime/identity is not owned by the USB `Session` object.

**Rationale**: A confirmed build/script/process can continue while Kivori USB is Degraded or Disconnected. On reconnect the device should reconcile current Running/Idle truth, not reinterpret transport loss as execution failure or replay historical Success.

**Alternatives**: Store current execution only inside the active device actor. Rejected because dropping/recreating that actor on USB failure would destroy observable host truth.

**Caveats**: If the Desktop process itself restarts, persistence/recovery of long-running job observation is a separate design question. MVP may legitimately classify some post-restart jobs as Unverified rather than invent continuity.

## R-21. Windows bindings: isolate Win32/COM/WinRT behind a dedicated backend

**Decision**: **SUGGESTED** — use Microsoft's `windows` Rust crate as the primary typed binding layer and isolate unsafe/COM/thread-affine OS code under `platform/windows` (or equivalent).

**Rationale**: Windows functionality spans Win32 window/session APIs, Core Audio COM interfaces, and WinRT media APIs. Keeping those handles/types out of AppCore preserves portability and makes capability/confirmation translation explicit.

**Alternatives**: Multiple small FFI crates or hand-written bindings. Possible for narrow APIs but increases inconsistent error/thread handling.

**Caveats**: Feature-gate the `windows` crate narrowly; enabling huge API surfaces increases compile time. COM apartment/thread rules should be owned by the backend rather than assumed by general async tasks.

## R-22. Windows foreground observation: event hook, not high-frequency polling

**Decision**: **SUGGESTED** — use `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ..., WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)` on a thread with a message loop. Publish foreground candidates into `ContextEngine`; apply the 300–500 ms stabilization there.

**Rationale**: Out-of-context hooks avoid code injection into target processes and Windows queues callback events in order. This matches Kivori's non-invasive profile rule and avoids a fast polling loop.

**Alternatives**: Poll `GetForegroundWindow()` every few milliseconds. Simpler but wasteful and easier to miss/overreact to transient changes. Occasional readback remains useful for reconciliation.

**Caveats**: Foreground HWND alone does not determine secure/protected context or overlay ownership. Context classification must combine additional OS-observable facts conservatively.

## R-23. Windows workspace and session observation: use public APIs only

**Decision**: **SUGGESTED** — use public session/window APIs: WTS session notifications for lock/unlock/switches, `IVirtualDesktopManager::IsWindowOnCurrentVirtualDesktop` for public virtual-desktop membership checks, power-setting notifications for per-session display/user-presence state, and input-desktop checks where required for protected desktop classification.

**Rationale**: The product does not require private Windows shell APIs to implement stable app profiles and workspace transitions. Public signals are enough to mark Task View/workspace transition as non-profile-owning and then stabilize the newly visible foreground app.

**Alternatives**: Reverse-engineered virtual-desktop COM interfaces to enumerate/control desktops. Rejected for the core path because they are undocumented and can break with Windows updates.

**Caveats**: No single API says “Kivori may safely inject into this window.” Protected/security classification is an aggregate decision and should fail closed for custom actions when evidence is insufficient.

## R-24. Windows master volume/mute: Core Audio endpoint state with callbacks

**Decision**: **SUGGESTED** — use Core Audio `IAudioEndpointVolume` and `IAudioEndpointVolumeCallback`, plus default-device notification/rebinding, for master render/capture endpoint controls.

**Rationale**: This gives Kivori both mutation and observable state. Callback/readback can produce `StateConfirmed` instead of assuming a successful method call means the system state changed. Event-context IDs can distinguish Kivori-initiated changes from external changes.

**Alternatives**: Multimedia key injection. Useful fallback for generic keyboards but much weaker for state confirmation and scope.

**Caveats**: Logical default endpoints can change (headphones, Bluetooth, virtual devices). Bind to the current logical default and rebind when Windows reports a default-device change rather than pinning a physical endpoint silently.

## R-25. Windows app audio: Core Audio sessions, with explicit ambiguity handling

**Decision**: **SUGGESTED** — investigate `IAudioSessionManager2`, `IAudioSessionControl2`, and `ISimpleAudioVolume` for app-specific audio. Associate sessions with process identity where reliable and expose ambiguity when a session spans more than one process.

**Rationale**: Windows provides first-class per-session volume control and process metadata, making app volume a realistic explicit action distinct from system volume.

**Alternatives**: Mixer automation or synthetic volume keys. Rejected as the primary path because they either depend on UI automation or change the wrong scope.

**Caveats**: `GetProcessId` can report a multi-process session condition; one app can also create multiple sessions. App matching therefore needs a session aggregation policy rather than assuming one PID = one slider forever.

## R-26. Windows microphone activity/state: treat capture-session mapping as a spike

**Decision**: **SPIKE REQUIRED** — evaluate Core Audio capture endpoints and audio sessions for observable microphone/capture activity and process association. Separate “default microphone muted” from “some process is actively capturing.”

**Rationale**: The product's privacy indicator needs truthful semantics. Windows exposes capture endpoints and session activity, but Kivori must verify which signals are reliable across conferencing apps, browser subprocesses, virtual microphones, and exclusive-mode clients.

**Alternatives**: Infer microphone activity from focused application name or call state. Rejected because it would invent privacy state.

**Caveats**: Windows' own privacy indicators may use privileged/internal knowledge unavailable to ordinary desktop applications. If Kivori cannot observe activity completely, the capability must be marked partial/unknown rather than pretending parity.

## R-27. Windows shortcuts: `SendInput` as Triggered/Unverified fallback

**Decision**: **SUGGESTED** — use `SendInput` for generic shortcut injection when the context is eligible, but classify the action `Triggered/Unverified` unless an independent observable postcondition confirms the intended result.

**Rationale**: Windows UIPI can block injection into higher-integrity targets, and the API does not reliably identify UIPI as the reason through its return/GetLastError behavior. Dispatch acknowledgement is therefore not proof that the target processed the shortcut.

**Alternatives**: Require elevated Kivori or install a privileged helper solely to inject into elevated applications. Rejected for MVP because it expands the security boundary and violates least privilege for a convenience fallback.

**Caveats**: Protected/secure desktop must be classified before dispatch. Per-app APIs or accessibility-like integrations may provide stronger confirmation for specific actions later.

## R-28. Windows global media: GSMTC is promising but packaging must be proven

**Decision**: **SPIKE REQUIRED** — test `GlobalSystemMediaTransportControlsSessionManager` in the actual shipped Tauri packaging model before relying on it. If usable, prefer it for observable media sessions; otherwise retain media-key injection as a weaker fallback.

**Rationale**: GSMTC exposes current/all media sessions and observable playback properties, which can provide better confirmation than key simulation.

**Alternatives**: Only synthesize media keys. Simpler, but typically Triggered/Unverified and cannot reliably identify which player owns the command.

**Caveats**: Microsoft documents a `globalMediaControl` capability requirement. Kivori must prove capability declaration/availability in its chosen distribution model rather than assuming UWP examples translate directly to an unpackaged Tauri binary.

## R-29. Windows DND: model interruption suitability, not an invented exact DND toggle

**Decision**: **SUGGESTED** — use documented notification-suitability signals such as `SHQueryUserNotificationState` as one input to Kivori feedback policy, and name the abstraction `InterruptionSuitability` (or equivalent) rather than claiming exact Windows DND state.

**Rationale**: The documented API distinguishes situations such as fullscreen, presentation, quiet time, and user-not-present. That is useful for buzzer/transient policy but is not the same as a guaranteed read of every modern Windows Focus/DND setting.

**Alternatives**: Read undocumented registry/shell state to mirror the UI toggle. Rejected for core behavior because it is brittle and could violate observable-truth guarantees after OS updates.

**Caveats**: Users may expect Kivori “DND” to match the OS label. Product copy should describe exactly what Kivori respects on each platform.

## R-30. Windows overlay detection: conservative scored classification from OS-visible facts

**Decision**: **SUGGESTED** — build overlay classification from multiple non-invasive signals: foreground/owner/root-owner relationships, process identity, relevant extended window styles, DWM visibility/cloaking, short transition history, known app rules, and explicit user overrides. No individual flag should prove “overlay.”

**Rationale**: `WS_EX_TOPMOST`, `WS_EX_NOACTIVATE`, `WS_EX_TOOLWINDOW`, ownership, and cloaking each describe one window property, not semantic ownership. A conservative classifier lets recognized Discord/Game Bar/etc transient surfaces preserve the game profile without injecting into game/anti-cheat processes.

**Alternatives**: Process injection, DLL hooks, game memory inspection, or anti-cheat integration. Rejected because they create security/compatibility risk disproportionate to profile selection.

**Caveats**: Unknown overlays must fall back to normal focus stabilization. The classifier should emit confidence/reason diagnostics useful for test tooling without logging sensitive window content unnecessarily.

## R-31. macOS app-level foreground profiles: NSWorkspace first

**Decision**: **SUGGESTED** — use `NSWorkspace.didActivateApplicationNotification` / `NSRunningApplication` for app-level focus and process identity. Do not require Accessibility permission merely to select an application profile.

**Rationale**: AppKit exposes activation notifications directly and supplies the affected running application. This is a cleaner permission story than asking for Accessibility on first launch when the user only needs app-level profiles.

**Alternatives**: Poll frontmost application or use Accessibility for everything. Polling is unnecessary; universal Accessibility permission would be over-privileged.

**Caveats**: Window-title/window-role matching is a separate capability and may require Accessibility. Keep app-level and window-level context capabilities distinct in the UI.

## R-32. macOS window-level context: Accessibility is permission-gated

**Decision**: **SUGGESTED** — put Accessibility-dependent window inspection/input under `SupportedWithPermission`, using supported trust checks such as `AXIsProcessTrustedWithOptions`. Request permission only when a feature needs it and explain the feature impact.

**Rationale**: This preserves least privilege and lets Kivori function at a useful app-profile level without broad UI inspection rights.

**Alternatives**: Make Accessibility mandatory during onboarding. Rejected unless later product scope proves almost every meaningful action depends on it.

**Caveats**: Accessibility can be revoked while Kivori is running. Capability state must refresh and affected actions should become Permission Required without breaking unrelated system controls.

## R-33. macOS Spaces and user-session transitions: NSWorkspace notifications

**Decision**: **SUGGESTED** — observe `activeSpaceDidChangeNotification`, `sessionDidResignActiveNotification`, `sessionDidBecomeActiveNotification`, and wake/sleep-related workspace notifications. Treat the Spaces transition itself as non-profile-owning and feed session-active state into `SessionOwnershipGate`.

**Rationale**: These public AppKit signals align directly with User Story semantics: stabilize the newly active app after a Space change and relinquish hardware before the user's session switches out.

**Alternatives**: Private Spaces APIs to enumerate/control workspaces. Rejected for baseline behavior.

**Caveats**: Apple documents that switched-out processes continue running; hardware release/acquisition can race between sessions. DeviceRegistry must tolerate transient busy ports.

## R-34. macOS system audio: Core Audio, but validate controllable-volume semantics

**Decision**: **SUGGESTED / SPIKE REQUIRED** — use modern Core Audio hardware APIs for default input/output device observation and available volume/mute controls, while testing behavior across built-in, HDMI, USB, Bluetooth, aggregate, and virtual devices.

**Rationale**: Core Audio is the supported system-level audio stack. Where a device exposes writable controls, Kivori can use readback for `StateConfirmed` semantics.

**Alternatives**: Shelling out to AppleScript or UI automation. Rejected as the primary system-volume path because observability and device coverage are weaker.

**Caveats**: Not every macOS output device exposes a software master-volume control. Unsupported devices must be reported honestly rather than treated as action failure after a fake slider change.

## R-35. macOS microphone/process audio observability: modern Core Audio process APIs are promising

**Decision**: **SPIKE REQUIRED** — evaluate `AudioHardwareProcess`-style process metadata/activity APIs (`bundleID`, PID, running input/output state) against Kivori's desired microphone/call indicators and minimum supported macOS version.

**Rationale**: Modern Core Audio exposes process-level audio activity useful for truthful indicators without guessing from focused app identity.

**Alternatives**: Infer microphone activity from known conferencing apps. Rejected because it would conflate “app is open” with “microphone is active.”

**Caveats**: Availability may force a newer deployment target. The technical choice must be made together with the product's supported macOS-version policy.

## R-36. macOS arbitrary per-app volume: do not promise it yet

**Decision**: **PLATFORM-LIMITED / SPIKE REQUIRED** — do not claim generic Windows-style per-application volume control on macOS until a supported implementation is demonstrated. Core Audio process taps are useful for capture/routing/muting scenarios but are not automatically a simple per-app volume setter.

**Rationale**: Explicit action scope is more important than feature parity. If `Discord Volume` is not supportable through a public robust API, Kivori should show that action as unavailable on macOS rather than silently changing master volume.

**Alternatives**: Install a virtual audio driver and route applications through it. Technically possible, but a major product/security/maintenance expansion inappropriate to assume for MVP.

**Caveats**: This may change with future macOS APIs. Keep the capability data-driven rather than hard-coding “never supported.”

## R-37. macOS global media control: public generic control of other apps remains unresolved

**Decision**: **SPIKE REQUIRED** — do not rely on private MediaRemote frameworks. Research a supported generic mechanism in the shipping environment; otherwise use synthetic media-key behavior as `Triggered/Unverified`.

**Rationale**: Apple's public MediaPlayer APIs such as Now Playing / Remote Command Center are principally documented for an application's own media session and command handling, not as a general desktop controller equivalent to Linux MPRIS.

**Alternatives**: Private MediaRemote APIs. Rejected for a product baseline because private frameworks can break across OS updates and complicate signing/review/distribution.

**Caveats**: App-specific AppleScript/Shortcuts integrations may be legitimate explicit actions, but they are not a universal media backend.

## R-38. macOS Focus status: permission-aware optional capability

**Decision**: **SUGGESTED** — where useful, integrate supported Focus-status APIs only after user authorization and expose the result as an optional feedback-policy capability.

**Rationale**: Kivori should respect user interruption preferences where the platform exposes them, but normal device control must not depend on Focus authorization.

**Alternatives**: Scrape Control Center/UI state. Rejected.

**Caveats**: Focus authorization and availability vary by OS version/configuration. Absence of authorization should not be reported as “Focus off”; it is Unknown/Permission Required for that capability.

## R-39. Treat Linux X11 and Wayland as distinct capability environments

**Decision**: **SUGGESTED** — the Linux backend should branch by session/display environment and dynamically advertise capabilities. Do not publish a single blanket “Linux supports foreground profiles/global shortcuts” claim.

**Rationale**: X11's architecture allows standardized cross-client window inspection and XTEST input synthesis. Wayland intentionally restricts those global capabilities and delegates privileged operations to compositor protocols/portals. Desktop environment and compositor support also vary.

**Alternatives**: Support only X11 initially. Valid release-policy option, but architecture should still avoid assumptions that block later Wayland support.

**Caveats**: `XDG_SESSION_TYPE` and environment variables help identify the session but do not themselves prove a specific protocol/portal capability. Probe the actual backend.

## R-40. Linux/X11 foreground and workspace context: EWMH

**Decision**: **SUGGESTED** — on EWMH-compliant X11 window managers, observe `_NET_ACTIVE_WINDOW` and `_NET_CURRENT_DESKTOP`, then map the active window to process/application identity as available.

**Rationale**: EWMH standardizes these root-window properties and is broadly implemented. It supports Kivori's focus/workspace semantics without polling arbitrary process lists.

**Alternatives**: Desktop-environment-specific APIs only. Keep as optional enrichment, not the baseline when EWMH is available.

**Caveats**: Not every X11 WM is fully EWMH-compliant, and process identity hints are not security boundaries. Capability should degrade cleanly when the WM does not expose required properties.

## R-41. Linux/X11 shortcut injection: XTEST candidate

**Decision**: **SUGGESTED** — use the XTEST extension for generic synthetic keyboard input when present, and classify generic shortcut dispatch according to observable postconditions rather than assuming success.

**Rationale**: XTEST is the standard X11 extension for fake key/button events and matches the desktop-control use case without per-application injection.

**Alternatives**: `xdotool` subprocesses. Candidate for prototypes only; embedding the protocol/API directly gives better error/control semantics and fewer external runtime dependencies.

**Caveats**: XTEST availability must be detected. Secure/elevated application semantics differ by desktop/session and should not be inferred from Windows rules.

## R-42. Linux/Wayland foreground context: compositor/protocol-dependent

**Decision**: **PLATFORM-LIMITED** — treat foreign-toplevel discovery as `SupportedWhenBackendAvailable`. The staging `ext-foreign-toplevel-list-v1` protocol is promising but not universal and can be restricted by compositor policy.

**Rationale**: Wayland intentionally does not expose X11-style global window inspection to every ordinary client. The protocol itself states that the compositor may restrict it to a special client and is still in a testing/staging phase.

**Alternatives**: Infer foreground app from process CPU usage, `/proc`, or recent launches. Rejected because it would guess context.

**Caveats**: GNOME, KDE, Sway, Hyprland, niri, and other compositors may expose different protocols/portals/extensions. Support must be tested per environment and reported dynamically.

## R-43. Linux/Wayland synthetic input: XDG RemoteDesktop portal is a candidate, not transparent fallback

**Decision**: **SPIKE REQUIRED** — evaluate the XDG Desktop Portal RemoteDesktop API (prefer EIS/libei where available) for authorized keyboard input. Treat it as a permission/session capability with explicit user UX, not an invisible replacement for XTEST.

**Rationale**: The portal is the supported cross-desktop mechanism for user-approved remote input on Wayland. It can request keyboard/pointer/touch access and establishes a user-authorized session.

**Alternatives**: `/dev/uinput` with elevated permissions/system service. Possible but significantly expands installation/privilege scope and bypasses compositor permission UX; reject as the default until portal feasibility is known.

**Caveats**: Portal `Start` can present a user dialog and session persistence behavior varies by backend/version. A physical macro controller may or may not justify maintaining such a session; this needs usability testing, not only API proof.

## R-44. Linux audio: PipeWire first, PulseAudio compatibility as fallback candidate

**Decision**: **SUGGESTED** — research/implement PipeWire as the modern Linux audio backend, using its event-driven main loop/registry and SPA volume/mute/process metadata. Keep PulseAudio compatibility as a candidate where required by supported distributions.

**Rationale**: PipeWire is the common modern desktop media graph and exposes device/node/stream metadata suitable for master and app-scoped controls. Event-driven observation fits Kivori's “desktop truth wins” model better than shell commands.

**Alternatives**: Invoke `pactl`, `wpctl`, or desktop mixer commands. Good for spikes/manual tooling but weaker as a product API due parsing/version/backend differences.

**Caveats**: Mapping streams to an “application” can still be ambiguous (browsers, sandboxed apps, portals). The backend should expose uncertainty rather than fabricate one-to-one identity.

## R-45. Linux media: prefer MPRIS for observable control

**Decision**: **SUGGESTED** — use MPRIS over the user D-Bus for players that implement it. Use exposed properties (`PlaybackStatus`, `Volume`, `CanControl`, etc.) to determine capability and confirmation.

**Rationale**: MPRIS standardizes play/pause/next/previous/seek and observable player state. For cooperating players, Linux can offer stronger confirmation than synthetic media keys.

**Alternatives**: Desktop media-key injection. Keep as a weaker fallback where MPRIS is unavailable, reported as Triggered/Unverified.

**Caveats**: Multiple media players can be present. Kivori needs an explicit selection/current-player policy instead of assuming the last D-Bus name is the user's intended target.

## R-46. Linux session and sleep ownership: systemd-logind candidate

**Decision**: **SUGGESTED** — on systemd-based desktops, use logind D-Bus session/seat state to determine active session and `PrepareForSleep` signals for suspend/resume transitions.

**Rationale**: logind already arbitrates seats/sessions and exposes whether a session is active. It maps well to `SessionOwnershipGate` and avoids inferring host sleep from USB disappearance.

**Alternatives**: Desktop-environment-specific screensaver/session APIs. Keep as supplements where needed, especially for lock-state detail.

**Caveats**: Not all Linux distributions use systemd/logind. Platform capability detection must represent alternate session managers or reduced support rather than making systemd a universal Linux requirement unless release policy explicitly chooses it.

## R-47. Linux DND: desktop-specific; do not invent a universal freedesktop state

**Decision**: **PLATFORM-LIMITED** — treat Linux interruption/DND awareness as desktop-environment-specific until GNOME/KDE/etc supported APIs are proven. The freedesktop notification specification itself does not define a universal DND query.

**Rationale**: A false cross-desktop abstraction would report wrong state on unsupported environments, directly conflicting with truthful feedback policy.

**Alternatives**: Infer DND from whether notification calls are suppressed. Rejected because suppression can have many causes and is not a reliable state query.

**Caveats**: Kivori's own DND preference remains available regardless of OS integration. OS-level DND awareness is an optional capability.

## R-48. Cross-platform serial discovery: keep `serialport`, add normalization

**Decision**: **SUGGESTED** — retain the current `serialport` transport unless measurements uncover a problem, but add a normalization layer between OS enumeration and DeviceRegistry.

**Rationale**: `serialport` supports Windows/macOS/Linux and current production code is already built/tested around its blocking handle on a dedicated thread. The old Feature 001 research preferred `tokio-serial`, but implementation evidence selected blocking `serialport`; that is a useful example of research being challengeable.

**Alternatives**: Rewrite to async `tokio-serial`. Rejected without evidence; a dedicated blocking actor is a valid concurrency model and avoids async serial platform quirks.

**Caveats**: Linux's richer enumeration normally uses `libudev`; macOS reports callout and tty forms; permissions differ across distributions. Normalize one physical Kivori before starting handshake actors, then trust handshake identity over path metadata.

## R-49. Native-core-owned, schema-versioned, atomic configuration persistence

**Decision**: **SUGGESTED** — keep profile/action/binding/device-assignment/preferences persistence in native Rust behind a `ConfigStore` interface. The storage format must be schema-versioned, migratable, atomic, per-user, and not directly writable by the webview.

**Rationale**: This preserves the existing least-privilege webview and makes migration/validation a host responsibility. Portable configuration can then include actions that may be unavailable on the current platform without corrupting their definition.

**Alternatives**: **CANDIDATE A** — bundled SQLite via `rusqlite` for transactions, relations, and migrations. **CANDIDATE B** — versioned JSON/TOML with atomic replace for a simpler early product. Both are valid; SQLite is suggested once relational complexity (profiles, actions, device assignment, macro steps) becomes material.

**Caveats**: Do not let a persistence choice leak into the User Story contract. If SQLite adds packaging/MSRV complexity before it pays for itself, start simpler and migrate deliberately.

## R-50. Keep platform-specific runtime loops thread-aware

**Decision**: **SUGGESTED** — do not force all OS integrations into one Tokio task model. Allow dedicated platform event threads/run loops (Windows message/COM thread, macOS AppKit/main-thread bridge, Linux D-Bus/logind loop, PipeWire loop) feeding bounded typed events into AppCore. Use Tauri/Tokio async runtime where it naturally fits.

**Rationale**: Several native APIs have thread affinity, message-loop, COM apartment, or callback requirements. The current dedicated device thread already demonstrates that “native owned thread + typed channel” is compatible with Tauri.

**Alternatives**: `tokio::spawn` every subsystem. Rejected as a blanket rule, not because Tokio is unsuitable generally.

**Caveats**: Event ordering matters for focus/session changes. Each backend should define its ordering/reconciliation strategy rather than assuming cross-thread delivery timing is identical across OSes.

## R-51. Refine offline-first: isolate network access to UpdateManager, never core operation

**Decision**: **SUGGESTED** — update the old “no network client anywhere” architecture rule to: core Kivori operation remains network-independent; firmware, shared crates, and webview remain network-free; only an approved native `UpdateManager` may perform release/update network access; startup/device control must never wait on the network.

**Rationale**: The new product contract intentionally includes update discovery and signed Desktop/firmware updates. Keeping network access in native Rust preserves the webview security boundary and offline-first behavior while resolving the current docs conflict.

**Alternatives**: Let React fetch GitHub/releases directly. Rejected because it expands webview network permissions and duplicates trust/verification logic in a less privileged boundary.

**Caveats**: CI guards must evolve from “no HTTP dependency anywhere” to an allowlist/boundary test that ensures network clients cannot leak into shared firmware/core paths.

## R-52. Desktop self-update: Tauri updater is the leading implementation

**Decision**: **SUGGESTED** — use Tauri's official updater for Desktop binaries/packages, with signed artifacts, explicit update status, release notes, and compatibility ordering managed by Kivori's native UpdateCoordinator.

**Rationale**: Tauri requires updater signatures and states that signature verification cannot be disabled. This matches Kivori's authenticated-artifact invariant and supports native cross-platform application update flows.

**Alternatives**: Custom download/replace installer logic. Rejected unless the official updater proves incompatible with a supported platform/distribution channel.

**Caveats**: Signing/distribution differs by OS. macOS direct distribution requires code signing/notarization; Windows signing is a release-policy/security decision; Linux package/update behavior differs by package format. Fast User Switching also means multiple running user-session instances may need update coordination.

## R-53. Use one release manifest to coordinate Desktop/firmware/hardware compatibility

**Decision**: **SUGGESTED** — maintain product release metadata that can express Desktop version, firmware version, supported hardware revision(s), protocol/capabilities, minimum compatible counterpart versions, artifact digest/signature metadata, severity, and dependency order.

**Rationale**: Desktop and firmware are independently versioned components but cannot be updated blindly. A manifest lets “Update All” resolve Desktop prerequisite first, reconnect, then flash only compatible firmware.

**Alternatives**: Encode all compatibility purely in GitHub Release naming. Possible early on but becomes brittle once multiple hardware revisions/protocol feature sets exist.

**Caveats**: The manifest endpoint is not a cloud dependency for normal operation. Cached/failed update checks must never degrade control functionality.

## R-54. Firmware update option A: Desktop-managed ESP ROM flashing

**Decision**: **CANDIDATE** — treat Desktop-managed ROM flashing as the lower-complexity early firmware-update path. Download/authenticate firmware in Desktop, intentionally close the Kivori app session, enter ESP32-C3 download mode, flash, reset, and require a fresh Kivori handshake.

**Rationale**: ESP32-C3 contains an immutable ROM downloader and native USB Serial/JTAG supports flashing. `espflash` is available as both CLI/library and Kivori already uses it for development flashing, reducing the amount of new device-side flash logic/protocol required.

**Alternatives**: Application-level A/B update (R-55). More robust but substantially more firmware/bootloader/partition complexity.

**Caveats**: ROM flashing interrupts normal Kivori rendering, lacks automatic previous-slot rollback, and power loss can leave the application image unusable until recovery. Desktop must show explicit progress because the device display may be unavailable. Dependency integration also needs an MSRV spike: current `espflash` releases may require a newer Rust toolchain than Kivori's current workspace policy.

## R-55. Firmware update option B: application-level A/B firmware with rollback

**Decision**: **CANDIDATE / SPIKE REQUIRED** — prefer A/B as a robustness target only if the actual flash size, image/assets size, partition layout, bootloader build, and device-side flash APIs make it practical.

**Rationale**: The ESP OTA model writes an inactive application slot, verifies it, updates OTA metadata, reboots into a pending image, and can retain/restore a known-good slot. This gives much better power-loss and failed-image behavior than destructive replacement.

**Alternatives**: ROM flashing (R-54) may be the right first shipping mechanism if A/B cost is too high.

**Caveats**: A/B requires at least two app slots plus OTA metadata and may require a custom OTA-capable bootloader rather than a stock prebuilt development bootloader. Kivori currently has no committed A/B partition map in the repo. Do not select this architecture until actual flash capacity/current release size are measured.

## R-56. Firmware transfer protocol: explicit offsets/acks if application-level updating is chosen

**Decision**: **SUGGESTED if R-55 is selected** — define a dedicated transaction such as `Begin(image_id,size,digest) -> Ready -> Chunk(offset,data) -> Ack(next_offset) -> Commit -> Verify/Status`, with bounded resume rules.

**Rationale**: Firmware bytes need reliable transfer while ordinary user input must never be replayed. Explicit offsets make power/USB interruption behavior testable and prevent generic sequence retransmission from affecting normal action semantics.

**Alternatives**: Trust only CRC on individual ordinary protocol frames and stream sequentially with no resume. Simpler but forces restart from byte zero on any interruption and needs very clear failure handling.

**Caveats**: The full image digest/signature must be checked independently of per-frame CRC. CRC protects accidental frame corruption; it is not artifact authenticity.

## R-57. Firmware authenticity: staged security, not premature irreversible eFuse policy

**Decision**: **SUGGESTED** — MVP update delivery should at minimum authenticate release metadata/artifact in Desktop and verify the complete expected image digest before activation/flashing. Research ESP32-C3 Secure Boot v2 and anti-rollback as a separate production/manufacturing security phase before burning security eFuses.

**Rationale**: Tauri already gives signed Desktop artifacts. Firmware needs equivalent trust, but ESP hardware secure boot/anti-rollback affects manufacturing, debugging, recovery, and irreversible eFuse state. It should be introduced with a threat model and factory process rather than enabled casually on prototypes.

**Alternatives**: Enable Secure Boot/anti-rollback immediately. Rejected for prototypes until recovery/manufacturing procedure is proven. No authenticity at all is also rejected for production update delivery.

**Caveats**: Hash verification alone only proves integrity against the expected hash source; authenticity depends on how that expected metadata is signed/trusted.

## R-58. ESP32-C3 ROM recovery: preserve BOOT/EN access below Kivori firmware

**Decision**: **SUGGESTED** — production hardware should expose a reliable way (button/recessed mechanism/test pads) to enter the immutable ESP32-C3 ROM download bootloader and reset the MCU even when Kivori application firmware cannot run.

**Rationale**: The User Story contract requires recovery below the application image. ESP32-C3 boot strapping uses GPIO9 low during reset for download mode, with GPIO8 high required for reliable joint download boot. This gives a recovery path after corrupt application firmware.

**Alternatives**: Depend only on the ~10-second application-observed recovery hold. Rejected because corrupted firmware may never observe it.

**Caveats**: Automatic USB download entry is useful in normal cases but cannot be the sole recovery mechanism. Physical BOOT/EN behavior must be documented for end users/support and tested after deliberately flashing a broken application.

## R-59. Review GPIO2/GPIO8/GPIO9/EN as one boot/recovery hardware problem

**Decision**: **HARDWARE SPIKE REQUIRED** — validate the existing display wiring and reset straps together before treating the production PCB recovery design as final.

**Rationale**: The validated Kivori profile uses GPIO2 for display D/C and GPIO8 for active-high backlight, while ESP32-C3 boot-strapping/download behavior involves GPIO2/GPIO8/GPIO9. External loads/pulls can affect reset-time strap levels even when runtime behavior is correct.

**Alternatives**: Assume firmware pin direction initialization removes all strap risk. Rejected because strap sampling occurs at reset before application firmware configures pins.

**Caveats**: This is electrical validation, not something a Rust unit test can prove. Record resistor/load measurements/schematic review and physical recovery evidence.

## R-60. Build/release matrix: add macOS before claiming macOS product support

**Decision**: **SUGGESTED** — extend CI/release builds to Windows, Linux, and macOS (arm64 and x86_64/universal strategy as product policy dictates) before declaring those hosts supported. Keep firmware/no_std/Wokwi/golden-frame gates separate from host packaging gates.

**Rationale**: Current CI has strong Windows/Linux host checks but no macOS product build/test lane. Cross-platform code paths will otherwise rot unnoticed. Tauri's tooling/GitHub action can produce native artifacts, but platform APIs must still be compiled/tested on their OS.

**Alternatives**: Cross-compile macOS from Linux. Not a practical replacement for signing/notarization/native framework linking and runtime tests.

**Caveats**: CI availability is not user-environment validation. Real Fast User Switching, permissions, audio devices, USB unplug/replug, and compositor behavior still require physical/manual matrices.

## R-61. macOS release: signing, notarization, and deployment target are architectural inputs

**Decision**: **SUGGESTED** — treat Apple signing/notarization and `minimumSystemVersion` as part of the technical release contract, not final packaging chores.

**Rationale**: Native AppKit/Accessibility/Core Audio API availability depends on deployment target, and direct distribution requires trusted signing/notarization for acceptable UX/security. Choosing a newer audio API can therefore change the product's minimum supported macOS version.

**Alternatives**: Build unsigned development binaries as the only macOS path. Fine for local spikes, not a shipping strategy.

**Caveats**: App Store distribution imposes additional sandbox/entitlement constraints and should be evaluated separately from direct distribution rather than assumed equivalent.

## R-62. Linux support policy should name environments, not only “Linux”

**Decision**: **SUGGESTED** — release documentation should eventually specify tested distributions/runtime baselines plus X11/Wayland/backend capability expectations. Package formats (AppImage/deb/rpm/etc.) and runtime dependencies (`libudev`, WebKitGTK, PipeWire/portal services) should be explicit.

**Rationale**: A Tauri package can launch on Linux while Kivori's OS integration is partially unavailable because of compositor, portal, audio server, permissions, or library versions. Feature-level capability reporting and release-level environment policy are both necessary.

**Alternatives**: “Best effort on any Linux desktop.” Possible for community builds but too vague for reliable product claims.

**Caveats**: Building AppImage/package artifacts on newer distributions can raise glibc/WebKitGTK compatibility baselines. Build on the oldest intended supported environment and test newer ones separately.

## R-63. Testing strategy: pure domain tests + backend contract tests + real platform evidence

**Decision**: **SUGGESTED** — use three layers: (1) pure deterministic AppCore/PresentationResolver/Binding/Execution tests; (2) backend contract tests with fakes/recorded OS events; (3) real OS/hardware validation for APIs whose truth cannot be simulated.

**Rationale**: Most User Story edge cases are deterministic and should not require a desktop automation harness. Platform APIs can be adapter-tested independently. Hardware/permissions/Fast User Switching/compositor behavior cannot be honestly “proven” by mocks.

**Alternatives**: Rely mainly on end-to-end UI automation. Rejected as the primary method because it is slower/flakier and cannot isolate semantic state-machine bugs well.

**Caveats**: Simulation evidence must remain labelled as simulation. Existing project practice already separates Wokwi from physical panel evidence; keep the same rigor for OS backends.

## R-64. Add model/property tests for no-stale-replay and gesture ownership

**Decision**: **SUGGESTED** — beyond example unit tests, add state-machine/property tests generating disconnects, focus changes, target loss, direction reversals, wake gestures, and reconnects to prove invariants such as “no command dispatched after gesture cancellation” and “reconnect never replays expired input.”

**Rationale**: These bugs emerge from event ordering rather than one happy path. Pure models are cheap to fuzz/model-test compared with reproducing timing races manually.

**Alternatives**: Only explicit hand-written examples. Keep them for readability, but supplement with generated sequences once the model exists.

**Caveats**: Property tests prove the model implementation, not that native callbacks are delivered exactly as modeled. Backend reconciliation still matters.

## R-65. Keep permission/restriction truth separate from transport health

**Decision**: **SUGGESTED** — platform adapters should publish permission/restriction capability changes independently from DeviceRegistry connection state. Losing Accessibility/portal permission must not look like USB disconnect; USB reconnect must not claim permission has recovered.

**Rationale**: This directly maps to the contract's Permission Required and localized uncertainty behavior and prevents a transport FSM from becoming overloaded with host-policy state.

**Alternatives**: Return “action failed” on every dispatch after permission loss. Rejected because the user would not know how to resolve it and Kivori would appear unreliable.

**Caveats**: Permission state can change while a gesture is active. If the restriction makes the action unavailable, the appropriate takeover/cancellation rule must win immediately.

## R-66. Platform-specific confirmation should be discoverable by configuration UI

**Decision**: **SUGGESTED** — expose action capability metadata to React so a user configuring a binding can see whether the current platform/backend offers Confirmed, Unverified, Permission Required, or Unsupported behavior before saving/testing it.

**Rationale**: Honest runtime feedback is better when paired with honest configuration UX. This prevents a user from creating `Discord Volume` on a platform with no supported implementation and discovering the limitation only after pressing hardware.

**Alternatives**: Allow all abstract actions everywhere and fail at runtime. Rejected for UX.

**Caveats**: Portable configuration should still retain unsupported action definitions when moved between machines; do not delete a Windows-specific binding simply because the same profile is viewed on macOS.

## R-67. `Test Action` should invoke the real ActionEngine with a test-origin context

**Decision**: **SUGGESTED** — Desktop's Test Action should use the same adapter/confirmation/execution path as physical input, tagged as deliberate software-originated intent, rather than maintaining a separate “test implementation.”

**Rationale**: Otherwise configuration may report success through a mock path while the real binding fails. A shared path also naturally applies permissions, scope, and confirmation classes while PresentationResolver handles the special temporary Display Sleep wake lease.

**Alternatives**: UI-only fake Success animation. Rejected except for explicit visual Preview Buddy State, which intentionally tests presentation rather than desktop execution.

**Caveats**: Test Action should bypass physical gesture ownership but must not bypass Protected/permission/device-assignment product restrictions that matter to the action's validity.

## R-68. Keep preview/testing presentation separate from production state ownership

**Decision**: **SUGGESTED** — model Preview Buddy State as an explicit preview lease/overlay controlled by Desktop UI, with an expiry and restoration to current truth. Do not mutate the persistent underlying execution/host state merely to preview a visual.

**Rationale**: This matches the existing temporary software-wake contract and prevents previews from becoming stale after configuration closes.

**Alternatives**: Send ordinary `SetState` and rely on a later reset. This is how the current foundation's simple desired-state model works, but it becomes unsafe once real product state is multi-axis.

**Caveats**: Device Studio remains dev-only and can keep stronger manual override tools, clearly separated from production user-facing preview semantics.

## R-69. Current heartbeat wiring needs verification before richer Degraded semantics

**Decision**: **SPIKE REQUIRED** — audit/measure the production desktop heartbeat scheduling path before building product-level `Degraded` behavior on it.

**Rationale**: `Session` exposes `send_ping()` and `heartbeat_timed_out()` APIs, but the current `runtime/device_task.rs` path inspected for this research visibly calls `session.pump()` and reconnect logic without an obvious periodic ping/timeout scheduler. The foundation documentation expects heartbeat behavior, so implementation and docs should be reconciled first.

**Alternatives**: Assume any serial I/O error is sufficient liveness detection. Rejected for an always-connected companion because a silent stalled link may remain open without delivering truth.

**Caveats**: This may already be covered indirectly elsewhere or have changed by the time implementation starts. Verify current code/tests before modifying it.

## R-70. Cross-platform capability matrix is a living evidence artifact

**Decision**: **SUGGESTED** — maintain the following matrix as research status, not marketing promise. Each cell should be updated after spikes/validation.

| Product capability | Windows | macOS | Linux X11 | Linux Wayland |
|---|---|---|---|---|
| USB Kivori session | **CURRENT/SUGGESTED** `serialport` | **SUGGESTED**, normalize tty/cu | **SUGGESTED**, libudev/permissions | **SUGGESTED**, same serial layer |
| Foreground app profile | **SUGGESTED** WinEvent | **SUGGESTED** NSWorkspace | **SUGGESTED** EWMH | **PLATFORM-LIMITED** compositor protocol |
| Window-specific profile | **SUGGESTED** Win32 metadata | **SupportedWithPermission** AX | **SUGGESTED** EWMH/X11 | **PLATFORM-LIMITED** |
| Workspace transition | **SUGGESTED** public virtual-desktop membership + focus | **SUGGESTED** Spaces notification | **SUGGESTED** EWMH | compositor-dependent |
| Generic shortcut | SendInput, usually **Unverified** | permission-dependent synthetic input | XTEST candidate | portal/compositor permission |
| Master audio | Core Audio endpoint | Core Audio device-dependent | PipeWire/Pulse candidate | PipeWire/Pulse candidate |
| App audio | strong candidate | **SPIKE / limited** | PipeWire candidate | PipeWire candidate |
| Mic activity | **SPIKE** capture sessions | **SPIKE** modern process audio APIs | PipeWire candidate | PipeWire candidate |
| Media control/state | **SPIKE** GSMTC | **SPIKE**, no generic public equivalent assumed | **SUGGESTED** MPRIS | **SUGGESTED** MPRIS |
| DND/interruption | partial suitability signal | permission-aware Focus candidate | DE-specific | DE-specific |
| User-session ownership | **SUGGESTED** WTS | **SUGGESTED** NSWorkspace | **SUGGESTED** logind | **SUGGESTED** logind |
| Sleep/display state | Windows power settings | NSWorkspace/power APIs | logind/DE | logind/DE |
| Overlay classification | rich but heuristic | app/window + permission-dependent detail | possible under X11 | restricted/compositor-dependent |

**Rationale**: The matrix makes product capability differences visible without hiding them behind a false cross-platform interface.

**Alternatives**: Maintain only a list of “supported OSes.” Rejected because that does not explain which User Stories are degraded or permission-dependent.

**Caveats**: “Supported” here is research confidence, not release support. Release claims require CI plus platform/manual evidence.

## R-71. Current code -> research direction map

**Decision**: **SUGGESTED** — migrate incrementally from current code rather than performing a broad rewrite.

| Current repository element | Research direction |
|---|---|
| `device/fsm.rs` transport connection FSM | Keep narrow as transport/link FSM |
| `device/session.rs` | Keep; extend capability-gated protocol handling |
| `device/serial.rs` blocking `serialport` | Keep unless measurements justify replacement |
| `runtime/device_task.rs` single device | Evolve toward DeviceRegistry + DeviceActor(s) |
| `first_candidate()` | Replace with normalized multi-device discovery |
| `orchestrator::Orchestrator { desired }` | Do not grow into product god object; supersede with AppCore + resolver |
| `kivori-model::ConnectionState` | Keep transport-only |
| `CompanionState` / `SendableState` | Preserve compatibility; introduce richer semantic presentation separately |
| `Capabilities::NONE` | Allocate concrete protocol feature bits |
| canonical renderer/assets | Preserve |
| webview `core:default` capability only | Preserve least privilege; add only narrowly justified commands |
| old offline network guard | Refine to allow native UpdateManager only |
| hard-coded physical `DeviceId` | Replace before true multi-device assignment |
| firmware `Clock`/`Transport`/`DisplaySink` ports | Extend same testable-port pattern for input/update where useful |
| Wokwi + host-sim evidence | Preserve, never treat as physical proof |

**Rationale**: This reduces implementation risk and lets each User Story capability land behind a stable boundary.

**Alternatives**: Architecture-v2 rewrite branch. Rejected for now.

**Caveats**: Some old Foundation-only semantics (for example simple desired-state resync) will need explicit compatibility/migration tests as richer presentation replaces them.

## R-72. User Story -> primary technical responsibility map

**Decision**: **SUGGESTED** — use this map to prevent future feature work from being placed in whichever module happens to be convenient.

| User Story area | Primary technical responsibility |
|---|---|
| US1 Physical Desktop Control | BindingResolver + ActionEngine + platform backend; firmware validated input |
| US2 Honest Confirmation | ActionAttempt/ExecutionTracker + platform observability |
| US3 Atomic Physical Interaction | Firmware GestureStateMachine + ContextEngine cancellation |
| US4 Observable Desktop State | Platform observers + product state axes + PresentationResolver |
| US5 App/User-Aware Controls | ContextEngine + SessionOwnershipGate + ConfigStore |
| US6 Restricted/Permission-Limited | Platform capability/permission model + PresentationResolver |
| US7 Visual Hierarchy | PresentationResolver + canonical renderer |
| US8 Localized Uncertainty | typed capability/state model, not transport fallback |
| US9 System/Feedback/Display Idle | platform observers + display-policy state + firmware display control |
| US10 Connection/Recovery/Continuity | DeviceRegistry/Session + ExecutionTracker + firmware recovery/update |
| US11 Configuration/Assignment/Updates | native ConfigStore + DeviceRegistry assignment + UpdateCoordinator + React UX |

**Rationale**: It ties research directly to the normative User Story document without turning this research into another product contract.

**Alternatives**: One subsystem per User Story. Rejected because several stories intentionally cross the same platform/execution/presentation components.

**Caveats**: This is ownership guidance, not a prohibition on collaboration between modules.

---

## Implementation-spike-required

The following should be answered with small evidence-producing spikes before the corresponding technical recommendation is frozen:

1. **Windows GSMTC in shipped Tauri packaging** — prove capability declaration/access and event/control behavior in the actual production packaging model.
2. **Windows microphone activity** — verify capture-session/process coverage across Teams/Discord/browser/virtual devices and identify incomplete cases.
3. **macOS global media control** — find a supported public mechanism or formally accept synthetic/unverified fallback.
4. **macOS arbitrary app-volume control** — prove a supported implementation or mark the abstract action unavailable on macOS.
5. **macOS modern Core Audio minimum version** — determine deployment-target impact of process activity APIs.
6. **Wayland foreground context** — test GNOME/Mutter, KDE/KWin, Sway/wlroots, Hyprland, niri, and other intended environments rather than assuming protocol presence.
7. **Wayland RemoteDesktop portal UX** — evaluate permission prompts, persistence tokens, reconnect behavior, and whether the session model is acceptable for a physical controller.
8. **Linux PipeWire mapping** — prove master volume, per-stream/app control, microphone activity, virtual-device behavior, and sandboxed app metadata.
9. **Fast User Switching** — measure hardware release/reacquisition and private-state invalidation on Windows/macOS/Linux.
10. **Tauri single-instance semantics across simultaneous local users** — verify one user's instance does not incorrectly prevent another user's per-session instance.
11. **Stable DeviceId provisioning** — choose prototype and manufacturing paths and test persistence across reflashes/updates.
12. **Firmware update architecture** — compare ROM flashing versus A/B with actual flash/image/update timing data.
13. **`espflash` integration/MSRV** — decide whether to pin a compatible library release, raise Kivori MSRV, ship a sidecar, or implement only required flashing primitives.
14. **Heartbeat production path** — reconcile code/tests/docs before introducing richer Degraded health semantics.
15. **Config persistence** — compare versioned JSON versus bundled SQLite using actual profile/macro/device-assignment schema complexity.
16. **macOS serial normalization** — validate `/dev/cu.*` preference/deduplication with real ESP32-C3 enumeration.
17. **Cross-session Desktop self-update** — verify behavior when multiple users have Kivori processes running.

## Hardware-validation-required (do NOT trust research alone)

1. **HW-040 quadrature/bounce characterization** — measure transition order, bounce duration, detents per logical step, and fastest realistic manual rotation.
2. **PCNT vs GPIO-interrupt decoder** — compare lost/false detents and CPU/load behavior on actual ESP32-C3 hardware.
3. **Recovery hold under input noise** — verify rotary bounce/auxiliary inputs cannot cancel/reset recovery timing.
4. **Actual flash capacity and layout** — record physical flash chip size, bootloader/partition footprint, firmware image size, asset size, and free margin.
5. **A/B feasibility** — if pursued, verify two image slots + metadata fit with safe margin and test rollback after intentionally bad firmware.
6. **Power-loss firmware update** — remove power at multiple flash/metadata phases and prove documented recovery outcome.
7. **ROM recovery** — deliberately install an unusable app image and recover using physical BOOT/EN path.
8. **GPIO2/GPIO8/GPIO9/EN straps** — validate reset-time electrical levels with display/backlight attached and document resistor/load requirements.
9. **USB Serial/JTAG recovery behavior** — verify automatic downloader entry where expected and manual recovery when the application cannot cooperate.
10. **Multi-device USB** — connect several physical Kivori units, confirm stable identity, assignment, Passive presentation, unplug/replug, and port renumbering.
11. **Sustained USB and render latency** — preserve the existing physical validation requirements for link stalls/reconnect and display performance.
12. **Buzzer/display power behavior** — validate wake latency, blanking/backlight behavior, and no-ambiguous-silence presentation on physical panel.

## Platform-validation-required

1. **Windows 10/11 intended baseline** — foreground hooks, WTS lock/switch, power notifications, Core Audio endpoint changes, UIPI restriction behavior, fullscreen/game overlays.
2. **Windows Fast User Switching** — two users with Kivori Desktop installed/running; verify only active session owns USB and no prior-user presentation leaks.
3. **Windows elevated/protected contexts** — UAC/secure desktop and elevated target behavior without running Kivori as admin.
4. **macOS Intel + Apple Silicon** — app focus, Spaces, Accessibility grant/revoke, session switching, sleep/wake, Core Audio devices, signing/notarized build.
5. **macOS multiple audio devices** — built-in, USB, Bluetooth, HDMI, aggregate/virtual where applicable.
6. **Linux X11** — at least one mainstream EWMH desktop/WM, PipeWire/Pulse setup, XTEST, session switching, serial permissions.
7. **GNOME Wayland** — foreground capability reality, portal remote input, PipeWire, logind, permissions.
8. **KDE Wayland** — same categories; do not assume GNOME portal/compositor behavior.
9. **wlroots-family Wayland** — validate foreign-toplevel protocol availability and portal/backend differences on intended environments.
10. **Linux packaging** — clean-machine installs for each claimed package format/distro baseline, including `libudev`, WebKitGTK, serial group/udev rules, PipeWire/portal dependencies.
11. **Offline operation on every supported OS** — block network completely and confirm control, profiles, rendering, device connection, and cached config work; update check failure must remain non-blocking.

## Sources

Primary/vendor/standards references used for this research (verify exact API/version again when implementing):

### Kivori repository

- [`docs/architecture.md`](./architecture.md) — current Device Connection Foundation architecture.
- [`docs/offline-boundary.md`](./offline-boundary.md) — current offline guard that needs refinement for UpdateManager.
- [`specs/001-device-connection-foundation/research.md`](../specs/001-device-connection-foundation/research.md) — original research format and historical decisions.
- `apps/desktop/src-tauri/src/runtime/device_task.rs`, `device/session.rs`, `device/serial.rs`, `orchestrator/mod.rs` — current Desktop runtime/session baseline.
- `crates/kivori-protocol` and `crates/kivori-model` — framing, message evolution, state/capability types.
- `firmware/esp32-c3/src/runtime.rs`, `transport.rs`, `physical_st7789.rs`, `profile.rs` — production firmware/runtime/hardware baseline.

### Tauri

- Tauri v2 Security: <https://v2.tauri.app/security/>
- Tauri capabilities/permissions: <https://v2.tauri.app/security/capabilities/>
- Tauri Updater plugin (signed updates): <https://v2.tauri.app/plugin/updater/>
- Tauri Autostart plugin: <https://v2.tauri.app/plugin/autostart/>
- Tauri async runtime: <https://docs.rs/tauri/latest/tauri/async_runtime/>
- Tauri distribution/signing guidance: <https://v2.tauri.app/distribute/>
- Tauri GitHub Action: <https://github.com/tauri-apps/tauri-action>
- Tauri Linux AppImage guidance: <https://v2.tauri.app/distribute/appimage/>

### Microsoft / Windows

- `SetWinEventHook`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook>
- Fast User Switching programming guidance: <https://learn.microsoft.com/en-us/windows/win32/shell/fastuserswitching>
- `WTSRegisterSessionNotification`: <https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/nf-wtsapi32-wtsregistersessionnotification>
- `IVirtualDesktopManager::IsWindowOnCurrentVirtualDesktop`: <https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ivirtualdesktopmanager-iswindowoncurrentvirtualdesktop>
- `OpenInputDesktop`: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openinputdesktop>
- Power setting GUIDs / session display status: <https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids>
- Core Audio Endpoint Volume: <https://learn.microsoft.com/en-us/windows/win32/coreaudio/endpointvolume-api>
- `IAudioSessionManager2`: <https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nn-audiopolicy-iaudiosessionmanager2>
- `IAudioSessionControl2::GetProcessId`: <https://learn.microsoft.com/en-us/windows/win32/api/audiopolicy/nf-audiopolicy-iaudiosessioncontrol2-getprocessid>
- Audio sessions: <https://learn.microsoft.com/en-us/windows/win32/coreaudio/audio-sessions>
- `SendInput` / UIPI limitation: <https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput>
- GSMTC Session Manager: <https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager>
- `SHQueryUserNotificationState`: <https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shqueryusernotificationstate>
- Extended window styles: <https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles>
- DWM window attributes (`DWMWA_CLOAKED`): <https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute>
- Microsoft `windows-rs`: <https://github.com/microsoft/windows-rs>

### Apple / macOS

- `NSWorkspace.didActivateApplicationNotification`: <https://developer.apple.com/documentation/appkit/nsworkspace/didactivateapplicationnotification>
- `NSWorkspace.activeSpaceDidChangeNotification`: <https://developer.apple.com/documentation/appkit/nsworkspace/activespacedidchangenotification>
- `NSWorkspace.sessionDidResignActiveNotification`: <https://developer.apple.com/documentation/appkit/nsworkspace/sessiondidresignactivenotification>
- `NSWorkspace.sessionDidBecomeActiveNotification`: <https://developer.apple.com/documentation/appkit/nsworkspace/sessiondidbecomeactivenotification>
- Accessibility trust (`AXIsProcessTrustedWithOptions`): <https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions>
- Core Audio `AudioHardwareSystem`: <https://developer.apple.com/documentation/coreaudio/audiohardwaresystem>
- Core Audio `AudioHardwareControl`: <https://developer.apple.com/documentation/coreaudio/audiohardwarecontrol>
- Core Audio `AudioHardwareProcess`: <https://developer.apple.com/documentation/coreaudio/audiohardwareprocess>
- Capturing system/process audio with Core Audio taps: <https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps>
- `INFocusStatusCenter`: <https://developer.apple.com/documentation/intents/infocusstatuscenter>
- `MPRemoteCommandCenter`: <https://developer.apple.com/documentation/mediaplayer/mpremotecommandcenter>
- `MPNowPlayingInfoCenter`: <https://developer.apple.com/documentation/mediaplayer/mpnowplayinginfocenter>

### Linux / freedesktop / Wayland

- EWMH / Window Manager Specification (`_NET_ACTIVE_WINDOW`, `_NET_CURRENT_DESKTOP`): <https://specifications.freedesktop.org/wm/latest-single/>
- XTEST fake input: <https://www.x.org/releases/X11R7.5/doc/man/man3/XTestFakeKeyEvent.3.html>
- Wayland `ext-foreign-toplevel-list-v1`: <https://wayland.app/protocols/ext-foreign-toplevel-list-v1>
- XDG Desktop Portal RemoteDesktop: <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html>
- PipeWire Rust bindings: <https://pipewire.pages.freedesktop.org/pipewire-rs/pipewire/index.html>
- PipeWire/SPA parameter properties: <https://docs.pipewire.org/devel/group__spa__param.html>
- MPRIS Player interface: <https://specifications.freedesktop.org/mpris/latest/Player_Interface.html>
- systemd logind D-Bus interface: <https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html>
- Desktop Notifications specification: <https://specifications.freedesktop.org/notification/latest-single/>

### Espressif / embedded Rust

- `esp-hal` PCNT module / quadrature example: <https://docs.rs/esp-hal/latest/esp_hal/pcnt/>
- `esp-hal` eFuse module: <https://docs.rs/esp-hal/latest/esp_hal/efuse/>
- ESP32-C3 boot mode / download selection: <https://docs.espressif.com/projects/esptool/en/latest/esp32c3/advanced-topics/boot-mode-selection.html>
- ESP32-C3 USB Serial/JTAG console/download behavior: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32c3/api-guides/usb-serial-jtag-console.html>
- ESP-IDF OTA update/rollback model: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/ota.html>
- `esp-bootloader-esp-idf` OTA module: <https://docs.espressif.com/projects/rust/esp-bootloader-esp-idf/latest/esp32c3/esp_bootloader_esp_idf/ota/index.html>
- ESP32-C3 Secure Boot v2: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32c3/security/secure-boot-v2.html>
- `espflash` crate/library: <https://docs.rs/espflash/latest/espflash/>

### Cross-platform serial / persistence candidates

- `serialport` crate: <https://docs.rs/serialport/latest/serialport/>
- `rusqlite` (SQLite candidate): <https://docs.rs/rusqlite/latest/rusqlite/>
