# Template / Scenario Screen Contract

**Audience:** Praxis Desktop contributors wiring the Scenario / Template experience. Updated 2025-12-08.

## Component map

- `PraxisShellLayout` - three-pane shell (left navigation, centre workspace, right properties).
  Wraps the Scenario view and owns sizing via design-system `ResizablePanelGroup`.
- `ProjectsSidebar` - left pane list of projects and scenarios using shadcn `Sidebar` + `ScrollArea`.
- `TemplateHeader` - centre-pane header showing Scenario + Template names, description, template
  selector, and primary actions (`Save template`, `Create widget`).
- `ScenarioSearchBar` - scoped search input ("Search scenarios, elements, artefacts") sitting under
  the header.
- `OverviewTabs` - `Tabs` with `Overview | Timeline | Activity`. The Overview tab hosts:
  - `SnapshotOverviewCard` - read-only metrics (nodes, edges, integrity, scenario).
  - `TimeCursorCard` - scenario select, valid-time selector, layer toggle, snapshot label.
- `PropertiesInspector` - right-pane inspector rendering contextual fields per selection kind
  (`widget | node | edge | none`), with an empty state call-to-action.

## Data + props

- Scenario data still comes from `listScenarios()`; until project metadata arrives, scenarios appear
  under a default "Projects" bucket.
- Time controls rely on `useTemporalPanel()` state/actions; `TimeCursorCard` accepts
  `{ state, actions, copy }`.
- Template selection uses existing `CanvasTemplate` models; callbacks are passed down from the
  Scenario page to update active template and trigger saves.
- Selection -> Inspector: `PropertiesInspector` accepts `selectionKind`, optional `selectionId`, and
  future property bags; the Scenario view is responsible for passing selection updates from
  widgets/store.

## Interaction flows (developer view)

1. **Load page:** `PraxisShellLayout` requests scenarios, populates `ProjectsSidebar`, sets default
   Scenario + Template.
2. **Change time:** `TimeCursorCard` dispatches `selectScenario`, `selectValidTime`, `selectLayer`,
   `refreshTimeBounds`.
3. **Select item:** Widgets raise `selection` updates; the page maps them to `selectionKind` + ids
   before rendering `PropertiesInspector`.

## Accessibility + copy

- All interactive elements use accessible labels from a shared `copy/templateScreen.ts` module
  (sentence case, no ALL CAPS).
- Tabs are keyboard focusable; select + slider components expose names/aria-labels for screen
  readers.
- Empty states describe the expected action ("Select a widget, node or edge to edit its
  properties.").

## Adding widgets or inspector sections

- Add new cards to the Overview tab inside `OverviewTabs` using `Card`/`CardContent`/`CardHeader`.
- Extend `PropertiesInspector` with additional sections by adding typed props (e.g.,
  `widgetProperties`) and rendering grouped `Field` components; keep sections small and
  well-labelled.
- Prefer composing new UI with design-system primitives first (`Card`, `Tabs`, `Button`, `Select`,
  `Slider`, `ScrollArea`) before adding bespoke styles.

## State matrix (Loading / Empty / Populated / Error)

| Area      | Loading                               | Empty                                          | Populated                                         | Error                                      |
| --------- | ------------------------------------- | ---------------------------------------------- | ------------------------------------------------- | ------------------------------------------ |
| Sidebar   | Skeleton lines for projects/scenarios | "No projects yet" with CTA to create/import    | Projects -> scenarios list, active highlights     | Inline banner + Retry button               |
| Centre    | Card skeletons for header + widgets   | "No template selected" with quick-create chips | Widgets grid rendered from template metadata      | Card-level alert with retry                |
| Inspector | Placeholder text "Select something"   | Same as Loading (disabled controls)            | Inputs bound to selection properties + save/reset | Red text + keep controls enabled for retry |
