import { render, screen, waitFor } from '@testing-library/react';
import type * as PraxisApi from 'praxis/praxis-api';
import type { PraxisGraphWidgetConfig as GraphWidgetConfig } from 'praxis/types';
import * as React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const getGraphViewMock = vi.fn<typeof PraxisApi.getGraphView>();

vi.mock('praxis/praxis-api', async () => {
  const actual = await vi.importActual<typeof PraxisApi>('praxis/praxis-api');
  return {
    ...actual,
    getGraphView: (...arguments_: Parameters<typeof actual.getGraphView>) =>
      getGraphViewMock(...arguments_),
  };
});

vi.mock('design-system/components/node-search', () => ({
  NodeSearchDialog: () => <div data-testid="node-search-dialog" />,
}));

interface Selection {
  readonly nodes?: { id: string }[];
  readonly edges?: { id: string }[];
}

let latestSelectionHandler: ((selection: Selection) => void) | undefined;
interface ContextMenuNode {
  id: string;
  data: { entityTypes?: string[] };
  selected?: boolean;
}

let latestContextMenuHandler:
  | ((event: { preventDefault: () => void }, node: ContextMenuNode) => void)
  | undefined;

vi.mock('@xyflow/react', () => {
  const { createElement } = React;
  return {
    ReactFlowProvider: ({ children }: { children?: React.ReactNode }) =>
      createElement('div', { 'data-testid': 'reactflow-provider' }, children),
    ReactFlow: ({
      children,
      onSelectionChange,
      onNodeContextMenu,
    }: {
      children?: React.ReactNode;
      onSelectionChange?: (selection: Selection) => void;
      onNodeContextMenu?: (event: { preventDefault: () => void }, node: ContextMenuNode) => void;
    }) => {
      latestSelectionHandler = onSelectionChange ?? undefined;
      latestContextMenuHandler = onNodeContextMenu ?? undefined;
      return createElement('div', { 'data-testid': 'reactflow' }, children);
    },
    Controls: () => createElement('div', { 'data-testid': 'controls' }),
    MiniMap: () => createElement('div', { 'data-testid': 'minimap' }),
    Background: () => createElement('div', { 'data-testid': 'background' }),
    BackgroundVariant: { Dots: 'dots' },
    useNodesState: () => {
      const [nodes, setNodes] = React.useState<unknown[]>([]);
      return [nodes, setNodes, vi.fn()];
    },
    useEdgesState: () => {
      const [edges, setEdges] = React.useState<unknown[]>([]);
      return [edges, setEdges, vi.fn()];
    },
    useReactFlow: () => ({
      getNodes: (): never[] => [],
      setNodes: (updater: (nodes: never[]) => never[]) => updater([]),
      fitView: vi.fn(),
    }),
    Panel: ({ children }: { children?: React.ReactNode }) =>
      createElement('div', { 'data-testid': 'panel' }, children),
  };
});

import { GraphWidget } from 'praxis/widgets/graph-widget';

const GRAPH_WIDGET: GraphWidgetConfig = {
  id: 'graph-widget',
  kind: 'graph',
  title: 'Customer Graph',
  view: {
    id: 'view-graph',
    name: 'Customer Experience',
    kind: 'graph',
    asOf: '2025-11-01T00:00:00.000Z',
  },
};

const GRAPH_VIEW: PraxisApi.GraphViewModel = {
  metadata: {
    id: 'view-graph',
    name: 'Customer Experience',
    asOf: '2025-11-01T00:00:00.000Z',
    fetchedAt: '2025-11-01T00:01:00.000Z',
    scenario: 'main',
    source: 'mock',
  },
  stats: { nodes: 1, edges: 0 },
  nodes: [
    {
      id: 'node-1',
      label: 'Customer Onboarding',
      type: 'Capability',
      position: { x: 10, y: 20 },
    },
  ],
  edges: [],
};

describe('GraphWidget', () => {
  beforeEach(() => {
    getGraphViewMock.mockReset();
    latestSelectionHandler = undefined;
  });

  it('loads the graph view on mount and surfaces metadata/context to parents', async () => {
    getGraphViewMock.mockResolvedValue(GRAPH_VIEW);
    const onViewChange = vi.fn();

    render(<GraphWidget widget={GRAPH_WIDGET} reloadVersion={0} onViewChange={onViewChange} />);

    await waitFor(() => {
      expect(onViewChange).toHaveBeenCalledWith(GRAPH_VIEW);
    });
    expect(getGraphViewMock).toHaveBeenCalledWith(expect.objectContaining({ id: 'view-graph' }));
    expect(screen.getByText('Customer Experience')).toBeInTheDocument();
  });

  it('propagates selection changes originating from React Flow', async () => {
    getGraphViewMock.mockResolvedValue(GRAPH_VIEW);
    const onSelectionChange = vi.fn();

    render(
      <GraphWidget widget={GRAPH_WIDGET} reloadVersion={0} onSelectionChange={onSelectionChange} />,
    );

    await waitFor(() => {
      expect(getGraphViewMock).toHaveBeenCalled();
      expect(latestSelectionHandler).toBeDefined();
    });

    await waitFor(() => {
      latestSelectionHandler?.({ nodes: [{ id: 'node-1' }], edges: [] });
      expect(onSelectionChange).toHaveBeenCalledWith({
        widgetId: 'graph-widget',
        nodeIds: ['node-1'],
        edgeIds: [],
      });
    });
  });

  it('renders an error overlay and emits the message when view loading fails', async () => {
    getGraphViewMock.mockRejectedValue(new Error('IPC offline'));
    const onError = vi.fn();

    render(<GraphWidget widget={GRAPH_WIDGET} reloadVersion={1} onError={onError} />);

    expect(await screen.findByText('IPC offline')).toBeInTheDocument();
    expect(onError).toHaveBeenCalledWith('IPC offline');
  });

  it('opens context menu and triggers meta-model focus', async () => {
    getGraphViewMock.mockResolvedValue({
      ...GRAPH_VIEW,
      nodes: [
        {
          id: 'node-1',
          label: 'Cap',
          type: 'Capability',
          position: { x: 0, y: 0 },
        },
      ],
      edges: [],
    });
    const onRequestMetaModelFocus = vi.fn();

    const { rerender } = render(
      <GraphWidget
        widget={GRAPH_WIDGET}
        reloadVersion={0}
        onRequestMetaModelFocus={onRequestMetaModelFocus}
      />,
    );

    await waitFor(() => {
      expect(getGraphViewMock).toHaveBeenCalled();
    });

    rerender(
      <GraphWidget
        widget={GRAPH_WIDGET}
        reloadVersion={0}
        onRequestMetaModelFocus={onRequestMetaModelFocus}
        selection={{ nodeIds: ['node-1'], edgeIds: [], sourceWidgetId: undefined }}
      />,
    );

    await waitFor(() => {
      expect(latestContextMenuHandler).toBeDefined();
    });
    latestContextMenuHandler?.(
      {
        preventDefault: () => {
          /* noop */
        },
      },
      { id: 'node-1', data: { entityTypes: ['Capability'] }, selected: true },
    );
    const menuButton = await screen.findByText(/View meta-model entry/);
    menuButton.click();
    expect(onRequestMetaModelFocus).toHaveBeenCalledWith(['Capability']);
  });
});
