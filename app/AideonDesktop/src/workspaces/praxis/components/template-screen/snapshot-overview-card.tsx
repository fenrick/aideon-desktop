import { templateScreenCopy } from 'praxis/copy/template-screen';
import type { TemporalPanelState } from 'praxis/time/use-temporal-panel';

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';

interface SnapshotOverviewCardProperties {
  readonly state: TemporalPanelState;
}

/**
 * Read-only snapshot metrics for the Overview tab.
 * @param root0
 * @param root0.state
 */
export function SnapshotOverviewCard({ state }: SnapshotOverviewCardProperties) {
  const snapshot = state.snapshot;
  const metrics = [
    { label: 'Nodes', value: snapshot?.nodes },
    { label: 'Edges', value: snapshot?.edges },
    {
      label: 'Confidence',
      value:
        typeof snapshot?.confidence === 'number'
          ? `${Math.round(snapshot.confidence * 100).toString()}%`
          : 'Not set',
    },
    { label: 'Scenario', value: snapshot?.scenario ?? 'Production' },
  ];
  const copy = templateScreenCopy.overview;

  return (
    <Card>
      <CardHeader className="space-y-2 p-4">
        <CardTitle>{copy.snapshotTitle}</CardTitle>
        <CardDescription>{copy.snapshotDescription}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 p-4">
        <div className="grid grid-cols-2 gap-4">
          {metrics.map((metric) => (
            <div
              key={metric.label}
              className="rounded-2xl border border-border/70 bg-muted/20 p-3"
              aria-label={`${metric.label} metric`}
            >
              <p className="text-xs text-muted-foreground">{metric.label}</p>
              <p className="text-2xl font-semibold">{formatMetric(metric.value)}</p>
            </div>
          ))}
        </div>
        {state.error ? <p className="text-xs text-destructive">{state.error}</p> : undefined}
      </CardContent>
    </Card>
  );
}

/**
 *
 * @param value
 */
function formatMetric(value: unknown): string {
  if (typeof value === 'number') {
    return value.toLocaleString();
  }
  if (typeof value === 'string') {
    return value;
  }
  return '—';
}
