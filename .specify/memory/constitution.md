<!--
SYNC IMPACT REPORT
==================
Version change: (unversioned template) → 1.0.0
Bump rationale: Initial ratification of the Kivori project constitution.

Principles defined (12; template shipped with 5 placeholder slots):
  I.    Product Experience First
  II.   One Canonical Visual Model
  III.  Deterministic Rendering (NON-NEGOTIABLE)
  IV.   Hardware-Conscious Design
  V.    Desktop Owns Orchestration
  VI.   Semantic Events, Not Drawing Commands
  VII.  Offline-First and Local-First
  VIII. Safe Boundaries
  IX.   Vertical Feature Delivery
  X.    Testable Hardware Contracts
  XI.   Source Assets Are Not Runtime Assets
  XII.  Controlled Scope

Sections added:
  - Engineering Quality Standards (Rust & TypeScript quality, documented
    contracts, formatted code, zero-warning CI)
  - Development Workflow (Architectural Decision Records, review & gates)
  - Governance (amendment procedure, versioning policy, compliance review)

Sections removed: none (all template placeholders resolved).

Templates & artifacts reviewed:
  ✅ .specify/templates/plan-template.md — the "Constitution Check" section is the
     dynamic coupling point and reads gates from this file; compatible, no edit needed.
  ✅ .specify/templates/spec-template.md — no constitution-specific coupling; compatible.
  ✅ .specify/templates/tasks-template.md — compatible. Mandatory-test task types
     (Principles III & X) are enforced through the plan Constitution Check rather than
     hardcoded into this generic per-feature scaffold.
  ✅ .specify/templates/checklist-template.md — generic scaffold; no changes required.
  ✅ Spec Kit command/skill names use hyphen form (/speckit-*), matching the Claude
     integration; no outdated agent-specific references found.

Deferred placeholders / follow-up TODOs: none.
RATIFICATION_DATE set to 2026-07-17 (initial adoption date).
-->

# Kivori Constitution

Kivori is a sellable physical desktop companion: an ESP32-C3 display device paired with a
Tauri desktop application. This constitution defines the non-negotiable principles that
govern how Kivori is designed, built, and shipped. It is authoritative; where any other
practice, document, or convention conflicts with it, this constitution prevails.

## Core Principles

### I. Product Experience First

Kivori MUST feel like a living desktop companion before it functions as a dashboard. Every
feature MUST be calm, delightful, understandable, and non-intrusive by default.

- Ambient, at-rest presence is the default state; information display is secondary and MUST
  NOT demand attention unless the user explicitly opted a given event into an alert.
- Notifications, animations, and sounds MUST NOT interrupt focus work unless the user
  configured that specific event as high-priority.
- A feature that raises information density at the cost of the companion feel MUST be
  redesigned or rejected.
- Every new behavior MUST be explainable to a non-technical owner in a single sentence.

**Rationale**: Kivori is a consumer product whose primary value is emotional and ambient.
Dashboards are everywhere; a companion that earns a permanent place on someone's desk does
so through presence and delight, not raw data throughput.

### II. One Canonical Visual Model

Device Studio (the desktop preview and editor) and the physical ESP32-C3 display MUST
consume a single shared scene model. That shared definition covers the scene graph,
animation timing, compiled assets, fonts, RGB565 color values, layout coordinates, and
renderer behavior.

- The renderer MUST exist as one implementation — a shared crate compiled for both the host
  (Device Studio) and the firmware target. React, Canvas, WebGL, or any web technology MUST
  NOT reimplement device rendering logic.
- Device Studio MAY add only host-side affordances (zoom, inspection, timeline scrubbing)
  around the shared renderer's output; it MUST NOT substitute its own drawing path for the
  device's pixels.
- Any divergence between Studio output and device output for identical inputs is a defect,
  not an accepted platform difference.

**Rationale**: Two renderers guarantee drift. A single canonical model is the only way to
honestly promise "what you see in Studio is what ships to the device" and to keep visual
work trustworthy.

### III. Deterministic Rendering (NON-NEGOTIABLE)

Given the same device profile, scene, assets, elapsed time, and random seed, rendering MUST
produce byte-identical logical RGB565 output across host and firmware targets.

- Rendering MUST be a pure function of its declared inputs. Wall-clock time, uninitialized
  memory, floating-point nondeterminism, and platform-specific behavior MUST NOT affect
  output.
- All randomness MUST derive from an explicit, seedable source.
- Any change that can alter rendered output MUST update golden-frame and/or frame-hash tests
  in the same change, with reviewer-visible before/after evidence.
- CI MUST fail on any unexplained golden-frame or frame-hash mismatch.

**Rationale**: Determinism is what makes Principle II testable and what makes visual
regressions catchable. Without byte-level reproducibility, "the same scene" is an
unverifiable claim.

### IV. Hardware-Conscious Design

Firmware MUST treat the ESP32-C3's RAM, flash, SPI bandwidth, and power constraints as
first-class design limits.

- Rendering MUST NOT depend on multiple full-screen framebuffers; partial, banded, or
  single-buffer strategies are required.
- Shared crates compiled into firmware MUST build under `no_std` where the firmware target
  requires it, and MUST NOT pull heap-heavy or allocator-hostile dependencies into that path.
- Memory, flash-footprint, and frame-timing budgets MUST be defined for firmware features;
  a change that exceeds a budget MUST be justified or rejected.
- Blocking operations MUST NOT stall rendering or protocol handling beyond documented timing
  budgets.

**Rationale**: The device is a fixed, low-cost target shipped to customers. Designs that
ignore its ceilings produce firmware that cannot run on the hardware being sold.

### V. Desktop Owns Orchestration

The Tauri desktop application is the brain. It MUST own integrations, operating-system event
handling, behavior decisions, configuration, updates, and device synchronization.

- Firmware MUST be limited to protocol handling, local storage, rendering, and device-health
  reporting.
- Behavior logic — which event maps to which scene, when, and for how long — MUST live on the
  desktop and MUST NOT be baked into firmware.
- Firmware MUST NOT contain integration-specific knowledge; no per-service logic is embedded
  in the device.

**Rationale**: Firmware updates are slow, risky, and constrained. Keeping intelligence on the
desktop lets Kivori evolve behavior and integrations without reflashing fielded devices.

### VI. Semantic Events, Not Drawing Commands

Integrations MUST emit normalized, semantic events (for example `ai.task.started`,
`media.started`, `system.locked`, `build.failed`). Integrations MUST NOT place pixels, choose
coordinates, pick colors, or select low-level drawing commands.

- The mapping from semantic event to scene or behavior MUST live in the behavior engine, not
  in the integration.
- A new integration MUST express itself purely as events plus payload data conforming to a
  documented event schema.
- An integration that needs a new visual MUST add or extend a scene/behavior mapping, never a
  direct draw call.

**Rationale**: Decoupling meaning from presentation lets one visual language serve many
integrations and lets the look evolve without touching every integration.

### VII. Offline-First and Local-First

Core companion behavior — USB communication, settings, rendering, and development tools — MUST
operate fully without any cloud service or network dependency.

- Kivori MUST be fully usable on a machine that has never been online.
- Cloud integrations are OPTIONAL capabilities layered on top. Their absence or failure MUST
  degrade gracefully and MUST NOT break core companion behavior.
- No core feature may hard-depend on a remote service, license check, or telemetry endpoint
  being reachable.

**Rationale**: A companion people paid for must keep working regardless of connectivity or
vendor uptime. Local-first is both a reliability guarantee and a trust commitment.

### VIII. Safe Boundaries

Security boundaries are non-negotiable and MUST be enforced by construction.

- Secrets (tokens, credentials, keys) MUST reside in native OS secure storage and MUST NOT be
  persisted in the webview, plaintext files, logs, or compiled assets.
- The webview MUST NOT receive arbitrary serial, filesystem, shell, or credential access; it
  communicates only through explicitly defined, least-privilege Tauri commands.
- Local APIs MUST bind to localhost only and MUST require authentication.
- Any new capability exposed to the webview MUST be reviewed as a security decision and
  documented.

**Rationale**: Kivori holds integration credentials and has direct physical-device access. A
leaky webview or an open local port turns a desktop companion into an attack surface on the
customer's machine.

### IX. Vertical Feature Delivery

Every feature MUST ship as the smallest complete vertical slice across each layer it touches:
shared model, desktop, firmware (where applicable), tests, and documentation.

- Large, disconnected scaffolding phases — building a layer with no consuming feature — are
  prohibited.
- A feature is "done" only when its full path works end-to-end and is tested and documented to
  the standards in this constitution.
- Work MUST be sequenced so that each merged increment leaves the product demonstrably
  working.

**Rationale**: Vertical slices keep the cross-stack contracts (event → behavior → scene →
pixels) honest and prevent speculative infrastructure that never meets a real feature.

### X. Testable Hardware Contracts

The shared model, renderer, protocol, asset compiler, and behavior engine MUST each have
automated tests.

- Hardware-dependent behavior MUST have host-side simulation or contract tests wherever
  physical automation is unavailable; "requires the device" is not an exemption from testing.
- Protocol changes MUST include contract tests covering both the desktop and firmware sides of
  the wire.
- Asset-compiler output MUST be covered by tests that assert the compiled runtime
  representation.

**Rationale**: The product's correctness lives in cross-boundary contracts. Automated and
simulated tests are the only way to keep those contracts verifiable without a physical device
in every CI run.

### XI. Source Assets Are Not Runtime Assets

SVG and PNG files are authoring inputs only. The asset compiler MUST produce the canonical
compiled runtime representation consumed by both Device Studio and firmware.

- Firmware and the runtime renderer MUST NOT parse or load source SVG/PNG at runtime.
- Compiled assets MUST be deterministic, reproducible outputs of the asset compiler from
  source inputs.
- Hand-editing a compiled runtime asset instead of recompiling it from source is prohibited.

**Rationale**: A compiled, canonical asset format is what lets identical bytes render
identically on host and device (Principles II and III) and fit hardware constraints
(Principle IV).

### XII. Controlled Scope

The initial foundation supports exactly one platform and one transport: Windows and USB
serial.

- Wi-Fi, Bluetooth, cloud sync, a marketplace, and third-party plugin execution are OUT of
  scope for the foundation and MUST NOT be built unless a specification explicitly adds them.
- Any scope expansion — a new platform, transport, or capability class — MUST be introduced
  through an explicit specification and recorded in an Architectural Decision Record before
  implementation begins.

**Rationale**: A tight initial scope is how a small team ships a sellable v1. Every deferred
surface is a real cost saved now and a deliberate, documented decision later.

## Engineering Quality Standards

These standards apply to all code in the repository and are enforced in CI where practical.

**Rust**

- CI MUST treat warnings as errors (for example, `-D warnings`) and MUST pass
  `cargo clippy` with no warnings.
- Code MUST be formatted with `rustfmt`; unformatted code MUST fail CI.
- `unsafe` MUST be localized and justified with a comment; shared and firmware crates MUST
  minimize it.
- Shared crates MUST declare and preserve `no_std` compatibility where firmware requires it
  (Principle IV).

**TypeScript**

- `strict` mode MUST be enabled. `any` MUST be avoided and explicitly justified where
  genuinely unavoidable.
- Code MUST pass the linter and formatter with no warnings.
- Public module boundaries MUST carry explicit types; implicit `any` at a boundary is
  prohibited.

**Contracts & Documentation**

- Every public contract — shared model types, the device protocol, the event schema,
  asset-compiler output, and Tauri commands — MUST be documented (Rustdoc / TSDoc) at its
  point of definition.
- The device protocol and the event schema MUST be versioned; a breaking change MUST bump the
  version and update all consumers in the same change.

**Continuous Integration**

- CI MUST run build, format check, lint, and the required test suites (Principles III and X)
  on every change.
- Zero-warning CI is the standard. Any warning suppression MUST be narrowly scoped,
  commented, and justified.

## Development Workflow

**Architectural Decision Records (ADRs)**

- Irreversible or hard-to-reverse architectural choices MUST be recorded as an ADR before
  implementation. This includes at minimum: the device wire protocol, the compiled asset
  format, the shared scene-model schema, the event schema, the secure-storage strategy, and
  any scope expansion under Principle XII.
- Each ADR MUST capture context, the decision, alternatives considered, and consequences.
  Superseding an ADR MUST reference the record it replaces.

**Review & Gates**

- Every change MUST pass the Constitution Check in the plan template before merge.
- Visual changes MUST include golden-frame / frame-hash evidence (Principle III).
- Changes crossing the desktop ↔ firmware boundary MUST include or update protocol contract
  tests (Principle X).
- Reviewers MUST verify compliance with the applicable principles. Violations MUST be fixed or
  explicitly justified in the plan's Complexity Tracking table before merge.

## Governance

- This constitution supersedes other development practices and conventions. Where a practice
  conflicts with a principle here, the principle prevails.
- **Amendments**: proposed via a pull request that modifies this document, including the
  rationale and a propagation note for affected templates and code. Amendments require the
  project maintainer's approval before merge.
- **Versioning**: this document uses semantic versioning. MAJOR covers backward-incompatible
  governance changes or the removal/redefinition of a principle; MINOR covers a new
  principle/section or materially expanded guidance; PATCH covers clarifications, wording, and
  non-semantic refinements.
- **Compliance review**: all pull requests and reviews MUST verify compliance with these
  principles. Complexity and deviations MUST be justified in-plan; unjustified violations
  block merge.
- **Runtime guidance**: contributor- and agent-facing guidance (for example, README,
  quickstart docs, and agent guidance files) MUST stay consistent with this constitution. On
  any conflict, this constitution is authoritative.

**Version**: 1.0.0 | **Ratified**: 2026-07-17 | **Last Amended**: 2026-07-17
