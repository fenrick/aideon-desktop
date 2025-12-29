'use client';

import { useEffect, useMemo, useRef, useState, type ReactElement, type ReactNode } from 'react';

import { AideonDesktopRoot } from '@/root';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SplashScreen as PraxisSplashScreen } from '../components/splash/splash-screen';
import { Badge } from '../design-system/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../design-system/components/ui/card';
import { RadioGroup, RadioGroupItem } from '../design-system/components/ui/radio-group';
import { useColorTheme } from '../design-system/theme/color-theme';
import { isTauriRuntime } from '../lib/runtime';

/**
 * Root screen for the main desktop window.
 */
export function MainScreen() {
  const isTauri = isTauriRuntime();
  const windowLabel = useMemo(() => {
    if (!isTauri) {
      return;
    }
    try {
      return getCurrentWindow().label;
    } catch {
      return;
    }
  }, [isTauri]);

  const shouldSignalFrontendReady = isTauri && windowLabel === 'main';

  return (
    <FrontendReady enabled={shouldSignalFrontendReady}>
      <AideonDesktopRoot />
    </FrontendReady>
  );
}

/**
 * Splash screen displayed while the host initializes.
 */
export function SplashScreenRoute() {
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
 * Minimal host status window.
 */
export function StatusScreen() {
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
 * About dialog content for the desktop shell.
 */
export function AboutScreen() {
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
 * Settings window for theme selection and preferences.
 */
export function SettingsScreen() {
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
 * Placeholder UI styleguide window.
 */
export function StyleguideScreen() {
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
 * Signals the host once the main window has rendered.
 * @param root0 - Component props.
 * @param root0.children - Content to render.
 * @param root0.enabled - Whether the signal should be emitted.
 */
export function FrontendReady({
  children,
  enabled = true,
}: {
  readonly children: ReactNode;
  readonly enabled?: boolean;
}): ReactElement | null {
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
 * Render a compact preview of theme tokens.
 * @param root0 - Preview props.
 * @param root0.themeId - Theme identifier.
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
