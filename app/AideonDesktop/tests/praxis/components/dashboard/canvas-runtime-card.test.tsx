import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/widgets/graph-widget', () => ({
  GraphWidget: ({
    reloadVersion,
    onViewChange,
    onError,
    onSelectionChange,
  }: {
    reloadVersion: number;
    onViewChange?: (view: {
      metadata: { asOf: string; scenario?: string };
      stats?: { nodes: number; edges: number };
    }) => void;
    onError?: (message: string) => void;
    onSelectionChange?: (selection: {
      nodeIds: string[];
      edgeIds: string[];
      widgetId: string;
    }) => void;
  }) => (
    <div data-testid="graph-widget" data-reload={reloadVersion}>
      <button
        data-testid="emit-view"
        onClick={() =>
          onViewChange?.({
            metadata: { asOf: '2025-01-01T00:00:00Z', scenario: 'dev' },
            stats: { nodes: 12, edges: 4 },
          })
        }
      />
      <button data-testid="emit-error" onClick={() => onError?.('boom')} />
      <button
        data-testid="emit-selection"
        onClick={() => onSelectionChange?.({ nodeIds: ['n1'], edgeIds: [], widgetId: 'w1' })}
      />
    </div>
  ),
}));

import { CanvasRuntimeCard } from 'praxis/components/dashboard/canvas-runtime-card';

describe('CanvasRuntimeCard', () => {
  it('renders stats and handles graph events', () => {
    const onSelectionChange = vi.fn();

    render(
      <CanvasRuntimeCard
        widgets={[
          {
            id: 'w1',
            kind: 'graph',
            title: 'Graph',
            view: { id: 'g1', name: 'Graph View', kind: 'graph', asOf: '2025-01-01T00:00:00Z' },
          },
        ]}
        selection={{ nodeIds: [], edgeIds: [] }}
        onSelectionChange={onSelectionChange}
      />,
    );

    expect(screen.getByText('Canvas runtime')).toBeInTheDocument();
    expect(screen.getByText(/React Flow GraphWidget/)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('emit-view'));
    expect(screen.getByText(/Nodes/).nextSibling).toHaveTextContent('12');
    expect(screen.getByText(/Edges/).nextSibling).toHaveTextContent('4');
    expect(screen.getByText(/dev/)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('emit-selection'));
    expect(onSelectionChange).toHaveBeenCalledWith({
      nodeIds: ['n1'],
      edgeIds: [],
      sourceWidgetId: 'w1',
    });

    fireEvent.click(screen.getByTestId('emit-error'));
    expect(screen.getByText('boom')).toBeInTheDocument();
  });

  it('disables refresh when no widgets and increments reload version when clicked', () => {
    const { container, unmount } = render(
      <CanvasRuntimeCard widgets={[]} selection={{ nodeIds: [], edgeIds: [] }} />,
    );
    const refreshButtons = [...container.querySelectorAll('button')].filter((button) =>
      /Refresh graph/i.test(button.textContent),
    );
    expect(refreshButtons.some((button) => button.hasAttribute('disabled'))).toBe(true);
    unmount();

    const { container: container2, getAllByTestId } = render(
      <CanvasRuntimeCard
        widgets={[
          {
            id: 'w1',
            kind: 'graph',
            title: 'Graph',
            view: { id: 'g1', name: 'Graph View', kind: 'graph', asOf: '2025-01-01T00:00:00Z' },
          },
        ]}
        selection={{ nodeIds: [], edgeIds: [] }}
      />,
    );
    const refresh = [...container2.querySelectorAll('button')].find((button) =>
      /Refresh graph/i.test(button.textContent),
    );
    if (!refresh) {
      throw new Error('refresh button missing');
    }
    fireEvent.click(refresh);
    const runtimes = getAllByTestId('graph-widget');
    expect(runtimes.some((element) => element.dataset.reload === '1')).toBe(true);
  });
});
