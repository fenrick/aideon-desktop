# Design Tokens

**Location:** `app/AideonDesktop/src/design-system/tokens.ts` and `src/styles.css` for CSS variables.

## Scales

- **Spacing:** `0.5 1 1.5 2 3 4 6 8 10 12 16` (rem-based; exported as `space.xs` … `space.3xl`).
- **Radius:** `xs=6px`, `sm=10px`, `md=14px`, `lg=18px`, `pill=999px`.
- **Typography:** `brand` stack → "Space Grotesk", fallback to system; `mono` → "JetBrains Mono".
- **Elevation:** `elevations.100|200|300` map to subtle shadow presets used by cards/dialogs.

## Color tokens (light theme)

Defined as CSS variables under `:root`:

- `--background` `210 33% 98%`
- `--foreground` `224 71% 4%`
- `--primary` `222 89% 60%`
- `--secondary` `218 33% 94%`
- `--accent` `221 89% 63%`
- `--muted` `214 32% 91%`
- `--destructive` `0 84% 60%`
- `--radius` `0.95rem`

Dark mode tokens can reuse the same file; keep semantic names and avoid hard-coded hexes in components.

## Usage

- Import `tokens` from `design-system/tokens` for JS/TS (layout + sizing decisions).
- Prefer CSS variables in components for colors/border radius; avoid ad-hoc inline colors.
- When adding new UI, reference tokens before introducing new values; update this doc if scales change.
