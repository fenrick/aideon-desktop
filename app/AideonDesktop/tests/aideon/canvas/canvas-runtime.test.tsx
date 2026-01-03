import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  AideonCanvasRuntime,
  type CanvasRuntimeLayoutPersistence,
} from 'aideon/canvas/canvas-runtime';
import type { CanvasWidgetLayout } from 'aideon/canvas/types';

describe('AideonCanvasRuntime', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

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

  it('loads and persists layout via provided persistence adapter', async () => {
    vi.spyOn(globalThis, 'requestAnimationFrame').mockImplementation((callback) => {
      callback(0);
      return 1;
    });
    vi.spyOn(globalThis, 'cancelAnimationFrame').mockImplementation(() => {
      return;
    });

    const layoutPersistence: CanvasRuntimeLayoutPersistence<CanvasWidgetLayout> = {
      load: vi.fn(() =>
        Promise.resolve({
          positions: { w1: { x: 10, y: 20 } },
          sizes: { w1: { w: 300, h: 400 } },
        }),
      ),
      save: vi.fn(() => Promise.resolve()),
    };

    render(
      <AideonCanvasRuntime
        widgets={widgets}
        layoutKey="doc::main::c1"
        layoutPersistence={layoutPersistence}
        renderWidget={(widget: CanvasWidgetLayout) => <div>{widget.id}</div>}
      />,
    );

    await waitFor(() => {
      expect(layoutPersistence.load).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(layoutPersistence.save).toHaveBeenCalled();
    });
  });
});
