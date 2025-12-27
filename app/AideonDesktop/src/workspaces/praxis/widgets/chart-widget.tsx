import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';

import { AlertTriangle, ArrowDownRight, ArrowUpRight } from 'lucide-react';

import { toErrorMessage } from 'praxis/lib/errors';
import { getChartView, type ChartPoint, type ChartViewModel } from 'praxis/praxis-api';

import type { PraxisChartWidgetConfig as ChartWidgetConfig } from 'praxis/types';
import { WidgetToolbar } from './widget-toolbar';

interface ChartWidgetProperties {
  readonly widget: ChartWidgetConfig;
  readonly reloadVersion: number;
}

/**
 *
 * @param root0
 * @param root0.widget
 * @param root0.reloadVersion
 */
export function ChartWidget({ widget, reloadVersion }: ChartWidgetProperties) {
  const [model, setModel] = useState<ChartViewModel | undefined>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();

  const definition = useMemo(() => {
    return { ...widget.view, asOf: new Date().toISOString() };
  }, [widget.view]);

  const loadView = useCallback(async () => {
    setLoading(true);
    setError(undefined);
    try {
      const view = await getChartView(definition);
      setModel(view);
    } catch (unknownError) {
      const message = toErrorMessage(unknownError);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [definition]);

  useEffect(() => {
    loadView().catch((_ignoredError: unknown) => {
      return;
    });
  }, [loadView, reloadVersion]);

  let body: ReactNode = <p className="text-sm text-muted-foreground">Loading chart…</p>;
  if (error) {
    body = <ChartError message={error} />;
  } else if (model) {
    body = <ChartContent model={model} />;
  }

  return (
    <div className="flex h-full flex-col">
      <WidgetToolbar
        metadata={model?.metadata}
        fallbackTitle={widget.title}
        loading={loading}
        onRefresh={() => {
          loadView().catch((_ignoredError: unknown) => {
            return;
          });
        }}
      />
      <div className="flex-1 rounded-2xl border border-border/60 bg-background/40 p-4">{body}</div>
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.model
 */
function ChartContent({ model }: { readonly model: ChartViewModel }) {
  if (model.chartType === 'kpi' && model.kpi) {
    return <KpiPanel summary={model.kpi} />;
  }
  if (model.chartType === 'line') {
    return <LineChart series={model.series[0]} />;
  }
  return <BarChart series={model.series} />;
}

/**
 *
 * @param root0
 * @param root0.message
 */
function ChartError({ message }: { readonly message: string }) {
  return (
    <p className="flex items-center gap-2 text-sm text-destructive">
      <AlertTriangle className="h-4 w-4" />
      {message}
    </p>
  );
}

/**
 *
 * @param root0
 * @param root0.summary
 */
function KpiPanel({ summary }: { readonly summary: ChartViewModel['kpi'] }) {
  if (!summary) {
    return;
  }
  const trendUp = summary.trend !== 'down';
  const TrendIcon = trendUp ? ArrowUpRight : ArrowDownRight;
  const trendClass = trendUp ? 'text-primary' : 'text-destructive';
  return (
    <div className="flex h-full flex-col justify-between">
      <div>
        <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">KPI</p>
        <p className="text-5xl font-semibold">
          {summary.value.toLocaleString()} {summary.units ?? ''}
        </p>
      </div>
      {typeof summary.delta === 'number' ? (
        <p className={`flex items-center text-sm font-medium ${trendClass}`}>
          <TrendIcon className="mr-1 h-4 w-4" />
          {Math.abs(summary.delta)} vs last span
        </p>
      ) : undefined}
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.series
 */
function LineChart({ series }: { readonly series?: ChartViewModel['series'][number] }) {
  if (!series) {
    return <p className="text-sm text-muted-foreground">No data</p>;
  }
  const resolvedSeries = series;
  const viewBox = { width: 260, height: 140, padding: 16 };
  const points = normalisePoints(resolvedSeries.points, viewBox);
  const path = points.map((point) => `${point.x.toFixed(2)},${point.y.toFixed(2)}`).join(' ');
  return (
    <div className="flex h-full flex-col justify-between">
      <p className="text-sm font-semibold text-foreground">{resolvedSeries.label}</p>
      <svg
        viewBox={`0 0 ${viewBox.width.toString()} ${viewBox.height.toString()}`}
        className="h-32 w-full"
        aria-label={`${resolvedSeries.label} trend`}
      >
        <polyline
          fill="none"
          stroke={resolvedSeries.color ?? 'var(--primary)'}
          strokeWidth={3}
          points={path}
        />
      </svg>
      <div className="flex flex-wrap gap-3 text-xs text-muted-foreground">
        {resolvedSeries.points.map((point) => (
          <span key={point.label}>
            {point.label}:{point.value.toFixed(0)}
          </span>
        ))}
      </div>
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.series
 */
function BarChart({ series }: { readonly series: ChartViewModel['series'] }) {
  if (series.length === 0) {
    return <p className="text-sm text-muted-foreground">No data</p>;
  }
  const primary = series[0];
  if (!primary) {
    return <p className="text-sm text-muted-foreground">No data</p>;
  }
  const categories = primary.points.map((point) => point.label);
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-4 text-xs text-muted-foreground">
        {series.map((entry) => (
          <span key={entry.id} className="flex items-center gap-1">
            <span
              className="h-2 w-4 rounded-sm"
              style={{ backgroundColor: entry.color ?? 'var(--primary)' }}
            />
            {entry.label}
          </span>
        ))}
      </div>
      <div className="mt-4 flex flex-1 gap-4">
        {categories.map((category) => (
          <div key={category} className="flex w-full flex-col items-center gap-2">
            <div className="flex w-full items-end gap-1">
              {series.map((entry) => {
                const point = entry.points.find((item) => item.label === category);
                const heightPercent = point ? Math.min(100, Math.max(5, point.value)) : 5;
                return (
                  <span
                    key={entry.id}
                    className="flex-1 rounded-full"
                    style={{
                      height: `${heightPercent.toString()}%`,
                      backgroundColor: entry.color ?? 'var(--primary)',
                    }}
                  />
                );
              })}
            </div>
            <p className="text-xs text-muted-foreground">{category}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 *
 * @param points
 * @param viewBox
 * @param viewBox.width
 * @param viewBox.height
 * @param viewBox.padding
 */
function normalisePoints(
  points: ChartPoint[],
  viewBox: { width: number; height: number; padding: number },
): { x: number; y: number }[] {
  if (points.length === 0) {
    return [];
  }
  const values = points.map((point) => point.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const spread = max - min || 1;
  const availableWidth = viewBox.width - viewBox.padding * 2;
  const divisor = Math.max(points.length - 1, 1);
  return points.map((point, index) => {
    const normalized = (point.value - min) / spread;
    const x = viewBox.padding + (index / divisor) * availableWidth;
    const y =
      viewBox.height - viewBox.padding - normalized * (viewBox.height - viewBox.padding * 2);
    return { x, y };
  });
}
