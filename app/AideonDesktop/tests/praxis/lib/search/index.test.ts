import { beforeEach, describe, expect, it, vi } from 'vitest';

import { searchStore } from '@/workspaces/praxis/lib/search';

const resetStore = () => {
  searchStore.clear();
  searchStore.setSidebarItems([]);
  searchStore.setCatalogEntities([]);
  searchStore.setRecentCommits([]);
};

describe('searchStore', () => {
  beforeEach(() => {
    resetStore();
  });

  it('returns no results for an empty query', () => {
    searchStore.setSidebarItems([{ id: 'db', label: 'Database' }]);

    const results = searchStore.search('   ');

    expect(results).toEqual([]);
    expect(searchStore.getState().results).toEqual([]);
  });

  it('ranks results by score then title', () => {
    searchStore.setSidebarItems([
      { id: 'db', label: 'Database' },
      { id: 'node-b', label: 'Node B' },
      { id: 'node-a', label: 'Node A' },
    ]);
    searchStore.setCatalogEntities([
      {
        id: 'catalog-db',
        name: 'Database',
        type: 'Service',
      },
    ]);

    const databaseResults = searchStore.search('database');

    expect(databaseResults[0]?.kind).toBe('sidebar');
    expect(databaseResults[1]?.kind).toBe('catalog');

    const nodeResults = searchStore.search('node');

    expect(nodeResults.map((item) => item.title)).toEqual(['Node A', 'Node B']);
  });

  it('notifies subscribers and stops after unsubscribe', () => {
    const subscriber = vi.fn();

    const unsubscribe = searchStore.subscribe(subscriber);
    searchStore.setSidebarItems([{ id: 'alpha', label: 'Alpha' }]);

    expect(subscriber).toHaveBeenCalledTimes(2);

    unsubscribe();
    searchStore.setSidebarItems([{ id: 'beta', label: 'Beta' }]);

    expect(subscriber).toHaveBeenCalledTimes(2);
  });
});
