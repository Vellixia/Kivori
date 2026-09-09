import { create } from 'zustand';
import type { CompanionState, AnimationEvent } from '../../lib/ipc/types';
import { MAX_ANIMATION_EVENTS } from '../../lib/ipc/types';

// Preview time stays absolute so transitions survive loop boundaries and backward seeks.
// The slider grows in four-second windows. STEP_MS is approximately one frame at 30fps.
export const SCENE_DURATION_MS = 4000;
export const STEP_MS = 33;

export interface StudioState {
  state: CompanionState;
  elapsedMs: number;
  playing: boolean;
  events: AnimationEvent[];
  reset: () => void;
  setState: (state: CompanionState) => void;
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
  reset: () => set({ state: 'idle', elapsedMs: 0, events: [], playing: false }),
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
