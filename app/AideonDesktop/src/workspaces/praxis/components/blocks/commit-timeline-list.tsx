import { Button } from 'design-system/components/ui/button';
import type { TemporalCommitSummary } from 'praxis/praxis-api';

interface CommitTimelineListProperties {
  readonly commits: TemporalCommitSummary[];
  readonly activeCommitId?: string;
  readonly onSelect: (commitId?: string) => void;
}

/**
 * Render a vertical list of moments with quick-select actions.
 * @param root0 - List properties.
 * @param root0.commits - Moments to display.
 * @param root0.activeCommitId - Currently selected moment id.
 * @param root0.onSelect - Selection callback.
 * @returns Ordered list element.
 */
export function CommitTimelineList({
  commits,
  activeCommitId,
  onSelect,
}: CommitTimelineListProperties) {
  if (commits.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">No moments recorded yet for this timeline.</p>
    );
  }

  return (
    <ol className="space-y-3">
      {commits.map((commit) => {
        const isActive = commit.id === activeCommitId;
        return (
          <li
            key={commit.id}
            className={`rounded-2xl border px-3 py-2 ${
              isActive ? 'border-primary/70 bg-primary/10' : 'border-border/70'
            }`}
          >
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm font-semibold">{commit.message}</p>
                <p className="text-xs text-muted-foreground">
                  {commit.time ? new Date(commit.time).toLocaleString() : 'Unknown time'} ·{' '}
                  {commit.changeCount} changes
                </p>
              </div>
              <Button
                size="sm"
                variant={isActive ? 'secondary' : 'outline'}
                onClick={() => {
                  onSelect(commit.id);
                }}
              >
                View
              </Button>
            </div>
          </li>
        );
      })}
    </ol>
  );
}
