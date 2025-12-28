# Aideon Design System – Internal Design

_Flatten note (December 2025): this design system now lives inside `app/AideonDesktop/src/design-system`
instead of a standalone workspace. Import directly from the `src/design-system` tree (no package
aliases)._

## Overview

The Aideon Design System centralises UI primitives and blocks for React-based modules. It wraps
shadcn/ui and React Flow UI registry components and exposes a small, opinionated set of blocks and
tokens to keep renderers consistent.

## Internal structure

- `src/components/ui`: generated shadcn/ui and React Flow primitives (treated as read-only).
- `src/ui`: thin wrappers and convenience exports.
- `src/blocks`: higher-level blocks (panel, sidebar, toolbar, modal, etc.).
- `src/reactflow`: React Flow wrapper components for Praxis-specific nodes/edges.
- `src/styles/globals.css`: CSS variables and Tailwind tokens shared by all consumers.

## Theme tokens (shadcn CSS variables)

Theme tokens live in `src/design-system/styles/globals.css` and follow the shadcn CSS variable
model. These variables are the single source of truth for color theming, and Tailwind utilities
resolve through the `@theme inline` mappings.

Core tokens (light + dark):

- `--background`, `--foreground`
- `--card`, `--card-foreground`
- `--popover`, `--popover-foreground`
- `--primary`, `--primary-foreground`
- `--secondary`, `--secondary-foreground`
- `--muted`, `--muted-foreground`
- `--accent`, `--accent-foreground`
- `--destructive`, `--destructive-foreground`
- `--border`, `--input`, `--ring`
- `--chart-1` through `--chart-5`
- Sidebar tokens: `--sidebar`, `--sidebar-foreground`, `--sidebar-primary`,
  `--sidebar-primary-foreground`, `--sidebar-accent`, `--sidebar-accent-foreground`,
  `--sidebar-border`, `--sidebar-ring`

Branding:

- Corporate blue is defined as `--primary` (and `--sidebar-primary`) with the paired foreground
  tokens (`--primary-foreground`, `--sidebar-primary-foreground`).
- Avoid hard-coded color utility classes in product code. Use semantic utilities
  (`bg-primary`, `text-foreground`, `border-border`, `bg-sidebar`, etc.) so themes can be swapped
  without refactors.
- Maintain the background/foreground pairing convention (`bg-primary` + `text-primary-foreground`,
  `bg-card` + `text-card-foreground`) to align with shadcn styling expectations.

Tailwind v4 mapping:

- All color tokens must be exposed via `@theme inline` to keep utilities consistent.
- Add new color tokens by defining CSS variables in `:root`/`.dark`, then mapping them to
  `--color-*` in `@theme inline`. Do not edit `components/ui` for theming.

## Color theme switching

- Color themes are defined in `src/design-system/styles/themes/*.css` as overrides for the core
  shadcn token set.
- The runtime applies a `data-color-theme` attribute to the document root; selectors in the theme
  CSS apply on `:root[data-color-theme=...]` and on descendant previews.
- Theme CSS is lazily loaded via dynamic import in `ColorThemeProvider`, then persisted to
  `localStorage` (key `aideon.colorTheme`). Default is `corp-blue`.
- Theme palettes `claude`, `graphite`, `green`, and `violet` are sourced from shadcn example themes
  and are kept as override files to make `components:refresh` safe.

## Desktop shell primitives

For application-level layout, use only the proxied shadcn primitives exposed by the design system:

- `Sidebar` (navigation + tree container).
- `SidebarInset` + `SidebarTrigger` (main content area + sidebar toggle).
- `Resizable` (pane splitting between main workspace + properties panel).
- `Menubar` or `NavigationMenu` + `Toolbar` (top bar composition).
- `ScrollArea`, `Card`, `Form` for properties panel content.

No other primitives should be used for the app shell; keep layout composition consistent across modules.

### Shadcn-aligned composition

- Follow the shadcn Sidebar layout: `SidebarProvider` wraps the shell; the workspace `Sidebar` sits
  as a sibling of `SidebarInset`; `SidebarTrigger` lives in the main header.
- Desktop shell headers mirror the shadcn sidebar blocks: trigger + vertical separator, with the
  toolbar slot occupying the remaining header space. Keep the collapsible-height transition on
  the header (`group-has-data-[collapsible=icon]/sidebar-wrapper:h-12`) to match sidebar block
  behaviour.
- Avoid editing generated sidebar component code. Instead, align layout and structure to the
  documented slots and data attributes, and keep custom styling in app-level blocks.
- When refreshing components, use `pnpm run components:refresh` to pull upstream shadcn defaults
  without local forks, then adjust usage in app/layout code as needed.

### Wrapped components

- We wrap shadcn primitives only when we need consistent variants or composition (e.g., `Panel` in
  `blocks/panel`, React Flow node/edge wrappers). Most usage should import directly from
  `src/design-system/components/ui/*`.
- New UI should **prefer existing primitives** (`Button`, `Badge`, `Select`, `ToggleGroup`,
  `ScrollArea`, React Flow wrappers) over creating bespoke component trees. Add a wrapper only when
  multiple features would share the exact composition.

## Data model and APIs

- Exposes React components and CSS only; no business logic or IPC.
- Provides stable components under `src/design-system/ui/*` and `src/design-system/blocks/*`.

## Interactions

- Consumed by Praxis workspace (and future React apps) as the single source of UI primitives/blocks.
- Refreshed via the `components:refresh` script, which updates generated components in
  `src/components/ui` from `components.json`.

## Constraints and invariants

- Generated components must not be edited directly; all customisation happens in wrappers/blocks.
- Tokens remain centralised in `globals.css` to avoid drift between apps.
- Keep `src/design-system/lib/utils.ts` as a compatibility shim for shadcn overwrites.
