import { create } from 'zustand';
import type {
  CompanionState,
  AnimationEvent,
  MascotAction,
  MascotActionAppliedDto,
  MascotActionEvent,
} from '../../lib/ipc/types';
import { MAX_ANIMATION_EVENTS } from '../../lib/ipc/types';

// Preview time stays absolute so transitions survive loop boundaries and backward seeks.
// The slider grows in four-second windows. STEP_MS is approximately one frame at 30fps.
export const SCENE_DURATION_MS = 4000;
export const STEP_MS = 33;

interface DeviceClockAnchor {
  connectionGeneration: number;
  deviceAtMs: number;
  studioAtMs: number;
}

export interface StudioState {
  state: CompanionState;
  elapsedMs: number;
  playing: boolean;
  events: AnimationEvent[];
  actionEvents: MascotActionEvent[];
  deviceClockAnchor: DeviceClockAnchor | null;
  reset: () => void;
  setState: (state: CompanionState) => void;
  playAction: (action: MascotAction, personality?: MascotActionEvent['personality']) => void;
  recordAppliedAction: (action: MascotActionAppliedDto, connectionGeneration: number) => void;
  seek: (elapsedMs: number) => void;
  step: (deltaMs?: number) => void;
  play: () => void;
  pause: () => void;
  toggle: () => void;
  /** Advances the timeline during playback (driven by the render loop). */
  advance: (deltaMs: number) => void;
}

function time(ms: number): number {
  return Math.max(0, Math.min(0xffffffff, Number.isFinite(ms) ? Math.round(ms) : 0));
}

function stateAt(events: AnimationEvent[], ms: number): CompanionState {
  let state: CompanionState = 'idle';
  for (const event of events) {
    if (event.atMs > ms) break;
    state = event.state;
  }
  return state;
}

export const useStudioStore = create<StudioState>((set) => ({
  state: 'idle',
  elapsedMs: 0,
  playing: false,
  events: [],
  actionEvents: [],
  deviceClockAnchor: null,
  reset: () =>
    set({
      state: 'idle',
      elapsedMs: 0,
      events: [],
      actionEvents: [],
      deviceClockAnchor: null,
      playing: false,
    }),
  setState: (state) =>
    set((s) =>
      s.state === state ||
      s.events.filter((e) => e.atMs <= s.elapsedMs).length >= MAX_ANIMATION_EVENTS
        ? {}
        : {
            state,
            events: [
              ...s.events.filter((event) => event.atMs <= Math.round(s.elapsedMs)),
              { state, atMs: Math.round(s.elapsedMs) },
            ],
          },
    ),
  playAction: (action, personality = 'cozy') =>
    set((s) => {
      const events = s.events.filter((event) => event.atMs <= Math.round(s.elapsedMs));
      const actionEvents = s.actionEvents.filter((event) => event.atMs <= Math.round(s.elapsedMs));
      if (events.length + actionEvents.length >= MAX_ANIMATION_EVENTS) return {};
      return {
        events,
        actionEvents: [
          ...actionEvents,
          {
            action,
            atMs: Math.round(s.elapsedMs),
            personality,
            seed: actionEvents.length + 1,
          },
        ],
      };
    }),
  recordAppliedAction: (action, connectionGeneration) =>
    set((s) => {
      if (
        s.actionEvents.some(
          (event) =>
            event.action === action.action &&
            event.personality === action.personality &&
            event.seed === action.seed &&
            event.deviceAppliedAtMs === action.appliedAtMs &&
            event.connectionGeneration === connectionGeneration,
        )
      )
        return {};
      if (s.events.length + s.actionEvents.length >= MAX_ANIMATION_EVENTS) return {};
      const previous = s.deviceClockAnchor;
      const sameSession = previous?.connectionGeneration === connectionGeneration;
      const deviceDelta = sameSession
        ? action.appliedAtMs >= previous.deviceAtMs
          ? action.appliedAtMs - previous.deviceAtMs
          : previous.deviceAtMs > 0xf0000000
            ? 0x100000000 - previous.deviceAtMs + action.appliedAtMs
            : 0
        : 0;
      const projectedAtMs = sameSession ? time(previous.studioAtMs + deviceDelta) : s.elapsedMs;
      const atMs = Math.max(Math.round(s.elapsedMs), previous?.studioAtMs ?? 0, projectedAtMs);
      const event: MascotActionEvent = {
        action: action.action,
        personality: action.personality,
        seed: action.seed,
        atMs,
        deviceAppliedAtMs: action.appliedAtMs,
        connectionGeneration,
      };
      return {
        actionEvents: [...s.actionEvents, event].sort((a, b) => a.atMs - b.atMs),
        deviceClockAnchor: {
          connectionGeneration,
          deviceAtMs: action.appliedAtMs,
          studioAtMs: atMs,
        },
      };
    }),
  seek: (ms) =>
    set((s) => ({ elapsedMs: time(ms), state: stateAt(s.events, time(ms)), playing: false })),
  step: (deltaMs = STEP_MS) =>
    set((s) => ({
      elapsedMs: time(s.elapsedMs + deltaMs),
      state: stateAt(s.events, time(s.elapsedMs + deltaMs)),
      playing: false,
    })),
  play: () => set({ playing: true }),
  pause: () => set({ playing: false }),
  toggle: () => set((s) => ({ playing: !s.playing })),
  advance: (deltaMs) =>
    set((s) => ({
      elapsedMs: time(s.elapsedMs + deltaMs),
      state: stateAt(s.events, time(s.elapsedMs + deltaMs)),
    })),
}));
