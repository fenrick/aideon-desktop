import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DebugOverlay } from 'praxis/components/debug-overlay';

const recentAnalyticsMock = vi.hoisted(() =>
  vi.fn(() => [
    { event: 'selection.change', payload: { nodes: 1 }, at: 1 },
    { event: 'time.cursor', payload: { branch: 'main' }, at: 2 },
  ]),
);

vi.mock('praxis/lib/analytics', () => ({
  recentAnalytics: recentAnalyticsMock,
}));

describe('DebugOverlay', () => {
  beforeEach(() => {
    recentAnalyticsMock.mockClear();
    recentAnalyticsMock.mockImplementation(() => [
      { event: 'selection.change', payload: { nodes: 1 }, at: 1 },
      { event: 'time.cursor', payload: { branch: 'main' }, at: 2 },
    ]);
  });

  it('hides when not visible', () => {
    const { container } = render(<DebugOverlay visible={false} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders selection info and recent analytics', () => {
    render(
      <DebugOverlay
        visible
        scenarioName="Scenario A"
        templateName="Template 1"
        branch="main"
        commitId="c1"
        selection={{ nodeIds: ['n1', 'n2'], edgeIds: [], sourceWidgetId: undefined }}
      />,
    );

    expect(screen.getByText('Scenario A')).toBeInTheDocument();
    expect(screen.getByText('2 node(s)')).toBeInTheDocument();
    expect(screen.getByText('selection.change')).toBeInTheDocument();
    expect(screen.getByText('time.cursor')).toBeInTheDocument();
  });

  it('renders edge/widget selections and empty analytics state', () => {
    recentAnalyticsMock.mockReturnValueOnce([]);

    render(
      <DebugOverlay
        visible
        selection={{ nodeIds: [], edgeIds: ['e1'], sourceWidgetId: undefined }}
      />,
    );
    expect(screen.getByText('1 edge(s)')).toBeInTheDocument();
    expect(screen.getByText(/No events yet/i)).toBeInTheDocument();

    render(<DebugOverlay visible selection={{ nodeIds: [], edgeIds: [], sourceWidgetId: 'w1' }} />);
    expect(screen.getByText('widget w1')).toBeInTheDocument();
  });
});
