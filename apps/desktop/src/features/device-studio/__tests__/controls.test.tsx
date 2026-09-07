import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import axe from 'axe-core';
import { beforeEach, describe, expect, it } from 'vitest';
import { strings } from '../../../lib/i18n/strings';
import { Controls } from '../Controls';
import { useStudioStore } from '../store';

beforeEach(() => {
  useStudioStore.setState({ state: 'idle', elapsedMs: 0, playing: false });
});

describe('Device Studio Controls', () => {
  it('selecting a state through the toggle group updates the store', async () => {
    const user = userEvent.setup();
    render(<Controls />);
    await user.click(screen.getByRole('button', { name: strings.studio.states.happy }));
    expect(useStudioStore.getState().state).toBe('happy');
  });

  it('play toggles playback', async () => {
    const user = userEvent.setup();
    render(<Controls />);
    await user.click(screen.getByRole('button', { name: strings.studio.play }));
    expect(useStudioStore.getState().playing).toBe(true);
  });

  it('mirror is disabled for non-sendable states', () => {
    useStudioStore.setState({ state: 'offline' });
    render(<Controls />);
    expect(screen.getByRole('button', { name: strings.studio.mirror })).toBeDisabled();
  });

  it('marks the current state as the pressed toggle', () => {
    useStudioStore.setState({ state: 'busy' });
    render(<Controls />);
    expect(screen.getByRole('button', { name: strings.studio.states.busy })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
  });

  it('exposes the timeline as an accessible slider', () => {
    render(<Controls />);
    expect(screen.getByRole('slider', { name: strings.studio.scrub })).toBeInTheDocument();
  });

  it('has no axe accessibility violations', async () => {
    const { container } = render(<Controls />);
    // color-contrast can't be computed under jsdom (no canvas text metrics); the rest still runs.
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations).toEqual([]);
  });
});
