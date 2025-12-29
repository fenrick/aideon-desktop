import { describe, expect, it, vi } from 'vitest';

import { fetchMetaModel } from 'praxis/lib/meta-model';

describe('meta-model fetch', () => {
  it('returns the sample schema and respects latency', async () => {
    const now = Date.now();
    vi.useFakeTimers();
    const promise = fetchMetaModel();
    vi.advanceTimersByTime(150);
    const schema = await promise;
    expect(schema.types.length).toBeGreaterThan(0);
    expect(schema.relationships).not.toHaveLength(0);
    expect(schema.relationships[0]!.from).toContain('Application');
    expect(Date.now() - now).toBeGreaterThanOrEqual(0);
    vi.useRealTimers();
  });
});
