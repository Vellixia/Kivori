import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import axe from 'axe-core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { strings } from '../../../lib/i18n/strings';
import { Controls } from '../Controls';
import { useStudioStore } from '../store';

function rect(width: number, height: number): DOMRect {
  return {
    x: 0,
    y: 0,
    width,
    height,
    top: 0,
    right: width,
    bottom: height,
    left: 0,
    toJSON: () => ({}),
  } as DOMRect;
}

beforeEach(() => {
  useStudioStore.setState({ state: 'idle', elapsedMs: 0, playing: false });

  // Base UI's edge-aligned slider intentionally hides its thumb until it can measure the control.
  // jsdom has no layout engine and otherwise reports 0x0 rectangles, which keeps the real nested
  // input[type=range] hidden from the accessibility tree. Provide realistic geometry so this test
  // exercises the same accessible control that a browser exposes after layout.
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.getAttribute('data-slot') === 'slider-thumb' ? rect(12, 12) : rect(240, 12);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
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

  it('exposes the timeline as an accessible slider', async () => {
    render(<Controls />);
    expect(await screen.findByRole('slider', { name: strings.studio.scrub })).toBeInTheDocument();
  });

  it('has no axe accessibility violations', async () => {
    const { container } = render(<Controls />);
    // Wait for Base UI's edge-aligned slider to complete its post-layout positioning before scanning.
    await screen.findByRole('slider', { name: strings.studio.scrub });
    // color-contrast can't be computed under jsdom (no canvas text metrics); the rest still runs.
    const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations).toEqual([]);
  });
});
