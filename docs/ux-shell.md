# Praxis Scenario / Template UX Shell

**Scope:** Scenario and Template experience inside the Praxis desktop app. Updated 2025-12-08.

## Current structure (before refactor)

- **Layout:** Desktop shell uses shadcn sidebar blocks: inset sidebar + `SidebarInset` for the main surface, with an icon rail and nested sections.
- **Navigation:** Sidebar uses the shadcn sidebar block patterns (inset + nested + file tree), and aligns to the suite shell tree where applicable.
- **Workspace:** `WorkspaceTabs` renders Overview | Timeline | Canvas | Activity plus a collection of widgets generated from the active template.
- **Time controls:** `TimeCursorCard` wraps `TimeControlPanel` (branch, commit, slider) but lives in the right card stack, not a dedicated timeline section.
- **Properties:** Selection inspector card is one of many right-column cards; properties are not a distinct pane.
- **Mental model gaps:** Scenario vs Template vs Timeline are intermingled; the shell is bespoke instead of reusing the design-system desktop shell.

## Target three-pane shell

Mental model: “In a Scenario, the user selects a Template, adjusts the Time cursor (branch / commit / timeline), and inspects widgets and their properties.”

- **Left sidebar (navigation):** Inset sidebar with icon rail, nested sections, and file-tree-style scenario groups; built with shadcn `Sidebar` primitives (`Sidebar`, `SidebarGroup`, `SidebarMenu`, `SidebarMenuSub`, `SidebarRail`). Hosts scenario selection and lightweight project metadata.
- **Centre pane (primary workspace):**
  - Template header: scenario name, template name + description, template selector, primary actions (“Save template”, “Create widget”).
  - Search bar scoped to “Search branches, nodes, catalogues”.
  - Tabs (`Tabs`): `Overview | Timeline | Activity`.
  - Overview tab: `SnapshotOverviewCard` (read-only metrics) and `TimeCursorCard` (branch dropdown, commit dropdown, slider, snapshot label).
- **Right pane (Properties inspector):** Contextual properties for the current selection (widget/node/edge). Empty state: “Select a widget, node or edge to edit its properties.”
- **Visual rules:** H1 for template title, H2 for section headings, sentence-case labels, card grid alignment, primary buttons filled; secondary outline/ghost.

## Key flows

1. **Scan executive overview at a point in time**
   - Land on Overview tab.
   - Read snapshot metrics card and template header.
   - Properties pane stays in empty-state unless something is selected.
2. **Change time / branch / commit and see overview update**
   - Use `TimeCursorCard` controls: branch select → commit select → slider scrub.
   - Snapshot metrics and Overview tab refresh to the selected commit.
3. **Select widget / node / edge and edit properties**
   - Select from canvas widgets (or mock selection during transition).
   - Properties inspector switches from empty state to grouped property fields (name, data source, layout, etc.).
   - Changes dispatch via typed callbacks ready to connect to the existing selection/store APIs.

## Implementation notes

- New `PraxisShellLayout` will wrap the Scenario/Template surface with slots for navigation, centre content, and properties, built on design-system `DesktopShell` parts.
- Time cursor stays backed by `useTemporalPanel`; branch/commit/slider remain typed and testable.
- Copy strings move to a shared copy module to avoid all-caps labels and to keep accessibility labels consistent.
- Where data is missing (e.g., project grouping for scenarios), default grouping will be documented and replaceable once host data lands.
