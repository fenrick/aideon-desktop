import type { CanvasWidgetLayout } from './types';

const COLUMN_WIDTH = 650;
const ROW_HEIGHT_FULL = 850;
const ROW_HEIGHT_HALF = 550;
const GUTTER = 40;
const COLS = 2;

interface LayoutState {
  colIndex: number;
  leftColumnY: number;
  rightColumnY: number;
}

/**
 * Places a widget in the next available slot and advances the running layout state.
 * @param widget
 * @param state
 */
function placeWidget(widget: CanvasWidgetLayout, state: LayoutState) {
  const col = state.colIndex % COLS;
  const currentY = col === 0 ? state.leftColumnY : state.rightColumnY;

  const x = 100 + col * (COLUMN_WIDTH + GUTTER);
  const y = currentY;

  const height = widget.size === 'half' ? ROW_HEIGHT_HALF : ROW_HEIGHT_FULL;
  const nextY = currentY + height + GUTTER;
  const isLeftColumn = col === 0;

  const nextState: LayoutState = {
    colIndex: state.colIndex + 1,
    leftColumnY: isLeftColumn ? nextY : state.leftColumnY,
    rightColumnY: isLeftColumn ? state.rightColumnY : nextY,
  };

  return { position: { x, y }, nextState };
}

/**
 * Computes deterministic initial widget positions for any widgets that do not
 * already provide a `position`.
 * @param widgets
 */
export function calculateInitialLayout(
  widgets: CanvasWidgetLayout[],
): Record<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();
  // Start with some top padding.
  let state: LayoutState = {
    colIndex: 0,
    leftColumnY: 100,
    rightColumnY: 100,
  };

  for (const widget of widgets) {
    if (widget.position) {
      positions.set(widget.id, widget.position);
    } else {
      const { position, nextState } = placeWidget(widget, state);
      positions.set(widget.id, position);
      state = nextState;
    }
  }

  return Object.fromEntries(positions) as Record<string, { x: number; y: number }>;
}
