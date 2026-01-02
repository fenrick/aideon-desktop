# Aideon Design System

## Purpose

Describe the structure and role of the Aideon Design System module (now flattened inside
`app/AideonDesktop/src/design-system`): how shadcn/ui and React Flow primitives are wrapped into
shared blocks, how tokens are managed, and how React-based renderers in Aideon Suite are expected to
consume them.

## Purpose

Centralize all vanilla shadcn/ui primitives, React Flow UI registry components, and Praxis-specific
proxy blocks so every renderer (Praxis workspace, legacy desktop, future apps) consumes the same
layered design system.

## Structure

The Tailwind tokens now live inside `src/styles/globals.css`; we removed the separate
`tailwind.config.ts`, so the CLI relies purely on that stylesheet instead of a dedicated config file.

```
app/AideonDesktop/src/design-system/
├── blocks/*              # reusable blocks (panel, toolbar, sidebar, modal)
├── components/ui/*       # shadcn CLI output (raw primitives + registry adapters)
├── hooks/*               # shared hooks (e.g., `useIsMobile`)
├── ui/*                  # wrapped/aggregated exports consumed by renderers
├── components/*          # React Flow UI registry components + Praxis proxies
├── lib/utils.ts          # shared utility helpers
├── styles/globals.css    # base CSS variables (import from consumers plus Tailwind tokens)
└── index.ts              # aggregated exports
```

The CLI targets `src/components/ui` via `components.json`, so treat that folder as generated output
and wrap its primitives inside `src/ui/*` or `src/blocks/*` before shipping.

## Refreshing components

Always run the refresh script inside the design-system package so every consumer stays aligned with
vanilla shadcn/reactflow:

```
pnpm --filter @aideon/desktop run components:refresh
```

This pulls the default shadcn primitives plus the React Flow UI registry entries (Base Node, Node
Tooltip, Node Search, Animated SVG Edge, etc.). Do not edit the generated files directly; wrap them
in proxy components under `src/reactflow` or `src/blocks` if you need design-system behavior.

## Proxy + block layers

- `src/blocks/panel.tsx` exposes the standard card/form stack (`Panel`, `PanelHeader`,
  `PanelField`, `PanelToolbar`) used by Praxis components and future apps.
- `src/blocks/modal.tsx` wraps shadcn `Dialog` so command palette, exports, and confirmations share
  the same chrome.
- `src/blocks/toolbar.tsx` defines toolbar rows/sections/separators for canvas controls.
- `src/blocks/sidebar.tsx` defines the shell/headings/nav sections for inspectors and navigation.
- New block concepts should be added here (forms, toolbars, modals, sidebars, inspectors, etc.) and
  then consumed downstream through `src/design-system/blocks/*`. Keep them vanilla-first (only
  compose the primitives) so refreshing registries remains a one-line command.

## Using the design system

- Add `src/design-system` as a workspace dependency and import via the documented subpaths
  (e.g., `src/design-system/ui/button`, `src/design-system/blocks/panel`,
  `src/design-system/reactflow/praxis-node`).
- Include `src/design-system/styles/globals.css` (or copy its CSS variables) in the renderer’s global CSS
  so the tokens match.
- Tailwind consumers must include `../AideonDesktop/src/design-system/**/*.{ts,tsx}` in their `content` globs
  so class names from the shared components are discovered.
- Renderer-specific helpers (`@/components/blocks/*`) should depend on the design system instead of
  duplicating shadcn components.
