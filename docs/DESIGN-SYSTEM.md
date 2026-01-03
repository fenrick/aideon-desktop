# Aideon Design System

## Purpose

Define how the React renderer consumes shared UI primitives and blocks. The design system is
flattened into `app/AideonDesktop/src/design-system` and is the **only** source of UI primitives for
product surfaces.

---

## Principles

- **Vanilla-first:** shadcn/ui and React Flow registry components are treated as generated sources.
- **Proxy blocks:** product UI composes blocks (panel, toolbar, sidebar, modal) rather than raw
  primitives.
- **Single source:** all renderers import from the design-system package, not from Radix/shadcn
  directly.

---

## Structure (authoritative)

```
app/AideonDesktop/src/design-system/
├── blocks/          # UI blocks (panel, toolbar, sidebar, modal)
├── components/ui/   # generated shadcn primitives + registry entries
├── components/      # React Flow UI registry proxies
├── hooks/           # shared hooks
├── styles/          # globals and theme tokens
└── index.ts         # consolidated exports
```

Generated primitives should not be edited directly. Wrap them in `blocks/` or proxy components.

---

## Tokens

Design tokens are part of the design system surface:

- CSS variables and theme defaults: `app/AideonDesktop/src/design-system/styles/globals.css`
- JS/TS helpers for spacing/layout decisions: `app/AideonDesktop/src/design-system/tokens.ts`

Product UI must not hard-code colors, radii, or spacing. Use tokens or CSS variables.

---

## Refreshing primitives

Use the design-system refresh command to sync shadcn and React Flow registries:

```
pnpm --filter @aideon/desktop run components:refresh
```

---

## Usage rules

- Import from `@aideon/design-system/*` (or local path in the desktop renderer).
- Do not import Radix/shadcn directly from product surfaces.
- Include `styles/globals.css` to pick up tokens.

---

## References

- UX contract: `docs/UX-DESIGN.md`
- Shell layout: `app/AideonDesktop/DESIGN.md`
