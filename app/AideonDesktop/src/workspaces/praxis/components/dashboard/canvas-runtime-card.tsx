import { useCallback, useEffect, useRef, useState } from 'react';

import { AideonCanvasRuntime } from 'aideon/canvas/canvas-runtime';
import { fromWidgetSelection } from 'aideon/canvas/selection';
import type { SelectionState, WidgetSelection } from 'aideon/canvas/types';
import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import type { GraphViewModel } from 'praxis/praxis-api';
import type {
  PraxisCanvasWidget,
  PraxisWidgetErrorEvent,
  PraxisWidgetViewEvent,
} from 'praxis/types';
import { CatalogueWidget } from 'praxis/widgets/catalogue-widget';
import { ChartWidget } from 'praxis/widgets/chart-widget';
import { GraphWidget } from 'praxis/widgets/graph-widget';
import { MatrixWidget } from 'praxis/widgets/matrix-widget';

interface CanvasRuntimeCardProperties {
  readonly widgets: PraxisCanvasWidget[];
  readonly selection: SelectionState;
  readonly onSelectionChange?: (selection: SelectionState) => void;
  readonly onRequestMetaModelFocus?: (types: string[]) => void;
  readonly reloadSignal?: number;
}

/**
 * Dashboard card that embeds the canvas runtime surface.
 * @param root0 - Card properties.
 * @param root0.widgets - Widgets to render.
 * @param root0.selection - Current selection shared across widgets.
 * @param root0.onSelectionChange - Selection handler.
 * @param root0.onRequestMetaModelFocus - Callback for meta-model focus requests.
 * @param root0.reloadSignal
 * @returns Card element wrapping the runtime.
 */
export function CanvasRuntimeCard({
  widgets,
  selection,
  onSelectionChange,
  onRequestMetaModelFocus,
  reloadSignal,
}: CanvasRuntimeCardProperties) {
  const [reloadVersion, setReloadVersion] = useState(0);
  const [metadata, setMetadata] = useState<GraphViewModel['metadata'] | undefined>();
  const [stats, setStats] = useState<GraphViewModel['stats'] | undefined>();
  const [error, setError] = useState<string | undefined>();
  const lastReloadSignal = useRef<number | undefined>(reloadSignal);

  useEffect(() => {
    if (typeof reloadSignal === 'number' && reloadSignal !== lastReloadSignal.current) {
      if (lastReloadSignal.current !== undefined) {
        queueMicrotask(() => {
          setReloadVersion((value) => value + 1);
        });
      }
      lastReloadSignal.current = reloadSignal;
    }
  }, [reloadSignal]);

  const handleGraphViewChange = useCallback((event: PraxisWidgetViewEvent) => {
    setMetadata(event.view.metadata);
    setStats(event.view.stats);
    setError(undefined);
  }, []);

  const handleGraphError = useCallback((event: PraxisWidgetErrorEvent) => {
    setError(event.message);
  }, []);

  const handleSelection = useCallback(
    (event: WidgetSelection) => {
      onSelectionChange?.(fromWidgetSelection(event));
    },
    [onSelectionChange],
  );

  return (
    <Card className="h-full">
      <CardHeader className="flex flex-row items-start justify-between gap-3">
        <div>
          <CardTitle>Canvas runtime</CardTitle>
          <CardDescription>
            {metadata
              ? `As of ${new Date(metadata.asOf).toLocaleString()} (${metadata.scenario ?? 'main'})`
              : 'React Flow GraphWidget powered by praxisApi'}
          </CardDescription>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => {
            setReloadVersion((value) => value + 1);
          }}
          disabled={widgets.length === 0}
        >
          Refresh graph
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <StatTile label="Nodes" value={stats?.nodes} />
          <StatTile label="Edges" value={stats?.edges} />
        </div>
        {error ? <p className="text-sm text-destructive">{error}</p> : undefined}
        <div className="h-[380px] w-full">
          <AideonCanvasRuntime
            widgets={widgets}
            renderWidget={(widget) => {
              if (widget.kind === 'graph') {
                return (
                  <GraphWidget
                    widget={widget}
                    reloadVersion={reloadVersion}
                    selection={selection}
                    onSelectionChange={handleSelection}
                    onViewChange={(view: GraphViewModel) => {
                      handleGraphViewChange({ widgetId: widget.id, view });
                    }}
                    onError={(message: string) => {
                      handleGraphError({ widgetId: widget.id, message });
                    }}
                    onRequestMetaModelFocus={onRequestMetaModelFocus}
                  />
                );
              }
              if (widget.kind === 'catalogue') {
                return (
                  <CatalogueWidget
                    widget={widget}
                    reloadVersion={reloadVersion}
                    selection={selection}
                    onSelectionChange={handleSelection}
                  />
                );
              }
              if (widget.kind === 'chart') {
                return <ChartWidget widget={widget} reloadVersion={reloadVersion} />;
              }
              return (
                <MatrixWidget
                  widget={widget}
                  reloadVersion={reloadVersion}
                  selection={selection}
                  onSelectionChange={handleSelection}
                />
              );
            }}
          />
        </div>
      </CardContent>
    </Card>
  );
}

interface StatTileProperties {
  readonly label: string;
  readonly value?: number;
}

/**
 *
 * @param root0
 * @param root0.label
 * @param root0.value
 */
function StatTile({ label, value }: StatTileProperties) {
  return (
    <div className="rounded-2xl border border-border/70 bg-card px-4 py-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-3xl font-semibold tracking-tight">
        {typeof value === 'number' ? value : '—'}
      </p>
    </div>
  );
}
