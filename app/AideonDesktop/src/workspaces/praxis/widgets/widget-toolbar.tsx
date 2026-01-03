import { Loader2, RefreshCcw } from 'lucide-react';

import { Button } from 'design-system/components/ui/button';
import type { ViewMetadata } from 'praxis/praxis-api';

interface WidgetToolbarProperties {
  readonly metadata?: ViewMetadata;
  readonly fallbackTitle: string;
  readonly loading: boolean;
  readonly onRefresh: () => void;
}

/**
 * Standard toolbar for widgets showing title, timestamp, and refresh.
 * @param root0 - Toolbar properties.
 * @param root0.metadata - Optional view metadata to display.
 * @param root0.fallbackTitle - Title when metadata is missing.
 * @param root0.loading - Whether a refresh is in progress.
 * @param root0.onRefresh - Refresh handler.
 * @returns Toolbar element.
 */
export function WidgetToolbar({
  metadata,
  fallbackTitle,
  loading,
  onRefresh,
}: WidgetToolbarProperties) {
  const title = metadata?.name ?? fallbackTitle;
  const subtitle = metadata ? `As of ${formatAsOf(metadata.asOf)}` : 'Awaiting twin data';

  return (
    <div className="mb-3 flex items-center justify-between">
      <div>
        <p className="text-xs uppercase tracking-[0.35em] text-muted-foreground">Widget</p>
        <p className="text-sm font-medium text-foreground">{title}</p>
        <p className="text-xs text-muted-foreground">{subtitle}</p>
      </div>
      <Button variant="secondary" size="sm" disabled={loading} onClick={onRefresh}>
        {loading ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <RefreshCcw className="mr-2 h-4 w-4" />
        )}
        Refresh
      </Button>
    </div>
  );
}

/**
 * Format an "as-of" identifier for display. Accepts timestamps or commit-like identifiers.
 * @param asOf - Timestamp or commit identifier.
 */
function formatAsOf(asOf: string): string {
  const parsed = new Date(asOf);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed.toLocaleString();
  }
  if (asOf.length > 16) {
    return `${asOf.slice(0, 8)}…`;
  }
  return asOf;
}
