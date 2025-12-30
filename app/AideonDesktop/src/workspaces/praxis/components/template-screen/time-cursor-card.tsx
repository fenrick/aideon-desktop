import { useMemo, type Ref } from 'react';

import { templateScreenCopy } from 'praxis/copy/template-screen';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';
import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import { Label } from 'design-system/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from 'design-system/components/ui/select';
import { Slider } from 'design-system/components/ui/slider';

interface TimeCursorCardProperties {
  readonly state?: TemporalPanelState;
  readonly actions?: TemporalPanelActions;
  readonly triggerRef?: Ref<HTMLButtonElement>;
}

/**
 * Timeline + moment selector with a timeline slider, wrapped in a card.
 * @param root0
 * @param root0.state
 * @param root0.actions
 * @param root0.triggerRef
 */
export function TimeCursorCard({ state, actions, triggerRef }: TimeCursorCardProperties) {
  const [hookState, hookActions] = useTemporalPanel();
  const viewState = state ?? hookState;
  const viewActions = actions ?? hookActions;
  const copy = templateScreenCopy.timeCursor;

  const branchOptions = useMemo(
    () => viewState.branches.map((branch) => branch.name),
    [viewState.branches],
  );
  const sliderValue = resolveSliderValue(viewState.commitId, viewState.commits);
  const sliderMax = Math.max(viewState.commits.length - 1, 0);
  const selectedCommit =
    viewState.commits.find((commit) => commit.id === viewState.commitId) ??
    viewState.commits.at(-1);

  return (
    <Card>
      <CardHeader className="space-y-2 p-4">
        <CardTitle>{copy.title}</CardTitle>
        <CardDescription>{copy.description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 p-4">
        <div className="space-y-2">
          <Label htmlFor="branch-select">{copy.branchLabel}</Label>
          <Select
            data-testid="branch-select"
            value={viewState.branch ?? undefined}
            disabled={viewState.loading || branchOptions.length === 0}
            onValueChange={(value: string) => {
              void viewActions.selectBranch(value);
            }}
          >
            <SelectTrigger
              id="branch-select"
              data-testid="branch-select"
              aria-label={copy.branchLabel}
              ref={triggerRef}
            >
              <SelectValue placeholder="Select timeline" />
            </SelectTrigger>
            <SelectContent>
              {branchOptions.map((option) => (
                <SelectItem key={option} value={option}>
                  {option}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="commit-select">{copy.commitLabel}</Label>
          <Select
            data-testid="commit-select"
            value={viewState.commitId ?? undefined}
            disabled={viewState.loading || viewState.commits.length === 0}
            onValueChange={(value: string) => {
              viewActions.selectCommit(value);
            }}
          >
            <SelectTrigger
              id="commit-select"
              data-testid="commit-select"
              aria-label={copy.commitLabel}
            >
              <SelectValue placeholder="Select moment" />
            </SelectTrigger>
            <SelectContent>
              {viewState.commits.map((commit) => (
                <SelectItem key={commit.id} value={commit.id}>
                  {commit.message}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {selectedCommit && (
            <p className="text-xs text-muted-foreground">{selectedCommit.message}</p>
          )}
        </div>
        <div className="space-y-2">
          <Label>{copy.sliderLabel}</Label>
          <Slider
            min={0}
            max={sliderMax}
            step={1}
            value={sliderValue}
            data-testid="timeline-slider"
            disabled={viewState.loading || viewState.commits.length === 0}
            onValueCommit={(values: number[]) => {
              const [position] = values;
              if (typeof position !== 'number' || position < 0) {
                return;
              }
              const nextCommit = viewState.commits.find((_, index) => index === position);
              if (nextCommit) {
                viewActions.selectCommit(nextCommit.id);
              }
            }}
          />
          <p className="text-xs text-muted-foreground">
            {copy.snapshotLabel}:{' '}
            {selectedCommit?.time
              ? new Date(selectedCommit.time).toLocaleString()
              : (selectedCommit?.id ?? 'Latest')}
          </p>
        </div>
        <div className="flex flex-wrap gap-4">
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              void viewActions.refreshBranches();
            }}
            disabled={viewState.loading}
          >
            {copy.refresh}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              viewActions.selectCommit(viewState.commitId);
            }}
            disabled={viewState.snapshotLoading || !viewState.commitId}
          >
            {copy.reload}
          </Button>
          {viewState.branch && viewState.branch !== 'main' ? (
            <Button
              size="sm"
              onClick={() => {
                void viewActions.mergeIntoMain();
              }}
              disabled={viewState.merging}
            >
              {viewState.merging ? 'Applying…' : copy.merge}
            </Button>
          ) : undefined}
        </div>
        {viewState.error && <p className="text-xs text-destructive">{viewState.error}</p>}
      </CardContent>
    </Card>
  );
}

/**
 *
 * @param commitId
 * @param commits
 */
function resolveSliderValue(
  commitId: string | undefined,
  commits: TemporalPanelState['commits'],
): number[] {
  if (commits.length === 0) {
    return [0];
  }
  if (!commitId) {
    return [commits.length - 1];
  }
  const index = commits.findIndex((commit) => commit.id === commitId);
  return [index === -1 ? commits.length - 1 : index];
}
