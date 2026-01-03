export type WidgetSize = 'full' | 'half';

export interface CanvasWidgetLayout {
  id: string;
  title?: string;
  size?: WidgetSize;
  position?: { x: number; y: number };
  dimensions?: { width: number; height: number };
}

export interface WidgetSelection {
  widgetId: string;
  nodeIds: string[];
  edgeIds: string[];
}

export interface SelectionState {
  sourceWidgetId?: string;
  nodeIds: string[];
  edgeIds: string[];
}

export const EMPTY_SELECTION: SelectionState = {
  nodeIds: [],
  edgeIds: [],
};
