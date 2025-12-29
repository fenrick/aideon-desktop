import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PropertiesInspector } from 'praxis/components/template-screen/properties-inspector';

describe('PropertiesInspector', () => {
  afterEach(() => {
    cleanup();
  });

  it('shows empty state when nothing is selected', () => {
    render(<PropertiesInspector selectionKind="none" />);

    expect(
      screen.getAllByText(/select a widget, node or edge to edit its properties/i).length,
    ).toBeGreaterThan(0);
  });

  it('renders widget fields when a widget is selected', () => {
    render(
      <PropertiesInspector
        selectionKind="widget"
        selectionId="widget-1"
        properties={{ name: 'Widget 1' }}
      />,
    );

    expect(screen.getByText(/widget properties/i)).toBeInTheDocument();
    expect(screen.getByDisplayValue(/Widget 1/i)).toBeInTheDocument();
  });

  it('disables actions without a selection id', () => {
    render(<PropertiesInspector selectionKind="node" />);
    expect(screen.getByRole('button', { name: /save changes/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /reset/i })).toBeDisabled();
  });

  it('invokes save/reset callbacks and renders error state', () => {
    const onSave = vi.fn(() => Promise.reject(new Error('boom')));
    const onReset = vi.fn();
    render(
      <PropertiesInspector
        selectionKind="edge"
        selectionId="edge-1"
        properties={{ name: 'Edge 1', description: 'Desc' }}
        onSave={onSave}
        onReset={onReset}
        error="Bad"
      />,
    );

    fireEvent.change(screen.getByLabelText(/name/i), { target: { value: 'Edge 2' } });
    fireEvent.click(screen.getByRole('button', { name: /save changes/i }));
    fireEvent.click(screen.getByRole('button', { name: /reset/i }));

    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ name: 'Edge 2' }));
    expect(onReset).toHaveBeenCalled();
    expect(screen.getByText('Bad')).toBeInTheDocument();
  });
});
