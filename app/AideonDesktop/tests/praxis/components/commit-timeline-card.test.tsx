import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TemporalPanelState } from 'praxis/time/use-temporal-panel';

vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: () => [mockState, mockActions] as const,
}));

const selectBranchSpy = vi.fn();
const selectCommitSpy = vi.fn();
const refreshBranchesSpy = vi.fn();
const mergeIntoMainSpy = vi.fn();

const mockActions = {
  selectBranch: selectBranchSpy,
  selectCommit: selectCommitSpy,
  refreshBranches: refreshBranchesSpy,
  mergeIntoMain: mergeIntoMainSpy,
};

afterEach(() => {
  cleanup();
});

let mockState: TemporalPanelState;

import { CommitTimelineCard } from 'praxis/components/dashboard/commit-timeline-card';

describe('CommitTimelineCard', () => {
  beforeEach(() => {
    selectBranchSpy.mockReset();
    selectCommitSpy.mockReset();
    refreshBranchesSpy.mockReset();
    mergeIntoMainSpy.mockReset();
    mockState = {
      branches: [
        { name: 'main', head: 'a1' },
        { name: 'chronaplay', head: 'b1' },
      ],
      branch: 'chronaplay',
      commits: [
        {
          id: 'commit-1',
          branch: 'chronaplay',
          parents: [],
          author: 'Chrona',
          message: 'Seed',
          tags: [],
          changeCount: 2,
        },
        {
          id: 'commit-2',
          branch: 'chronaplay',
          parents: ['commit-1'],
          author: 'Chrona',
          message: 'Add dashboards',
          tags: ['ui'],
          changeCount: 4,
        },
      ],
      commitId: 'commit-1',
      snapshot: undefined,
      loading: false,
      snapshotLoading: false,
      error: undefined,
      mergeConflicts: undefined,
      merging: false,
    } satisfies TemporalPanelState;
  });

  it('renders commits and selects them on click', () => {
    render(<CommitTimelineCard />);

    expect(screen.getByText('Add dashboards')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Add dashboards'));
    expect(selectCommitSpy).toHaveBeenCalledWith('commit-2');
  });

  it('shows merge conflicts when provided', () => {
    mockState = {
      ...mockState,
      mergeConflicts: [{ reference: 'cap-1', kind: 'node', message: 'Diverged' }],
    };

    render(<CommitTimelineCard />);

    expect(screen.getByText('Merge conflicts')).toBeInTheDocument();
    expect(screen.getByText(/cap-1/)).toBeInTheDocument();
  });

  it('invokes branch selection when chips are clicked', () => {
    render(<CommitTimelineCard />);

    const mainChip = screen.getAllByText('main')[0];
    if (!mainChip) {
      throw new Error('Expected main branch chip.');
    }
    fireEvent.click(mainChip);
    expect(selectBranchSpy).toHaveBeenCalledWith('main');
  });
});
