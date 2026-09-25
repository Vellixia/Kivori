import { beforeEach, describe, expect, it } from 'vitest';
import { SCENE_DURATION_MS, STEP_MS, useStudioStore } from '../store';

beforeEach(() => {
  useStudioStore.getState().reset();
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

  it('records seeded social actions at the preview timestamp', () => {
    const s = useStudioStore.getState;
    s().seek(100);
    s().playAction('tickle');
    expect(s().actionEvents).toEqual([
      { action: 'tickle', atMs: 100, personality: 'cozy', seed: 1 },
    ]);
  });

  it('maps each device acknowledgment once without moving or pausing the preview', () => {
    const cue = {
      action: 'tickle' as const,
      personality: 'playful' as const,
      seed: 42,
      appliedAtMs: 900,
    };
    const s = useStudioStore.getState;
    s().seek(1_200);
    s().play();
    s().recordAppliedAction(cue, 1);
    s().recordAppliedAction(cue, 1);
    expect(s().actionEvents).toEqual([
      {
        action: 'tickle',
        personality: 'playful',
        seed: 42,
        atMs: 1_200,
        deviceAppliedAtMs: 900,
        connectionGeneration: 1,
      },
    ]);
    expect(s().elapsedMs).toBe(1_200);
    expect(s().playing).toBe(true);
  });

  it('rebases reset device uptime into a monotonic Studio timeline across reconnects', () => {
    const s = useStudioStore.getState;
    s().seek(5_000);
    s().recordAppliedAction(
      { action: 'greet', personality: 'cozy', seed: 1, appliedAtMs: 20_000 },
      3,
    );
    s().recordAppliedAction(
      { action: 'pet', personality: 'cozy', seed: 2, appliedAtMs: 20_400 },
      3,
    );
    s().seek(5_900);
    s().recordAppliedAction(
      { action: 'surprise', personality: 'calm', seed: 3, appliedAtMs: 25 },
      4,
    );

    expect(s().actionEvents.map((event) => event.atMs)).toEqual([5_000, 5_400, 5_900]);
    expect(s().actionEvents.at(-1)).toMatchObject({
      action: 'surprise',
      personality: 'calm',
      seed: 3,
      deviceAppliedAtMs: 25,
      connectionGeneration: 4,
    });
    expect(s().elapsedMs).toBe(5_900);
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
