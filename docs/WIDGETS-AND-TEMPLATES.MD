# Widgets and Templates

**Audience:** Canvas engineers adding or wiring widgets. Defines widget types, registry metadata, and how templates compose them.

## Widget types (current)

- **Graph** – XYFlow graph view; inputs: `GraphViewDefinition`; outputs: `GraphViewModel` with nodes/edges.
- **Catalogue** – tabular list; inputs: `CatalogueViewDefinition` (columns/filters); outputs: rows keyed by id.
- **Matrix** – relationship grid; inputs: `MatrixViewDefinition`; outputs: axes + cells.
- **Chart** – KPI/line/bar; inputs: `ChartViewDefinition`; outputs: series/kpi payloads.

## Registry metadata

Each widget entry defines: `type`, `label`, `icon`, `description`, `defaultSize`, and a **default view definition**. The registry lives in `canvas/widgets/registry.ts` and is the single source for creation menus and templates.

## Template assembly

- **Template** = `{ id, name, description, widgets[] }`.
- Widgets in a template reference registry types but can override titles, size, and view filters.
- Templates are stored via the host adapter; renderer can instantiate them with runtime context (scenario, `asOf`).

## Create widget flow

1. User clicks **Create widget**.
2. Library dialog lists registry entries; picking one inserts a widget into the active template with default props.
3. Newly inserted widget becomes selected → inspector opens with editable defaults.

## Rendering path

Template → `instantiateTemplate()` (injects `asOf`/scenario) → widget registry definitions → `CanvasRuntime` renders each widget card → widgets fetch data via `praxisApi`.
