import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { DesktopPropertiesPanel } from './desktop-properties-panel';

describe('DesktopPropertiesPanel', () => {
  it('shows empty-state when nothing is selected', () => {
    render(<DesktopPropertiesPanel />);

    expect(screen.getByText(/No selection yet/i)).toBeInTheDocument();
  });

  it('summarises node and edge counts', () => {
    render(
      <DesktopPropertiesPanel
        selection={{ sourceWidgetId: 'test', nodeIds: ['n1', 'n2'], edgeIds: ['e1'] }}
      />,
    );

    expect(
      screen.getByText((_, node) => node?.textContent === 'Nodes selected: 2'),
    ).toBeInTheDocument();
    expect(
      screen.getByText((_, node) => node?.textContent === 'Edges selected: 1'),
    ).toBeInTheDocument();
    const sourceLabels = screen.getAllByText((_, node) => {
      if (!node) {
        return false;
      }
      return (node.textContent || '').includes('Source widget:');
    });
    expect(sourceLabels.length).toBeGreaterThan(0);
  });
});
