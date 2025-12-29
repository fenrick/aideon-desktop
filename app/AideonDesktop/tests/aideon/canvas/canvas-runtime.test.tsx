import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { AideonCanvasRuntime } from 'aideon/canvas/canvas-runtime';
import type { CanvasWidgetLayout } from 'aideon/canvas/types';

describe('AideonCanvasRuntime', () => {
  const widgets: CanvasWidgetLayout[] = [
    { id: 'w1', size: 'half' },
    { id: 'w2', size: 'full' },
  ];

  it('renders each widget via renderWidget callback', () => {
    render(
      <AideonCanvasRuntime
        widgets={widgets}
        renderWidget={(widget: CanvasWidgetLayout) => <div>{widget.id}</div>}
      />,
    );

    expect(screen.getAllByText('w1').length).toBeGreaterThan(0);
    expect(screen.getAllByText('w2').length).toBeGreaterThan(0);
  });
});
