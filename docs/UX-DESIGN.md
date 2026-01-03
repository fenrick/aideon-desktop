# Aideon Suite - UX Contract

## Purpose

Define the **UX contract** for rendering Praxis artefacts in the desktop shell. This document
specifies interaction semantics and layout expectations, not pixel-level design.

---

## Artefacts as primary outputs

The UI renders **artefact results** produced by Praxis:

- Views
- Catalogues
- Matrices
- Maps
- Reports and Pages

All artefacts execute with explicit **time + scenario** context. The UI must surface these
parameters and pass them on every request.

---

## Diagram model

Artefact results include **diagram specs** that the UI renders without semantic inference:

- Visual nodes and edges
- Containers/groups
- Overlays and legends
- Layout hints (layered, hierarchy, swimlane, matrix)

---

## Core interaction rules

- **Selection is global**: node/edge/cell selection drives inspector and actions.
- **Drill-down is universal**: every artefact supports selection -> explain -> task.
- **Bounded results**: large artefacts return partial results with explicit warnings.

### Selection model (required)

- Selection kinds: `node`, `edge`, `cell`, `artefact`, `none`.
- Single primary selection; multi-select is allowed but one item is primary.
- Selection includes the originating artefact/widget id for context.

### Editing flow (required)

1. Selection updates the global store.
2. Inspector renders fields for the selection kind.
3. Save triggers a Praxis task via adapters (no direct graph mutation).
4. Artefact caches are invalidated and re-run as needed.

---

## Time and scenario UX

- Time controls are always visible in the primary workspace.
- UI must show valid time, layer, and optional scenario.
- Time changes trigger re-execution or cache invalidation, not ad-hoc UI mutation.

---

## Layout contract (desktop shell)

The shell owns chrome and layout. Workspaces fill slots:

- Navigation
- Toolbar
- Content surface
- Inspector

See `app/AideonDesktop/DESIGN.md` for the shell layout contract.

---

## Accessibility and performance

- Keyboard navigation for artefacts and inspector.
- No color-only meaning; overlays must include legends.
- Virtualize large tables; use level-of-detail for diagrams.
