import { CommitTimelineList } from 'praxis/components/blocks/commit-timeline-list';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';

interface ActivityTimelinePanelProperties {
  readonly title?: string;
  readonly description?: string;
  readonly state: TemporalPanelState;
  readonly actions: TemporalPanelActions;
}

/**
 * Timeline panel surfacing moments with quick actions.
 * @param root0 - Panel properties.
 * @param root0.title - Heading text.
 * @param root0.description - Helper text.
 * @param root0.state - Temporal panel state to render.
 * @param root0.actions - Temporal panel actions.
 * @returns Activity timeline panel.
 */
export function ActivityTimelinePanel({
  title = 'Activity & diagnostics',
  description = 'Jump between timeline, diff, and canvas views when chasing events.',
  state,
  actions,
}: ActivityTimelinePanelProperties) {
  const commits = state.commits;
  const activeCommitId = state.commitId;

  return (
    <Card>
      <CardHeader className="space-y-1">
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              void actions.refreshBranches();
            }}
            disabled={state.loading}
          >
            Refresh timeline
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              actions.selectCommit(state.commitId);
            }}
            disabled={!state.commitId}
          >
            Reset to latest
          </Button>
        </div>
        <CommitTimelineList
          commits={commits}
          activeCommitId={activeCommitId}
          onSelect={actions.selectCommit}
        />
      </CardContent>
    </Card>
  );
}
