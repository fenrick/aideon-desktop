import type { WidgetSize } from 'aideon/canvas/types';
import type {
  CatalogueViewDefinition,
  ChartViewDefinition,
  GraphViewDefinition,
  MatrixViewDefinition,
} from 'praxis/praxis-api';
import type { PraxisCanvasWidget } from 'praxis/types';

type GraphTemplateView = Omit<GraphViewDefinition, 'asOf' | 'scenario'>;
type CatalogueTemplateView = Omit<CatalogueViewDefinition, 'asOf' | 'scenario'>;
type MatrixTemplateView = Omit<MatrixViewDefinition, 'asOf' | 'scenario'>;
type ChartTemplateView = Omit<ChartViewDefinition, 'asOf' | 'scenario'>;

interface TemplateWidgetBase {
  id: string;
  title: string;
  size?: WidgetSize;
}

export type TemplateWidgetConfig =
  | (TemplateWidgetBase & { kind: 'graph'; view: GraphTemplateView })
  | (TemplateWidgetBase & { kind: 'catalogue'; view: CatalogueTemplateView })
  | (TemplateWidgetBase & { kind: 'matrix'; view: MatrixTemplateView })
  | (TemplateWidgetBase & { kind: 'chart'; view: ChartTemplateView });

export interface CanvasTemplate {
  id: string;
  name: string;
  description: string;
  widgets: TemplateWidgetConfig[];
}

export interface TemplateContext {
  scenario?: string;
  asOf?: string;
}

/**
 * Instantiate a canvas template for a specific runtime context.
 * @param template - Template definition to materialize.
 * @param context - Runtime context to apply (scenario, as-of).
 */
export function instantiateTemplate(
  template: CanvasTemplate,
  context: TemplateContext,
): PraxisCanvasWidget[] {
  const asOf = context.asOf ?? new Date().toISOString();
  return template.widgets.map((widget) => {
    if (widget.kind === 'graph') {
      return {
        ...widget,
        view: { ...widget.view, asOf, scenario: context.scenario },
      } satisfies PraxisCanvasWidget;
    }
    if (widget.kind === 'catalogue') {
      return {
        ...widget,
        view: { ...widget.view, asOf, scenario: context.scenario },
      } satisfies PraxisCanvasWidget;
    }
    if (widget.kind === 'matrix') {
      return {
        ...widget,
        view: { ...widget.view, asOf, scenario: context.scenario },
      } satisfies PraxisCanvasWidget;
    }
    return {
      ...widget,
      view: { ...widget.view, asOf, scenario: context.scenario },
    } satisfies PraxisCanvasWidget;
  });
}

/**
 * Capture a template from runtime widgets.
 * @param name - Display name for the template.
 * @param description - Summary shown in template pickers.
 * @param widgets - Runtime widgets to capture.
 */
export function captureTemplateFromWidgets(
  name: string,
  description: string,
  widgets: PraxisCanvasWidget[],
): CanvasTemplate {
  return {
    id: `custom-${Date.now().toString(36)}`,
    name,
    description,
    widgets: widgets.map((widget) => convertWidgetToTemplate(widget)),
  };
}

/**
 * Convert a runtime widget into its template-safe shape.
 * @param widget - Runtime widget to convert.
 */
function convertWidgetToTemplate(widget: PraxisCanvasWidget): TemplateWidgetConfig {
  const { size, title, id } = widget;
  if (widget.kind === 'graph') {
    return { id, title, size, kind: 'graph', view: withoutRuntimeFields(widget.view) };
  }
  if (widget.kind === 'catalogue') {
    return { id, title, size, kind: 'catalogue', view: withoutRuntimeFields(widget.view) };
  }
  if (widget.kind === 'matrix') {
    return { id, title, size, kind: 'matrix', view: withoutRuntimeFields(widget.view) };
  }
  return { id, title, size, kind: 'chart', view: withoutRuntimeFields(widget.view) };
}

/**
 * Strip runtime-only fields from a view definition.
 * @param view - View definition including runtime fields.
 */
function withoutRuntimeFields<T extends { asOf: string; scenario?: string }>(
  view: T,
): Omit<T, 'asOf' | 'scenario'> {
  const entries = Object.entries(view).filter(([key]) => key !== 'asOf' && key !== 'scenario');
  return Object.fromEntries(entries) as Omit<T, 'asOf' | 'scenario'>;
}

const GRAPH_OVERVIEW: GraphTemplateView = {
  id: 'executive-overview',
  name: 'Executive Overview',
  kind: 'graph',
  filters: {
    nodeTypes: ['Capability', 'Application'],
    edgeTypes: ['depends_on', 'supports'],
  },
};

const CATALOGUE_BASE: CatalogueTemplateView = {
  id: 'capability-catalogue',
  name: 'Capability Catalogue',
  kind: 'catalogue',
  columns: [
    { id: 'name', label: 'Name', type: 'string' },
    { id: 'owner', label: 'Owner', type: 'string' },
    { id: 'state', label: 'State', type: 'string' },
  ],
};

const MATRIX_BASE: MatrixTemplateView = {
  id: 'capability-to-service',
  name: 'Capability ↔ Service Matrix',
  kind: 'matrix',
  rowType: 'Capability',
  columnType: 'Service',
  relationship: 'depends_on',
};

const KPI_CHART: ChartTemplateView = {
  id: 'kpi-operational',
  name: 'Operational KPIs',
  kind: 'chart',
  chartType: 'kpi',
  measure: 'Operational readiness',
};

const LINE_CHART: ChartTemplateView = {
  id: 'velocity-line',
  name: 'Velocity trend',
  kind: 'chart',
  chartType: 'line',
  measure: 'Velocity',
};

const BAR_CHART: ChartTemplateView = {
  id: 'heatmap-bar',
  name: 'Capability maturity',
  kind: 'chart',
  chartType: 'bar',
  measure: 'Maturity',
};

export const BUILT_IN_TEMPLATES: CanvasTemplate[] = [
  {
    id: 'template-executive',
    name: 'Executive overview',
    description: 'Graph + KPI + catalogue snapshot for leadership reviews.',
    widgets: [
      {
        id: 'graph-overview',
        kind: 'graph',
        title: 'Twin overview graph',
        size: 'full',
        view: GRAPH_OVERVIEW,
      },
      {
        id: 'kpi-services',
        kind: 'chart',
        title: 'Critical services KPI',
        size: 'half',
        view: KPI_CHART,
      },
      {
        id: 'velocity-line',
        kind: 'chart',
        title: 'Velocity trend',
        size: 'half',
        view: LINE_CHART,
      },
      {
        id: 'catalogue-primary',
        kind: 'catalogue',
        title: 'Capability catalogue',
        size: 'full',
        view: CATALOGUE_BASE,
      },
    ],
  },
  {
    id: 'template-explorer',
    name: 'Explorer workspace',
    description: 'Graph, matrix, and comparative chart for deeper analysis.',
    widgets: [
      {
        id: 'graph-explorer',
        kind: 'graph',
        title: 'Explorer graph',
        size: 'full',
        view: GRAPH_OVERVIEW,
      },
      {
        id: 'matrix-coverage',
        kind: 'matrix',
        title: 'Capability ↔ Service coverage',
        size: 'half',
        view: MATRIX_BASE,
      },
      {
        id: 'maturity-bars',
        kind: 'chart',
        title: 'Capability maturity',
        size: 'half',
        view: BAR_CHART,
      },
      {
        id: 'catalogue-explorer',
        kind: 'catalogue',
        title: 'Capability rollup',
        size: 'full',
        view: CATALOGUE_BASE,
      },
    ],
  },
];
