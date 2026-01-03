import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { TemporalCommandMenu } from 'praxis/components/blocks/temporal-command-menu';

vi.mock('design-system/components/ui/command', () => ({
  CommandDialog: ({ open, children }: { open: boolean; children?: React.ReactNode }) =>
    open ? <div data-testid="command-dialog">{children}</div> : undefined,
  CommandInput: ({ placeholder }: { placeholder?: string }) => (
    <input aria-label="command-input" placeholder={placeholder} />
  ),
  CommandList: ({ children }: { children?: React.ReactNode }) => <div>{children}</div>,
  CommandEmpty: ({ children }: { children?: React.ReactNode }) => <div>{children}</div>,
  CommandSeparator: () => <hr />,
  CommandGroup: ({ heading, children }: { heading?: string; children?: React.ReactNode }) => (
    <section>
      <h3>{heading}</h3>
      {children}
    </section>
  ),
  CommandShortcut: ({ children }: { children?: React.ReactNode }) => (
    <span data-testid="shortcut">{children}</span>
  ),
  CommandItem: ({ children, onSelect }: { children?: React.ReactNode; onSelect?: () => void }) => (
    <button type="button" onClick={() => onSelect?.()}>
      {children}
    </button>
  ),
}));

vi.mock('design-system/components/ui/dialog', () => ({
  DialogTitle: ({ children }: { children?: React.ReactNode }) => <div>{children}</div>,
  DialogDescription: ({ children }: { children?: React.ReactNode }) => <div>{children}</div>,
}));

describe('TemporalCommandMenu', () => {
  afterEach(() => {
    cleanup();
  });

  it('shows loading vs empty states and closes after actions', () => {
    const onOpenChange = vi.fn();
    const onRefreshBranches = vi.fn();

    const { rerender } = render(
      <TemporalCommandMenu
        open
        onOpenChange={onOpenChange}
        branches={[]}
        commits={[]}
        loading
        onSelectBranch={vi.fn()}
        onSelectCommit={vi.fn()}
        onRefreshBranches={onRefreshBranches}
      />,
    );

    expect(screen.getByText(/Loading twin data/i)).toBeInTheDocument();
    fireEvent.click(screen.getByText('Refresh timelines'));
    expect(onRefreshBranches).toHaveBeenCalled();
    expect(onOpenChange).toHaveBeenCalledWith(false);

    rerender(
      <TemporalCommandMenu
        open
        onOpenChange={onOpenChange}
        branches={[]}
        commits={[]}
        loading={false}
        onSelectBranch={vi.fn()}
        onSelectCommit={vi.fn()}
        onRefreshBranches={onRefreshBranches}
      />,
    );
    expect(screen.getByText(/No results found/i)).toBeInTheDocument();
  });

  it('sorts branches and commits and supports catalogue/meta-model handlers', () => {
    const onOpenChange = vi.fn();
    const onSelectBranch = vi.fn();
    const onSelectCommit = vi.fn();
    const onSelectCatalogueEntry = vi.fn();
    const onSelectMetaModelEntry = vi.fn();

    render(
      <TemporalCommandMenu
        open
        onOpenChange={onOpenChange}
        branches={[
          { name: 'chronaplay', head: 'c1' },
          { name: 'main', head: 'c2' },
        ]}
        activeBranch="main"
        commits={[
          { id: 'c1', branch: 'main', parents: [], message: 'Old', tags: [], changeCount: 0 },
          {
            id: 'c2',
            branch: 'main',
            parents: [],
            message: 'New',
            tags: ['t1'],
            changeCount: 0,
            time: '2025-01-01T00:00:00Z',
          },
          {
            id: 'c3',
            branch: 'main',
            parents: [],
            message: 'Invalid time',
            tags: [],
            changeCount: 0,
            time: 'not-a-date',
          },
        ]}
        loading={false}
        onSelectBranch={onSelectBranch}
        onSelectCommit={onSelectCommit}
        onRefreshBranches={vi.fn()}
        catalogueEntries={[
          { id: 'cap-1', label: 'Capability', owner: undefined, state: undefined },
        ]}
        metaModelEntries={[
          { id: 't1', label: 'Type A', category: 'Business', kind: 'type' },
          { id: 'r1', label: 'Rel A', category: 'A → B', kind: 'relationship' },
        ]}
        onSelectCatalogueEntry={onSelectCatalogueEntry}
        onSelectMetaModelEntry={onSelectMetaModelEntry}
      />,
    );

    expect(screen.getByText('Timelines')).toBeInTheDocument();
    expect(screen.getAllByText('main').length).toBeGreaterThan(0);
    expect(screen.getAllByTestId('shortcut').some((node) => node.textContent === 'Active')).toBe(
      true,
    );

    fireEvent.click(screen.getByText('chronaplay'));
    expect(onSelectBranch).toHaveBeenCalledWith('chronaplay');
    expect(onOpenChange).toHaveBeenCalledWith(false);

    fireEvent.click(screen.getByText('New'));
    expect(onSelectCommit).toHaveBeenCalledWith('c2');

    expect(screen.getByRole('heading', { name: 'Catalogue' })).toBeInTheDocument();
    fireEvent.click(screen.getByText('Capability'));
    expect(onSelectCatalogueEntry).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'cap-1', label: 'Capability' }),
    );

    expect(screen.getByRole('heading', { name: 'Meta-model' })).toBeInTheDocument();
    fireEvent.click(screen.getByText('Type A'));
    expect(onSelectMetaModelEntry).toHaveBeenCalledWith(
      expect.objectContaining({ id: 't1', kind: 'type' }),
    );
    fireEvent.click(screen.getByText('Rel A'));
    expect(onSelectMetaModelEntry).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'r1', kind: 'relationship' }),
    );
  });
});
