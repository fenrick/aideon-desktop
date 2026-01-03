import { useCallback, useEffect, useRef, useState } from 'react';

import { AideonCanvasRuntime, type CanvasRuntimeLayoutPersistence } from 'aideon/canvas/canvas-runtime';
import { fromWidgetSelection } from 'aideon/canvas/selection';
import type { SelectionState, WidgetSelection } from 'aideon/canvas/types';
import type { GraphViewModel } from 'praxis/praxis-api';

import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import { Separator } from 'design-system/components/ui/separator';
import { Skeleton } from 'design-system/components/ui/skeleton';

import type {
  PraxisCanvasWidget,
  PraxisWidgetErrorEvent,
  PraxisWidgetViewEvent,
} from 'praxis/types';
import { CatalogueWidget } from 'praxis/widgets/catalogue-widget';
import { ChartWidget } from 'praxis/widgets/chart-widget';
import { GraphWidget } from 'praxis/widgets/graph-widget';
import { MatrixWidget } from 'praxis/widgets/matrix-widget';
import type { ReactElement } from 'react';

interface RenderPraxisWidgetParameters {
  readonly widget: PraxisCanvasWidget;
  readonly reloadVersion: number;
  readonly selection: SelectionState;
  readonly onSelection: (event: WidgetSelection) => void;
  readonly onGraphViewChange: (event: PraxisWidgetViewEvent) => void;
  readonly onGraphError: (event: PraxisWidgetErrorEvent) => void;
  readonly onRequestMetaModelFocus?: (types: string[]) => void;
}

/**
 * Renders a single Praxis widget into the canvas runtime.
 * @param root0
 * @param root0.widget
 * @param root0.reloadVersion
 * @param root0.selection
 * @param root0.onSelection
 * @param root0.onGraphViewChange
 * @param root0.onGraphError
 * @param root0.onRequestMetaModelFocus
 */
function renderPraxisWidget({
  widget,
  reloadVersion,
  selection,
  onSelection,
  onGraphViewChange,
  onGraphError,
  onRequestMetaModelFocus,
}: RenderPraxisWidgetParameters): ReactElement {
  switch (widget.kind) {
    case 'graph': {
      return (
        <GraphWidget
          widget={widget}
          reloadVersion={reloadVersion}
          selection={selection}
          onSelectionChange={onSelection}
          onViewChange={(view: GraphViewModel) => {
            onGraphViewChange({ widgetId: widget.id, view });
          }}
          onError={(message: string) => {
            onGraphError({ widgetId: widget.id, message });
          }}
          onRequestMetaModelFocus={onRequestMetaModelFocus}
        />
      );
    }
    case 'catalogue': {
      return (
        <CatalogueWidget
          widget={widget}
          reloadVersion={reloadVersion}
          selection={selection}
          onSelectionChange={onSelection}
        />
      );
    }
    case 'chart': {
      return <ChartWidget widget={widget} reloadVersion={reloadVersion} />;
    }
    case 'matrix': {
      return (
        <MatrixWidget
          widget={widget}
          reloadVersion={reloadVersion}
          selection={selection}
          onSelectionChange={onSelection}
        />
      );
    }
  }
}

/**
 * Manages a local reload counter that can be bumped by signals or user action.
 * @param reloadSignal
 */
function useReloadVersion(reloadSignal?: number) {
  const [reloadVersion, setReloadVersion] = useState(0);
  const lastReloadSignal = useRef<number | undefined>(reloadSignal);

  useEffect(() => {
    if (typeof reloadSignal !== 'number' || reloadSignal === lastReloadSignal.current) {
      return;
    }

    if (lastReloadSignal.current !== undefined) {
      queueMicrotask(() => {
        setReloadVersion((value) => value + 1);
      });
    }
    lastReloadSignal.current = reloadSignal;
  }, [reloadSignal]);

  const triggerReload = useCallback(() => {
    setReloadVersion((value) => value + 1);
  }, []);

  return { reloadVersion, triggerReload };
}

export interface PraxisCanvasWorkspaceProperties {
  readonly widgets: PraxisCanvasWidget[];
  readonly canvasLayoutKey?: string;
  readonly canvasLayoutPersistence?: CanvasRuntimeLayoutPersistence<PraxisCanvasWidget>;
  readonly selection: SelectionState;
  readonly onSelectionChange?: (selection: SelectionState) => void;
  readonly onRequestMetaModelFocus?: (types: string[]) => void;
  readonly onAddWidget?: () => void;
  readonly reloadSignal?: number;
  readonly showPageBreaks?: boolean;
  readonly onGraphStatsChange?: (stats: GraphViewModel['stats']) => void;
  readonly onGraphMetadataChange?: (metadata: GraphViewModel['metadata']) => void;
  readonly onGraphErrorMessage?: (message?: string) => void;
  readonly errorMessage?: string;
}

const SUGGESTED_WIDGETS = ['KPI', 'Graph', 'Catalogue snapshot'] as const;

/**
 * Full-height canvas surface with overlay stats and actions.
 * @param root0
 * @param root0.widgets
 * @param root0.selection
 * @param root0.onSelectionChange
 * @param root0.onRequestMetaModelFocus
 * @param root0.onAddWidget
 * @param root0.reloadSignal
 * @param root0.showPageBreaks
 * @param root0.onGraphStatsChange
 * @param root0.onGraphMetadataChange
 * @param root0.onGraphErrorMessage
 * @param root0.errorMessage
 */
export function PraxisCanvasWorkspace({
  widgets,
  canvasLayoutKey,
  canvasLayoutPersistence,
  selection,
  onSelectionChange,
  onRequestMetaModelFocus,
  onAddWidget,
  reloadSignal,
  showPageBreaks,
  onGraphStatsChange,
  onGraphMetadataChange,
  onGraphErrorMessage,
  errorMessage,
}: PraxisCanvasWorkspaceProperties) {
  const { reloadVersion } = useReloadVersion(reloadSignal);
  const [stats, setStats] = useState<GraphViewModel['stats'] | undefined>();
  const [error, setError] = useState<string | undefined>();
  const localShowPageBreaks = showPageBreaks ?? false;

  const activeError = errorMessage ?? error;

  const handleGraphViewChange = useCallback(
    (event: PraxisWidgetViewEvent) => {
      setStats(event.view.stats);
      setError(undefined);
      onGraphMetadataChange?.(event.view.metadata);
      onGraphStatsChange?.(event.view.stats);
      onGraphErrorMessage?.();
    },
    [onGraphErrorMessage, onGraphMetadataChange, onGraphStatsChange],
  );

  const handleGraphError = useCallback(
    (event: PraxisWidgetErrorEvent) => {
      setError(event.message);
      onGraphErrorMessage?.(event.message);
    },
    [onGraphErrorMessage],
  );

  const handleSelection = useCallback(
    (event: WidgetSelection) => {
      onSelectionChange?.(fromWidgetSelection(event));
    },
    [onSelectionChange],
  );

  const emptyCanvas = widgets.length === 0;
  const showLoadingPlaceholder = emptyCanvas && !stats && !error;
  const handleAddWidget = useCallback(() => {
    onAddWidget?.();
  }, [onAddWidget]);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      {activeError ? <p className="text-sm text-destructive">{activeError}</p> : undefined}

      <Card className="flex-1 overflow-hidden relative rounded-2xl border border-dashed border-border/60 bg-muted/30">
        {showLoadingPlaceholder && (
          <div className="absolute inset-0">
            <Skeleton className="h-full w-full" />
          </div>
        )}
        {emptyCanvas && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center px-4">
            <Card className="pointer-events-auto w-full max-w-xl space-y-4 rounded-2xl shadow-xl">
              <CardHeader className="space-y-2 p-4">
                <CardTitle className="text-lg font-semibold text-foreground">
                  Nothing on this page yet
                </CardTitle>
                <CardDescription>
                  Add a widget to start building your executive overview.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4 p-4">
                <p className="text-sm text-muted-foreground">
                  Choose a widget type or browse templates to jump-start the storyboard.
                </p>
                <div className="flex flex-wrap gap-2">
                  {SUGGESTED_WIDGETS.map((widgetLabel) => (
                    <Badge
                      key={widgetLabel}
                      variant="outline"
                      className="px-3 py-1 text-xs uppercase tracking-wide text-muted-foreground"
                    >
                      {widgetLabel}
                    </Badge>
                  ))}
                </div>
              </CardContent>
              <Separator />
              <CardFooter className="flex flex-wrap gap-4 p-4">
                <Button variant="default" size="sm" onClick={handleAddWidget}>
                  Add widget
                </Button>
                <Button variant="secondary" size="sm">
                  Browse templates
                </Button>
              </CardFooter>
            </Card>
          </div>
        )}
        <AideonCanvasRuntime<PraxisCanvasWidget>
          widgets={widgets}
          showPageBreaks={localShowPageBreaks}
          layoutKey={canvasLayoutKey}
          layoutPersistence={canvasLayoutPersistence}
          renderWidget={(widget: PraxisCanvasWidget) =>
            renderPraxisWidget({
              widget,
              reloadVersion,
              selection,
              onSelection: handleSelection,
              onGraphViewChange: handleGraphViewChange,
              onGraphError: handleGraphError,
              onRequestMetaModelFocus,
            })
          }
        />
      </Card>
    </div>
  );
}
