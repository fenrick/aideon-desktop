import { useState } from 'react';

import { templateScreenCopy } from 'praxis/copy/template-screen';
import { searchStore } from 'praxis/lib/search';

import { Input } from 'design-system/components/ui/input';
import { Label } from 'design-system/components/ui/label';

interface ScenarioSearchBarProperties {
  readonly onSearch?: (query: string) => void;
}

/**
 * Search input scoped to branches, nodes, and catalogues.
 * @param root0
 * @param root0.onSearch
 */
export function ScenarioSearchBar({ onSearch }: ScenarioSearchBarProperties) {
  const [query, setQuery] = useState('');
  const copy = templateScreenCopy;

  const handleChange = (value: string) => {
    setQuery(value);
    onSearch?.(value);
    if (value.trim()) {
      searchStore.search(value);
    } else {
      searchStore.clear();
    }
  };

  return (
    <div className="space-y-1">
      <Label htmlFor="scenario-search">{copy.searchLabel}</Label>
      <Input
        id="scenario-search"
        type="search"
        aria-label={copy.searchLabel}
        placeholder={copy.searchLabel}
        value={query}
        onChange={(event) => {
          handleChange(event.target.value);
        }}
        className="bg-background/80"
      />
    </div>
  );
}
