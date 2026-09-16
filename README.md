# Kivori

Kivori is a physical desktop companion that lets people control their computer through tactile input while representing important desktop state on a dedicated display.

> **Product thesis:** Control the desktop physically. Understand the desktop visually.

## Project status

Feature 001 (Device Connection Foundation) established the current connection, protocol, deterministic rendering, Device Studio, firmware simulation, and ESP32-C3/ST7789 runtime foundation. Its software work is complete; remaining physical/platform acceptance items are recorded in its validation ledger.

The product contract now extends beyond Feature 001. Product behavior is defined by the PRD and User Story Contract. Technical research records implementation possibilities and uncertainties; accepted durable technical choices belong in ADRs.

## Documentation map

| Need | Source |
|---|---|
| Product goals, scope, and acceptance gates | [`docs/product/prd.md`](docs/product/prd.md) |
| Exact user-visible behavior | [`docs/product/user-story-contract.md`](docs/product/user-story-contract.md) |
| Engineering invariants and decision discipline | [`docs/engineering-principles.md`](docs/engineering-principles.md) |
| Cross-platform technical research | [`docs/research/technical-research.md`](docs/research/technical-research.md) |
| Accepted durable architecture decisions | [`docs/adr/`](docs/adr/) |
| Feature 001 requirements, implementation record, contracts, and evidence | [`docs/features/001-device-connection-foundation/`](docs/features/001-device-connection-foundation/) |
| Feature 001 closure status | [`docs/features/001-device-connection-foundation/closure-status.md`](docs/features/001-device-connection-foundation/closure-status.md) |
| Physical/manual validation ledger | [`docs/features/001-device-connection-foundation/validation-checklist.md`](docs/features/001-device-connection-foundation/validation-checklist.md) |
| Superpowers design records | [`docs/superpowers/specs/`](docs/superpowers/specs/) |
| Superpowers implementation plans | [`docs/superpowers/plans/`](docs/superpowers/plans/) |
| Wokwi simulation | [`sim/wokwi/README.md`](sim/wokwi/README.md) |

## Document authority

When documents disagree, use this hierarchy:

1. **PRD + User Story Contract** — product behavior and user guarantees.
2. **Engineering Principles** — implementation invariants and development discipline.
3. **ADRs** — durable technical decisions that have been explicitly accepted.
4. **Technical Research** — researched suggestions, alternatives, caveats, and required validation; intentionally challengeable.
5. **Feature records** — requirements, architecture, validation, contracts, and evidence for a particular implemented slice.
6. **Superpowers specs/plans** — design and execution artifacts for individual changes.

## Development workflow

Kivori uses **Superpowers** as the active workflow for new engineering work. Spec Kit is retired.

```text
problem / idea
    ↓
brainstorm + research
    ↓
approved design (docs/superpowers/specs/)
    ↓
implementation plan (docs/superpowers/plans/)
    ↓
implementation + tests
    ↓
verification / review
    ↓
ADR when a durable architecture choice is accepted
```

Research recommendations are not mandates. If implementation evidence disproves a recommendation, update the research or ADR while preserving the product contract.

## Repository layout

```text
Kivori/
├── README.md
├── docs/
│   ├── product/
│   │   ├── prd.md
│   │   └── user-story-contract.md
│   ├── engineering-principles.md
│   ├── research/
│   │   └── technical-research.md
│   ├── adr/
│   ├── features/
│   │   └── 001-device-connection-foundation/
│   └── superpowers/
│       ├── specs/
│       └── plans/
├── apps/
├── crates/
├── firmware/
├── sim/
├── tests/
└── tools/
```

Feature records contain durable project knowledge, not workflow scaffolding. The retired Feature 001 Spec Kit `plan.md`, `tasks.md`, and requirement-writing checklist remain available in Git history if historical investigation is needed.

## Common commands

The repository uses [`just`](https://github.com/casey/just) as a convenience command runner.

```bash
just lint             # Rust + TypeScript formatting/lint/type checks
just test             # host workspace + frontend tests
just build            # host workspace + frontend build
just fw-check          # shared no_std crates for the RISC-V target
just fw-test           # firmware core host-simulation tests
just fw-build          # build physical ESP32-C3 firmware
just fw-flash          # flash and monitor the verified physical profile
just sim-test          # Wokwi integration scenarios
just golden            # deterministic rendering golden-frame tests
just check-boundaries  # shared-crate dependency firewall
```

See [`docs/features/001-device-connection-foundation/quickstart.md`](docs/features/001-device-connection-foundation/quickstart.md) and [`sim/wokwi/README.md`](sim/wokwi/README.md) for environment-specific setup and validation detail.
