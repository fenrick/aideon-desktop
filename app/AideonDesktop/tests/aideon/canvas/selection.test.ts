import { describe, expect, it } from 'vitest';

import {
  dedupeIds,
  fromWidgetSelection,
  isSelectionEmpty,
  selectionSummary,
} from 'aideon/canvas/selection';
import { EMPTY_SELECTION } from 'aideon/canvas/types';

describe('canvas selection helpers', () => {
  it('deduplicates and strips empty identifiers', () => {
    expect(dedupeIds(['id-a', 'id-a', '   ', 'id-b'])).toEqual(['id-a', 'id-b']);
  });

  it('builds consistent selection state from widget events', () => {
    const state = fromWidgetSelection({ widgetId: 'graph', nodeIds: ['x', 'x'], edgeIds: [] });
    expect(state).toEqual({ sourceWidgetId: 'graph', nodeIds: ['x'], edgeIds: [] });
  });

  it('detects empty selections', () => {
    expect(isSelectionEmpty()).toBe(true);
    expect(isSelectionEmpty(EMPTY_SELECTION)).toBe(true);
  });

  it('summarises node and edge counts', () => {
    const summary = selectionSummary({
      sourceWidgetId: 'graph',
      nodeIds: ['a', 'b'],
      edgeIds: ['e'],
    });
    expect(summary).toBe('2 nodes, 1 edge');
    expect(selectionSummary()).toBe('No selection');
  });
});
