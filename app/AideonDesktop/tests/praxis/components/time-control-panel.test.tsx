import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type * as React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { TimeControlPanel } from 'praxis/components/blocks/time-control-panel';

vi.mock('../../../src/design-system/components/ui/select', () => {
  interface SelectProperties {
    onValueChange?: (value: string) => void;
    value?: string;
    disabled?: boolean;
    children?: React.ReactNode;
  }
  const Select = ({ onValueChange, value, disabled, children }: SelectProperties) => (
    <div>
      <button
        data-testid={`select-${value ?? 'unset'}`}
        disabled={disabled}
        onClick={() => onValueChange?.('main')}
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

vi.mock('../../../src/design-system/components/ui/slider', () => ({
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
        data-testid="slider-valid"
        disabled={disabled}
        data-value={JSON.stringify(value ?? [])}
        onClick={() => onValueCommit?.([1])}
      >
        slider
      </button>
      <button data-testid="slider-negative" onClick={() => onValueCommit?.([-1])}>
        slider-negative
      </button>
      <button
        data-testid="slider-non-number"
        onClick={() => onValueCommit?.([undefined as unknown as number])}
      >
        slider-non-number
      </button>
    </div>
  ),
}));

describe('TimeControlPanel', () => {
  afterEach(() => {
    cleanup();
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
    snapshot: { asOf: 'c1', nodes: 42, edges: 10 },
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

  it('invokes actions from buttons and slider', () => {
    render(<TimeControlPanel state={state} actions={actions} />);

    fireEvent.click(screen.getByText('Refresh branches'));
    expect(actions.refreshBranches).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Reload snapshot'));
    expect(actions.selectCommit).toHaveBeenCalledWith('c1');

    fireEvent.click(screen.getByText('Merge into main'));
    expect(actions.mergeIntoMain).toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('slider-valid'));
    expect(actions.selectCommit).toHaveBeenCalledWith('c2');
  });

  it('handles empty commit lists, main branch, and invalid slider events', () => {
    const localActions: TemporalPanelActions = {
      selectBranch: vi.fn(() => Promise.resolve()),
      selectCommit: vi.fn(),
      refreshBranches: vi.fn(() => Promise.resolve()),
      mergeIntoMain: vi.fn(() => Promise.resolve()),
    };
    const localState: TemporalPanelState = {
      branches: [],
      branch: 'main',
      commits: [],
      commitId: undefined,
      snapshot: undefined,
      loading: true,
      snapshotLoading: true,
      error: 'Boom',
      mergeConflicts: [{ reference: 'r1', kind: 'node', message: 'conflict' }],
      merging: true,
      diff: undefined,
    };

    render(<TimeControlPanel state={localState} actions={localActions} />);

    expect(screen.getByText(/Load a branch to view commits/i)).toBeInTheDocument();
    expect(screen.getByText('Boom')).toBeInTheDocument();
    expect(screen.queryByText(/Merge into main/i)).not.toBeInTheDocument();

    expect(screen.getByText('Refresh branches')).toBeDisabled();
    expect(screen.getByText('Reload snapshot')).toBeDisabled();

    fireEvent.click(screen.getByTestId('slider-negative'));
    fireEvent.click(screen.getByTestId('slider-non-number'));
    expect(localActions.selectCommit).not.toHaveBeenCalled();
  });
});
