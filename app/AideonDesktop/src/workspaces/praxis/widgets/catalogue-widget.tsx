import { useCallback, useEffect, useMemo, useState } from 'react';

import { AlertTriangle } from 'lucide-react';

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from 'design-system/components/ui/table';
import { toErrorMessage } from 'praxis/lib/errors';
import { getCatalogueView, type CatalogueRow, type CatalogueViewModel } from 'praxis/praxis-api';

import type {
  PraxisCatalogueWidgetConfig as CatalogueWidgetConfig,
  SelectionState,
  WidgetSelection,
} from 'praxis/types';
import { WidgetToolbar } from './widget-toolbar';

interface CatalogueWidgetProperties {
  readonly widget: CatalogueWidgetConfig;
  readonly reloadVersion: number;
  readonly selection?: SelectionState;
  readonly onSelectionChange?: (selection: WidgetSelection) => void;
}

/**
 *
 * @param root0
 * @param root0.widget
 * @param root0.reloadVersion
 * @param root0.selection
 * @param root0.onSelectionChange
 */
export function CatalogueWidget({
  widget,
  reloadVersion,
  selection,
  onSelectionChange,
}: CatalogueWidgetProperties) {
  const [model, setModel] = useState<CatalogueViewModel | undefined>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();

  const definition = useMemo(() => widget.view, [widget.view]);

  const loadView = useCallback(async () => {
    setLoading(true);
    setError(undefined);
    try {
      const view = await getCatalogueView(definition);
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

  const handleRowActivate = useCallback(
    (row: CatalogueRow) => {
      onSelectionChange?.({ widgetId: widget.id, nodeIds: [row.id], edgeIds: [] });
    },
    [onSelectionChange, widget.id],
  );

  const selectedNodes = selection?.nodeIds ?? [];
  const activeRowIds = new Set(selectedNodes);

  let body: React.ReactNode = <Placeholder message="Loading catalogue..." />;
  if (error) {
    body = <ErrorMessage message={error} />;
  } else if (model) {
    body = (
      <CatalogueTable
        rows={model.rows}
        columns={model.columns}
        loading={loading}
        activeRowIds={activeRowIds}
        onRowActivate={handleRowActivate}
      />
    );
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
      <div className="flex-1 rounded-2xl border border-border/60 bg-background/40 p-3">{body}</div>
    </div>
  );
}

interface CatalogueTableProperties {
  readonly rows: CatalogueRow[];
  readonly columns: CatalogueViewModel['columns'];
  readonly activeRowIds: Set<string>;
  readonly loading: boolean;
  readonly onRowActivate: (row: CatalogueRow) => void;
}

/**
 *
 * @param root0
 * @param root0.rows
 * @param root0.columns
 * @param root0.activeRowIds
 * @param root0.loading
 * @param root0.onRowActivate
 */
function CatalogueTable({
  rows,
  columns,
  activeRowIds,
  loading,
  onRowActivate,
}: CatalogueTableProperties) {
  if (rows.length === 0) {
    return (
      <Placeholder message={loading ? 'Loading catalogue...' : 'No entities match this view'} />
    );
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {columns.map((column) => (
            <TableHead key={column.id}>{column.label}</TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => {
          const isSelected = activeRowIds.has(row.id);
          return (
            <TableRow
              key={row.id}
              data-state={isSelected ? 'selected' : undefined}
              data-testid={`catalogue-row-${row.id}`}
              className="cursor-pointer"
              onClick={() => {
                onRowActivate(row);
              }}
            >
              {columns.map((column) => (
                <TableCell key={`${row.id}-${column.id}`} className="text-sm">
                  <button
                    type="button"
                    className="w-full rounded-md px-2 py-1 text-left transition hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    data-testid={`catalogue-cell-${row.id}-${column.id}`}
                    onClick={() => {
                      onRowActivate(row);
                    }}
                  >
                    {formatValue(row.values[column.id])}
                  </button>
                </TableCell>
              ))}
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}

/**
 *
 * @param value
 */
function formatValue(value: string | number | boolean | null | undefined): string {
  if (typeof value === 'boolean') {
    return value ? 'Yes' : 'No';
  }
  if (value === null || value === undefined) {
    return '—';
  }
  return String(value);
}

/**
 *
 * @param root0
 * @param root0.message
 */
function Placeholder({ message }: { readonly message: string }) {
  return <p className="text-sm text-muted-foreground">{message}</p>;
}

/**
 *
 * @param root0
 * @param root0.message
 */
function ErrorMessage({ message }: { readonly message: string }) {
  return (
    <p className="flex items-center gap-2 text-sm text-destructive">
      <AlertTriangle className="h-4 w-4" />
      {message}
    </p>
  );
}
