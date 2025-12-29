import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import type { SelectionState } from 'aideon/canvas/types';
import { PraxisCanvasWorkspace } from 'praxis/components/canvas/praxis-canvas-workspace';
import type { GraphViewModel } from 'praxis/praxis-api';
import type { PraxisCanvasWidget } from 'praxis/types';

/**
 * Minimal canvas runtime stand-in so we can exercise the Praxis workspace wrapper logic.
 * @param root0
 * @param root0.widgets
 * @param root0.showPageBreaks
 * @param root0.renderWidget
 */
function CanvasRuntimeMock<TWidget extends { id: string }>({
  widgets,
  showPageBreaks,
  renderWidget,
}: {
  readonly widgets: readonly TWidget[];
  readonly showPageBreaks?: boolean;
  readonly renderWidget: (widget: TWidget) => ReactNode;
}): ReactElement {
  return (
    <div data-testid="canvas-runtime" data-pages={String(showPageBreaks)}>
      {widgets.map((widget) => (
        <div key={widget.id}>{renderWidget(widget)}</div>
      ))}
    </div>
  );
}

vi.mock('aideon/canvas/canvas-runtime', () => ({
  AideonCanvasRuntime: CanvasRuntimeMock,
}));

vi.mock('praxis/widgets/graph-widget', () => ({
  GraphWidget: ({
    reloadVersion,
    onViewChange,
    onError,
  }: {
    readonly reloadVersion: number;
    readonly onViewChange: (view: GraphViewModel) => void;
    readonly onError: (message: string) => void;
  }) => (
    <div>
      <div>graph reload:{reloadVersion}</div>
      <button
        type="button"
        onClick={() => {
          onViewChange({
            metadata: { asOf: '2025-01-01T00:00:00Z' },
            stats: { nodes: 1, edges: 2 },
          } as unknown as GraphViewModel);
        }}
      >
        emit-view
      </button>
      <button
        type="button"
        onClick={() => {
          onError('boom');
        }}
      >
        emit-error
      </button>
    </div>
  ),
}));

vi.mock('praxis/widgets/catalogue-widget', () => ({
  CatalogueWidget: () => <div>catalogue</div>,
}));

vi.mock('praxis/widgets/chart-widget', () => ({
  ChartWidget: () => <div>chart</div>,
}));

vi.mock('praxis/widgets/matrix-widget', () => ({
  MatrixWidget: () => <div>matrix</div>,
}));

describe('PraxisCanvasWorkspace', () => {
  it('renders widgets, reflects page breaks, and forwards view/error state', async () => {
    const selection: SelectionState = { nodeIds: [], edgeIds: [] };
    const widgets: PraxisCanvasWidget[] = [
      { id: 'g1', kind: 'graph', title: 'Graph', view: {} } as unknown as PraxisCanvasWidget,
      {
        id: 'c1',
        kind: 'catalogue',
        title: 'Catalogue',
        view: {},
      } as unknown as PraxisCanvasWidget,
      { id: 'h1', kind: 'chart', title: 'Chart', view: {} } as unknown as PraxisCanvasWidget,
      { id: 'm1', kind: 'matrix', title: 'Matrix', view: {} } as unknown as PraxisCanvasWidget,
    ];

    const onGraphStatsChange = vi.fn();
    const onGraphMetadataChange = vi.fn();
    const onGraphErrorMessage = vi.fn();

    const { rerender } = render(
      <PraxisCanvasWorkspace
        widgets={widgets}
        selection={selection}
        showPageBreaks
        reloadSignal={0}
        onGraphStatsChange={onGraphStatsChange}
        onGraphMetadataChange={onGraphMetadataChange}
        onGraphErrorMessage={onGraphErrorMessage}
      />,
    );

    expect(screen.getByText('graph reload:0')).toBeInTheDocument();
    expect(screen.getByText('catalogue')).toBeInTheDocument();
    expect(screen.getByText('chart')).toBeInTheDocument();
    expect(screen.getByText('matrix')).toBeInTheDocument();

    expect(screen.getByTestId('canvas-runtime')).toHaveAttribute('data-pages', 'true');

    fireEvent.click(screen.getByText('emit-view'));
    expect(onGraphMetadataChange).toHaveBeenCalled();
    expect(onGraphStatsChange).toHaveBeenCalled();
    expect(onGraphErrorMessage).toHaveBeenCalledWith();

    fireEvent.click(screen.getByText('emit-error'));
    expect(screen.getByText('boom')).toBeInTheDocument();
    expect(onGraphErrorMessage).toHaveBeenCalledWith('boom');

    rerender(<PraxisCanvasWorkspace widgets={widgets} selection={selection} reloadSignal={1} />);
    await waitFor(() => {
      expect(screen.getByText('graph reload:1')).toBeInTheDocument();
    });
  });
});
