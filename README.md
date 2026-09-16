# Kivori

Kivori is a physical desktop companion that lets people control their computer through tactile input while representing important desktop state on a dedicated display.

> **Product thesis:** Control the desktop physically. Understand the desktop visually.

## Project status

The repository currently contains the completed software foundation for Feature 001 (device connection, deterministic rendering, protocol/session handling, Device Studio, firmware host simulation, and the physical ESP32-C3/ST7789 runtime). Feature 001 is **software complete; manual acceptance remains outstanding** for the physical/platform checks recorded in the validation ledger.

The product contract has since expanded beyond that foundation. Product behavior is defined by the PRD and User Story Contract; technical research documents possible implementations but is intentionally challengeable.

## Documentation map

| Need | Source |
|---|---|
| Product goals, scope, and acceptance gates | [`PRD.md`](PRD.md) |
| Exact user-visible behavioral contract | [`docs/user-story-contract.md`](docs/user-story-contract.md) |
| Engineering invariants and decision hierarchy | [`docs/engineering-principles.md`](docs/engineering-principles.md) |
| Cross-platform implementation research | [`docs/technical-research.md`](docs/technical-research.md) |
| Accepted durable architecture decisions | [`docs/adr/`](docs/adr/) |
| Current Feature 001 built-system overview | [`docs/architecture.md`](docs/architecture.md) |
| Feature 001 requirements, research, contracts, plan/history | [`specs/001-device-connection-foundation/`](specs/001-device-connection-foundation/) |
| Feature 001 closure status | [`specs/001-device-connection-foundation/closure-status.md`](specs/001-device-connection-foundation/closure-status.md) |
| Physical/manual validation ledger | [`docs/validation-checklist.md`](docs/validation-checklist.md) |
| Superpowers design records | [`docs/superpowers/specs/`](docs/superpowers/specs/) |
| Superpowers implementation plans | [`docs/superpowers/plans/`](docs/superpowers/plans/) |
| Wokwi simulation | [`sim/wokwi/README.md`](sim/wokwi/README.md) |

### Document authority

For future product work, use this order when documents appear to disagree:

1. **PRD + User Story Contract** — product behavior and user guarantees.
2. **Engineering Principles** — implementation invariants and development discipline.
3. **ADRs** — accepted durable technical decisions.
4. **Technical Research** — researched suggestions, alternatives, caveats, and required spikes; not immutable.
5. **Feature records** — requirements/evidence for a particular implemented slice.
6. **Superpowers specs/plans** — design and execution artifacts for a particular change.

Historical Feature 001 `plan.md`, `tasks.md`, and checklists remain in the repository as audit history. They are **not the active planning workflow**.

## Development workflow

Kivori uses **Superpowers-style brainstorming, design, planning, implementation, debugging, review, and verification** for new engineering work. Spec Kit is retired from this repository.

A typical change should flow as:

```text
problem / idea
    ↓
brainstorm and research
    ↓
approved design (docs/superpowers/specs/)
    ↓
implementation plan (docs/superpowers/plans/)
    ↓
implementation + tests
    ↓
verification / review
    ↓
ADR update when a durable architecture choice was made
```

Do not turn temporary implementation suggestions into product requirements. If implementation evidence disproves a research recommendation, update the research/ADR while preserving the PRD/User Story contract.

## Repository layout

```text
apps/desktop/                 Tauri native core + React UI
crates/                       shared host/firmware Rust crates
firmware/esp32-c3/            isolated no_std ESP32-C3 workspace
assets/                       source visual assets
sim/wokwi/                    hardware simulation scenarios
specs/                        historical feature requirement/evidence packages
docs/adr/                     durable architecture decisions
docs/superpowers/             active design and implementation planning records
tests/                        cross-cutting host tests / golden frames
tools/                        build/development tools
scripts/                      CI and validation scripts
```

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

See the feature quickstart and Wokwi README for environment-specific setup and physical/simulator validation.
