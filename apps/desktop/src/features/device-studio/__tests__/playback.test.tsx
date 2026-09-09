import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { DeviceStudio } from '../DeviceStudio';
import { useStudioStore } from '../store';

const bridge = vi.hoisted(() => ({
  update: vi.fn(),
  close: vi.fn(),
  open: vi.fn(),
}));
vi.mock('../../../lib/ipc', () => ({ openPreviewStream: bridge.open, mirrorState: vi.fn() }));
vi.mock('../../../lib/canvas/DevicePreview', () => ({
  DevicePreview: (props: { frame?: Uint8ClampedArray; streaming?: boolean }) => (
    <div
      data-testid="preview"
      data-has-frame={Boolean(props.frame)}
      data-streaming={props.streaming}
    />
  ),
}));
vi.mock('../Controls', () => ({ Controls: () => null }));

beforeEach(() => {
  vi.useFakeTimers();
  useStudioStore.getState().reset();
  bridge.update.mockReset().mockResolvedValue(undefined);
  bridge.close.mockReset().mockResolvedValue(undefined);
  bridge.open.mockReset().mockImplementation((_state, _fps, onFrame) => {
    onFrame(new Uint8ClampedArray(4));
    return Promise.resolve({ update: bridge.update, close: bridge.close });
  });
});
afterEach(() => vi.useRealTimers());

it('keeps one live stream when the expression changes and sends event history', async () => {
  useStudioStore.getState().play();
  render(<DeviceStudio />);
  await act(async () => {
    await Promise.resolve();
  });
  act(() => useStudioStore.getState().setState('happy'));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(40);
  });
  expect(bridge.open).toHaveBeenCalledTimes(1);
  expect(bridge.update.mock.calls.at(-1)?.[0].events).toEqual([{ atMs: 0, state: 'happy' }]);
});

it('drops stale frames and enables exact-frame fallback when an update fails', async () => {
  useStudioStore.getState().play();
  render(<DeviceStudio />);
  await act(async () => {
    await Promise.resolve();
  });
  expect(screen.getByTestId('preview')).toHaveAttribute('data-has-frame', 'true');
  bridge.update.mockRejectedValue(new Error('preview unavailable'));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(40);
  });
  expect(screen.getByRole('alert')).toHaveTextContent('preview unavailable');
  expect(screen.getByTestId('preview')).toHaveAttribute('data-has-frame', 'false');
  expect(screen.getByTestId('preview')).toHaveAttribute('data-streaming', 'false');
});
