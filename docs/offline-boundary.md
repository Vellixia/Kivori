# Offline runtime boundary

**Scope**: Feature 001 / current implemented foundation  
**Requirements**: FR-029, SC-006 | **Enforced by**: `scripts/check-offline-deps.sh`,
`scripts/check-frontend-offline.mjs`, `apps/desktop/src-tauri/tests/offline_smoke.rs`

> This document describes the network boundary implemented by Feature 001. It is not a permanent prohibition on all future native networking. The current product PRD permits network-backed update discovery while requiring already-installed core functionality to remain offline-first. When update delivery is implemented, the dependency guards should be narrowed deliberately so approved native update code can use the network without granting network access to firmware/shared crates/the webview or making normal startup depend on connectivity.

Kivori is **offline-first**. The only link between the desktop app and the device is USB serial. No
core behaviour may depend on the internet, a cloud service, a CDN, telemetry, or a licence check.

## What must work with no network

Everything in the Feature 001 foundation. With the machine fully offline:

- Device discovery (USB VID/PID enumeration) and the handshake.
- The connection lifecycle: connect, heartbeat, incompatible-device handling, bounded reconnect, and
  within-process desired-state restoration.
- Rendering: the shared renderer, the compiled asset blob, and the dev-only Device Studio preview.
- State control (`set_desired_state` / `mirror_state`) and connection/diagnostics surfacing.
- App launch: the core starts and reaches its normal idle without waiting on any network I/O.

## What is prohibited by the current Feature 001 implementation boundary

- **First-party network clients.** No current first-party crate (either workspace) may declare an
  HTTP/socket client (`reqwest`, `ureq`, `hyper`, `isahc`, `surf`, `curl`, `tonic`, …) as a direct
  dependency. Future update networking requires an explicit, isolated native boundary rather than
  weakening the entire application.
- **Remote frontend assets.** The webview must reference no remote URL — no CDN script/style/font, no
  remote image, no `fetch`/XHR/WebSocket to a remote origin. Fonts, icons, scripts, and styles are
  bundled locally by Vite; the only permitted `http(s)` origin is the local dev server
  (`localhost`/`127.0.0.1`) during development.
- **Telemetry / analytics / phone-home** of any kind, and any **startup network wait**.
- Cloud accounts and licence servers. Automatic update checks are not implemented by Feature 001;
  future update discovery follows the PRD/User Story contract and `docs/technical-research.md`.

## Enforcement

| Guard | Checks |
|-------|--------|
| `scripts/check-offline-deps.sh` | No current first-party crate (root **and** firmware workspaces) directly depends on a network-client crate (`cargo metadata --no-deps`). |
| `scripts/check-frontend-offline.mjs` | No remote URLs in `apps/desktop/index.html` or `apps/desktop/src/**` (only the local dev origin is allowed). |
| `apps/desktop/src-tauri/tests/offline_smoke.rs` | The native core + host-sim device start and reach idle with no external services and no startup network wait. |

All three run in host CI (`.github/workflows/host.yml`); none require hardware or the internet.
