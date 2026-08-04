import { create } from 'zustand';
import type { CompanionState } from '../../lib/ipc/types';

// The shared renderer loops scenes, so any `elapsedMs` is valid; this constant just bounds the scrub
// slider to one nominal loop. STEP_MS ≈ one frame at 30fps (Principle III deterministic timing).
export const SCENE_DURATION_MS = 4000;
export const STEP_MS = 33;

export interface StudioState {
  state: CompanionState;
  elapsedMs: number;
  playing: boolean;
  setState: (state: CompanionState) => void;
  seek: (elapsedMs: number) => void;
  step: (deltaMs?: number) => void;
  play: () => void;
  pause: () => void;
  toggle: () => void;
  /** Advances the timeline during playback (driven by the render loop). */
  advance: (deltaMs: number) => void;
}

/** Wraps a millisecond offset into `[0, SCENE_DURATION_MS)`, matching the renderer's looping. */
function loop(ms: number): number {
  const wrapped = Math.round(ms) % SCENE_DURATION_MS;
  return wrapped < 0 ? wrapped + SCENE_DURATION_MS : wrapped;
}

export const useStudioStore = create<StudioState>((set) => ({
  state: 'idle',
  elapsedMs: 0,
  playing: false,
  setState: (state) => set({ state }),
  seek: (elapsedMs) => set({ elapsedMs: loop(elapsedMs) }),
  step: (deltaMs = STEP_MS) =>
    set((s) => ({ elapsedMs: loop(s.elapsedMs + deltaMs), playing: false })),
  play: () => set({ playing: true }),
  pause: () => set({ playing: false }),
  toggle: () => set((s) => ({ playing: !s.playing })),
  advance: (deltaMs) => set((s) => ({ elapsedMs: loop(s.elapsedMs + deltaMs) })),
}));
