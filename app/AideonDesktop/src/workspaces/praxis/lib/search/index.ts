import { useEffect, useState } from 'react';

import { buildCatalogIndex, buildCommitIndex, buildSidebarIndex } from './builders';
import type {
  CatalogEntitySummary,
  CatalogSelectHandler,
  CommitSelectHandler,
  SearchIndexItem,
  SearchResult,
  SidebarSelectHandler,
  SidebarTreeNode,
  TemporalCommitSummary,
} from './types';
import { scoreItem } from './utilities';

export interface SearchStoreState {
  query: string;
  items: SearchIndexItem[];
  results: SearchResult[];
}

const createState = (): SearchStoreState => ({
  query: '',
  items: [],
  results: [],
});

type Subscriber = (state: SearchStoreState) => void;

/**
 * Internal observable store for global search. Maintains index sources and results.
 * Subscribers receive updates whenever the index is rebuilt or a query runs.
 */
function createStore() {
  let state: SearchStoreState = createState();
  const subscribers = new Set<Subscriber>();
  const sources = new Map<string, SearchIndexItem[]>();

  const notify = () => {
    for (const subscriber of subscribers) {
      subscriber(state);
    }
  };

  const rebuildIndex = () => {
    const items = [...sources.values()].flat();
    state = { ...state, items };
    notify();
  };

  const subscribe = (subscriber: Subscriber): (() => void) => {
    subscriber(state);
    subscribers.add(subscriber);
    return () => {
      subscribers.delete(subscriber);
    };
  };

  const search = (query: string, limit = 12): SearchResult[] => {
    const trimmed = query.trim();
    const tokens = trimmed
      .split(/\s+/)
      .map((token) => token.trim().toLowerCase())
      .filter(Boolean);
    if (tokens.length === 0) {
      state = { ...state, query, results: [] };
      notify();
      return [];
    }
    const scored = state.items
      .map((item) => ({ item, score: scoreItem(item, tokens) }))
      .filter(({ score }) => score > 0)
      .toSorted((a, b) => {
        if (b.score !== a.score) {
          return b.score - a.score;
        }
        return a.item.title.localeCompare(b.item.title);
      });
    const results = scored.slice(0, limit).map(({ item }) => item);
    state = { ...state, query, results };
    notify();
    return results;
  };

  const clear = () => {
    state = { ...state, query: '', results: [] };
    notify();
  };

  const setSource = (key: string, items: SearchIndexItem[]) => {
    sources.set(key, items);
    rebuildIndex();
  };

  return {
    search,
    clear,
    subscribe,
    setSidebarItems: (items: SidebarTreeNode[], onSelect?: SidebarSelectHandler) => {
      setSource('sidebar', buildSidebarIndex(items, onSelect));
    },
    setRecentCommits: (commits: TemporalCommitSummary[], onSelect?: CommitSelectHandler) => {
      setSource('commits', buildCommitIndex(commits, onSelect));
    },
    setCatalogEntities: (entities: CatalogEntitySummary[], onSelect?: CatalogSelectHandler) => {
      setSource('catalog', buildCatalogIndex(entities, onSelect));
    },
    getState: () => state,
  };
}

const store = createStore();

export const searchStore = store;

/**
 * React hook that subscribes to the global search store and returns its state.
 * @returns latest search query, index items, and results.
 */
export function useSearchStoreState() {
  const [state, setState] = useState(store.getState());
  useEffect(() => {
    const unsubscribe = store.subscribe(setState);
    return () => {
      unsubscribe();
    };
  }, []);
  return state;
}
