# Selection and Editing

**Scope:** Praxis Desktop (React canvas). Describes what can be selected, how it behaves, and how edits flow to the domain model.

## What is selectable

- **Widgets** – any card on the canvas surface. Selecting a widget focuses its defaults in the inspector.
- **Nodes** – graph entities surfaced by Graph, Matrix, or Catalogue widgets.
- **Edges** – relationships shown in Graph widgets.

## Rules

- **Single primary selection** at all times; multi-select is allowed for Nodes/Edges but the first item is the editable primary.
- **Source-aware** – selection tracks the originating widget id to keep edits contextual.
- **Reset on context change** – switching Scenario, Template, or Commit clears selection to avoid stale edits.

## Flow

1. Widget emits `{ widgetId, nodeIds[], edgeIds[] }` → selection store normalises + dedupes.
2. Selection store exposes derived `kind` (`widget|node|edge|none`) and `primaryId`.
3. Inspector renders fields for the current kind. Only editable when a primary id exists.
4. Saving in the inspector dispatches typed operations (e.g., `updateNode`) back through the adapters; success triggers a widget refresh.
5. Undo/redo replays selection and edit commands from the command stack.

## Inspector expectations

- Show the primary id and friendly label if available.
- Provide fields for name/label, data source, and layout hints; disable when nothing is selected.
- Display validation/host errors inline; keep actions (`Save`, `Reset`) keyboard accessible.

## Visual affordances

- Hover: muted highlight across widgets; Selected: accent border + background.
- Graph: XYFlow selection state drives stroke + halo; Matrix/Catalogue rows use `data-state="selected"` with consistent tokens.
- Inspector header always states what is selected (Widget/Node/Edge/None).
