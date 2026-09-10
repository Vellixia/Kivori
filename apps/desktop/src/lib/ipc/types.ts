// DTOs crossing the desktop-core ↔ webview IPC boundary. Mirrors contracts/ipc.md §4 exactly; these
// are the ONLY shapes the webview sees (no raw device ids, payload bytes, serial handles, or paths).

export type SendableState = 'idle' | 'happy' | 'busy' | 'sleeping';
export type CompanionState = 'booting' | 'idle' | 'happy' | 'busy' | 'sleeping' | 'offline';
export type MascotPersonality = 'cozy' | 'playful' | 'calm';
export type MascotAction = 'greet' | 'pet' | 'tickle' | 'surprise' | 'comfort';
export type ConnectionState =
  'connecting' | 'connected' | 'incompatible' | 'disconnected' | 'error';

export interface ProtocolVersion {
  major: number;
  minor: number;
}

export interface AppInfoDto {
  appVersion: string;
  protocolVersion: ProtocolVersion;
  supportedMajors: number[];
  deviceStudioEnabled: boolean;
}

export interface DeviceInfoDto {
  firmwareVersion: string;
  protocolVersion: ProtocolVersion;
  deviceIdHashShort: string;
}

export interface FirmwareStatusDto {
  available: boolean;
  phase: 'idle' | 'preparing' | 'flashing' | 'reconnecting' | 'succeeded' | 'failed';
  message: string;
  imageSize: number;
}

export interface ConnectionStatusDto {
  connection: ConnectionState;
  desired: SendableState;
  reported: CompanionState | null;
  device: DeviceInfoDto | null;
  incompatibleReason: string | null;
  retryCount: number;
  /** Within-process device-session identity; changes when device uptime may reset. */
  connectionGeneration: number;
  mascotInteraction: boolean;
  mascotAction: MascotActionAppliedDto | null;
}

export interface MascotActionAppliedDto {
  action: MascotAction;
  personality: MascotPersonality;
  seed: number;
  appliedAtMs: number;
}

export type DiagnosticCategory =
  'io' | 'handshake' | 'version' | 'framing' | 'checksum' | 'timeout' | 'busy' | 'bad_payload';

export interface DiagnosticEventDto {
  at: string;
  connection: ConnectionState;
  category: DiagnosticCategory;
  messageType: string | null;
  payloadLen: number | null;
  seq: number | null;
  retryCount: number;
  elapsedMs: number;
  deviceIdHashShort: string | null;
}

/** Every companion state, in canonical display order. */
export const COMPANION_STATES: readonly CompanionState[] = [
  'booting',
  'idle',
  'happy',
  'busy',
  'sleeping',
  'offline',
];

/** States a host may request the device enter (a subset of {@link COMPANION_STATES}). */
export const SENDABLE_STATES: readonly SendableState[] = ['idle', 'happy', 'busy', 'sleeping'];

/** Panel edge length in pixels (square). Matches `DeviceProfile::KIVORI_240`. */
export const PREVIEW_DIM = 240;

/** Canonical preview stream rate (frames per second); the native side caps to this. */
export const PREVIEW_FPS = 30;
/** Semantic changes replayed by the shared Rust animator; no animation math in React. */
export interface AnimationEvent {
  atMs: number;
  state: CompanionState;
}

/** One deterministic social reaction replayed by the native mascot animator. */
export interface MascotActionEvent {
  atMs: number;
  action: MascotAction;
  personality: MascotPersonality;
  seed: number;
  /** Original device uptime from an applied-action acknowledgment. */
  deviceAppliedAtMs?: number;
  /** Device session in which the action was applied. */
  connectionGeneration?: number;
}

export interface AnimationTimeline {
  initialState: CompanionState;
  events: AnimationEvent[];
  actionEvents: MascotActionEvent[];
}
export const MAX_ANIMATION_EVENTS = 256;
