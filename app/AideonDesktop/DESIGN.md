# Aideon Desktop – UX Shell Design

## Purpose

- Aideon Desktop is the primary UX shell for the desktop product. All workspaces (Praxis workspace and future tools) render inside this shell instead of shipping their own chrome.
- The shell owns global navigation, window/tool switching, and layout scaffolding; individual workspaces supply workspace-specific navigation/toolbar/content/inspector components.

## Layout regions

- **Top toolbar:** global actions, workspace switching, and app-level menus plus a workspace toolbar slot.
- **Left navigation:** workspace-provided navigation (projects/scenarios in Praxis today).
- **Centre workspace:** active workspace content (Praxis initially).
- **Right inspector:** workspace-provided contextual details and forms.

## Principles

- Every workspace renders inside the shell; no separate sidebars or headers per workspace.
- Use design-system primitives for all shell structure (Sidebar, Resizable, Menubar/Toolbar). Do not introduce ad‑hoc layout components.
- Keep the shell local-first and Tauri-friendly: no renderer HTTP, typed IPC only.
- Desktop keyboard shortcuts should be registered in the Tauri native menu (accelerators) and dispatched to the renderer; browser preview keeps lightweight fallback handlers.

## Tree and properties panels

- Left navigation is provided by each workspace module (Praxis uses the projects/scenarios sidebar).
- Right inspector is provided by the workspace module (Praxis uses the properties inspector).

## Shell layout contract

The shell is defined by a small set of slots that callers fill:

- `navigation` – workspace navigation.
- `toolbar` – top toolbar content (Aideon global chrome + workspace toolbar slot).
- `content` – workspace surface.
- `inspector` – contextual inspector/details.

Layout sketch:

```
[ Toolbar / Menubar ]
[ Sidebar ][ Main workspace ][ Properties ]
```

The implementation uses the design-system proxies for Sidebar, Resizable, and Menubar/Toolbar components. Default sizing keeps the sidebar and properties panels narrow (≈20%) with the main workspace as the dominant pane.

## Entry point

- `AideonDesktopRoot` is the React entry that selects the active workspace module and composes `AideonDesktopShell` with navigation/toolbar/content/inspector slots.
- Workspace modules are registered in `src/workspaces/registry.ts` and follow the `WorkspaceModule` contract in `src/workspaces/types.ts`.
- Tauri loads the static Next.js export (`app/AideonDesktop/out`) with window routes mapped to `app/*/page.tsx` and the shared screen logic in `src/app/app-screens.tsx`; workspace modules mount inside the shell rather than owning the window.

## Next.js static export constraints

- The renderer is built with `output: "export"`; all screens must be renderable at build time (no request-time SSR).
- Do not use `getServerSideProps` or `next start`-only features in this app. Use static data or IPC-driven client effects instead.
- Route Handlers are allowed only for static `GET` responses that are emitted during `next build`.
- Client Components are pre-rendered during `next build`; browser-only APIs (`window`, `localStorage`, etc.) must be accessed inside client effects.

## App Router stability

- The App Router (`app/`) is the stable default as of Next.js 13.4; no `appDir` flag is required.
- Treat App Router as the canonical model for layouts, routing, and data boundaries in the renderer.

## Style guide (dev)

- A small UI Style Guide exists at `/styleguide` to showcase shell/design-system primitives during UX iteration (including Replit browser preview).
- Desktop builds can open it from the native menu (Debug → UI Style Guide) or from the command palette when running a development build.
