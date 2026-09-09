import { beforeEach, describe, expect, it } from 'vitest';
import { SCENE_DURATION_MS, STEP_MS, useStudioStore } from '../store';

beforeEach(() => {
  useStudioStore.setState({ state: 'idle', elapsedMs: 0, playing: false, events: [] });
});

describe('studio store', () => {
  it('normalizes fractional timestamps once and bounds event history', () => {
    const s = useStudioStore.getState;
    s().seek(1.6);
    expect(s().elapsedMs).toBe(2);
    for (let i = 0; i < 257; i++) s().setState(i % 2 ? 'idle' : 'happy');
    expect(s().events).toHaveLength(256);
    s().reset();
    expect(s().events).toHaveLength(0);
    expect(s().elapsedMs).toBe(0);
  });
  it('sets the companion state', () => {
    useStudioStore.getState().setState('happy');
    expect(useStudioStore.getState().state).toBe('happy');
  });

  it('seek preserves the absolute animation timeline', () => {
    useStudioStore.getState().seek(SCENE_DURATION_MS + 100);
    expect(useStudioStore.getState().elapsedMs).toBe(SCENE_DURATION_MS + 100);
    useStudioStore.getState().seek(-50);
    expect(useStudioStore.getState().elapsedMs).toBe(0);
  });

  it('step advances one frame and pauses playback', () => {
    useStudioStore.getState().play();
    useStudioStore.getState().step();
    expect(useStudioStore.getState().elapsedMs).toBe(STEP_MS);
    expect(useStudioStore.getState().playing).toBe(false);
  });

  it('play, pause, and toggle flip the playing flag', () => {
    useStudioStore.getState().play();
    expect(useStudioStore.getState().playing).toBe(true);
    useStudioStore.getState().pause();
    expect(useStudioStore.getState().playing).toBe(false);
    useStudioStore.getState().toggle();
    expect(useStudioStore.getState().playing).toBe(true);
  });

  it('advance crosses loop boundaries without resetting transitions', () => {
    useStudioStore.getState().seek(SCENE_DURATION_MS - 10);
    useStudioStore.getState().advance(30);
    expect(useStudioStore.getState().elapsedMs).toBe(SCENE_DURATION_MS + 20);
  });

  it('retains interrupted changes for exact native replay when seeking', () => {
    const s = useStudioStore.getState;
    s().seek(100);
    s().setState('happy');
    s().seek(250);
    s().setState('sleeping');
    s().seek(175);
    expect(s().events).toEqual([
      { atMs: 100, state: 'happy' },
      { atMs: 250, state: 'sleeping' },
    ]);
    expect(s().state).toBe('happy');
    s().setState('busy');
    expect(s().events).toEqual([
      { atMs: 100, state: 'happy' },
      { atMs: 175, state: 'busy' },
    ]);
  });
});
