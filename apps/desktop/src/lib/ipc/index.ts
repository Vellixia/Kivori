// Typed IPC wrappers — the frontend's ONLY door to the native core (contracts/ipc.md §1).
//
// Selection is decided at the module boundary, not merely at runtime (T124):
//   • Inside Tauri  → real `invoke`/`listen`.
//   • DEV browser   → the lazily-imported `./mock` (the static `import.meta.env.DEV` guard lets Vite
//                     drop `./mock` from production bundles).
//   • Production, not Tauri → throw. Production NEVER silently falls back to mock behaviour.

import type {
  AppInfoDto,
  CompanionState,
  ConnectionStatusDto,
  DiagnosticEventDto,
  SendableState,
} from './types';

/// Handle returned by an event subscription; call it to unsubscribe.
export type Unlisten = () => void;

/// A live preview-frame stream (contracts/ipc.md §3). `close()` cancels it natively.
export interface PreviewStream {
  close: () => Promise<void>;
}

/// True when running inside the Tauri webview (the core injects this global in v2).
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const core = await import('@tauri-apps/api/core');
  return core.invoke<T>(command, args);
}

// Dev-only browser mock, loaded lazily and ONLY in dev builds. The static `import.meta.env.DEV` guard
// is compiled to `false` in production, so bundlers eliminate the `./mock` import from prod output.
async function devMock(): Promise<typeof import('./mock')> {
  if (!import.meta.env.DEV) {
    throw new Error(
      'Kivori: the native runtime is unavailable and there is no mock in production.',
    );
  }
  return import('./mock');
}

function unavailable(): never {
  throw new Error('Kivori: the native runtime is unavailable.');
}

export async function getAppInfo(): Promise<AppInfoDto> {
  if (isTauri()) return invoke<AppInfoDto>('get_app_info');
  if (import.meta.env.DEV) return (await devMock()).mockAppInfo();
  return unavailable();
}

export async function getConnectionStatus(): Promise<ConnectionStatusDto> {
  if (isTauri()) return invoke<ConnectionStatusDto>('get_connection_status');
  if (import.meta.env.DEV) return (await devMock()).mockConnectionStatus();
  return unavailable();
}

export async function listStates(): Promise<CompanionState[]> {
  if (isTauri()) return invoke<CompanionState[]>('list_states');
  if (import.meta.env.DEV) return (await devMock()).mockListStates();
  return unavailable();
}

export async function setDesiredState(state: SendableState): Promise<void> {
  if (isTauri()) return invoke<void>('set_desired_state', { state });
  if (import.meta.env.DEV) return;
  return unavailable();
}

export async function getDiagnostics(limit: number): Promise<DiagnosticEventDto[]> {
  if (isTauri()) return invoke<DiagnosticEventDto[]>('get_diagnostics', { limit });
  if (import.meta.env.DEV) return (await devMock()).mockDiagnostics();
  return unavailable();
}

/**
 * Fetches a rendered preview frame as RGBA8888 (240×240, row-major) — the canonical RGB565→RGBA
 * expansion happens in Rust (constraint 2). Dev-only command; used for scrub/step, where an exact
 * frame for an exact millisecond is required (SC-011).
 */
export async function renderPreviewFrame(
  state: CompanionState,
  elapsedMs: number,
): Promise<Uint8ClampedArray> {
  if (isTauri()) {
    const buffer = await invoke<ArrayBuffer>('render_preview_frame', { state, elapsedMs });
    return new Uint8ClampedArray(buffer);
  }
  if (import.meta.env.DEV) return (await devMock()).mockPreviewFrame(state, elapsedMs);
  return unavailable();
}

/** Device Studio affordance: identical to {@link setDesiredState} but labelled as a developer action. */
export async function mirrorState(state: SendableState): Promise<void> {
  if (isTauri()) return invoke<void>('mirror_state', { state });
  if (import.meta.env.DEV) return;
  return unavailable();
}

/** Subscribes to connection-status changes; resolves to an unsubscribe handle. */
export async function onConnectionStatus(
  handler: (status: ConnectionStatusDto) => void,
): Promise<Unlisten> {
  if (isTauri()) {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<ConnectionStatusDto>('connection://status', (event) => handler(event.payload));
  }
  if (import.meta.env.DEV) {
    handler((await devMock()).mockConnectionStatus());
    return () => {};
  }
  return unavailable();
}

/** Subscribes to safe diagnostic events; resolves to an unsubscribe handle. */
export async function onDiagnostic(
  handler: (diagnostic: DiagnosticEventDto) => void,
): Promise<Unlisten> {
  if (isTauri()) {
    const { listen } = await import('@tauri-apps/api/event');
    return listen<DiagnosticEventDto>('diagnostics://event', (event) => handler(event.payload));
  }
  if (import.meta.env.DEV) return () => {};
  return unavailable();
}

/**
 * Opens a native preview-frame stream at a capped `fps` (contracts/ipc.md §3). Each frame arrives as
 * raw RGBA8888 straight from the shared renderer and is acknowledged, which is what releases the
 * native back-pressure slot. The canvas only blits these bytes — no frame is produced in TypeScript.
 */
export async function openPreviewStream(
  state: CompanionState,
  fps: number,
  onFrame: (frame: Uint8ClampedArray) => void,
): Promise<PreviewStream> {
  if (isTauri()) {
    const { Channel } = await import('@tauri-apps/api/core');
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (buffer): void => {
      onFrame(new Uint8ClampedArray(buffer));
      // Acknowledge so the native producer may render the next frame (bounded in-flight frames).
      void invoke<boolean>('ack_preview_frame', { handle: channel.id });
    };
    const handle = await invoke<number>('open_preview_stream', { state, fps, channel });
    return {
      close: async (): Promise<void> => {
        await invoke<boolean>('close_preview_stream', { handle });
      },
    };
  }
  if (import.meta.env.DEV) return (await devMock()).mockPreviewStream(state, fps, onFrame);
  return unavailable();
}
