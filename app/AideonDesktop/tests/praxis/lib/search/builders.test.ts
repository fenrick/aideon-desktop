import { describe, expect, it, vi } from 'vitest';

import { buildCatalogIndex, buildCommitIndex, buildSidebarIndex } from 'praxis/lib/search/builders';

describe('search builders', () => {
  it('builds sidebar entries recursively and supports selection handlers', async () => {
    const onSelect = vi.fn();
    const items = [
      {
        id: 'root',
        label: 'Root',
        children: [
          { id: 'child-1', label: 'Child 1' },
          { id: 'child-2', label: 'Child 2', children: [{ id: 'grand', label: 'Grand' }] },
        ],
      },
      { id: 'solo', label: 'Solo' },
    ];

    const index = buildSidebarIndex(items, onSelect);
    const ids = index.map((entry) => entry.id);
    expect(ids).toContain('sidebar:root');
    expect(ids).toContain('sidebar:child-1');
    expect(ids).toContain('sidebar:child-2');
    expect(ids).toContain('sidebar:grand');
    expect(ids).toContain('sidebar:solo');

    const runnable = index.find((entry) => entry.id === 'sidebar:child-1');
    expect(runnable?.run).toBeTypeOf('function');
    await runnable?.run?.();
    expect(onSelect).toHaveBeenCalledWith('child-1');
  });

  it('builds commit entries and swallows rejected async selection handlers', async () => {
    const onSelect = vi.fn(() => Promise.reject(new Error('boom')));
    const commits = [
      {
        id: 'c1',
        branch: 'main',
        parents: [],
        author: 'A',
        time: '2025-01-01T00:00:00.000Z',
        message: 'Fix',
        tags: ['ui'],
        changeCount: 0,
      },
      {
        id: 'c2',
        branch: 'dev',
        parents: [],
        message: 'Add',
        tags: [],
        changeCount: 20,
      },
    ];

    const index = buildCommitIndex(commits, onSelect);
    expect(index[0]?.kind).toBe('commit');
    expect(index[0]?.subtitle).toContain('2025');
    expect(index[1]?.subtitle).toBeUndefined();
    expect(index[1]?.priority).toBeGreaterThan(index[0]?.priority ?? 0);

    await index[0]?.run?.();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(onSelect).toHaveBeenCalledWith('c1');

    const noHandler = buildCommitIndex(commits);
    expect(noHandler[0]?.run).toBeUndefined();
  });

  it('builds catalog entries with optional fields and selection handler', async () => {
    const onSelect = vi.fn();
    const index = buildCatalogIndex(
      [
        {
          id: 'cap-1',
          name: 'Capability',
          type: 'Capability',
          description: 'Business capability',
          sidebarId: 'catalogues',
        },
        { id: 'cap-2', name: 'NoDesc', type: 'Capability' },
      ],
      onSelect,
    );

    expect(index[0]?.subtitle).toContain('Business');
    expect(index[1]?.subtitle).toBe('Capability');

    await index[0]?.run?.();
    expect(onSelect).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'cap-1', name: 'Capability' }),
    );
  });
});
