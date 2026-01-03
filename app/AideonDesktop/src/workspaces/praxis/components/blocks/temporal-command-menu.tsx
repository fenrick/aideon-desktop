import { useMemo } from 'react';

import type { TemporalBranchSummary, TemporalCommitSummary } from 'praxis/praxis-api';

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from 'design-system/components/ui/command';
import { DialogDescription, DialogTitle } from 'design-system/components/ui/dialog';

interface TemporalCommandMenuProperties {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly branches: TemporalBranchSummary[];
  readonly activeBranch?: string;
  readonly commits: TemporalCommitSummary[];
  readonly loading: boolean;
  readonly onSelectBranch: (branch: string) => void;
  readonly onSelectCommit: (commitId?: string) => void;
  readonly onRefreshBranches: () => void;
  readonly catalogueEntries?: CatalogueCommandEntry[];
  readonly metaModelEntries?: MetaModelCommandEntry[];
  readonly onSelectCatalogueEntry?: (entry: CatalogueCommandEntry) => void;
  readonly onSelectMetaModelEntry?: (entry: MetaModelCommandEntry) => void;
}

export interface CatalogueCommandEntry {
  readonly id: string;
  readonly label: string;
  readonly owner?: string;
  readonly state?: string;
}

export interface MetaModelCommandEntry {
  readonly id: string;
  readonly label: string;
  readonly category: string;
  readonly kind: 'type' | 'relationship';
}

/**
 * Command palette for time navigation and catalogue/meta-model search.
 * @param root0 - Command menu properties.
 * @param root0.open - Whether the dialog is open.
 * @param root0.onOpenChange - Toggle handler.
 * @param root0.branches - Available timelines.
 * @param root0.activeBranch - Current timeline name.
 * @param root0.commits - Moments to list.
 * @param root0.loading - Loading indicator.
 * @param root0.onSelectBranch - Handler when a timeline is chosen.
 * @param root0.onSelectCommit - Handler when a moment is chosen.
 * @param root0.onRefreshBranches - Handler to refresh timelines.
 * @param root0.catalogueEntries - Catalogue entries to search.
 * @param root0.metaModelEntries - Meta-model entries to search.
 * @param root0.onSelectCatalogueEntry - Handler for catalogue selection.
 * @param root0.onSelectMetaModelEntry - Handler for meta-model selection.
 * @returns Command menu dialog element.
 */
export function TemporalCommandMenu({
  open,
  onOpenChange,
  branches,
  activeBranch,
  commits,
  loading,
  onSelectBranch,
  onSelectCommit,
  onRefreshBranches,
  catalogueEntries = [],
  metaModelEntries = [],
  onSelectCatalogueEntry,
  onSelectMetaModelEntry,
}: TemporalCommandMenuProperties) {
  const sortedBranches = useMemo(() => {
    return branches.toSorted((left, right) => left.name.localeCompare(right.name));
  }, [branches]);

  const commitItems = useMemo(() => {
    return commits.toSorted((left, right) => {
      const leftTime = left.time ? Date.parse(left.time) : 0;
      const rightTime = right.time ? Date.parse(right.time) : 0;
      return rightTime - leftTime;
    });
  }, [commits]);

  const closeAfter = (callback: () => void) => {
    callback();
    onOpenChange(false);
  };

  return (
    <CommandDialog open={open} onOpenChange={onOpenChange}>
      <DialogTitle className="sr-only">Temporal command palette</DialogTitle>
      <DialogDescription className="sr-only">
        Search timelines, moments, catalogue entries, and meta-model references.
      </DialogDescription>
      <CommandInput placeholder="Search timelines, moments, tags" />
      <CommandList>
        <CommandEmpty>{loading ? 'Loading twin data…' : 'No results found.'}</CommandEmpty>
        <CommandGroup heading="Actions">
          <CommandItem
            value="refresh"
            onSelect={() => {
              closeAfter(onRefreshBranches);
            }}
          >
            Refresh timelines
            <CommandShortcut>R</CommandShortcut>
          </CommandItem>
        </CommandGroup>
        {sortedBranches.length > 0 && (
          <CommandGroup heading="Timelines">
            {sortedBranches.map((branch) => (
              <CommandItem
                key={branch.name}
                value={`branch-${branch.name}`}
                keywords={[branch.name]}
                onSelect={() => {
                  closeAfter(() => {
                    onSelectBranch(branch.name);
                  });
                }}
              >
                <span className="flex-1 text-sm font-medium">{branch.name}</span>
                {branch.name === activeBranch && <CommandShortcut>Active</CommandShortcut>}
              </CommandItem>
            ))}
          </CommandGroup>
        )}
        {commitItems.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Moments">
              {commitItems.slice(0, 25).map((commit) => (
                <CommandItem
                  key={commit.id}
                  value={`commit-${commit.id}`}
                  keywords={[commit.branch, ...commit.tags]}
                  onSelect={() => {
                    closeAfter(() => {
                      onSelectCommit(commit.id);
                    });
                  }}
                >
                  <div className="flex w-full flex-col text-left">
                    <span className="text-sm font-medium">{commit.message}</span>
                    <span className="text-xs text-muted-foreground">
                      {commit.branch} ·{formatCommitTime(commit.time)}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
        {catalogueEntries.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Catalogue">
              {catalogueEntries.slice(0, 25).map((entry) => (
                <CommandItem
                  key={entry.id}
                  value={`catalogue-${entry.id}`}
                  keywords={[entry.label, entry.owner ?? '', entry.state ?? '']}
                  onSelect={() => {
                    closeAfter(() => {
                      onSelectCatalogueEntry?.(entry);
                    });
                  }}
                >
                  <div className="flex w-full flex-col text-left">
                    <span className="text-sm font-medium">{entry.label}</span>
                    <span className="text-xs text-muted-foreground">
                      {entry.owner ?? 'Unassigned'} ·{entry.state ?? 'No state'}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
        {metaModelEntries.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Meta-model">
              {metaModelEntries.slice(0, 25).map((entry) => (
                <CommandItem
                  key={`${entry.kind}-${entry.id}`}
                  value={`${entry.kind}-${entry.id}`}
                  keywords={[entry.label, entry.category, entry.kind]}
                  onSelect={() => {
                    closeAfter(() => {
                      onSelectMetaModelEntry?.(entry);
                    });
                  }}
                >
                  <div className="flex w-full flex-col text-left">
                    <span className="text-sm font-medium">{entry.label}</span>
                    <span className="text-xs text-muted-foreground">
                      {entry.category} ·{entry.kind === 'type' ? 'Type' : 'Relationship'}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  );
}

/**
 *
 * @param value
 */
function formatCommitTime(value?: string) {
  if (!value) {
    return 'Unknown';
  }
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    return value;
  }
  return new Date(parsed).toLocaleString();
}
