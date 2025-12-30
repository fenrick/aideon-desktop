import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { useEffect } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  ColorThemeProvider,
  useColorTheme,
  type ColorThemeId,
} from 'design-system/theme/color-theme';

/**
 *
 * @param root0
 * @param root0.onReady
 */
function ThemeProbe({
  onReady,
}: {
  readonly onReady: (context: ReturnType<typeof useColorTheme>) => void;
}) {
  const context = useColorTheme();
  useEffect(() => {
    onReady(context);
  }, [context, onReady]);
  return <div data-testid="theme">{context.colorTheme}</div>;
}

describe('color theme provider', () => {
  const storage = new Map<string, string>();
  const localStorageMock = {
    getItem(key: string) {
      return storage.get(key) ?? undefined;
    },
    setItem(key: string, value: string) {
      storage.set(key, value);
    },
    removeItem(key: string) {
      storage.delete(key);
    },
    clear() {
      storage.clear();
    },
  };

  beforeEach(() => {
    storage.clear();
    Object.defineProperty(globalThis, 'localStorage', {
      value: localStorageMock,
      configurable: true,
    });
  });

  afterEach(() => {
    cleanup();
    localStorageMock.clear();
    delete document.documentElement.dataset.colorTheme;
  });

  it('defaults to corp-blue when no stored theme exists', async () => {
    let context: ReturnType<typeof useColorTheme> | undefined;

    render(
      <ColorThemeProvider>
        <ThemeProbe
          onReady={(value) => {
            context = value;
          }}
        />
      </ColorThemeProvider>,
    );

    await waitFor(() => {
      expect(context?.colorTheme).toBe('corp-blue');
    });
    expect(screen.getByTestId('theme')).toHaveTextContent('corp-blue');
    await waitFor(() => {
      expect(document.documentElement.dataset.colorTheme).toBeUndefined();
    });
  });

  it('hydrates the stored theme and applies dataset', async () => {
    localStorage.setItem('aideon.colorTheme', 'green');
    let context: ReturnType<typeof useColorTheme> | undefined;

    render(
      <ColorThemeProvider>
        <ThemeProbe
          onReady={(value) => {
            context = value;
          }}
        />
      </ColorThemeProvider>,
    );

    await waitFor(() => {
      expect(context?.colorTheme).toBe('green');
    });
    await waitFor(() => {
      expect(document.documentElement.dataset.colorTheme).toBe('green');
    });
  });

  it('falls back to default theme when unknown id is set', async () => {
    let context: ReturnType<typeof useColorTheme> | undefined;

    render(
      <ColorThemeProvider>
        <ThemeProbe
          onReady={(value) => {
            context = value;
          }}
        />
      </ColorThemeProvider>,
    );

    await waitFor(() => {
      expect(context).toBeDefined();
    });

    act(() => {
      context?.setColorTheme('unknown-theme' as ColorThemeId);
    });

    await waitFor(() => {
      expect(screen.getByTestId('theme')).toHaveTextContent('corp-blue');
      expect(localStorage.getItem('aideon.colorTheme')).toBe('corp-blue');
    });
  });

  it('preloads available themes without error', async () => {
    let context: ReturnType<typeof useColorTheme> | undefined;

    render(
      <ColorThemeProvider>
        <ThemeProbe
          onReady={(value) => {
            context = value;
          }}
        />
      </ColorThemeProvider>,
    );

    await waitFor(() => {
      expect(context).toBeDefined();
    });

    const preloadPromise = context?.preloadThemes();
    if (!preloadPromise) {
      throw new Error('Expected preload promise to be available.');
    }
    await expect(preloadPromise).resolves.toBeUndefined();
  });
});
