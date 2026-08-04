import { act, render, screen, waitFor } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ConnectionStatusDto } from '../../../lib/ipc/types';

// Hoisted mock handles so the `vi.mock` factory can reference them. `emit.fn` captures the live
// status callback the component registers, so tests can drive events + verify cleanup deterministically.
const h = vi.hoisted(() => ({
  subscribed: { count: 0 },
  emit: { fn: null as ((s: ConnectionStatusDto) => void) | null },
  state: { current: null as ConnectionStatusDto | null },
}));

vi.mock('../../../lib/ipc', () => ({
  getConnectionStatus: (): Promise<ConnectionStatusDto | null> => Promise.resolve(h.state.current),
  onConnectionStatus: (cb: (s: ConnectionStatusDto) => void): Promise<() => void> => {
    h.subscribed.count += 1;
    h.emit.fn = cb;
    return Promise.resolve(() => {});
  },
}));

import { strings } from '../../../lib/i18n/strings';
import { ConnectionStatus } from '../ConnectionStatus';

function status(overrides: Partial<ConnectionStatusDto> = {}): ConnectionStatusDto {
  return {
    connection: 'disconnected',
    desired: 'idle',
    reported: null,
    device: null,
    incompatibleReason: null,
    retryCount: 0,
    ...overrides,
  };
}

afterEach(() => {
  h.subscribed.count = 0;
  h.emit.fn = null;
  h.state.current = null;
});

describe('ConnectionStatus', () => {
  it('renders the connected status with device info and both state axes', async () => {
    h.state.current = status({
      connection: 'connected',
      desired: 'busy',
      reported: 'happy',
      device: {
        firmwareVersion: '1.4.2',
        protocolVersion: { major: 1, minor: 0 },
        deviceIdHashShort: 'deadbeef',
      },
    });
    render(<ConnectionStatus />);
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(strings.connection.status.connected),
    );
    expect(screen.getByTestId('desired')).toHaveTextContent('busy');
    expect(screen.getByTestId('reported')).toHaveTextContent('happy');
    expect(screen.getByText('deadbeef')).toBeInTheDocument();
  });

  it('shows a reconnecting state with the attempt count', async () => {
    h.state.current = status({ connection: 'disconnected', retryCount: 3 });
    render(<ConnectionStatus />);
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(strings.connection.status.reconnecting),
    );
    expect(screen.getByRole('status')).toHaveTextContent('3');
  });

  it('surfaces an incompatible reason', async () => {
    h.state.current = status({
      connection: 'incompatible',
      incompatibleReason: 'device protocol major v2 is unsupported',
    });
    render(<ConnectionStatus />);
    await waitFor(() => expect(screen.getByText(/v2 is unsupported/)).toBeInTheDocument());
  });

  it('applies a live status event, then ignores events after unmount (cleanup)', async () => {
    h.state.current = status({ connection: 'connecting' });
    const { unmount } = render(<ConnectionStatus />);
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(strings.connection.status.connecting),
    );
    await waitFor(() => expect(h.emit.fn).not.toBeNull());

    // A pushed event updates the live UI.
    act(() => h.emit.fn?.(status({ connection: 'connected' })));
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(strings.connection.status.connected),
    );

    // After unmount, a late event is dropped by the `active` guard — no throw, no work.
    unmount();
    expect(() => h.emit.fn?.(status({ connection: 'disconnected' }))).not.toThrow();
  });

  it('has no axe accessibility violations', async () => {
    h.state.current = status();
    const { container } = render(<ConnectionStatus />);
    await waitFor(() => expect(screen.getByRole('status')).toBeInTheDocument());
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations).toEqual([]);
  });
});
