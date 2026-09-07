import { act, render, screen, waitFor } from '@testing-library/react';
import axe from 'axe-core';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DiagnosticEventDto } from '../../../lib/ipc/types';

// Hoisted handles so the `vi.mock` factory can reference them; `emit.fn` captures the live diagnostic
// callback so tests can push events deterministically.
const h = vi.hoisted(() => ({
  initial: { events: [] as unknown[] },
  emit: { fn: null as ((d: unknown) => void) | null },
}));

vi.mock('../../../lib/ipc', () => ({
  getDiagnostics: (limit: number): Promise<unknown[]> =>
    Promise.resolve(h.initial.events.slice(-limit)),
  onDiagnostic: (cb: (d: unknown) => void): Promise<() => void> => {
    h.emit.fn = cb;
    return Promise.resolve(() => {});
  },
}));

import { strings } from '../../../lib/i18n/strings';
import { Diagnostics, VIEW_LIMIT } from '../Diagnostics';

function event(overrides: Partial<DiagnosticEventDto> = {}): DiagnosticEventDto {
  return {
    at: '2026-07-24T10:00:00Z',
    connection: 'connected',
    category: 'handshake',
    messageType: null,
    payloadLen: null,
    seq: null,
    retryCount: 0,
    elapsedMs: 120,
    deviceIdHashShort: null,
    ...overrides,
  };
}

afterEach(() => {
  h.initial.events = [];
  h.emit.fn = null;
});

describe('Diagnostics view', () => {
  it('shows the empty state when nothing has been recorded', async () => {
    render(<Diagnostics />);
    await waitFor(() => expect(screen.getByText(strings.diagnostics.empty)).toBeInTheDocument());
    expect(screen.queryByRole('log')).not.toBeInTheDocument();
  });

  it('renders recorded entries in the diagnostics table', async () => {
    h.initial.events = [
      event({ at: '2026-07-24T10:00:01Z', category: 'io', connection: 'error', retryCount: 2 }),
      event({ at: '2026-07-24T10:00:02Z', category: 'version', connection: 'incompatible' }),
    ];
    render(<Diagnostics />);
    await waitFor(() => expect(screen.getByRole('log')).toBeInTheDocument());
    expect(screen.getAllByRole('row')).toHaveLength(3);
    expect(screen.getByText('io')).toBeInTheDocument();
    expect(screen.getByText('incompatible')).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`${strings.diagnostics.retry} 2`))).toBeInTheDocument();
  });

  it('appends live events pushed from the native runtime', async () => {
    h.initial.events = [event({ category: 'handshake' })];
    render(<Diagnostics />);
    await waitFor(() => expect(screen.getAllByRole('row')).toHaveLength(2));

    act(() => h.emit.fn?.(event({ at: '2026-07-24T10:05:00Z', category: 'timeout' })));
    await waitFor(() => expect(screen.getAllByRole('row')).toHaveLength(3));
    expect(screen.getByText('timeout')).toBeInTheDocument();
  });

  it('bounds the view to the most recent entries (ring behaviour)', async () => {
    h.initial.events = Array.from({ length: VIEW_LIMIT + 5 }, (_, i) =>
      event({ at: `2026-07-24T10:00:${String(i).padStart(2, '0')}Z`, elapsedMs: i }),
    );
    render(<Diagnostics />);
    await waitFor(() => expect(screen.getAllByRole('row')).toHaveLength(VIEW_LIMIT + 1));

    // Pushing more keeps the cap and drops the oldest. The extra row is the table header.
    act(() => h.emit.fn?.(event({ at: '2026-07-24T11:11:11Z', category: 'busy' })));
    await waitFor(() => expect(screen.getByText('busy')).toBeInTheDocument());
    expect(screen.getAllByRole('row')).toHaveLength(VIEW_LIMIT + 1);
    expect(screen.queryByText('2026-07-24T10:00:00Z')).not.toBeInTheDocument();
  });

  it('displays only redacted fields — identity appears solely as a short hash', async () => {
    h.initial.events = [
      event({
        category: 'checksum',
        messageType: 'SetState',
        payloadLen: 7,
        seq: 42,
        deviceIdHashShort: 'deadbeef',
      }),
    ];
    const { container } = render(<Diagnostics />);
    await waitFor(() => expect(screen.getByRole('log')).toBeInTheDocument());

    const text = container.textContent ?? '';
    expect(text).toContain('SetState');
    expect(text).toContain('deadbeef');
    expect(text).toContain('7');
    // Nothing resembling raw bytes, a path, or a full 32-hex device id may be rendered.
    expect(text).not.toMatch(/0x[0-9a-f]{4,}/i);
    expect(text).not.toMatch(/[0-9a-f]{16,}/i);
    expect(text).not.toMatch(/\/(Users|home|var)\//);
    expect(text).not.toMatch(/[A-Z]:\\/);
  });

  it('has no axe accessibility violations (empty and populated)', async () => {
    const empty = render(<Diagnostics />);
    await waitFor(() => expect(screen.getByText(strings.diagnostics.empty)).toBeInTheDocument());
    const emptyResults = await axe.run(empty.container, {
      rules: { 'color-contrast': { enabled: false } },
    });
    expect(emptyResults.violations).toEqual([]);
    empty.unmount();

    h.initial.events = [event({ category: 'io', retryCount: 1 })];
    const populated = render(<Diagnostics />);
    await waitFor(() => expect(screen.getByRole('log')).toBeInTheDocument());
    const results = await axe.run(populated.container, {
      rules: { 'color-contrast': { enabled: false } },
    });
    expect(results.violations).toEqual([]);
  });
});
