import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { fetchMetaModel } from 'praxis/lib/meta-model';
import type * as PraxisApi from 'praxis/praxis-api';
import { getCatalogueView } from 'praxis/praxis-api';
import type { TemporalPanelState } from 'praxis/time/use-temporal-panel';

const selectCommitSpy = vi.fn();
const selectBranchSpy = vi.fn();
const refreshBranchesSpy = vi.fn();
const selectNodesSpy = vi.fn();
const focusMetaModelSpy = vi.fn();
const scrollIntoViewMock = vi.fn();
const originalScrollIntoView = Element.prototype.scrollIntoView.bind(Element.prototype);

class ResizeObserverMock {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}

beforeAll(() => {
  vi.stubGlobal('ResizeObserver', ResizeObserverMock);
  Element.prototype.scrollIntoView = scrollIntoViewMock;
});

afterAll(() => {
  vi.unstubAllGlobals();
  Element.prototype.scrollIntoView = originalScrollIntoView;
});

afterEach(() => {
  cleanup();
});

vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: () =>
    [
      mockState,
      {
        selectCommit: selectCommitSpy,
        selectBranch: selectBranchSpy,
        refreshBranches: refreshBranchesSpy,
        mergeIntoMain: vi.fn(() => Promise.resolve()),
      },
    ] as const,
}));

vi.mock('praxis/lib/meta-model', () => ({
  fetchMetaModel: vi.fn(),
}));

const fetchMetaModelMock = vi.mocked(fetchMetaModel);

vi.mock('praxis/praxis-api', async () => {
  const actual = await vi.importActual<typeof PraxisApi>('praxis/praxis-api');
  return {
    ...actual,
    getCatalogueView: vi.fn(),
  };
});

const getCatalogueViewMock = vi.mocked(getCatalogueView);

let mockState: TemporalPanelState;

import { GlobalSearchCard } from 'praxis/components/dashboard/global-search-card';

describe('GlobalSearchCard', () => {
  beforeEach(() => {
    selectCommitSpy.mockReset();
    selectBranchSpy.mockReset();
    refreshBranchesSpy.mockReset();
    selectNodesSpy.mockReset();
    focusMetaModelSpy.mockReset();
    scrollIntoViewMock.mockReset();
    fetchMetaModelMock.mockResolvedValue({
      version: 'test',
      description: 'Test schema',
      types: [{ id: 'Capability', label: 'Business Capability', category: 'Business' }],
      relationships: [
        { id: 'supports', label: 'Supports', from: ['Application'], to: ['Capability'] },
      ],
    });
    getCatalogueViewMock.mockResolvedValue({
      metadata: {
        id: 'cmd',
        name: 'Command palette quick search',
        asOf: new Date().toISOString(),
        fetchedAt: new Date().toISOString(),
        source: 'mock',
      },
      columns: [
        { id: 'name', label: 'Name', type: 'string' },
        { id: 'owner', label: 'Owner', type: 'string' },
        { id: 'state', label: 'State', type: 'string' },
      ],
      rows: [
        {
          id: 'cap-onboarding',
          values: { name: 'Customer Onboarding', owner: 'CX', state: 'Pilot' },
        },
      ],
    });
    mockState = {
      branches: [
        { name: 'main', head: 'a1' },
        { name: 'chronaplay', head: 'b2' },
      ],
      branch: 'main',
      commits: [
        {
          id: 'commit-1',
          branch: 'main',
          parents: [],
          author: 'Chrona',
          message: 'Add catalogue widgets',
          tags: ['ui', 'catalogue'],
          changeCount: 6,
          time: '2025-11-10T10:30:00.000Z',
        },
        {
          id: 'commit-2',
          branch: 'chronaplay',
          parents: [],
          author: 'Metis',
          message: 'Scenario polish',
          tags: ['scenario'],
          changeCount: 2,
          time: '2025-11-11T12:00:00.000Z',
        },
      ],
      commitId: undefined,
      snapshot: undefined,
      loading: false,
      snapshotLoading: false,
      error: undefined,
      mergeConflicts: undefined,
      merging: false,
    } satisfies TemporalPanelState;
  });

  it('shows recent commits preview actions', async () => {
    render(
      <GlobalSearchCard onSelectNodes={selectNodesSpy} onFocusMetaModel={focusMetaModelSpy} />,
    );

    const switchBranchButton = screen.getAllByText('Switch timeline')[0];
    if (!switchBranchButton) {
      throw new Error('Expected switch timeline button.');
    }
    fireEvent.click(switchBranchButton);
    expect(selectBranchSpy).toHaveBeenCalledWith('chronaplay');

    const jumpButton = screen.getAllByText('Jump to moment')[0];
    if (!jumpButton) {
      throw new Error('Expected jump button.');
    }
    fireEvent.click(jumpButton);
    expect(selectCommitSpy).toHaveBeenCalledWith('commit-2');

    await waitFor(() => {
      expect(getCatalogueViewMock).toHaveBeenCalled();
    });
  });

  it('uses the command palette to trigger actions', { timeout: 15_000 }, async () => {
    render(
      <GlobalSearchCard onSelectNodes={selectNodesSpy} onFocusMetaModel={focusMetaModelSpy} />,
    );

    const openButton = screen.getByRole('button', { name: /Open command palette/i });
    fireEvent.click(openButton);
    await waitFor(() => {
      expect(screen.getByText('chronaplay')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('chronaplay'));
    expect(selectBranchSpy).toHaveBeenCalledWith('chronaplay');

    fireEvent.click(openButton);
    const catalogueEntry = await screen.findByText('Customer Onboarding');
    fireEvent.click(catalogueEntry);
    const lastCommandBadges = screen.getAllByText((_, node) =>
      (node?.textContent ?? '').includes('Last command'),
    );
    expect(
      lastCommandBadges.some((node) =>
        (node.textContent || '').includes('Catalogue · Customer Onboarding'),
      ),
    ).toBe(true);
    expect(selectNodesSpy).toHaveBeenCalledWith(['cap-onboarding']);

    fireEvent.click(openButton);
    const metaEntry = await screen.findByText('Business Capability');
    fireEvent.click(metaEntry);
    expect(
      lastCommandBadges.some((node) =>
        (node.textContent || '').includes('Meta-model · Business Capability'),
      ),
    ).toBe(true);
    expect(focusMetaModelSpy).toHaveBeenCalledWith({
      id: 'Capability',
      label: 'Business Capability',
      category: 'Business',
      kind: 'type',
    });

    fireEvent.click(openButton);
    fireEvent.click(screen.getByText('Refresh timelines'));
    expect(refreshBranchesSpy).toHaveBeenCalled();
  });

  it('shows empty commits and surfaces catalogue errors', async () => {
    const keyTarget = globalThis as unknown as Window;
    mockState = { ...mockState, commits: [] };
    getCatalogueViewMock.mockRejectedValueOnce(new Error('catalogue down'));

    render(<GlobalSearchCard />);

    await waitFor(() => {
      expect(screen.getByText(/No recent moments available/i)).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByText(/catalogue down/i)).toBeInTheDocument();
    });

    fireEvent.keyDown(keyTarget, { key: 'k', ctrlKey: true });
    await waitFor(() => {
      expect(screen.getAllByText('chronaplay').length).toBeGreaterThan(0);
    });
  });
});
