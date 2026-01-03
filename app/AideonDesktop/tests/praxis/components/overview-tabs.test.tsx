import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/components/canvas/praxis-canvas-workspace', () => ({
  PraxisCanvasWorkspace: () => <div>Canvas workspace</div>,
}));

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { OverviewTabs } from 'praxis/components/template-screen/overview-tabs';

const temporalState: TemporalPanelState = {
  branches: [{ name: 'main', head: 'c2' }],
  branch: 'main',
  commits: [
    { id: 'c1', branch: 'main', parents: [], message: 'Init', tags: [], changeCount: 1 },
    { id: 'c2', branch: 'main', parents: ['c1'], message: 'Add flow', tags: [], changeCount: 2 },
  ],
  commitId: 'c2',
  snapshot: { asOf: 'c2', nodes: 10, edges: 4, scenario: 'main' },
  loading: false,
  snapshotLoading: false,
  error: undefined,
  mergeConflicts: undefined,
  merging: false,
} as TemporalPanelState;

const temporalActions: TemporalPanelActions = {
  selectBranch: vi.fn(() => Promise.resolve()),
  selectCommit: vi.fn(),
  refreshBranches: vi.fn(() => Promise.resolve()),
  mergeIntoMain: vi.fn(() => Promise.resolve()),
};

describe('OverviewTabs', () => {
  it('renders tabs and switches to Activity', () => {
    render(
      <OverviewTabs
        state={temporalState}
        actions={temporalActions}
        widgets={[]}
        selection={{ nodeIds: [], edgeIds: [], sourceWidgetId: undefined }}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
        initialTab="activity"
        activityContent={<div>Activity content</div>}
        timelineContent={<div>Timeline content</div>}
      />,
    );

    expect(screen.getByRole('tab', { name: /canvas/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /overview/i })).toBeInTheDocument();
    expect(screen.getByText(/activity content/i)).toBeInTheDocument();
  });
});
