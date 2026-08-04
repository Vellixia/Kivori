import { beforeEach, describe, expect, it } from 'vitest';
import { SCENE_DURATION_MS, STEP_MS, useStudioStore } from '../store';

beforeEach(() => {
  useStudioStore.setState({ state: 'idle', elapsedMs: 0, playing: false });
});

describe('studio store', () => {
  it('sets the companion state', () => {
    useStudioStore.getState().setState('happy');
    expect(useStudioStore.getState().state).toBe('happy');
  });

  it('seek wraps into the loop range', () => {
    useStudioStore.getState().seek(SCENE_DURATION_MS + 100);
    expect(useStudioStore.getState().elapsedMs).toBe(100);
    useStudioStore.getState().seek(-50);
    expect(useStudioStore.getState().elapsedMs).toBe(SCENE_DURATION_MS - 50);
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

  it('advance wraps past the end of the loop', () => {
    useStudioStore.getState().seek(SCENE_DURATION_MS - 10);
    useStudioStore.getState().advance(30);
    expect(useStudioStore.getState().elapsedMs).toBe(20);
  });
});
