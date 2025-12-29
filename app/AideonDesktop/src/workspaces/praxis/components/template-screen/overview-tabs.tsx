import type { ReactNode, Ref } from 'react';

import { ActivityTimelinePanel } from 'praxis/components/blocks/activity-timeline-panel';
import { CommitTimelineList } from 'praxis/components/blocks/commit-timeline-list';
import { templateScreenCopy } from 'praxis/copy/template-screen';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';
import type { PraxisCanvasWidget, SelectionState } from 'praxis/types';

import { Tabs, TabsContent, TabsList, TabsTrigger } from 'design-system/components/ui/tabs';
import { cn } from 'design-system/lib/utilities';
import { PraxisCanvasWorkspace } from 'praxis/components/canvas/praxis-canvas-workspace';
import { SnapshotOverviewCard } from './snapshot-overview-card';
import { TimeCursorCard } from './time-cursor-card';

interface OverviewTabsProperties {
  readonly state: TemporalPanelState;
  readonly actions: TemporalPanelActions;
  readonly widgets: PraxisCanvasWidget[];
  readonly selection: SelectionState;
  readonly onSelectionChange: (selection: SelectionState) => void;
  readonly onRequestMetaModelFocus: (types: string[]) => void;
  readonly onAddWidget?: () => void;
  readonly timelineContent?: ReactNode;
  readonly activityContent?: ReactNode;
  readonly initialTab?: 'canvas' | 'overview' | 'timeline' | 'activity';
  readonly className?: string;
  readonly reloadSignal?: number;
  readonly branchTriggerRef?: Ref<HTMLButtonElement>;
}

/**
 * Tabs for Overview | Timeline | Activity within the Scenario workspace.
 * @param root0
 * @param root0.state
 * @param root0.actions
 * @param root0.widgets
 * @param root0.selection
 * @param root0.onSelectionChange
 * @param root0.onRequestMetaModelFocus
 * @param root0.timelineContent
 * @param root0.activityContent
 * @param root0.initialTab
 * @param root0.className
 * @param root0.reloadSignal
 * @param root0.branchTriggerRef
 */
export function OverviewTabs({
  state,
  actions,
  widgets,
  selection,
  onSelectionChange,
  onRequestMetaModelFocus,
  onAddWidget,
  timelineContent,
  activityContent,
  initialTab = 'canvas',
  className,
  reloadSignal,
  branchTriggerRef,
}: OverviewTabsProperties) {
  const copy = templateScreenCopy.tabs;

  return (
    <Tabs defaultValue={initialTab} className={cn('flex min-h-0 flex-col gap-4', className)}>
      <TabsList className="grid grid-cols-4 gap-2 rounded-xl border border-border/70 bg-muted/40 p-1">
        <TabsTrigger value="canvas">Canvas</TabsTrigger>
        <TabsTrigger value="overview">{copy.overview}</TabsTrigger>
        <TabsTrigger value="timeline">{copy.timeline}</TabsTrigger>
        <TabsTrigger value="activity">{copy.activity}</TabsTrigger>
      </TabsList>

      <TabsContent value="canvas" className="min-h-0 flex-1">
        <PraxisCanvasWorkspace
          widgets={widgets}
          selection={selection}
          onSelectionChange={onSelectionChange}
          onRequestMetaModelFocus={onRequestMetaModelFocus}
          onAddWidget={onAddWidget}
          reloadSignal={reloadSignal}
        />
      </TabsContent>

      <TabsContent value="overview" className="min-h-0 flex-1">
        <div className="grid gap-4 lg:grid-cols-2">
          <SnapshotOverviewCard state={state} />
          <TimeCursorCard triggerRef={branchTriggerRef} />
        </div>
      </TabsContent>

      <TabsContent value="timeline" className="min-h-0 flex-1">
        {timelineContent ?? (
          <CommitTimelineList
            commits={state.commits}
            activeCommitId={state.commitId}
            onSelect={actions.selectCommit}
          />
        )}
      </TabsContent>

      <TabsContent value="activity" className="min-h-0 flex-1">
        {activityContent ?? <ActivityTimelinePanel />}
      </TabsContent>
    </Tabs>
  );
}
