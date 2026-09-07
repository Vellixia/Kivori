# Feature 001 shadcn + Bun UI Design

**Date:** 2026-09-07

## Goal

Modernize the Feature 001 desktop interface inside PR #1 using the current shadcn design system and Bun tooling, while preserving the existing Tauri, React, Vite, Vitest, Zustand, renderer, IPC, device lifecycle, and accessibility contracts.

## Product direction

Use a polished modern companion dashboard. The visual language is intentionally restrained: neutral shadcn tokens, compact desktop spacing, clear status hierarchy, and no custom ornamental component library layered on top of shadcn.

## Frontend stack

- Tauri v2 remains the native shell.
- React 18 remains the UI runtime.
- Vite remains the dev server and bundler.
- Vitest remains the frontend test runner.
- Zustand remains Device Studio state management.
- Bun 1.4.x becomes the JavaScript package manager and workspace script runner.
- Tailwind CSS 4 provides utility styling.
- shadcn uses the current Base UI foundation and semantic shadcn Tailwind layer.
- Lucide supplies interface icons.

The migration does not switch to Bun's bundler/test runner and does not upgrade React. Those are unrelated variables and would make regressions harder to isolate.

## shadcn rules

1. Local primitives live under `apps/desktop/src/components/ui/` using shadcn component names and APIs.
2. `components.json` is the source-of-truth configuration for future `bunx shadcn@latest add ...` usage.
3. Styling uses shadcn semantic tokens and Tailwind utilities. Do not create a parallel theme or bespoke button/card/tab system.
4. Use shadcn components when a matching component exists. Feature components may compose them but should not clone their behavior.
5. The baseline is neutral, light-first, and compatible with shadcn's dark token set. Theme switching is not part of Feature 001.

## Application shell

The desktop window becomes a compact dashboard with:

- a top application header containing Kivori identity, a development badge when Device Studio is present, and connection context;
- a shadcn tab navigation with Overview, Device Studio when enabled, and Diagnostics;
- an Overview surface that presents the existing connection information as cards and badges;
- Device Studio as a two-column desktop workspace: canonical 240x240 preview on the left, shadcn controls on the right;
- Diagnostics as a bounded, scrollable shadcn table/card surface.

No routing library is added. Feature 001 has only three local surfaces and shadcn Tabs are sufficient.

## Connection UI

`ConnectionStatus` keeps the same IPC subscriptions and six UI states. Presentation changes to:

- status badge with an icon and semantic variant;
- device metadata card for firmware, protocol, and redacted device id;
- desired/reported state summary;
- incompatible/error explanation in the existing accessible status region.

The live region behavior remains intact.

## Device Studio

The canonical render path is unchanged. `DevicePreview` continues to display bytes produced by the shared Rust renderer.

Controls use shadcn components:

- ToggleGroup for six companion states;
- Slider for elapsed time;
- Button controls with Lucide icons for play/pause, step, and mirror;
- Card/Badge/Separator for layout and state context.

The existing Zustand store remains authoritative. `booting` and `offline` remain preview-only and disable mirror.

## Diagnostics

The native redaction contract is unchanged. The UI continues to render only the safe DTO fields already exposed by the backend. The view uses Card + ScrollArea + Table + Badge and keeps the existing DOM entry limit of 20.

## Bun workspace migration

Root `package.json` becomes the workspace definition using `workspaces: ["apps/*", "packages/*"]` and declares Bun as the package manager. `pnpm-workspace.yaml` and `pnpm-lock.yaml` are removed after a committed `bun.lock` exists.

Root scripts become Bun workspace commands but keep the same behavior: typecheck, lint, format, test, and build.

Host CI and Windows startup CI install Bun and use `bun install --frozen-lockfile`; Node remains available where repository guard scripts explicitly invoke `node`.

## Accessibility

Existing requirements remain mandatory:

- connection state remains a polite live region;
- diagnostics remain a live log;
- all controls retain accessible names;
- keyboard operation works through Base UI/shadcn primitives;
- visible focus treatment comes from shadcn tokens;
- axe coverage remains green.

## Offline boundary

All CSS, icons, JavaScript, fonts, and component code remain local. No CDN, analytics, hosted font, or network runtime dependency is introduced.

## Testing and acceptance

The migration is complete only when:

1. Bun installs from a committed lockfile on Linux and Windows CI;
2. frontend typecheck, ESLint, Prettier, Vitest, axe, and Vite build pass;
3. existing ConnectionStatus, Diagnostics, Device Studio, canvas, and store behavior tests pass, adjusted only for equivalent shadcn accessibility semantics;
4. offline and production mock-exclusion guards pass;
5. the real Windows Tauri Device Studio binary starts successfully;
6. Rust host, firmware, determinism, and Wokwi workflows remain green;
7. no Feature 002 files are changed.

## Non-goals

- React 19 migration
- Bun bundler or bun:test migration
- router introduction
- backend/device protocol changes
- renderer changes
- Feature 002 work
- theme selector or user customization
- new hardware acceptance claims
