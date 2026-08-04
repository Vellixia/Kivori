# Offline runtime boundary

**Requirements**: FR-029, SC-006 | **Enforced by**: `scripts/check-offline-deps.sh`,
`scripts/check-frontend-offline.mjs`, `apps/desktop/src-tauri/tests/offline_smoke.rs`

Kivori is **offline-first**. The only link between the desktop app and the device is USB serial. No
core behaviour may depend on the internet, a cloud service, a CDN, telemetry, or a licence check.

## What must work with no network

Everything. With the machine fully offline:

- Device discovery (USB VID/PID enumeration) and the handshake.
- The connection lifecycle: connect, heartbeat, incompatible-device handling, bounded reconnect, and
  within-process desired-state restoration.
- Rendering: the shared renderer, the compiled asset blob, and the dev-only Device Studio preview.
- State control (`set_desired_state` / `mirror_state`) and connection/diagnostics surfacing.
- App launch: the core starts and reaches its normal idle without waiting on any network I/O.

## What is prohibited

- **First-party network clients.** No crate in this repository (either workspace) may declare an
  HTTP/socket client (`reqwest`, `ureq`, `hyper`, `isahc`, `surf`, `curl`, `tonic`, …) as a direct
  dependency. The device link is USB only; there is no network surface.
- **Remote frontend assets.** The webview must reference no remote URL — no CDN script/style/font, no
  remote image, no `fetch`/XHR/WebSocket to a remote origin. Fonts, icons, scripts, and styles are
  bundled locally by Vite; the only permitted `http(s)` origin is the local dev server
  (`localhost`/`127.0.0.1`) during development.
- **Telemetry / analytics / phone-home** of any kind, and any **startup network wait**.
- Cloud accounts, licence servers, and auto-update checks (also out of scope per Non-Goals).

## Enforcement

| Guard | Checks |
|-------|--------|
| `scripts/check-offline-deps.sh` | No first-party crate (root **and** firmware workspaces) directly depends on a network-client crate (`cargo metadata --no-deps`). |
| `scripts/check-frontend-offline.mjs` | No remote URLs in `apps/desktop/index.html` or `apps/desktop/src/**` (only the local dev origin is allowed). |
| `apps/desktop/src-tauri/tests/offline_smoke.rs` | The native core + host-sim device start and reach idle with no external services and no startup network wait. *(Lands with the run-loop wiring.)* |

All three run in host CI (`.github/workflows/host.yml`); none require hardware or the internet.
