import { memo } from 'react';

import type { SelectionState } from 'aideon/canvas/types';
import { recentAnalytics } from 'praxis/lib/analytics';

interface DebugOverlayProperties {
  readonly scenarioName?: string;
  readonly templateName?: string;
  readonly selection?: SelectionState;
  readonly commitId?: string;
  readonly branch?: string;
  readonly visible: boolean;
}

export const DebugOverlay = memo(function DebugOverlay({
  scenarioName,
  templateName,
  selection,
  commitId,
  branch,
  visible,
}: DebugOverlayProperties) {
  if (!visible) {
    return;
  }
  const events = recentAnalytics().slice(0, 6);

  return (
    <div className="fixed bottom-4 right-4 z-50 w-[340px] rounded-xl border border-border/60 bg-background/95 p-3 text-xs shadow-lg">
      <div className="mb-2 flex items-center justify-between font-semibold">
        <span>Debug overlay</span>
        <span className="text-muted-foreground">dev-only</span>
      </div>
      <dl className="space-y-1">
        <DebugRow label="Scenario" value={scenarioName ?? '—'} />
        <DebugRow label="Template" value={templateName ?? '—'} />
        <DebugRow label="Timeline" value={branch ?? '—'} />
        <DebugRow label="Moment" value={commitId ?? '—'} />
        <DebugRow label="Selection" value={formatSelection(selection)} />
      </dl>
      <div className="mt-3 space-y-1">
        <p className="font-semibold">Recent analytics</p>
        {events.length === 0 ? (
          <p className="text-muted-foreground">No events yet.</p>
        ) : (
          <ul className="divide-y divide-border/60">
            {events.map((event) => (
              <li key={`${event.event}-${String(event.at)}`} className="py-1">
                <div className="flex items-center justify-between">
                  <span className="font-medium">{event.event}</span>
                  <span className="text-[10px] text-muted-foreground">
                    {new Date(event.at).toLocaleTimeString()}
                  </span>
                </div>
                <pre className="mt-1 overflow-x-auto rounded bg-muted/30 p-1 text-[10px] leading-tight text-muted-foreground">
                  {JSON.stringify(event.payload ?? {}, undefined, 2)}
                </pre>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
});

/**
 *
 * @param root0
 * @param root0.label
 * @param root0.value
 */
function DebugRow({ label, value }: { readonly label: string; readonly value: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-semibold">{value}</span>
    </div>
  );
}

/**
 *
 * @param selection
 */
function formatSelection(selection?: SelectionState): string {
  if (!selection) {
    return 'none';
  }
  if (selection.nodeIds.length > 0) {
    return `${selection.nodeIds.length.toString()} node(s)`;
  }
  if (selection.edgeIds.length > 0) {
    return `${selection.edgeIds.length.toString()} edge(s)`;
  }
  return selection.sourceWidgetId ? `widget ${selection.sourceWidgetId}` : 'none';
}
