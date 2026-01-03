import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: () => [
    {
      loading: false,
      snapshot: undefined,
      branch: undefined,
      diff: undefined,
      mergeConflicts: undefined,
    },
    { refresh: vi.fn() },
  ],
}));

vi.mock('praxis/components/blocks/time-control-panel', () => ({
  TimeControlPanel: ({ state }: { state: { loading: boolean } }) => (
    <div data-testid="time-control" data-loading={state.loading} />
  ),
}));

import { TimeCursorCard } from 'praxis/components/dashboard/time-cursor-card';

describe('TimeCursorCard', () => {
  it('renders time control panel with state from hook', () => {
    render(<TimeCursorCard />);
    const panel = screen.getByTestId('time-control');
    expect(panel).toBeInTheDocument();
    expect(panel).toHaveAttribute('data-loading', 'false');
  });
});
