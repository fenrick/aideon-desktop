import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { ScenarioSearchBar } from 'praxis/components/template-screen/scenario-search-bar';
import { searchStore } from 'praxis/lib/search';

describe('ScenarioSearchBar', () => {
  it('exposes an accessible search input and propagates queries', () => {
    const handleSearch = vi.fn();

    vi.useFakeTimers();
    searchStore.clear();
    render(<ScenarioSearchBar onSearch={handleSearch} />);

    const input = screen.getByRole('searchbox', { name: /search timelines, nodes, catalogues/i });
    fireEvent.change(input, { target: { value: 'resilience' } });

    vi.runAllTimers();
    expect(handleSearch).toHaveBeenCalledWith('resilience');
    vi.useRealTimers();
  });
});
