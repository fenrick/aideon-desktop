## Stage 0 – Baseline and assumptions

Before starting, the agent should assume:

- Tauri + React + shadcn/ui are already wired in the repo.
- `app/PraxisCanvas` exists and currently behaves like a standalone app (has its own header/layout).
- `app/AideonDesktop/src/design-system` (or similar) exists and is where shadcn components are proxied.

If any of these are missing, the agent should:

- Record that in `AGENTS.md` “Assumptions and gaps” section.
- Only then proceed to create missing packages as part of Stage 1–2.

---

## Stage 1 – Define the UX shell in documentation

**Status:** ✅ Completed 2025-11-30.

### 1.1 Document Aideon Desktop as the primary UX layer

**Docs**

1. ✅ Create `app/AideonDesktop/DESIGN.md` (if it does not exist) with:
   - Purpose: Aideon Desktop is the main UX shell for the desktop product.
   - Regions:
     - Top toolbar (global actions, workspace switches).
     - Left tree (projects/workspaces/nodes).
     - Centre workspace (PraxisCanvas now, others later).
     - Right properties panel (contextual details for selection).

   - Principles:
     - All workspaces render inside this shell.
     - No other app has its own sidebar/header chrome.

2. ✅ Update `AGENTS.md`:
   - Add a section “UX Shell (Aideon Desktop)”:
     - “When building or modifying application-level layout, target Aideon Desktop, not individual workspaces.”
     - “Use shadcn Sidebar + Resizable + Menubar primitives; do not implement custom layout primitives.”

**Tests**

- No tests yet; this stage is documentation only.

**Code**

- No code changes in this stage.

**Definition of done**

- A new agent can read `app/AideonDesktop/DESIGN.md` + `AGENTS.md` and understand:
  - Aideon Desktop is the host shell.
  - Other UX modules (PraxisCanvas, future tools) live inside it.

---

## Stage 2 – Install shadcn primitives and layout contracts

**Status:** ✅ Completed 2025-11-30.

### 2.1 Decide and document the core UI primitives

**Docs**

1. ✅ Update `app/AideonDesktop/src/design-system/DESIGN.md`:
   - Add “Desktop shell primitives” section listing:
     - `Sidebar` (navigation + tree).
     - `Resizable` (pane splitting).
     - `Menubar` or `NavigationMenu` + `Toolbar` (top bar).
     - `ScrollArea`, `Card`, `Form` for properties panel content.

   - Clarify: these are the only primitives to use for app-level layout.

2. ✅ Update `AGENTS.md`:
   - Under coding standards for React:
     - “For app shell: always use design system proxies for Sidebar, Resizable, Menubar, Toolbar, not raw third-party imports.”

### 2.2 Install shadcn components and add DS proxies

**Code**

1. ✅ Use shadcn CLI to add components (actual commands depend on your setup, but conceptually):
   - `sidebar`, `resizable`, `menubar`, `scroll-area`, `card`, `form`.

2. ✅ In `app/AideonDesktop/src/design-system/src` create proxied components, e.g.:

   ```tsx
   // app/AideonDesktop/src/design-system/src/desktop-shell/sidebar.tsx
   export {
     Sidebar,
     SidebarProvider,
     SidebarInset,
     SidebarTrigger,
     // …etc
   } from '@/components/ui/sidebar';
   ```

   Do similar for `Resizable` and `Menubar`.

3. ✅ Add a placeholder `DesktopShellLayout` type definition (no implementation yet), just the prop contract:

   ```tsx
   export type DesktopShellSlots = {
     tree: React.ReactNode;
     toolbar: React.ReactNode;
     main: React.ReactNode;
     properties: React.ReactNode;
   };
   ```

**Tests**

- Add a minimal test to ensure design system proxies compile and export correctly (simple type-level or smoke test if needed).

**Definition of done**

- Design system exposes the primitives the shell will use.
- No workspace code is using raw shadcn imports for sidebar/menubar/resizable.

---

## Stage 3 – Implement a bare `DesktopShell` layout (no real content)

**Status:** ✅ Completed 2025-11-30.

### 3.1 Implement the shell structure as a pure layout component

**Docs**

1. ✅ Extend `app/AideonDesktop/DESIGN.md` with:
   - A simple ASCII/diagram of layout:
     - `[Menubar/Toolbar]`
     - `[Sidebar][Main][Properties]` with resizable separators.

   - A short explanation of `DesktopShell` slots: `tree`, `toolbar`, `main`, `properties`.

**Code**

2. ✅ In the design system, implement `DesktopShell`:

   ```tsx
   // app/AideonDesktop/src/design-system/src/desktop-shell/DesktopShell.tsx
   import {
     SidebarProvider,
     Sidebar,
     SidebarInset,
     // ...
   } from './sidebar-proxy';
   import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from './resizable-proxy';

   export function DesktopShell({ tree, toolbar, main, properties }: DesktopShellSlots) {
     return (
       <SidebarProvider>
         {/* Top toolbar */}
         <header>{toolbar}</header>

         {/* Three resizable panels */}
         <ResizablePanelGroup direction="horizontal">
           <ResizablePanel defaultSize={20} minSize={10}>
             <Sidebar>{tree}</Sidebar>
           </ResizablePanel>

           <ResizableHandle />

           <ResizablePanel defaultSize={60} minSize={40}>
             {main}
           </ResizablePanel>

           <ResizableHandle />

           <ResizablePanel defaultSize={20} minSize={10}>
             {properties}
           </ResizablePanel>
         </ResizablePanelGroup>
       </SidebarProvider>
     );
   }
   ```

   For now, `tree`, `toolbar`, `main`, `properties` can be simple `<div>TODO</div>` from the caller.

### 3.2 Add basic tests

**Tests**

- ✅ In `app/AideonDesktop/src/design-system` tests:
  - Render `DesktopShell` with dummy slot content (e.g. “Tree”, “Toolbar”, etc.).
  - Assert that:
    - It renders without crashing.
    - All four regions appear in the DOM.
    - Default sizes don’t throw, if your test environment can check them.

**Definition of done**

- `DesktopShell` exists, compiles, and can render dummy content.
- It uses only design system proxies, not raw third-party imports.

---

## Stage 4 – Create the Aideon Desktop app entry bound to Tauri

**Status:** ✅ Completed 2025-11-30.

### 4.1 Wire a new React entrypoint

**Docs**

1. ✅ Update `app/AideonDesktop/DESIGN.md`:
   - Describe the main entry component, e.g. `AideonDesktopRoot`.
   - Clarify: Tauri points at Aideon Desktop index.html; PraxisCanvas no longer owns the window.

**Code**

2. ✅ In `app/AideonDesktop/src` create:

   ```tsx
   // app/AideonDesktop/src/root.tsx
   import { DesktopShell } from 'app/AideonDesktop/src/design-system';

   export function AideonDesktopRoot() {
     return (
       <DesktopShell
         toolbar={<div>Toolbar</div>}
         tree={<div>Tree</div>}
         main={<div>Main workspace placeholder</div>}
         properties={<div>Properties</div>}
       />
     );
   }
   ```

3. ✅ Hook this into your bundler/entrypoint (e.g. `main.tsx`) and ensure Tauri’s `tauri.conf.json` points to the new app HTML/JS entry.

### 4.2 Smoke tests

**Tests**

- ✅ Add a high-level React test for `AideonDesktopRoot`:
  - Verifies that the placeholder content renders inside the expected DOM structure.

- Optionally, add a very simple Tauri smoke test or script that boots the app and asserts the main window loads (depending on your existing approach).

**Definition of done**

- Running the desktop app brings up the Aideon Desktop shell with dummy content.
- PraxisCanvas is not yet integrated, but the shell is real and visible.

---

## Stage 5 – Extract a pure `PraxisCanvasSurface` and plug into centre panel

**Status:** ✅ Completed 2025-11-30.

### 5.1 Refactor PraxisCanvas to separate chrome from content

**Docs**

1. ✅ Update `app/PraxisCanvas/DESIGN.md`:
   - Clarify that PraxisCanvas now exposes a **surface component** without global chrome:
     - `PraxisCanvasSurface` = canvas view, timelines, nodes, etc.

   - Note any legacy app-level layout in PraxisCanvas as deprecated: “To be removed once all usage is via Aideon Desktop.”

**Code**

2. ✅ In `app/PraxisCanvas/src`:
   - Extract the current main view into `PraxisCanvasSurface`:
     - Remove any top-level headers/sidebars from this component.
     - Ensure it consumes layout from parent (fills available area).

   - If PraxisCanvas previously had its own `App` with header/sidebar:
     - Keep it temporarily as `LegacyPraxisCanvasApp` for backwards compatibility, but mark it as legacy.

3. ✅ In `AideonDesktopRoot`, replace the main placeholder with PraxisCanvas:

   ```tsx
   import { PraxisCanvasSurface } from 'app/PraxisCanvas';

   export function AideonDesktopRoot() {
     return (
       <DesktopShell
         toolbar={<div>Toolbar</div>}
         tree={<div>Tree</div>}
         main={<PraxisCanvasSurface />}
         properties={<div>Properties</div>}
       />
     );
   }
   ```

### 5.2 Tests

**Tests**

- ✅ PraxisCanvas:
  - Add/update tests for `PraxisCanvasSurface` to ensure it renders correctly in a generic container.

- ✅ AideonDesktop:
  - Update root tests to assert that PraxisCanvas-specific DOM appears in the main panel.

**Definition of done**

- The live app shows Aideon Desktop shell with a real PraxisCanvas in the centre.
- Old PraxisCanvas chrome is effectively unused in the new path and clearly marked as legacy.

---

## Stage 6 – Implement the left tree panel (stub → real data)

**Status:** ✅ Completed 2025-12-01.

### 6.1 Stubbed tree using shadcn tree/sidebar

**Docs**

1. Extend `app/AideonDesktop/DESIGN.md`:
   - Describe the tree’s purpose (projects/workspaces/nodes).
   - Specify that it uses the design system’s tree view (from shadcn) inside the Sidebar.

**Code**

2. ✅ Add a `DesktopTree` component in Aideon Desktop:

   ```tsx
   // app/AideonDesktop/src/DesktopTree.tsx
   export function DesktopTree() {
     // For now: hard-coded nodes
     const items = [
       { id: "project-1", label: "Project 1", children: [...] },
       // ...
     ];
     return <TreeComponent items={items} />;
   }
   ```

3. ✅ Replace the `tree` slot in `AideonDesktopRoot` with `<DesktopTree />`.

### 6.2 Wire to real domain data

**Code (second pass)**

4. ✅ Wire to real data:
   - Tree now uses `listScenarios` from the Praxis adapters (via `@aideon/PraxisCanvas`) and maps scenarios to workspace entries under a Scenarios project group.
   - Added `useWorkspaceTree` hook with loading/error handling.
   - `DesktopTree` renders loading skeletons, error state, and empty state.

**Tests**

- ✅ `DesktopTree` tests mock `listScenarios` to verify loaded items and error display.

**Definition of done**

- Left panel shows a working tree driven by host data (scenarios as workspaces).
- No other part of the app implements its own project/workspace tree.

---

## Stage 7 – Implement the right properties panel (selection-driven)

**Status:** ✅ Basic selection plumbing completed 2025-11-30.

### 7.1 Basic selection + properties contract

**Docs**

1. Update `app/PraxisCanvas/DESIGN.md`:
   - Define how selection state is exposed:
     - e.g. a `useSelection` hook, or a context that Aideon Desktop can consume.

   - Clarify the shape of selected entities (nodes, edges, timelines).

2. Update `app/AideonDesktop/DESIGN.md`:
   - Define the contract:
     - Aideon Desktop subscribes to selection from PraxisCanvas.
     - It passes selection into a `PropertiesPanel` component.

**Code**

3. ✅ In PraxisCanvas: expose selection via `onSelectionChange` on `PraxisCanvasSurface`.

4. ✅ In Aideon Desktop:
   - `DesktopPropertiesPanel` renders selection summary and empty state.
   - Shell keeps selection state and passes it down.

### 7.2 Tests

**Tests**

- ✅ `DesktopPropertiesPanel` tests cover empty and selected states; root test mocks selection
  propagation.

**Definition of done**

- Selecting something in PraxisCanvas updates the right properties panel.
- The properties layout uses shadcn `Card`, `Form`, `ScrollArea` from the design system.

---

## Stage 8 – Clean-up and harden the pattern

**Status:** ✅ Completed 2025-12-01 (docs and examples added; legacy chrome kept only for legacy route).

**Docs**

- Update `AGENTS.md` “Examples” section with:
  - Pointers to `DesktopShell`, `AideonDesktopRoot`, `DesktopTree`, `DesktopPropertiesPanel`, and `PraxisCanvasSurface` as golden patterns.

- Mark any old workspace-specific headers/sidebars as deprecated and note target refactors.

**Tests**

- Ensure coverage thresholds remain satisfied with the new code.
- Add any missing tests for edge cases (resizing panels, very large trees, empty workspaces).

**Code**

- Remove unused old layout components from PraxisCanvas and any other apps now fully hosted inside Aideon Desktop.
- Verify all new code paths use the design system proxies, not raw third-party components.
