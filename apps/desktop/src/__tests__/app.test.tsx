import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { App } from '../App';

describe('App navigation', () => {
  it('keeps the dev build to Overview, Device Studio, and Log tabs', async () => {
    render(<App />);
    await waitFor(() => expect(screen.getAllByRole('tab')).toHaveLength(3));
    expect(screen.getByRole('tab', { name: 'Log' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Diagnostics' })).not.toBeInTheDocument();
  });
});
