import type {
  CatalogueViewDefinition,
  ChartViewDefinition,
  GraphViewDefinition,
  GraphViewModel,
  MatrixViewDefinition,
} from 'praxis/praxis-api';

import type { CanvasWidgetLayout } from 'aideon/canvas/types';

export type PraxisWidgetKind = 'graph' | 'catalogue' | 'matrix' | 'chart';

interface BaseWidgetConfig<TView> extends CanvasWidgetLayout {
  title: string;
  view: TView;
}

export interface PraxisGraphWidgetConfig extends BaseWidgetConfig<GraphViewDefinition> {
  kind: 'graph';
}

export interface PraxisCatalogueWidgetConfig extends BaseWidgetConfig<CatalogueViewDefinition> {
  kind: 'catalogue';
}

export interface PraxisMatrixWidgetConfig extends BaseWidgetConfig<MatrixViewDefinition> {
  kind: 'matrix';
}

export interface PraxisChartWidgetConfig extends BaseWidgetConfig<ChartViewDefinition> {
  kind: 'chart';
}

export type PraxisCanvasWidget =
  | PraxisGraphWidgetConfig
  | PraxisCatalogueWidgetConfig
  | PraxisMatrixWidgetConfig
  | PraxisChartWidgetConfig;

export interface PraxisWidgetViewEvent {
  widgetId: string;
  view: GraphViewModel;
}

export interface PraxisWidgetErrorEvent {
  widgetId: string;
  message: string;
}

export type { SelectionState, WidgetSelection } from 'aideon/canvas/types';
