import { useMemo } from 'react';

import type { TemporalCommitSummary, TemporalMergeConflict } from 'praxis/praxis-api';
import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

import { Alert, AlertDescription, AlertTitle } from 'design-system/components/ui/alert';
import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import { ScrollArea } from 'design-system/components/ui/scroll-area';
import { ToggleGroup, ToggleGroupItem } from 'design-system/components/ui/toggle-group';

/**
 *
 */
export function CommitTimelineCard() {
  const [state, actions] = useTemporalPanel();

  const commits = state.commits;
  const selectedCommitId = state.commitId;
  const hasNonMainBranch = state.branch !== undefined && state.branch !== 'main';

  const sortedBranches = useMemo(() => {
    return state.branches.toSorted((a, b) => a.name.localeCompare(b.name));
  }, [state.branches]);

  return (
    <Card>
      <CardHeader className="space-y-1">
        <CardTitle>Timeline moments</CardTitle>
        <CardDescription>
          Review timeline activity and pin a moment for the current canvas snapshot.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 text-sm">
        <BranchList
          branches={sortedBranches.map((branch) => branch.name)}
          activeBranch={state.branch}
          loading={state.loading}
          onSelect={(branch) => {
            void actions.selectBranch(branch);
          }}
        />
        <CommitList
          commits={commits}
          selectedCommitId={selectedCommitId}
          loading={state.loading}
          onSelectCommit={actions.selectCommit}
        />
        {state.mergeConflicts && state.mergeConflicts.length > 0 && (
          <MergeConflicts conflicts={state.mergeConflicts} />
        )}
        {state.error && <p className="text-xs text-destructive">{state.error}</p>}
        <div className="flex justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              void actions.refreshBranches();
            }}
            disabled={state.loading}
          >
            Refresh timelines
          </Button>
          <Button
            size="sm"
            onClick={() => {
              void actions.mergeIntoMain();
            }}
            disabled={!hasNonMainBranch || state.merging}
          >
            {state.merging ? 'Applying…' : 'Apply to primary'}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

/**
 *
 * @param root0
 * @param root0.branches
 * @param root0.activeBranch
 * @param root0.loading
 * @param root0.onSelect
 */
function BranchList({
  branches,
  activeBranch,
  loading,
  onSelect,
}: {
  readonly branches: string[];
  readonly activeBranch?: string;
  readonly loading: boolean;
  readonly onSelect: (branch: string) => void;
}) {
  if (branches.length === 0) {
    return <p className="text-xs text-muted-foreground">No timelines available.</p>;
  }
  return (
    <ToggleGroup
      type="single"
      value={activeBranch}
      onValueChange={(value: string | undefined) => {
        if (value) {
          onSelect(value);
        }
      }}
      className="flex flex-wrap gap-2"
      disabled={loading}
    >
      {branches.map((branch) => (
        <ToggleGroupItem key={branch} value={branch} className="capitalize">
          {branch}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}

/**
 *
 * @param root0
 * @param root0.commits
 * @param root0.selectedCommitId
 * @param root0.loading
 * @param root0.onSelectCommit
 */
function CommitList({
  commits,
  selectedCommitId,
  loading,
  onSelectCommit,
}: {
  readonly commits: TemporalCommitSummary[];
  readonly selectedCommitId?: string;
  readonly loading: boolean;
  readonly onSelectCommit: (commitId?: string) => void;
}) {
  if (loading && commits.length === 0) {
    return <p className="text-xs text-muted-foreground">Loading moments…</p>;
  }
  if (commits.length === 0) {
    return <p className="text-xs text-muted-foreground">No moments on this timeline.</p>;
  }
  return (
    <ScrollArea className="max-h-64 pr-2">
      <div className="space-y-3">
        {commits.map((commit) => {
          const isSelected = commit.id === selectedCommitId;
          return (
            <Button
              key={commit.id}
              type="button"
              variant={isSelected ? 'secondary' : 'ghost'}
              className="w-full justify-start text-left"
              onClick={() => {
                onSelectCommit(commit.id);
              }}
            >
              <div className="flex w-full flex-col items-start gap-1">
                <span className="text-sm font-medium">{commit.message}</span>
                <span className="text-xs text-muted-foreground">
                  {commit.time ? new Date(commit.time).toLocaleString() : 'Unknown time'}
                </span>
                <Badge variant="outline" className="text-[11px]">
                  {commit.changeCount} changes
                </Badge>
              </div>
            </Button>
          );
        })}
      </div>
    </ScrollArea>
  );
}

/**
 *
 * @param root0
 * @param root0.conflicts
 */
function MergeConflicts({ conflicts }: { readonly conflicts: TemporalMergeConflict[] }) {
  return (
    <Alert variant="destructive">
      <AlertTitle>Overlap conflicts</AlertTitle>
      <AlertDescription>
        <ul className="mt-1 space-y-1 text-xs">
          {conflicts.map((conflict, index) => (
            <li key={`${conflict.reference}-${index.toString()}`}>
              <span className="font-medium">{conflict.reference}</span>:{conflict.message}
            </li>
          ))}
        </ul>
      </AlertDescription>
    </Alert>
  );
}
