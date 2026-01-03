import { describe, expect, it, vi } from 'vitest';

import { recentAnalytics, setAnalyticsSink, track } from 'praxis/lib/analytics';

describe('analytics', () => {
  it('invokes sink and keeps a bounded buffer', () => {
    const sink = vi.fn();
    setAnalyticsSink(sink);

    for (let index = 0; index < 55; index += 1) {
      track('selection.change', { idx: index, fn: () => 'noop' });
    }

    expect(sink).toHaveBeenCalled();
    const recent = recentAnalytics();
    expect(recent.length).toBeLessThanOrEqual(50);
    expect(recent).not.toHaveLength(0);
    const first = recent[0];
    if (!first) {
      throw new Error('Expected analytics entry.');
    }
    expect(first.payload?.fn).toBeUndefined();
  });
});
