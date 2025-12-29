import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import * as React from 'react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: vi.fn(),
}));

vi.mock('@radix-ui/react-tabs', () => {
  interface TabsContextValue {
    value?: string;
    onChange?: (value: string) => void;
  }
  const TabsContext = React.createContext<TabsContextValue>({});

  const Root = ({
    value,
    onValueChange,
    children,
  }: React.PropsWithChildren<{ value?: string; onValueChange?: (value: string) => void }>) => {
    const [internal, setInternal] = React.useState<string | undefined>(value);
    React.useEffect(() => {
      setInternal(value);
    }, [value]);

    const memoValue = React.useMemo(
      () => ({
        value: internal,
        onChange: (next: string) => {
          setInternal(next);
          onValueChange?.(next);
        },
      }),
      [internal, onValueChange],
    );

    return <TabsContext.Provider value={memoValue}>{children}</TabsContext.Provider>;
  };

  const List = ({
    children,
    ...properties
  }: React.PropsWithChildren<React.HTMLAttributes<HTMLDivElement>>) => (
    <div role="tablist" {...properties}>
      {children}
    </div>
  );

  const Trigger = ({
    value,
    children,
    ...properties
  }: React.PropsWithChildren<
    React.ButtonHTMLAttributes<HTMLButtonElement> & { value: string }
  >) => {
    const context = React.useContext(TabsContext);
    const handleClick = () => context.onChange?.(value);
    return (
      <button role="tab" onClick={handleClick} {...properties}>
        {children}
      </button>
    );
  };

  const Content = ({
    value,
    children,
    ...properties
  }: React.PropsWithChildren<{ value: string } & React.HTMLAttributes<HTMLDivElement>>) => {
    const context = React.useContext(TabsContext);
    if (context.value !== value) {
      return <div hidden {...properties} />;
    }
    return (
      <div role="tabpanel" {...properties}>
        {children}
      </div>
    );
  };

  return { Root, List, Trigger, Content };
});

vi.mock('praxis/components/blocks/activity-timeline-panel', () => ({
  ActivityTimelinePanel: () => <div data-testid="activity-panel">Activity</div>,
}));

vi.mock('praxis/components/blocks/commit-timeline-list', () => ({
  CommitTimelineList: () => <div data-testid="commit-list">Commits</div>,
}));

vi.mock('praxis/components/dashboard/canvas-runtime-card', () => ({
  CanvasRuntimeCard: ({ widgets }: { widgets: unknown[] }) => (
    <div data-testid="canvas-runtime">{widgets.length} widgets</div>
  ),
}));

import type { SelectionState } from 'aideon/canvas/types';
import type { PraxisCanvasWidget } from 'praxis/types';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';
import { WorkspaceTabs } from 'praxis/components/workspace-tabs';
import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

const mockUseTemporalPanel = vi.mocked(useTemporalPanel);

const baseSelection: SelectionState = { nodeIds: [], edgeIds: [], sourceWidgetId: undefined };

const baseTemporalState: TemporalPanelState = {
  branches: [],
  commits: [],
  loading: false,
  snapshotLoading: false,
  merging: false,
};

const baseTemporalActions: TemporalPanelActions = {
  selectBranch: vi.fn().mockResolvedValue(undefined),
  selectCommit: vi.fn(),
  refreshBranches: vi.fn().mockResolvedValue(undefined),
  mergeIntoMain: vi.fn().mockResolvedValue(undefined),
};

const canvasWidget: PraxisCanvasWidget = {
  id: 'w1',
  kind: 'chart',
  title: 'Widget',
  size: 'full',
  view: {
    id: 'view-1',
    name: 'Metric',
    kind: 'chart',
    asOf: 'c1',
    chartType: 'line',
    measure: 'm1',
  },
};

describe('WorkspaceTabs', () => {
  it('shows loading state when snapshot is pending', () => {
    mockUseTemporalPanel.mockReturnValue([
      {
        ...baseTemporalState,
        loading: true,
        branch: undefined,
        snapshot: undefined,
        diff: undefined,
        mergeConflicts: undefined,
      },
      baseTemporalActions,
    ]);

    render(
      <WorkspaceTabs
        widgets={[]}
        selection={baseSelection}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
      />,
    );

    expect(screen.getByText(/Loading snapshot/)).toBeInTheDocument();
  });

  it('renders overview metrics and handles tab changes', async () => {
    mockUseTemporalPanel.mockReturnValue([
      {
        ...baseTemporalState,
        loading: false,
        snapshot: { asOf: 'c1', nodes: 10, edges: 5, confidence: 0.42, scenario: 'Test' },
        branch: 'dev',
        diff: {
          from: 'c0',
          to: 'c1',
          metrics: { nodeAdds: 1, nodeMods: 2, nodeDels: 0, edgeAdds: 3, edgeMods: 0, edgeDels: 1 },
        },
        mergeConflicts: [{ kind: 'edge', reference: 'e1', message: 'conflict' }],
      },
      baseTemporalActions,
    ]);
    render(
      <WorkspaceTabs
        widgets={[canvasWidget]}
        selection={baseSelection}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
      />,
    );

    expect(screen.getByText('Snapshot overview')).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText(/42%/)).toBeInTheDocument();
    expect(screen.getAllByText(/conflict/i).length).toBeGreaterThanOrEqual(1);

    fireEvent.click(screen.getByRole('tab', { name: 'Canvas' }));

    await waitFor(() => expect(screen.getByTestId('canvas-runtime')).toBeInTheDocument());
  });

  it('supports controlled tabs and timeline states', async () => {
    const onValueChange = vi.fn();
    mockUseTemporalPanel.mockReturnValue([
      {
        ...baseTemporalState,
        loading: true,
        snapshot: { asOf: 'c1', nodes: 0, edges: 0, scenario: undefined, confidence: undefined },
        branch: undefined,
        commits: [],
        commitId: undefined,
        diff: undefined,
        mergeConflicts: undefined,
        error: 'Boom',
      },
      baseTemporalActions,
    ]);

    render(
      <WorkspaceTabs
        widgets={[]}
        selection={baseSelection}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
        value="timeline"
        onValueChange={onValueChange}
      />,
    );

    expect(screen.getByText(/Loading commits/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Canvas' }));
    await waitFor(() => {
      expect(screen.getByTestId('canvas-runtime')).toBeInTheDocument();
    });

    render(
      <WorkspaceTabs
        widgets={[]}
        selection={baseSelection}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
        value="overview"
        onValueChange={onValueChange}
      />,
    );
    expect(screen.getAllByText('Boom').length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Temporal branches pending/).length).toBeGreaterThan(0);

    mockUseTemporalPanel.mockReturnValueOnce([
      {
        ...baseTemporalState,
        loading: false,
        snapshot: { asOf: 'c1', nodes: 0, edges: 0, scenario: undefined, confidence: undefined },
        branch: 'main',
        commits: [],
        commitId: undefined,
        diff: undefined,
        mergeConflicts: undefined,
        error: undefined,
      },
      baseTemporalActions,
    ]);

    render(
      <WorkspaceTabs
        widgets={[]}
        selection={baseSelection}
        onSelectionChange={vi.fn()}
        onRequestMetaModelFocus={vi.fn()}
        value="timeline"
      />,
    );
    await waitFor(() => {
      expect(screen.getByText(/No commits recorded yet/i)).toBeInTheDocument();
    });
  });
});
