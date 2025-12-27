import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  type ReactNode,
} from 'react';

export type ColorThemeId = 'corp-blue' | 'claude' | 'graphite' | 'green' | 'violet';

export interface ColorThemeOption {
  id: ColorThemeId;
  label: string;
  description: string;
  source?: string;
}

const DEFAULT_COLOR_THEME: ColorThemeId = 'corp-blue';
const STORAGE_KEY = 'aideon.colorTheme';

const THEME_OPTIONS: ColorThemeOption[] = [
  {
    id: 'corp-blue',
    label: 'Corporate Blue',
    description: 'Default Aideon palette.',
    source: 'Aideon',
  },
  {
    id: 'violet',
    label: 'Violet',
    description: 'Bright violet accents with crisp neutrals.',
    source: 'shadcn examples',
  },
  {
    id: 'green',
    label: 'Green',
    description: 'Fresh emerald accents with cool neutrals.',
    source: 'shadcn examples',
  },
  {
    id: 'claude',
    label: 'Claude',
    description: 'Warm clay tones with an amber primary.',
    source: 'shadcn examples',
  },
  {
    id: 'graphite',
    label: 'Graphite',
    description: 'Low-saturation graphite accents.',
    source: 'shadcn examples',
  },
];

const loadedThemes = new Set<ColorThemeId>();

/**
 * Dynamically import a theme stylesheet.
 * @param themeId - Theme identifier to load.
 */
async function loadThemeStyles(themeId: ColorThemeId) {
  switch (themeId) {
    case 'corp-blue': {
      return;
    }
    case 'claude': {
      await import('../styles/themes/claude.css');
      return;
    }
    case 'graphite': {
      await import('../styles/themes/graphite.css');
      return;
    }
    case 'green': {
      await import('../styles/themes/green.css');
      return;
    }
    case 'violet': {
      await import('../styles/themes/violet.css');
      return;
    }
    default: {
      return;
    }
  }
}

/**
 * Ensure a theme stylesheet is loaded once.
 * @param themeId - Theme identifier to load.
 */
async function ensureThemeStyles(themeId: ColorThemeId) {
  if (themeId === DEFAULT_COLOR_THEME) {
    return;
  }
  if (loadedThemes.has(themeId)) {
    return;
  }
  await loadThemeStyles(themeId);
  loadedThemes.add(themeId);
}

/**
 * Preload all theme styles to avoid flash on first change.
 */
async function preloadAllThemeStyles() {
  await Promise.all(
    THEME_OPTIONS.filter((theme) => theme.id !== DEFAULT_COLOR_THEME).map((theme) =>
      ensureThemeStyles(theme.id),
    ),
  );
}

/**
 * Read the persisted theme from local storage.
 */
function resolveStoredTheme(): ColorThemeId {
  try {
    const stored = globalThis.window.localStorage.getItem(STORAGE_KEY);
    if (stored && THEME_OPTIONS.some((theme) => theme.id === stored)) {
      return stored as ColorThemeId;
    }
  } catch {
    return DEFAULT_COLOR_THEME;
  }
  return DEFAULT_COLOR_THEME;
}

/**
 * Persist the selected theme for later sessions.
 * @param themeId - Theme identifier to persist.
 */
function persistTheme(themeId: ColorThemeId) {
  try {
    globalThis.window.localStorage.setItem(STORAGE_KEY, themeId);
  } catch {
    // Ignore persistence failures (e.g., private mode)
  }
}

/**
 * Apply the theme selector attribute to the document root.
 * @param themeId - Theme identifier to apply.
 */
function applyThemeAttribute(themeId: ColorThemeId) {
  const root = document.documentElement;
  if (themeId === DEFAULT_COLOR_THEME) {
    delete root.dataset.colorTheme;
    return;
  }
  root.dataset.colorTheme = themeId;
}

interface ColorThemeContextValue {
  colorTheme: ColorThemeId;
  setColorTheme: (themeId: ColorThemeId) => void;
  options: ColorThemeOption[];
  preloadThemes: () => Promise<void>;
}

const ColorThemeContext = createContext<ColorThemeContextValue | undefined>(undefined);

/**
 * Provide runtime color theme selection and persistence.
 * @param root0 - Provider props.
 * @param root0.children - Children to render.
 */
export function ColorThemeProvider({ children }: { readonly children: ReactNode }) {
  const resolvedTheme = resolveStoredTheme();
  const [colorTheme, setColorThemeState] = useReducer(
    (_state: ColorThemeId, next: ColorThemeId) => next,
    resolvedTheme,
  );

  useEffect(() => {
    const controller = new AbortController();

    void (async () => {
      await ensureThemeStyles(colorTheme);
      if (controller.signal.aborted) {
        return;
      }
      applyThemeAttribute(colorTheme);
      persistTheme(colorTheme);
    })();

    return () => {
      controller.abort();
    };
  }, [colorTheme]);

  const setColorTheme = useCallback((themeId: ColorThemeId) => {
    const resolved = THEME_OPTIONS.some((theme) => theme.id === themeId)
      ? themeId
      : DEFAULT_COLOR_THEME;
    setColorThemeState(resolved);
  }, []);

  const contextValue = useMemo<ColorThemeContextValue>(
    () => ({
      colorTheme,
      setColorTheme,
      options: THEME_OPTIONS,
      preloadThemes: preloadAllThemeStyles,
    }),
    [colorTheme, setColorTheme],
  );

  return <ColorThemeContext.Provider value={contextValue}>{children}</ColorThemeContext.Provider>;
}

/**
 * Access color theme state and controls.
 */
export function useColorTheme() {
  const context = useContext(ColorThemeContext);
  if (!context) {
    throw new Error('useColorTheme must be used within ColorThemeProvider');
  }
  return context;
}
