import type { ChangeEvent, FocusEvent, KeyboardEvent } from 'react';
import { useEffect, useRef, useState } from 'react';

import { Input } from 'design-system/components/ui/input';
import { searchStore, useSearchStoreState } from 'praxis/lib/search';
import type { SearchResult } from 'praxis/lib/search/types';

const KIND_LABEL: Record<SearchResult['kind'], string> = {
  sidebar: 'Navigation',
  catalog: 'Catalogue',
  commit: 'Commit',
};

const DEBOUNCE_MS = 180;

/**
 *
 */
export function SearchBar() {
  const { results } = useSearchStoreState();
  const [query, setQuery] = useState('');
  const [focused, setFocused] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const inputReference = useRef<HTMLInputElement | null>(null);
  const debounceReference = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (debounceReference.current) {
        clearTimeout(debounceReference.current);
      }
    };
  }, []);

  const overlayOpen = focused && results.length > 0;

  const scheduleSearch = (value: string) => {
    if (debounceReference.current) {
      clearTimeout(debounceReference.current);
    }
    if (!value.trim()) {
      searchStore.clear();
      return;
    }
    debounceReference.current = setTimeout(() => {
      searchStore.search(value);
    }, DEBOUNCE_MS);
  };

  const handleInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setQuery(value);
    scheduleSearch(value);
  };

  const handleFocus = () => {
    setFocused(true);
    if (query.trim()) {
      searchStore.search(query);
    }
  };

  const handleBlur = (event: FocusEvent<HTMLFormElement>) => {
    const next = event.relatedTarget as HTMLElement | null;
    if (next?.closest('.search-results')) {
      return;
    }
    setFocused(false);
    setHighlightedIndex(0);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (!overlayOpen) {
      return;
    }
    switch (event.key) {
      case 'ArrowDown': {
        event.preventDefault();
        setHighlightedIndex((previous) => (previous + 1) % results.length);

        break;
      }
      case 'ArrowUp': {
        event.preventDefault();
        setHighlightedIndex((previous) => (previous - 1 + results.length) % results.length);

        break;
      }
      case 'Enter': {
        event.preventDefault();
        selectResult(highlightedIndex);

        break;
      }
      case 'Escape': {
        event.preventDefault();
        searchStore.clear();
        setQuery('');
        inputReference.current?.blur();
        setFocused(false);

        break;
      }
      // No default
    }
  };

  const selectResult = (index: number) => {
    const item = results.at(index);
    if (!item) {
      return;
    }
    const outcome = item.run?.();
    if (outcome instanceof Promise) {
      outcome.catch(() => false);
    }
    searchStore.clear();
    setQuery('');
    setFocused(false);
    inputReference.current?.blur();
  };

  return (
    <form className="relative w-full" role="search" onBlur={handleBlur}>
      <Input
        ref={inputReference}
        placeholder="Search branches, nodes, catalogues…"
        value={query}
        onChange={handleInputChange}
        onFocus={handleFocus}
        onKeyDown={handleKeyDown}
        className="bg-background/80"
      />
      {overlayOpen ? (
        <div className="search-results absolute left-0 top-full z-20 mt-2 max-h-64 w-full divide-y divide-border overflow-hidden rounded-2xl border border-border bg-muted/90 text-sm text-foreground shadow-lg">
          {results.map((result, index) => (
            <button
              key={result.id}
              type="button"
              onClick={() => {
                selectResult(index);
              }}
              className={`flex w-full flex-col gap-0.5 px-4 py-3 text-left transition hover:bg-background/70 ${
                index === highlightedIndex ? 'bg-accent/40' : ''
              }`}
            >
              <span className="text-xs uppercase tracking-[0.3em] text-muted-foreground">
                {KIND_LABEL[result.kind]}
              </span>
              <span className="font-semibold">{result.title}</span>
              {result.subtitle ? (
                <span className="text-xs text-muted-foreground">{result.subtitle}</span>
              ) : undefined}
            </button>
          ))}
        </div>
      ) : undefined}
    </form>
  );
}
