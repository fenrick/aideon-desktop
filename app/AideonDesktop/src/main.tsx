import { Fragment, StrictMode, useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';

import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeProvider } from 'next-themes';
import { SplashScreen as PraxisSplashScreen } from './components/splash/splash-screen';
import { Badge } from './design-system/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from './design-system/components/ui/card';
import { RadioGroup, RadioGroupItem } from './design-system/components/ui/radio-group';
import { Toaster } from './design-system/components/ui/sonner';
import { ColorThemeProvider, useColorTheme } from './design-system/theme/color-theme';
import { ErrorBoundary } from './error-boundary';
import { AideonDesktopRoot } from './root';
import './styles.css';

const isVitest = Boolean((import.meta as { env?: { VITEST?: boolean } }).env?.VITEST);
if (!isVitest) {
  const container = document.querySelector('#root');
  if (!container) {
    throw new Error('Unable to locate root element');
  }

  const useStrictMode = !isTauriRuntime();
  const RootWrapper = useStrictMode ? StrictMode : Fragment;

  createRoot(container).render(
    <RootWrapper>
      <ThemeProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange>
        <ColorThemeProvider>
          <ErrorBoundary>
            <>
              <AppEntry />
              <Toaster />
            </>
          </ErrorBoundary>
        </ColorThemeProvider>
      </ThemeProvider>
    </RootWrapper>,
  );
}

/**
 *
 */
export function AppEntry() {
  const [windowLabel, setWindowLabel] = useState<string | undefined>();
  const isTauri = isTauriRuntime();

  useEffect(() => {
    queueMicrotask(() => {
      if (!isTauri) {
        setWindowLabel(undefined);
        return;
      }
      try {
        const currentWindow = getCurrentWindow();
        setWindowLabel(currentWindow.label);
      } catch {
        setWindowLabel(undefined);
      }
    });
  }, [isTauri]);

  const hashPath = globalThis.location.hash.replace(/^#/, '').replace(/\/$/, '') || '/';
  const route = isTauri ? (windowLabel ?? 'splash') : hashPath;
  const wantsSplash = route === 'splash' || route === '/splash';
  // Only bypass splash automatically in browser mode when it wasn't explicitly requested in hash.
  const normalizedRoute = !isTauri && wantsSplash && hashPath === '/splash' ? '/' : route;

  let view: React.ReactNode = <AideonDesktopRoot />;
  switch (normalizedRoute) {
    case 'splash':
    case '/splash': {
      view = <SplashRoute />;

      break;
    }
    case 'status':
    case '/status': {
      view = <StatusScreen />;

      break;
    }
    case 'about':
    case '/about': {
      view = <AboutScreen />;

      break;
    }
    case 'settings':
    case '/settings': {
      view = <SettingsScreen />;

      break;
    }
    case 'styleguide':
    case '/styleguide': {
      view = <StyleguideScreen />;

      break;
    }
    // No default
  }

  const shouldSignalFrontendReady = isTauri && windowLabel === 'main';
  return <FrontendReady enabled={shouldSignalFrontendReady}>{view}</FrontendReady>;
}

/**
 *
 * @param root0
 * @param root0.children
 * @param root0.enabled
 */
export function FrontendReady({
  children,
  enabled = true,
}: {
  readonly children: React.ReactNode;
  readonly enabled?: boolean;
}): React.ReactElement | null {
  const didSignal = useRef(false);
  useEffect(() => {
    if (!enabled || didSignal.current) {
      return;
    }
    if (!isTauriRuntime()) {
      return;
    }
    didSignal.current = true;

    invoke('set_complete', { task: 'frontend' })
      .then(() => true)
      .catch(() => false);
  }, [enabled]);
  return children as React.ReactElement | null;
}

/**
 * Detect whether the code is executing inside a Tauri runtime.
 * Uses both the optional globals and the compile-time env flags so it works
 * when `withGlobalTauri` is disabled (default in this repo).
 */
export function isTauriRuntime(): boolean {
  const metaEnvironment = (import.meta as { env?: { TAURI_PLATFORM?: string } }).env;
  if (metaEnvironment?.TAURI_PLATFORM) {
    return true;
  }

  const global = globalThis as { __TAURI__?: unknown; __TAURI_INTERNALS__?: unknown };
  return Boolean(global.__TAURI__ ?? global.__TAURI_INTERNALS__);
}

/**
 *
 */
function SplashRoute() {
  const loadLines = useMemo(
    () => [
      'Reticulating splines…',
      'Weaving twin orbits…',
      'Replaying future states…',
      'Cooling hot paths…',
      'Aligning decision matrices…',
      'Seeding knowledge graph…',
      'Collapsing branches to present…',
      'Normalising capability models…',
      'Hardening isolation layer…',
      'Bootstrapping sidecar…',
      'Calibrating maturity plateaus…',
      'Scheduling time-dimension renders…',
    ],
    [],
  );

  const [currentLine, setCurrentLine] = useState<string>(loadLines[0] ?? '');

  // Rotate status lines
  useEffect(() => {
    let ix = 0;
    const interval = setInterval(() => {
      setCurrentLine(loadLines[ix % loadLines.length] ?? '');
      ix += 1;
    }, 800);
    return () => {
      clearInterval(interval);
    };
  }, [loadLines]);

  return <PraxisSplashScreen line={currentLine} />;
}

/**
 *
 */
function StatusScreen() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <div className="rounded-lg border border-border/70 bg-card/90 px-6 py-4 shadow-md">
        <p className="text-sm font-medium">Host status</p>
        <p className="text-xs text-muted-foreground">All services initialising…</p>
      </div>
    </div>
  );
}

/**
 *
 */
function AboutScreen() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <div className="space-y-2 rounded-lg border border-border/60 bg-card/90 px-6 py-5 shadow">
        <h1 className="text-lg font-semibold">Aideon</h1>
        <p className="text-sm text-muted-foreground">
          Desktop shell for Praxis workspace and tools.
        </p>
      </div>
    </div>
  );
}

/**
 *
 */
function SettingsScreen() {
  const { colorTheme, options, preloadThemes, setColorTheme } = useColorTheme();

  useEffect(() => {
    preloadThemes().catch(() => false);
  }, [preloadThemes]);

  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <Card className="w-full max-w-2xl">
        <CardHeader>
          <CardTitle>Settings</CardTitle>
          <CardDescription>Personalize the desktop experience.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-3">
            <div>
              <p className="text-sm font-semibold">Color theme</p>
              <p className="text-xs text-muted-foreground">
                Choose the primary color palette for the UI. Changes persist automatically.
              </p>
            </div>
            <RadioGroup
              value={colorTheme}
              onValueChange={(value) => {
                setColorTheme(value as typeof colorTheme);
              }}
              className="space-y-3"
            >
              {options.map((option) => (
                <label
                  key={option.id}
                  htmlFor={`color-theme-${option.id}`}
                  className="flex cursor-pointer items-center gap-4 rounded-lg border border-border/60 bg-card/80 p-4 transition hover:bg-muted/40"
                >
                  <RadioGroupItem value={option.id} id={`color-theme-${option.id}`} />
                  <div className="flex-1 space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{option.label}</span>
                      {option.id === 'corp-blue' ? (
                        <Badge variant="secondary">Default</Badge>
                      ) : undefined}
                      {option.source ? <Badge variant="outline">{option.source}</Badge> : undefined}
                    </div>
                    <p className="text-xs text-muted-foreground">{option.description}</p>
                  </div>
                  <ThemePreview themeId={option.id} />
                </label>
              ))}
            </RadioGroup>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

/**
 *
 */
function StyleguideScreen() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background text-foreground">
      <div className="space-y-3 rounded-lg border border-border/60 bg-card/90 px-6 py-5 shadow">
        <h1 className="text-lg font-semibold">Styleguide</h1>
        <p className="text-sm text-muted-foreground">Design system documentation pending.</p>
      </div>
    </div>
  );
}

/**
 * Render a compact preview of theme tokens.
 * @param root0 - Preview props.
 * @param root0.themeId - Theme identifier to preview.
 */
function ThemePreview({ themeId }: { readonly themeId: string }) {
  const dataTheme = themeId === 'corp-blue' ? undefined : themeId;
  return (
    <div
      data-color-theme={dataTheme}
      className="grid grid-cols-3 gap-1 rounded-md border border-border/60 bg-background p-2 text-foreground"
    >
      <span className="h-3 w-3 rounded-full bg-primary" />
      <span className="h-3 w-3 rounded-full bg-accent" />
      <span className="h-3 w-3 rounded-full bg-muted" />
    </div>
  );
}
