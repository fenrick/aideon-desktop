import { EMPTY_SELECTION, type SelectionState, type WidgetSelection } from './types';

/**
 * Remove duplicates and blank values from a list of selection identifiers.
 * @param ids raw identifiers supplied by a widget.
 * @returns unique, truthy ids in insertion order.
 */
export function dedupeIds(ids: readonly string[]): string[] {
  return [...new Set(ids.filter((value): value is string => Boolean(value.trim())))];
}

/**
 * Convert a widget selection payload into the canonical SelectionState shape.
 * @param selection optional widget-level selection.
 * @returns normalized selection with empty arrays when nothing is selected.
 */
export function fromWidgetSelection(selection?: WidgetSelection): SelectionState {
  if (!selection) {
    return EMPTY_SELECTION;
  }
  return {
    sourceWidgetId: selection.widgetId,
    nodeIds: dedupeIds(selection.nodeIds),
    edgeIds: dedupeIds(selection.edgeIds),
  };
}

/**
 * Determine whether a selection carries any node or edge identifiers.
 * @param selection selection to inspect.
 * @returns true when the selection is empty or undefined.
 */
export function isSelectionEmpty(selection?: SelectionState): boolean {
  if (!selection) {
    return true;
  }
  return selection.nodeIds.length === 0 && selection.edgeIds.length === 0;
}

/**
 * Calculate node/edge counts for the current selection.
 * @param selection selection to inspect.
 * @returns object containing node and edge counts (defaults to zero).
 */
export function selectionCounts(selection?: SelectionState): { nodes: number; edges: number } {
  return {
    nodes: selection?.nodeIds.length ?? 0,
    edges: selection?.edgeIds.length ?? 0,
  };
}

/**
 * Build a short human-readable summary of the current selection.
 * @param selection selection to summarise.
 * @returns sentence fragment describing counts (e.g., "2 nodes, 1 edge").
 */
export function selectionSummary(selection?: SelectionState): string {
  if (isSelectionEmpty(selection)) {
    return 'No selection';
  }
  const { nodes, edges } = selectionCounts(selection);
  const parts: string[] = [];
  if (nodes > 0) {
    parts.push(`${nodes.toString()} ${nodes === 1 ? 'node' : 'nodes'}`);
  }
  if (edges > 0) {
    parts.push(`${edges.toString()} ${edges === 1 ? 'edge' : 'edges'}`);
  }
  return parts.join(', ');
}
