import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type * as React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { TimeCursorCard } from 'praxis/components/template-screen/time-cursor-card';

vi.mock('design-system/components/ui/select', () => {
  interface SelectProperties {
    onValueChange?: (value: string) => void;
    value?: string;
    disabled?: boolean;
    children?: React.ReactNode;
    ['data-testid']?: string;
  }
  const Select = ({
    onValueChange,
    value,
    disabled,
    children,
    'data-testid': testId,
  }: SelectProperties) => (
    <div>
      <button
        data-testid={testId ?? `select-${value ?? 'unset'}`}
        disabled={disabled}
        onClick={() => {
          const nextValue = testId?.includes('branch') ? 'main' : 'c1';
          onValueChange?.(nextValue);
        }}
      >
        select
      </button>
      {children}
    </div>
  );
  const SelectTrigger = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  const SelectValue = (properties: Record<string, unknown>) => <div {...properties} />;
  const SelectContent = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  const SelectItem = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  return { Select, SelectTrigger, SelectValue, SelectContent, SelectItem };
});

vi.mock('design-system/components/ui/slider', () => ({
  Slider: ({
    onValueCommit,
    disabled,
    value,
  }: {
    onValueCommit?: (values: number[]) => void;
    disabled?: boolean;
    value?: number[];
  }) => (
    <div>
      <button
        data-testid="timeline-slider"
        disabled={disabled}
        data-value={JSON.stringify(value ?? [])}
        onClick={() => onValueCommit?.([1])}
      >
        slider
      </button>
      <button data-testid="timeline-slider-negative" onClick={() => onValueCommit?.([-1])}>
        slider-negative
      </button>
      <button
        data-testid="timeline-slider-non-number"
        onClick={() => onValueCommit?.([undefined as unknown as number])}
      >
        slider-non-number
      </button>
    </div>
  ),
}));

describe('TimeCursorCard', () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    return;
  });

  const state: TemporalPanelState = {
    branches: [
      { name: 'main', head: 'a1' },
      { name: 'chronaplay', head: 'b1' },
    ],
    branch: 'chronaplay',
    commits: [
      {
        id: 'c1',
        branch: 'chronaplay',
        parents: [],
        message: 'Seed',
        tags: [],
        changeCount: 1,
      },
      {
        id: 'c2',
        branch: 'chronaplay',
        parents: ['c1'],
        message: 'Add UI',
        tags: ['ui'],
        changeCount: 3,
      },
    ],
    commitId: 'c1',
    snapshot: {
      asOf: 'c1',
      nodes: 42,
      edges: 10,
      confidence: 0.9,
      scenario: 'chronaplay',
    },
    loading: false,
    snapshotLoading: false,
    error: undefined,
    mergeConflicts: undefined,
    merging: false,
  } as TemporalPanelState;

  const actions: TemporalPanelActions = {
    selectBranch: vi.fn(() => Promise.resolve()),
    selectCommit: vi.fn(),
    refreshBranches: vi.fn(() => Promise.resolve()),
    mergeIntoMain: vi.fn(() => Promise.resolve()),
  };

  it('emits branch, commit, and slider changes', () => {
    render(<TimeCursorCard state={state} actions={actions} />);

    fireEvent.click(screen.getByTestId('branch-select'));
    expect(actions.selectBranch).toHaveBeenCalledWith('main');

    fireEvent.click(screen.getByTestId('commit-select'));
    expect(actions.selectCommit).toHaveBeenCalledWith('c1');

    fireEvent.click(screen.getByTestId('timeline-slider'));
    expect(actions.selectCommit).toHaveBeenCalledWith('c2');
  });

  it('uses hook defaults and handles empty commits + invalid slider inputs', () => {
    const emptyActions: TemporalPanelActions = {
      selectBranch: vi.fn(() => Promise.resolve()),
      selectCommit: vi.fn(),
      refreshBranches: vi.fn(() => Promise.resolve()),
      mergeIntoMain: vi.fn(() => Promise.resolve()),
    };
    const emptyState: TemporalPanelState = {
      branches: [{ name: 'main', head: 'c1' }],
      branch: 'main',
      commits: [],
      commitId: undefined,
      snapshot: undefined,
      loading: true,
      snapshotLoading: true,
      error: 'No timeline loaded',
      mergeConflicts: undefined,
      merging: false,
      diff: undefined,
    };
    render(<TimeCursorCard state={emptyState} actions={emptyActions} />);

    expect(screen.getByText(/No timeline loaded/i)).toBeInTheDocument();
    expect(screen.queryByText(/Apply to primary/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('timeline-slider-negative'));
    fireEvent.click(screen.getByTestId('timeline-slider-non-number'));
    expect(emptyActions.selectCommit).not.toHaveBeenCalled();
  });
});
