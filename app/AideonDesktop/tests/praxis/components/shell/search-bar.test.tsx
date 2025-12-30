import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { SearchBar } from 'praxis/components/shell/search-bar';
import { searchStore } from 'praxis/lib/search';

describe('SearchBar', () => {
  const sidebarItems = [
    {
      id: 'n1',
      label: 'Node One',
      children: [],
    },
  ];

  beforeEach(() => {
    vi.useFakeTimers();
    searchStore.clear();
    searchStore.setSidebarItems(sidebarItems, vi.fn());
  });

  afterEach(() => {
    vi.useRealTimers();
    searchStore.clear();
    cleanup();
  });

  it('debounces search input and shows results', () => {
    const searchSpy = vi.spyOn(searchStore, 'search');
    render(<SearchBar />);

    const input = screen.getByPlaceholderText(/search timelines/i);
    fireEvent.change(input, { target: { value: 'Node' } });
    expect(searchSpy).not.toHaveBeenCalled();

    vi.advanceTimersByTime(200);
    expect(searchSpy).toHaveBeenCalledWith('Node');

    // Focus opens overlay when results exist
    fireEvent.focus(input);
    expect(screen.getByText('Navigation')).toBeInTheDocument();
  });

  it('navigates results with keyboard and clears on selection', () => {
    const onSelect = vi.fn();
    searchStore.setSidebarItems(sidebarItems, onSelect);
    render(<SearchBar />);

    const input = screen.getByPlaceholderText(/search timelines/i);
    fireEvent.change(input, { target: { value: 'Node' } });
    vi.advanceTimersByTime(200);
    fireEvent.focus(input);

    fireEvent.keyDown(input, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledWith('n1');
    expect((input as HTMLInputElement).value).toBe('');
  });

  it('clears results on escape', () => {
    render(<SearchBar />);
    const input = screen.getByPlaceholderText(/search timelines/i);
    fireEvent.change(input, { target: { value: 'Node' } });
    vi.advanceTimersByTime(200);
    fireEvent.focus(input);

    fireEvent.keyDown(input, { key: 'Escape' });
    expect(searchStore.getState().results).toHaveLength(0);
    expect((input as HTMLInputElement).value).toBe('');
  });
});
