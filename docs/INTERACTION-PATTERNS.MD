# Interaction Patterns

**Scope:** Praxis Desktop template screen. Defines canonical shortcuts and interaction behaviours.

## Keyboard shortcuts (desktop)

- `Shift+Space` - focus time cursor (scenario selector).
- `Left / Right` - scrub time steps (previous/next tick) when timeline is focused.
- `Cmd/Ctrl+Z` - undo last selection/edit/time change.
- `Cmd/Ctrl+Shift+Z` or `Ctrl+Y` - redo.
- `Cmd/Ctrl+Shift+D` - toggle debug overlay (dev only).

## Panels and resizing

- Sidebar / content / inspector use `ResizablePanelGroup` with persisted widths under
  `praxis-shell-panels` (local storage).
- Collapse respects min widths (sidebar >= 14%, inspector >= 16%).

## Undo/redo model

- Command stack records selection changes, inspector edits, and time-cursor moves.
- Each command stores `redo` and `undo` functions; replay triggers the same adapter calls to keep
  UI and data in sync.

## Navigation + focus

- Selection clears when changing Scenario, Template, or Time Context.
- Inspector opens whenever selection is non-empty; properties are tab-focusable.
- Time slider commits on `onValueCommit` only; arrow keys move one tick at a time.

## Undo/redo-safe operations

- Selection changes (widget/node/edge).
- Property edits that result in `applyOperations` requests.
- Time cursor scenario/time/layer changes that refresh artefacts.
