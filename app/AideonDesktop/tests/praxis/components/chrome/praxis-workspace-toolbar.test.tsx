import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { PraxisWorkspaceToolbar } from 'praxis/components/chrome/praxis-workspace-toolbar';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

const search = vi.fn();
const clear = vi.fn();

vi.mock('praxis/lib/search', () => ({
  searchStore: {
    search: (...arguments_: unknown[]) => {
      search(...arguments_);
    },
    clear: (...arguments_: unknown[]) => {
      clear(...arguments_);
    },
  },
}));

vi.mock('praxis/platform', () => ({ isTauri: () => false }));

describe('PraxisWorkspaceToolbar', () => {
  it('dispatches search queries', () => {
    const temporalState: TemporalPanelState = {
      branches: [],
      commits: [],
      loading: false,
      snapshotLoading: false,
      merging: false,
      branch: 'main',
      commitId: undefined,
      snapshot: undefined,
      error: undefined,
      diff: undefined,
      mergeConflicts: undefined,
    };
    const temporalActions: TemporalPanelActions = {
      selectBranch: vi.fn().mockResolvedValue(undefined),
      selectCommit: vi.fn(),
      refreshBranches: vi.fn().mockResolvedValue(undefined),
      mergeIntoMain: vi.fn().mockResolvedValue(undefined),
    };
    const onTemplateChange = vi.fn();
    const onTemplateSave = vi.fn();
    const onCreateWidget = vi.fn();

    render(
      <PraxisWorkspaceToolbar
        scenarioName="Scenario"
        templates={[]}
        activeTemplateId=""
        onTemplateChange={onTemplateChange}
        onTemplateSave={onTemplateSave}
        onCreateWidget={onCreateWidget}
        temporalState={temporalState}
        temporalActions={temporalActions}
      />,
    );

    const input = screen.getByLabelText('Search');
    fireEvent.change(input, { target: { value: 'Node' } });
    expect(search).toHaveBeenCalledWith('Node');

    fireEvent.change(input, { target: { value: '' } });
    expect(clear).toHaveBeenCalledTimes(1);
  });
});
