# Aideon Design System (flattened)

This module used to live in `app/AideonDesignSystem`. All React UI primitives, shadcn wrappers,
and React Flow proxies are now co-located inside the Aideon Desktop package at
`app/AideonDesktop/src/design-system`. Import directly from that tree (no package aliases).

## Purpose

Centralises shadcn/ui primitives, React Flow registry components, and Praxis-specific proxy blocks
so React renderers share a consistent look and feel and token set.

## Responsibilities

- Provide design-system wrappers (ui, blocks, desktop shell) for the desktop app.
- Expose React Flow nodes/edges used by the Praxis canvas.
- Keep tokens in `src/design-system/styles/globals.css`.

## Notes after flattening

- Do **not** recreate a separate workspace; add/modify components directly under
  `src/design-system/*`.
- Legacy registry files from `app/AideonDesignSystem` were moved with history into the new path.
- Guard script `tools/design-system-guard.mjs` prevents stray UI under `src/lib/ui`.
