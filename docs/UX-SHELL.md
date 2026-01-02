# Aideon Desktop Shell - Praxis Scenario / Template Surface

**Scope:** Scenario and Template experience inside the Aideon Desktop shell. Updated 2025-12-08.

## Current structure (before refactor)

- **Layout:** Desktop shell uses shadcn sidebar blocks: inset sidebar + `SidebarInset` for the main
  surface, with an icon rail and nested sections.
- **Navigation:** Sidebar uses the shadcn sidebar block patterns (inset + nested + file tree), and
  aligns to the suite shell tree where applicable.
- **Workspace:** `WorkspaceTabs` renders Overview | Timeline | Canvas | Activity plus a collection
  of widgets generated from the active template.
- **Time controls:** `TimeCursorCard` wraps `TimeControlPanel` (scenario + time + layer) but lives in
  the right card stack, not a dedicated timeline section.
- **Properties:** Selection inspector card is one of many right-column cards; properties are not a
  distinct pane.
- **Mental model gaps:** Scenario vs Template vs Timeline are intermingled; the workspace still
  behaves as if it owns its own shell instead of filling the Aideon Desktop shell slots.

## Target shell contract

Mental model: "In a Scenario, the user selects a Template, adjusts the Time cursor (valid time and
layer), and inspects widgets and their properties."

- **Shell navigation slot:** Inset sidebar with icon rail, nested sections, and file-tree-style
  scenario groups; built with design-system `Sidebar` primitives. Hosts scenario selection and
  lightweight project metadata.
- **Shell content slot:**
  - Template header: scenario name, template name + description, template selector, primary actions
    ("Save template", "Create widget").
  - Search bar scoped to "Search scenarios, elements, artefacts".
  - Tabs (`Tabs`): `Overview | Timeline | Activity`.
  - Overview tab: `SnapshotOverviewCard` (read-only metrics) and `TimeCursorCard` (scenario selector,
    valid-time selector, layer toggle, snapshot label).
- **Shell inspector slot:** Contextual properties for the current selection (widget/node/edge).
  Empty state: "Select a widget, node or edge to edit its properties."
- **Visual rules:** H1 for template title, H2 for section headings, sentence-case labels, card grid
  alignment, primary buttons filled; secondary outline/ghost.

## Key flows

1. **Scan executive overview at a point in time**
   - Land on Overview tab.
   - Read snapshot metrics card and template header.
   - Properties pane stays in empty-state unless something is selected.
2. **Change time and see overview update**
   - Use `TimeCursorCard` controls: scenario select -> valid-time adjust -> layer toggle.
   - Snapshot metrics and Overview tab refresh to the selected time context.
3. **Select widget / node / edge and edit properties**
   - Select from canvas widgets (or mock selection during transition).
   - Properties inspector switches from empty state to grouped property fields (name, data source,
     layout, etc.).
   - Changes dispatch via typed callbacks ready to connect to the existing selection/store APIs.

## Implementation notes

- `AideonDesktopShell` owns layout and chrome; Praxis provides slot components via its
  `WorkspaceModule` registration (navigation, toolbar, content, inspector).
- Shell slot sizing, toolbar composition, and navigation constraints are defined in
  `app/AideonDesktop/DESIGN.md`.
- Time cursor stays backed by `useTemporalPanel`; scenario/time/layer controls remain typed and
  testable.
- Copy strings move to a shared copy module to avoid all-caps labels and to keep accessibility
  labels consistent.
- Where data is missing (e.g., project grouping for scenarios), default grouping will be
  documented and replaceable once host data lands.
