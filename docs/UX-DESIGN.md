# Aideon Suite — Praxis UX Design Ground Rules (M0 → M1)

## Purpose

Define UX goals, interaction patterns, and layout decisions for the Praxis desktop module within
Aideon Suite. This document guides how the canvas, sidebar, toolbars, and supporting surfaces should
behave and look, independent of specific implementation details in React or Svelte.

Date: 2025-10-28  
Status: Draft (implements M0 baseline; informs M1 scope)

> **Renderer migration:** Historic UX notes reference Svelte components. Use them as conceptual
> guidance only; new implementation work must target the React + React Flow + shadcn/ui stack.

## Goals

- Feel native and respectful on each OS while retaining a coherent visual language.
- Keep menus and window chrome platform-native where possible; keep the in-content component kit small, fast, and token-driven.
- Allow quick OS “preview” toggles in dev (Style Guide) without rebuilds.

## 1) Window & Chrome

- Menus: use platform-native menus (Tauri Menu API). No HTML top nav.
- Titlebar: default to system window chrome. Allow optional frameless+drag regions per OS in a later ADR.
- Effects: optional per-OS vibrancy/mica when available; keep it opt-in.

## 2) Component strategies and layout

- Layout: modern toolbar-driven shell with top toolbar, left sidebar, main content, and bottom status bar for connection/health and short messages.
- Toolbars: grouped icon buttons similar to ribbons. Idle state uses regular outline; active/toggled uses filled variant and picks up the accent color from tokens.
- Sidebar: tree/navigation for catalogues, meta-model, and views; React implementation should rely on shadcn sidebar block patterns (inset + nested sections + file tree) via design-system proxies.
- Content: detail panels for selected items (objects, catalogues, meta-models, visualisations, etc.).

Per-OS component strategies:

- Windows-first: Fluent UI Web Components (FAST) for in-content widgets where native feel is important.
- macOS-first: native menus + minimalist custom title area; system fonts and restrained controls; Puppertino-style CSS acceptable for in-content tweaks.
- Neutral: shadcn/ui + Tailwind utilities via the shared design system.

We will not lock into a single third-party kit; we compose small wrappers where needed.

### React/shadcn layering principles

- Use vanilla shadcn components in their default theme so upstream updates remain easy to adopt and regression surfaces stay small.
- Layer the Praxis design system on top by composing “blocks” (cards, inspectors, toolbars, command palettes) out of shadcn primitives plus our tokens, rather than forking the primitives themselves.
- Sync the primitives before editing with `pnpm --filter @aideon/desktop run components:refresh`; the script pulls the vanilla shadcn kit plus the React Flow UI registry components so we can refresh or rebase without manual edits.
- Build UX screens from those reusable blocks first, only reaching for raw shadcn elements when defining a new block; once a pattern appears twice, promote it to a block and document its props.
- Keep Tailwind utility usage scoped inside the blocks; feature code should mostly consume block variants so the renderer remains consistent across React views.
- Start from shadcn’s out-of-box blocks (e.g., Command palette, Sidebar/Nav, Dashboard cards) whenever they cover the UX layer; wrap them with Praxis tokens instead of recreating equivalent scaffolding.
- React Flow’s UI registry already provides Base Node, Node Tooltip, Node Search, Button/animated edges, and other canvas affordances—import them as-is and compose Praxis-specific wrappers (forms, toolbars, modals, sidebars, inspectors) so we respect their vanilla behaviors.
- Treat every imported primitive as a proxy: blocks now live under `app/AideonDesktop/src/design-system/src/blocks` (panels, toolbars, sidebars, modals) and React Flow wrappers live under `app/AideonDesktop/src/design-system/src/reactflow`. Wrap the vanilla component once, export it from `@aideon/design-system`, and consume those proxies everywhere so UX surfaces stay consistent.
- Track concrete block implementations and compliance status in `docs/praxis-design-system-blocks.md`; update it whenever new blocks land or existing ones change roles.

## 3) Theming & Tokens

- Tokens live in `app/AideonDesktop/src/design-system/src/styles/globals.css` and are consumed via `@aideon/design-system` wrappers; platform accents are previewed in the Praxis workspace Style Guide window.
- Primary token: `--color-accent` drives primary buttons, focus rings, selected state.
- Platform dev-preview:
  - mac: `--color-accent: #0a84ff`
  - win: `--color-accent: #0078d4`
  - linux: `--color-accent: #16a34a`
- Future: read system theme and accent where APIs permit; allow user override.

## 4) Interaction & A11y

- Keyboard: Cmd on macOS, Ctrl on Windows/Linux. Define accelerators in the native menu.
- Focus rings: use tokenized visible focus; avoid removing outlines.
- Tabs: close affordance must be keyboard-operable.
- Split panes: mouse drag; keyboard adjustment optional (arrows) where a11y rules allow.

## 5) Dev Experience

- A Style Guide window (Debug → UI Style Guide) previews tokens and controls; includes a platform toggle (auto/mac/win/linux) to preview platform accents and token overrides.
- The Style Guide does not ship to production.

## 6) Implementation Notes

- Keep renderer code free of backend logic; use typed IPC only.
- Keep per-OS tweaks purely in CSS tokens or shallow component wrappers; avoid forking components by OS.

## 7) Acceptance

- Switching the Style Guide platform toggle updates `--color-accent` live; Primary buttons, Switches and focus rings visibly change.
- Menus are native on all three OSes.
- Lint/typecheck/tests pass; no “non reactive update” warnings.
- Selecting Windows registers Fluent 2 components; selecting macOS injects Puppertino CSS; selecting Neutral injects Tailwind CSS and removes other injected styles.
