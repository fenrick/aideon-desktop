import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { PraxisWorkspaceToolbar } from 'praxis/components/chrome/praxis-workspace-toolbar';
import type { CanvasTemplate } from 'praxis/templates';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

vi.mock('praxis/platform', () => ({
  isTauri: () => true,
}));

describe('PraxisWorkspaceToolbar', () => {
  it('renders the header and wires actions', async () => {
    const templates: CanvasTemplate[] = [
      { id: 't1', name: 'Template A', description: 'Desc A' } as CanvasTemplate,
    ];
    const temporalState: TemporalPanelState = {
      branches: [{ name: 'main', head: 'commit-main-001' }],
      branch: 'main',
      commits: [
        {
          id: 'commit-main-001',
          branch: 'main',
          parents: [],
          tags: [],
          message: 'Initial commit',
          changeCount: 1,
        },
        {
          id: 'commit-main-002',
          branch: 'main',
          parents: [],
          tags: [],
          message: 'Second commit',
          changeCount: 2,
        },
      ],
      loading: false,
      snapshotLoading: false,
      merging: false,
      commitId: 'commit-main-002',
      snapshot: {
        asOf: 'commit-main-002',
        scenario: 'Scenario A',
        confidence: 0.95,
        nodes: 2,
        edges: 1,
      },
      error: undefined,
      diff: undefined,
      mergeConflicts: undefined,
    };
    const temporalActions: TemporalPanelActions = {
      selectBranch: vi.fn(() => Promise.resolve()),
      selectCommit: vi.fn(() => {
        void 0;
      }),
      refreshBranches: vi.fn(() => Promise.resolve()),
      mergeIntoMain: vi.fn(() => Promise.resolve()),
    };
    const onTemplateChange = vi.fn();
    const onTemplateSave = vi.fn();
    const onCreateWidget = vi.fn();

    render(
      <PraxisWorkspaceToolbar
        scenarioName="Mainline FY25"
        templateName="Executive overview"
        templates={templates}
        activeTemplateId="t1"
        onTemplateChange={onTemplateChange}
        onTemplateSave={onTemplateSave}
        onCreateWidget={onCreateWidget}
        temporalState={temporalState}
        temporalActions={temporalActions}
      />,
    );

    expect(screen.getByText(/Mainline FY25/i)).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /Executive overview/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Add widget/i }));
    expect(onCreateWidget).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: /Save/i }));
    expect(onTemplateSave).toHaveBeenCalled();

    const select = screen.getByLabelText('Select template');
    fireEvent.change(select, { target: { value: 't1' } });
    expect(onTemplateChange).toHaveBeenCalledWith('t1');

    const moreActionsTrigger = screen.getByLabelText('More workspace actions');
    fireEvent.pointerDown(moreActionsTrigger);
    fireEvent.pointerUp(moreActionsTrigger);
    const refreshButton = await screen.findByText('Refresh');
    fireEvent.click(refreshButton);
    expect(temporalActions.refreshBranches).toHaveBeenCalled();

    fireEvent.pointerDown(moreActionsTrigger);
    fireEvent.pointerUp(moreActionsTrigger);
    const timeButton = await screen.findByText('Time');
    fireEvent.click(timeButton);
    expect(await screen.findByText(/Time controls/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /^Close$/i }));
  });
});
