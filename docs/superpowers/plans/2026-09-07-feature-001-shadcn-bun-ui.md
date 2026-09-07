# Feature 001 shadcn + Bun UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the Feature 001 desktop frontend from pnpm/raw markup to Bun + Tailwind 4 + current shadcn Base UI components and deliver a polished modern dashboard without changing device behavior.

**Architecture:** Keep Tauri/React/Vite/Vitest/Zustand and every IPC/rendering contract intact. Replace only the package-manager/workspace layer and frontend presentation layer. Introduce shadcn as local UI primitives under the desktop app, then compose existing Feature 001 screens from those primitives.

**Tech Stack:** Bun 1.4.x, React 18, Vite 5, Vitest 2, Tailwind CSS 4, shadcn, Base UI, Lucide, Zustand, Tauri v2.

**Spec:** `docs/superpowers/specs/2026-09-07-feature-001-shadcn-bun-ui-design.md`

## Global Constraints

- Stay in Feature 001 and PR #1.
- No Feature 002 files.
- Preserve React 18, Vite, Vitest, Zustand, Tauri, renderer, IPC, protocol, and firmware behavior.
- Bun is package manager/workspace runner only; do not switch bundler or test runner.
- Use shadcn components and semantic tokens rather than a parallel custom component system.
- Preserve all existing accessibility and offline-runtime requirements.
- Do not claim new physical-hardware evidence.

---

### Task 1: Bun workspace and shadcn foundation

**Files:**
- Modify: `package.json`
- Delete after lock migration: `pnpm-workspace.yaml`, `pnpm-lock.yaml`
- Create: `bun.lock`
- Modify: `.github/workflows/host.yml`
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/tsconfig.json`
- Modify: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/components.json`
- Create: `apps/desktop/src/index.css`
- Modify: `apps/desktop/src/main.tsx`

**Interfaces:**
- Produces a Bun workspace installable with `bun install --frozen-lockfile`.
- Produces `@/* -> apps/desktop/src/*` resolution for TypeScript and Vite.
- Produces Tailwind 4 + shadcn global CSS available to every frontend component.

- [ ] Replace root pnpm metadata/scripts with Bun workspace metadata and equivalent workspace scripts.
- [ ] Add current shadcn/Base UI/Tailwind/Lucide dependencies to `kivori-desktop-ui` while retaining React 18/Vite/Vitest.
- [ ] Configure Vite with `@tailwindcss/vite` and the `@` alias.
- [ ] Configure TypeScript with the same alias.
- [ ] Add `components.json` using Base UI, neutral base color, CSS variables, Lucide, and `@/components` / `@/lib` aliases.
- [ ] Add global CSS importing Tailwind, `tw-animate-css`, and `shadcn/tailwind.css`, then define shadcn neutral theme tokens and a compact desktop body baseline.
- [ ] Import `index.css` exactly once from `main.tsx`.
- [ ] Update host/Windows CI to install Bun and execute Bun workspace commands; retain Node setup only where repository guard scripts invoke `node`.
- [ ] Generate and commit `bun.lock`, then remove pnpm lock/workspace files.
- [ ] Verify install, typecheck, lint, Prettier, Vitest, and Vite build before moving to UI composition.

---

### Task 2: Local shadcn primitives

**Files:**
- Create: `apps/desktop/src/components/ui/button.tsx`
- Create: `apps/desktop/src/components/ui/card.tsx`
- Create: `apps/desktop/src/components/ui/badge.tsx`
- Create: `apps/desktop/src/components/ui/tabs.tsx`
- Create: `apps/desktop/src/components/ui/separator.tsx`
- Create: `apps/desktop/src/components/ui/slider.tsx`
- Create: `apps/desktop/src/components/ui/toggle.tsx`
- Create: `apps/desktop/src/components/ui/toggle-group.tsx`
- Create: `apps/desktop/src/components/ui/scroll-area.tsx`
- Create: `apps/desktop/src/components/ui/table.tsx`

**Interfaces:**
- Produces the standard shadcn component surface consumed by Feature 001 screens.
- Components use Base UI primitives where behavior is non-trivial and semantic shadcn classes from `shadcn/tailwind.css`.

- [ ] Add Button with `default`, `secondary`, `outline`, `ghost`, and `destructive` variants plus icon sizes.
- [ ] Add Card family (`Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardAction`, `CardContent`, `CardFooter`).
- [ ] Add Badge variants for status labels.
- [ ] Add Base UI Tabs with default/line list variants.
- [ ] Add Separator and ScrollArea.
- [ ] Add Base UI Slider with controlled-value support.
- [ ] Add Toggle and ToggleGroup for companion-state selection.
- [ ] Add Table primitives for diagnostics.
- [ ] Run frontend typecheck immediately to catch Base UI API/version drift before feature components depend on the wrappers.

---

### Task 3: Dashboard shell and connection overview

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/features/connection/ConnectionStatus.tsx`
- Test: `apps/desktop/src/features/connection/__tests__/connection.test.tsx`

**Interfaces:**
- `App` continues to call `getAppInfo()` and conditionally exposes Device Studio only when both frontend DEV and backend feature flags allow it.
- `ConnectionStatus` keeps its current IPC subscription and test ids (`desired`, `reported`).

- [ ] Extend/adjust the connection tests first to assert the dashboard retains the live status region, device metadata, desired/reported values, and incompatible reason.
- [ ] Build a compact app header and shadcn Tabs shell with Overview, conditional Device Studio, and Diagnostics surfaces.
- [ ] Convert connection status into shadcn Card/Badge presentation with Lucide status icons.
- [ ] Preserve exact status derivation semantics and IPC lifecycle.
- [ ] Run connection tests + axe coverage and fix presentation-only regressions without changing domain behavior.

---

### Task 4: Device Studio shadcn controls

**Files:**
- Modify: `apps/desktop/src/features/device-studio/DeviceStudio.tsx`
- Modify: `apps/desktop/src/features/device-studio/Controls.tsx`
- Test: `apps/desktop/src/features/device-studio/__tests__/controls.test.tsx`

**Interfaces:**
- `useStudioStore` remains authoritative.
- `DevicePreview` remains the only preview surface and continues receiving canonical RGBA bytes.
- `mirrorState` receives only `SendableState` values.

- [ ] Update controls tests first to express equivalent accessible behavior through ToggleGroup/Slider/Button semantics.
- [ ] Compose Device Studio as preview Card + controls Card.
- [ ] Replace raw state buttons with shadcn ToggleGroup.
- [ ] Replace native range markup with shadcn Slider while preserving integer elapsed-ms values and `STEP_MS`.
- [ ] Replace transport buttons with shadcn Button + Lucide icons and preserve disabled mirror behavior.
- [ ] Keep all preview stream/open/close and animation effects untouched except presentation wrappers.
- [ ] Run Device Studio tests, store tests, canvas tests, and axe.

---

### Task 5: Diagnostics shadcn surface

**Files:**
- Modify: `apps/desktop/src/features/connection/Diagnostics.tsx`
- Test: `apps/desktop/src/features/connection/__tests__/diagnostics.test.tsx`

**Interfaces:**
- Keep `VIEW_LIMIT = 20`.
- Keep the same safe DTO formatter; no new diagnostic fields.
- Keep `role="log"`, polite live announcements, and bounded event list semantics.

- [ ] Update diagnostics tests only where DOM structure changes while retaining all redaction/bounds assertions.
- [ ] Render the diagnostics surface with Card + ScrollArea + Table + Badge.
- [ ] Preserve empty state, safe detail formatting, timestamps, categories, connection state, and note text.
- [ ] Run diagnostics tests and axe.

---

### Task 6: Full verification and PR reconciliation

**Files:**
- Modify: `docs/superpowers/plans/2026-09-07-feature-001-shadcn-bun-ui.md` checkboxes/results.
- Modify: PR #1 title/body if needed to include the UI/tooling migration.

**Interfaces:**
- Consumes final branch head.
- Produces fresh reviewable CI evidence.

- [ ] Run/verify `bun install --frozen-lockfile` on CI.
- [ ] Verify frontend typecheck, ESLint, Prettier, Vitest/axe, Vite build, offline guard, and mock-exclusion guard.
- [ ] Verify Ubuntu and Windows Rust host jobs.
- [ ] Verify real Windows Device Studio startup job.
- [ ] Verify firmware, determinism, and Wokwi workflows.
- [ ] Review final diff for Feature 001-only scope and no accidental protocol/renderer/firmware behavior change.
- [ ] Update PR metadata with Bun + shadcn UI scope and exact final run/head evidence.
- [ ] Leave PR #1 open and unmerged.
