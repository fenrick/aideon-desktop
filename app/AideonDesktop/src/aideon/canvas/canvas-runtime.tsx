import { Button } from 'design-system/components/ui/button';
import { cn } from 'design-system/lib/utilities';
import { Maximize, MousePointer2, ZoomIn, ZoomOut } from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { DraggableWidgetWrapper } from './draggable-widget-wrapper';
import { useInfiniteCanvas } from './hooks/use-infinite-canvas';
import { calculateInitialLayout } from './layout-engine';
import type { CanvasWidgetLayout } from './types';
import { WidgetFrame } from './widget-frame';

export interface WidgetPosition {
  x: number;
  y: number;
}

export interface WidgetSize {
  w: number;
  h: number;
}

export interface CanvasRuntimeLayoutSnapshot {
  readonly positions: Record<string, WidgetPosition>;
  readonly sizes: Record<string, WidgetSize>;
}

export interface CanvasRuntimeLayoutPersistence<TWidget extends CanvasWidgetLayout> {
  readonly load: (widgets: readonly TWidget[]) => Promise<CanvasRuntimeLayoutSnapshot | undefined>;
  readonly save: (
    widgets: readonly TWidget[],
    snapshot: CanvasRuntimeLayoutSnapshot,
  ) => Promise<void>;
}

interface AideonCanvasRuntimeProperties<TWidget extends CanvasWidgetLayout> {
  readonly widgets: TWidget[];
  readonly renderWidget: (widget: TWidget) => React.ReactNode;
  readonly className?: string;
  readonly showPageBreaks?: boolean;
  readonly layoutKey?: string;
  readonly layoutPersistence?: CanvasRuntimeLayoutPersistence<TWidget>;
}

/**
 * Returns default widget dimensions based on layout size.
 * @param widget
 */
function getWidgetDefaultSize(widget: CanvasWidgetLayout): WidgetSize {
  const isHalf = widget.size === 'half';
  return { w: isHalf ? 650 : 1300, h: isHalf ? 550 : 850 };
}

/**
 * Ensures widget positions exist for the current widget set.
 * @param previous
 * @param widgets
 */
function syncWidgetPositions(
  previous: Record<string, WidgetPosition>,
  widgets: CanvasWidgetLayout[],
): Record<string, WidgetPosition> {
  const existing = { ...previous };
  const missing = widgets.filter((widget) => existing[widget.id] === undefined);
  if (missing.length === 0) {
    return existing;
  }
  return { ...existing, ...calculateInitialLayout(missing) };
}

/**
 * Merges sizes from storage into the current widget size state.
 * @param previous
 */
function syncWidgetSizes(
  previous: Record<string, WidgetSize>,
  widgets: CanvasWidgetLayout[],
): Record<string, WidgetSize> {
  let didChange = false;
  const next: Record<string, WidgetSize> = { ...previous };

  for (const widget of widgets) {
    if (next[widget.id] === undefined) {
      didChange = true;
      next[widget.id] = getWidgetDefaultSize(widget);
    }
  }

  return didChange ? next : previous;
}

/**
 * Lay out and render multiple canvas widgets on an infinite surface.
 * @param root0
 * @param root0.widgets
 * @param root0.renderWidget
 * @param root0.className
 * @param root0.showPageBreaks
 * @returns Canvas runtime surface.
 */
function AideonCanvasRuntimeImpl<TWidget extends CanvasWidgetLayout>({
  widgets,
  renderWidget,
  className,
  showPageBreaks,
  layoutKey,
  layoutPersistence,
}: AideonCanvasRuntimeProperties<TWidget>) {
  const { viewport, setViewport, containerReference, events } = useInfiniteCanvas({
    minScale: 0.1,
    maxScale: 3,
    initialScale: 0.8,
  });

  const [widgetPositions, setWidgetPositions] = useState<Record<string, WidgetPosition>>({});
  const [widgetSizes, setWidgetSizes] = useState<Record<string, WidgetSize>>({});
  const [layoutHydrated, setLayoutHydrated] = useState(() => !layoutPersistence || !layoutKey);

  const widgetPositionsReference = useRef(widgetPositions);
  const widgetSizesReference = useRef(widgetSizes);
  const layoutLoadCancelledReference = useRef(false);

  useEffect(() => {
    widgetPositionsReference.current = widgetPositions;
  }, [widgetPositions]);

  useEffect(() => {
    widgetSizesReference.current = widgetSizes;
  }, [widgetSizes]);

  const persistSnapshot = useCallback(
    (positions: Record<string, WidgetPosition>, sizes: Record<string, WidgetSize>) => {
      if (!layoutHydrated) {
        return;
      }
      if (!layoutPersistence || !layoutKey) {
        return;
      }
      void layoutPersistence.save(widgets, { positions, sizes });
    },
    [layoutHydrated, layoutKey, layoutPersistence, widgets],
  );

  useEffect(() => {
    if (!layoutPersistence || !layoutKey) {
      setLayoutHydrated(true);
      return;
    }
    layoutLoadCancelledReference.current = false;
    setLayoutHydrated(false);
    void (async () => {
      try {
        const snapshot = await layoutPersistence.load(widgets);
        if (layoutLoadCancelledReference.current) {
          return;
        }
        const positions = snapshot?.positions ?? {};
        const sizes = snapshot?.sizes ?? {};
        widgetPositionsReference.current = positions;
        widgetSizesReference.current = sizes;
        setWidgetPositions(positions);
        setWidgetSizes(sizes);
      } catch {
        // ignore (layout persistence unavailable)
      } finally {
        if (!layoutLoadCancelledReference.current) {
          setLayoutHydrated(true);
        }
      }
    })();

    return () => {
      layoutLoadCancelledReference.current = true;
    };
  }, [layoutKey, layoutPersistence, widgets]);

  useEffect(() => {
    if (!layoutHydrated) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      const currentPositions = widgetPositionsReference.current;
      const currentSizes = widgetSizesReference.current;
      const nextPositions = syncWidgetPositions(currentPositions, widgets);
      const nextSizes = syncWidgetSizes(currentSizes, widgets);

      const positionsChanged = nextPositions !== currentPositions;
      const sizesChanged = nextSizes !== currentSizes;

      if (positionsChanged) {
        widgetPositionsReference.current = nextPositions;
        setWidgetPositions(nextPositions);
      }
      if (sizesChanged) {
        widgetSizesReference.current = nextSizes;
        setWidgetSizes(nextSizes);
      }
      if (positionsChanged || sizesChanged) {
        persistSnapshot(nextPositions, nextSizes);
      }
    });

    return () => {
      cancelAnimationFrame(frame);
    };
  }, [layoutHydrated, persistSnapshot, widgets]);

  const handleDragEnd = useCallback((id: string, x: number, y: number) => {
    setWidgetPositions((previous) => {
      const next = { ...previous, [id]: { x, y } };
      widgetPositionsReference.current = next;
      persistSnapshot(next, widgetSizesReference.current);
      return next;
    });
  }, [persistSnapshot]);

  const handleResizeEnd = useCallback((id: string, w: number, h: number) => {
    setWidgetSizes((previous) => {
      const next = { ...previous, [id]: { w, h } };
      widgetSizesReference.current = next;
      persistSnapshot(widgetPositionsReference.current, next);
      return next;
    });
  }, [persistSnapshot]);

  const viewportTransform = useMemo(() => {
    const x = viewport.x.toString();
    const y = viewport.y.toString();
    const scale = viewport.scale.toString();
    return `translate(${x}px, ${y}px) scale(${scale})`;
  }, [viewport.scale, viewport.x, viewport.y]);

  const handleZoomIn = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      setViewport((previous) => ({
        ...previous,
        scale: Math.min(previous.scale + 0.1, 3),
      }));
    },
    [setViewport],
  );

  const handleZoomOut = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      setViewport((previous) => ({
        ...previous,
        scale: Math.max(previous.scale - 0.1, 0.1),
      }));
    },
    [setViewport],
  );

  const handleResetView = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      setViewport({ x: 0, y: 0, scale: 0.8 });
    },
    [setViewport],
  );

  return (
    <div
      ref={containerReference}
      className={cn(
        'relative h-full w-full overflow-hidden bg-background bg-dot-pattern cursor-grab active:cursor-grabbing group/canvas',
        className,
      )}
      {...events}
    >
      {/* Viewport Container */}
      <div
        style={{
          transform: viewportTransform,
          transformOrigin: '0 0',
          width: '100%',
          height: '100%',
          position: 'absolute',
          top: 0,
          left: 0,
        }}
      >
        {/* Page Breaks Overlay */}
        {showPageBreaks ? (
          <div
            className="pointer-events-none absolute -top-[10000px] -left-[10000px] w-[20000px] h-[20000px] z-0 opacity-20"
            style={{
              backgroundImage:
                'linear-gradient(to right, currentColor 1px, transparent 1px), linear-gradient(to bottom, currentColor 1px, transparent 1px)',
              // Approx A4 @ 96 DPI.
              backgroundSize: '794px 1123px',
              color: 'var(--border)',
            }}
          />
        ) : undefined}

        {/* Widgets */}
        {widgets.map((widget) => {
          const position = widgetPositions[widget.id] ?? { x: 0, y: 0 };
          const size = widgetSizes[widget.id] ?? getWidgetDefaultSize(widget);

          return (
            <DraggableWidgetWrapper
              key={widget.id}
              id={widget.id}
              x={position.x}
              y={position.y}
              width={size.w}
              height={size.h}
              scale={viewport.scale}
              onDragEnd={handleDragEnd}
              onResizeEnd={handleResizeEnd}
            >
              <WidgetFrame title={widget.title ?? widget.id}>{renderWidget(widget)}</WidgetFrame>
            </DraggableWidgetWrapper>
          );
        })}
      </div>

      {/* Floating Canvas Controls */}
      <div className="absolute bottom-6 right-6 flex flex-col gap-2 rounded-lg border border-border/50 bg-background/80 p-1.5 shadow-lg backdrop-blur-md transition-opacity hover:opacity-100 opacity-60">
        <Button variant="ghost" size="icon-sm" onClick={handleZoomIn} title="Zoom In">
          <ZoomIn className="h-4 w-4" />
        </Button>
        <div className="text-center text-[10px] font-mono text-muted-foreground select-none">
          {Math.round(viewport.scale * 100)}%
        </div>
        <Button variant="ghost" size="icon-sm" onClick={handleZoomOut} title="Zoom Out">
          <ZoomOut className="h-4 w-4" />
        </Button>
        <div className="h-px bg-border/50 my-0.5" />
        <Button variant="ghost" size="icon-sm" onClick={handleResetView} title="Reset View">
          <Maximize className="h-4 w-4" />
        </Button>
      </div>

      {/* Hint */}
      <div className="absolute bottom-6 left-6 pointer-events-none opacity-0 group-hover/canvas:opacity-40 transition-opacity">
        <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground bg-background/50 px-2 py-1 rounded-md backdrop-blur-sm">
          <MousePointer2 className="h-3 w-3" />
          <span>Middle Click or Shift+Drag to Pan · Scroll to Zoom</span>
        </div>
      </div>
    </div>
  );
}

export const AideonCanvasRuntime = memo(AideonCanvasRuntimeImpl) as typeof AideonCanvasRuntimeImpl;
