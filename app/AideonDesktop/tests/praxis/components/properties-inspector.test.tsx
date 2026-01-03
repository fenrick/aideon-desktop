import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { SelectionState } from 'aideon/canvas/types';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PropertiesInspector } from 'praxis/components/template-screen/properties-inspector';

const emptySelection: SelectionState = { sourceWidgetId: undefined, nodeIds: [], edgeIds: [] };
const widgetSelection: SelectionState = {
  sourceWidgetId: 'widget-1',
  nodeIds: [],
  edgeIds: [],
};
const nodeSelection: SelectionState = {
  sourceWidgetId: undefined,
  nodeIds: ['n1'],
  edgeIds: [],
};

describe('PropertiesInspector', () => {
  afterEach(() => {
    cleanup();
  });

  it('shows empty state when nothing is selected', () => {
    render(<PropertiesInspector selectionKind="none" selection={emptySelection} />);

    expect(
      screen.getAllByText(/select a widget to edit its data, display, or interactions/i),
    ).toHaveLength(2);
  });

  it('renders widget fields when a widget is selected', () => {
    render(
      <PropertiesInspector
        selectionKind="widget"
        selectionId="widget-1"
        selection={widgetSelection}
        properties={{ name: 'Widget 1' }}
      />,
    );

    expect(screen.getByText(/widget properties/i)).toBeInTheDocument();
    expect(screen.getByDisplayValue(/Widget 1/i)).toBeInTheDocument();
  });

  it('shows bulk action controls when non-widget items are selected', () => {
    render(<PropertiesInspector selectionKind="node" selection={nodeSelection} />);
    expect(screen.getByRole('button', { name: /align/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /distribute/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /delete/i })).toBeInTheDocument();
  });

  it('invokes save/reset callbacks and renders error state for a widget', async () => {
    const onSave = vi.fn(() => Promise.resolve());
    const onReset = vi.fn();
    render(
      <PropertiesInspector
        selectionKind="widget"
        selectionId="widget-1"
        selection={widgetSelection}
        properties={{ name: 'Widget 1', description: 'Desc' }}
        onSave={onSave}
        onReset={onReset}
        error="Bad"
      />,
    );

    fireEvent.change(screen.getByLabelText(/name/i), { target: { value: 'Widget 2' } });
    fireEvent.click(screen.getByRole('button', { name: /save changes/i }));
    fireEvent.click(screen.getByRole('button', { name: /reset/i }));

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ name: 'Widget 2' }));
    });
    expect(onReset).toHaveBeenCalled();
    expect(screen.getByText('Bad')).toBeInTheDocument();
  });
});
