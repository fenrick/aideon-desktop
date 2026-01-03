// Central design tokens used across the Aideon desktop shell.
// Keep values in sync with CSS variables defined in src/styles.css.

export const tokens = {
  space: {
    xs: 4,
    sm: 8,
    md: 12,
    lg: 16,
    xl: 24,
    '2xl': 32,
    '3xl': 40,
    '4xl': 48,
  },
  radius: {
    xs: 6,
    sm: 8,
    md: 10,
    lg: 12,
    xl: 16,
    pill: 999,
  },
  typography: {
    heading: "'Space Grotesk', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif",
    brand: "'Space Grotesk', 'Inter', system-ui, -apple-system, 'Segoe UI', sans-serif",
    mono: "'JetBrains Mono', 'SFMono-Regular', Menlo, monospace",
  },
  elevations: {
    100: '0 4px 12px rgba(15, 23, 42, 0.08)',
    200: '0 8px 20px rgba(15, 23, 42, 0.10)',
    300: '0 16px 40px rgba(15, 23, 42, 0.12)',
  },
};

export type TokenKeys = keyof typeof tokens;
