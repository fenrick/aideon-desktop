import { describe, expect, it } from 'vitest';

import { areStringSetsEqual, selectionFromEvent } from './graph-selection';

describe('graph-selection helpers', () => {
  it('maps selection event to snapshot ids', () => {
    const snapshot = selectionFromEvent({
      nodes: [{ id: 'n1' }, { id: 'n2' }],
      edges: [{ id: 'e1' }],
    });

    expect(snapshot).toEqual({ nodeIds: ['n1', 'n2'], edgeIds: ['e1'] });
  });

  it('treats string arrays as sets', () => {
    expect(areStringSetsEqual(['a', 'b'], ['b', 'a'])).toBe(true);
    expect(areStringSetsEqual(['a', 'b', 'b'], ['a', 'b'])).toBe(true);
    expect(areStringSetsEqual(['a', 'b'], ['a', 'c'])).toBe(false);
  });
});
